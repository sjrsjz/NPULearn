use aibackend::interface::AIChat;
use history_msg::history::{load_history, save_history};
use history_msg::history::{ChatHistory, ChatMessage, ChatMessageType};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, Window};

use tauri_plugin_fs::FsExt;

mod document_renderer;

mod aibackend;
mod setting;

mod history_msg;

// 定义一个全局状态来存储聊天历史
static CHAT_HISTORY: Lazy<Mutex<HashMap<u32, ChatHistory>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// 定义当前活跃的对话ID
static CURRENT_CHAT_ID: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(1)); // 默认为对话1
static NEXT_CHAT_ID: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(2)); // 下一个新建对话的ID

// 聊天历史项目（不包含内容，用于列表展示）
#[derive(Clone, Serialize, Deserialize)]
struct ChatHistoryItem {
    id: u32,
    title: String,
    time: String,
}

fn initialize_history() {
    match load_history() {
        Ok(map) => {
            // println!("load history: {:?}", map);
            let mut history = CHAT_HISTORY.lock().unwrap();

            // 检查是否需要更新 NEXT_CHAT_ID
            if !map.is_empty() {
                let max_id = map.keys().max().unwrap_or(&3);
                let mut next_id = NEXT_CHAT_ID.lock().unwrap();
                if *max_id >= *next_id {
                    *next_id = max_id + 1;
                }
            }

            // Move map after we've used it
            *history = map;
        }
        Err(e) => {
            println!("Failed to load history: {}", e);
        }
    }
}

// 获取聊天历史列表
#[tauri::command]
fn get_chat_history() -> Vec<ChatHistoryItem> {
    let history = CHAT_HISTORY.lock().unwrap();
    let mut history_items: Vec<ChatHistoryItem> = history
        .values()
        .map(|h| ChatHistoryItem {
            id: h.id,
            title: h.title.clone(),
            time: h.time.clone(),
        })
        .collect();

    // 按ID排序，最新的在前面
    history_items.sort_by(|a, b| b.id.cmp(&a.id));
    history_items
}

// 获取指定ID的聊天内容
#[tauri::command]
fn get_chat_by_id(id: u32) -> Vec<ChatMessage> {
    let mut current_id = CURRENT_CHAT_ID.lock().unwrap();
    *current_id = id; // 更新当前对话ID

    let history = CHAT_HISTORY.lock().unwrap();
    if let Some(chat) = history.get(&id) {
        ChatMessage::markdown_to_html_vec(&chat.content)
    } else {
        vec![]
    }
}

/**
获取当前聊天内容
*/
#[tauri::command]
fn get_chat_html() -> Vec<ChatMessage> {
    let current_id = *CURRENT_CHAT_ID.lock().unwrap();
    let history = CHAT_HISTORY.lock().unwrap();

    if let Some(chat) = history.get(&current_id) {
        ChatMessage::markdown_to_html_vec(&chat.content)
    } else {
        vec![]
    }
}

/*
创建新对话
*/

#[tauri::command]
fn create_new_chat() -> Vec<ChatMessage> {
    // 获取新ID
    let mut next_id = NEXT_CHAT_ID.lock().unwrap();
    let new_id = *next_id;
    *next_id += 1;

    // 更新当前对话ID
    let mut current_id = CURRENT_CHAT_ID.lock().unwrap();
    *current_id = new_id;

    // 创建新对话
    let now = chrono::Local::now();
    let today = now.format("%H:%M").to_string();
    let new_chat = ChatHistory {
        id: new_id,
        title: format!("对话 {}", new_id),
        time: today.clone(),
        content: vec![],
    };

    let content = new_chat.content.clone();

    // 添加到历史记录
    let mut history = CHAT_HISTORY.lock().unwrap();
    history.insert(new_id, new_chat);
    save_history(&history).unwrap_or_else(|e| {
        println!("Failed to save history: {}", e);
    });

    ChatMessage::markdown_to_html_vec(&content)
}

// 以流式方式处理用户消息
#[tauri::command]
fn process_message_stream(window: Window, message: String) {
    // 克隆窗口以便在新线程中使用
    let window_clone = window.clone();

    // 创建一个新线程处理消息
    std::thread::spawn(move || {
        // 获取API密钥
        let api_key_list = aibackend::apikey::get_api_key_list_or_create("api_keys.json");
        let gemini_keys = api_key_list.filter_by_type(aibackend::apikey::ApiKeyType::Gemini);

        if gemini_keys.keys.is_empty() {
            // 如果没有API密钥，发送错误消息
            let _ = window_clone.emit(
                "stream-message",
                "未找到API密钥，请先在设置中添加Gemini API密钥",
            );
            let _ = window_clone.emit("stream-complete", "");
            return;
        }

        // 随机选择一个API密钥
        let api_key = gemini_keys.keys[0].clone(); // 或者使用random_key()随机选择

        // 初始化AI聊天实例
        let mut chat = aibackend::gemini::GeminiChat::new();

        // 设置系统提示语
        let _ = chat.set_system_prompt("你是NPULearn应用的AI助手，请尽可能提供专业、准确的回答。支持使用Markdown语法丰富你的回答。".to_string());

        // 获取当前聊天上下文
        let current_chat_id = *CURRENT_CHAT_ID.lock().unwrap();
        let mut current_chat_context = {
            let history = CHAT_HISTORY.lock().unwrap();
            if let Some(history_chat) = history.get(&current_chat_id) {
                history_chat.clone()
            } else {
                ChatHistory {
                    id: current_chat_id,
                    title: String::new(),
                    time: String::new(),
                    content: vec![],
                }
            }
        };

        // 加载聊天历史到AI聊天实例
        if let Err(e) = chat.load_from(&current_chat_context) {
            println!("无法加载聊天历史: {}", e);
        }

        // 创建临时用户消息，用于实时显示
        let mut cloned_context = current_chat_context.clone();
        cloned_context.content.push(ChatMessage {
            msgtype: ChatMessageType::User,
            time: chrono::Local::now().format("%H:%M").to_string(),
            content: message.clone(),
        });

        // 临时显示用户消息
        let content: &ChatHistory = &ChatHistory::markdown_to_html(&cloned_context);
        let _ = window_clone.emit("stream-message", content);

        // 显示正在加载
        cloned_context.content.push(ChatMessage {
            msgtype: ChatMessageType::Assistant,
            time: chrono::Local::now().format("%H:%M").to_string(),
            content: "正在思考...".to_string(),
        });

        let content: &ChatHistory = &ChatHistory::markdown_to_html(&cloned_context);
        let _ = window_clone.emit("stream-message", content);

        // 创建一个锁定的变量用于存储累积的响应内容
        let accumulated_markdown = Arc::new(Mutex::new(String::new()));

        // 创建一个回调函数，用于处理流式响应的每个部分
        let callback = {
            let window_clone = window_clone.clone();
            let mut cloned_context = cloned_context.clone();
            let accumulated_markdown = Arc::clone(&accumulated_markdown);

            // 移除"正在思考..."消息
            if !cloned_context.content.is_empty() {
                cloned_context.content.pop();
            }

            // 添加实际的聊天消息，内容将在回调中更新
            cloned_context.content.push(ChatMessage {
                msgtype: ChatMessageType::Assistant,
                time: chrono::Local::now().format("%H:%M").to_string(),
                content: String::new(), // 初始为空，将在回调中更新
            });

            move |text: String| {
                // 累积流式响应内容
                let mut accumulated = accumulated_markdown.lock().unwrap();
                accumulated.push_str(&text);

                // 更新最后一条消息的内容
                let last_idx = cloned_context.content.len() - 1;
                cloned_context.content[last_idx].content = accumulated.clone();

                // 将内容转换为HTML并立即发送到前端
                let content: &ChatHistory = &ChatHistory::markdown_to_html(&cloned_context);
                println!("Sending stream message: {}", text.clone());
                let _ = window_clone.emit("stream-message", content);
            }
        };

        // 创建一个tokio运行时
        let runtime = tokio::runtime::Runtime::new().unwrap();

        // 创建一个通道用于获取最终结果
        let (tx, rx) = std::sync::mpsc::channel();

        // Clone message before moving it into the async block
        let message_for_async = message.clone();
        println!("message_for_async: {}", message_for_async);
        
        // 在运行时内启动异步任务，但不阻塞等待它完成
        runtime.spawn(async move {
            // 执行流式响应生成
            let result = chat
                .generate_response_stream(api_key, message_for_async, callback)
                .await;

            // 将结果映射错误为String以使其可以安全地在线程间传递
            let send_result = result.map_err(|e| e.to_string());

            // 将结果发送回主线程
            let _ = tx.send(send_result);
        });
        
        println!("Waiting for response...");
        // 等待异步任务完成并获取结果
        let response_result = rx.recv().unwrap();

        // 处理最终结果
        match response_result {
            Ok(final_response) => {
                // 储存当前对话的内容
                let current_id = *CURRENT_CHAT_ID.lock().unwrap();
                let mut history = CHAT_HISTORY.lock().unwrap();
                if let Some(chat_history) = history.get_mut(&current_id) {
                    // 添加用户消息和助手响应
                    chat_history.content.push(ChatMessage {
                        msgtype: ChatMessageType::User,
                        time: chrono::Local::now().format("%H:%M").to_string(),
                        content: message.clone(),
                    });
                    chat_history.content.push(ChatMessage {
                        msgtype: ChatMessageType::Assistant,
                        time: chrono::Local::now().format("%H:%M").to_string(),
                        content: final_response,
                    });
                    chat_history.time = chrono::Local::now().format("%H:%M").to_string();
                    
                    // 保存历史记录
                    save_history(&history).unwrap_or_else(|e| {
                        println!("Failed to save history: {}", e);
                    });
                }
            }
            Err(e) => {
                // 处理错误情况
                let error_message = format!("生成回复时出错: {}", e);

                // 更新最后一条消息为错误信息
                let mut cloned_context = current_chat_context.clone();
                cloned_context.content.push(ChatMessage {
                    msgtype: ChatMessageType::User,
                    time: chrono::Local::now().format("%H:%M").to_string(),
                    content: message.clone(),
                });
                cloned_context.content.push(ChatMessage {
                    msgtype: ChatMessageType::Assistant,
                    time: chrono::Local::now().format("%H:%M").to_string(),
                    content: error_message.clone(),
                });

                let content: &ChatHistory = &ChatHistory::markdown_to_html(&cloned_context);
                let _ = window_clone.emit("stream-message", content);

                // 储存当前对话的内容，包括错误信息
                let current_id = *CURRENT_CHAT_ID.lock().unwrap();
                let mut history = CHAT_HISTORY.lock().unwrap();
                if let Some(chat) = history.get_mut(&current_id) {
                    chat.content.push(ChatMessage {
                        msgtype: ChatMessageType::User,
                        time: chrono::Local::now().format("%H:%M").to_string(),
                        content: message.clone(),
                    });
                    chat.content.push(ChatMessage {
                        msgtype: ChatMessageType::Assistant,
                        time: chrono::Local::now().format("%H:%M").to_string(),
                        content: error_message,
                    });
                    chat.time = chrono::Local::now().format("%H:%M").to_string();
                    // 保存历史记录
                    save_history(&history).unwrap_or_else(|e| {
                        println!("Failed to save history: {}", e);
                    });
                }
            }
        }

        // 通知前端流式传输完成
        let _ = window_clone.emit("stream-complete", "");
    });

    // 主线程立即返回，不会被阻塞
}

#[tauri::command]
fn regenerate_message(window: Window, message_index: usize) -> Result<(), String> {
    // 克隆窗口以便在新线程中使用
    let window_clone = window.clone();

    // 创建一个新线程处理消息重新生成
    std::thread::spawn(move || {
        // 获取当前聊天ID
        let current_id = *CURRENT_CHAT_ID.lock().unwrap();
        
        // 从锁定的历史中获取聊天记录的克隆，避免长时间持有锁
        let chat_clone = {
            let history = CHAT_HISTORY.lock().unwrap();
            match history.get(&current_id) {
                Some(chat) => chat.clone(),
                None => {
                    let _ = window_clone.emit("stream-message", "找不到当前对话");
                    let _ = window_clone.emit("stream-complete", "");
                    return;
                }
            }
        };
        
        // 检查消息索引是否有效
        if message_index >= chat_clone.content.len() {
            let _ = window_clone.emit("stream-message", "无效的消息索引");
            let _ = window_clone.emit("stream-complete", "");
            return;
        }
        
        // 检查是否是助手消息
        if chat_clone.content[message_index].msgtype != ChatMessageType::Assistant {
            let _ = window_clone.emit("stream-message", "只能重新生成助手的消息");
            let _ = window_clone.emit("stream-complete", "");
            return;
        }
        
        // 获取用户的上一条消息
        let user_message = if message_index > 0 && chat_clone.content[message_index - 1].msgtype == ChatMessageType::User {
            chat_clone.content[message_index - 1].content.clone()
        } else {
            let _ = window_clone.emit("stream-message", "找不到对应的用户消息");
            let _ = window_clone.emit("stream-complete", "");
            return;
        };
        
        // 获取API密钥
        let api_key_list = aibackend::apikey::get_api_key_list_or_create("api_keys.json");
        let gemini_keys = api_key_list.filter_by_type(aibackend::apikey::ApiKeyType::Gemini);
        
        if gemini_keys.keys.is_empty() {
            // 如果没有API密钥，发送错误消息
            let _ = window_clone.emit("stream-message", "未找到API密钥，请先在设置中添加Gemini API密钥");
            let _ = window_clone.emit("stream-complete", "");
            return;
        }
        
        // 随机选择一个API密钥
        let api_key = gemini_keys.keys[0].clone(); // 或者使用random_key()随机选择
        
        // 初始化AI聊天实例
        let mut ai_chat = aibackend::gemini::GeminiChat::new();
        
        // 设置系统提示语
        let _ = ai_chat.set_system_prompt("你是NPULearn应用的AI助手，请尽可能提供专业、准确的回答。支持使用Markdown语法丰富你的回答。".to_string());
        
        // 截断聊天历史，只保留到用户的消息（丢弃所有后续内容）
        let mut chat_history = chat_clone.clone();
        chat_history.content.truncate(message_index);
        
        // 加载聊天历史到AI聊天实例
        if let Err(e) = ai_chat.load_from(&chat_history) {
            println!("无法加载聊天历史: {}", e);
            let _ = window_clone.emit("stream-message", format!("无法加载聊天历史: {}", e));
            let _ = window_clone.emit("stream-complete", "");
            return;
        }
        
        // 创建用于显示的上下文
        let mut display_context = chat_history.clone();
        
        // 添加"正在思考..."消息
        display_context.content.push(ChatMessage {
            msgtype: ChatMessageType::Assistant,
            time: chrono::Local::now().format("%H:%M").to_string(),
            content: "正在思考...".to_string(),
        });
        
        // 显示临时状态
        let display_content = &ChatHistory::markdown_to_html(&display_context);
        let _ = window_clone.emit("stream-message", display_content);
        
        // 创建一个锁定的变量用于存储累积的响应内容
        let accumulated_markdown = Arc::new(Mutex::new(String::new()));
        
        // 创建一个回调函数，用于处理流式响应的每个部分
        let callback = {
            let window_clone = window_clone.clone();
            let mut display_context = display_context.clone();
            let accumulated_markdown = Arc::clone(&accumulated_markdown);
            
            // 移除"正在思考..."消息
            if !display_context.content.is_empty() {
                display_context.content.pop();
            }
            
            // 添加实际的聊天消息，内容将在回调中更新
            display_context.content.push(ChatMessage {
                msgtype: ChatMessageType::Assistant,
                time: chrono::Local::now().format("%H:%M").to_string(),
                content: String::new(), // 初始为空，将在回调中更新
            });
            
            move |text: String| {
                // 累积流式响应内容
                let mut accumulated = accumulated_markdown.lock().unwrap();
                accumulated.push_str(&text);
                
                // 更新最后一条消息的内容
                let last_idx = display_context.content.len() - 1;
                display_context.content[last_idx].content = accumulated.clone();
                
                // 将内容转换为HTML并立即发送到前端
                let content = &ChatHistory::markdown_to_html(&display_context);
                println!("Sending stream message: {}", text.clone());
                let _ = window_clone.emit("stream-message", content);
            }
        };
        
        // 创建一个tokio运行时
        let runtime = tokio::runtime::Runtime::new().unwrap();
        
        // 创建一个通道用于获取最终结果
        let (tx, rx) = std::sync::mpsc::channel();
        
        // 在运行时内启动异步任务
        runtime.spawn(async move {
            // 使用regenerate_response_stream方法重新生成响应
            let result = ai_chat.regenerate_response_stream(api_key, callback).await;
            
            // 将结果映射错误为String以使其可以安全地在线程间传递
            let send_result = result.map_err(|e| e.to_string());
            
            // 将结果发送回主线程
            let _ = tx.send(send_result);
        });
        
        println!("Waiting for regenerated response...");
        // 等待异步任务完成并获取结果
        let response_result = rx.recv().unwrap();
        
        // 完成后，获取锁并更新实际的历史记录
        let mut history = CHAT_HISTORY.lock().unwrap();
        let chat = match history.get_mut(&current_id) {
            Some(chat) => chat,
            None => {
                let _ = window_clone.emit("stream-complete", "");
                return; // 如果此时找不到对话，直接返回
            }
        };
        
        // 处理最终结果
        match response_result {
            Ok(final_response) => {
                // 添加新的助手回复
                chat.content.push(ChatMessage {
                    msgtype: ChatMessageType::Assistant,
                    time: chrono::Local::now().format("%H:%M").to_string(),
                    content: final_response,
                });
                
                chat.time = chrono::Local::now().format("%H:%M").to_string();
                
                // 保存历史记录
                save_history(&history).unwrap_or_else(|e| {
                    println!("Failed to save history: {}", e);
                });
            },
            Err(e) => {
                // 处理错误情况
                let error_message = format!("重新生成回复时出错: {}", e);
                
                // 添加错误消息
                chat.content.push(ChatMessage {
                    msgtype: ChatMessageType::Assistant,
                    time: chrono::Local::now().format("%H:%M").to_string(),
                    content: error_message.clone(),
                });
                
                chat.time = chrono::Local::now().format("%H:%M").to_string();
                
                
                // 显示错误消息
                let display_context = chat.clone();
                let display_content = &ChatHistory::markdown_to_html(&display_context);
                let _ = window_clone.emit("stream-message", display_content);
                // 保存历史记录
                save_history(&history).unwrap_or_else(|e| {
                    println!("Failed to save history: {}", e);
                });

            }
        }

        // 通知前端流式传输完成
        let _ = window_clone.emit("stream-complete", "");
    });

    Ok(())
}

// 确保在 run 函数中注册所有命令
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_chat_html,
            get_chat_history,
            get_chat_by_id,
            create_new_chat,
            process_message_stream,
            regenerate_message,
            aibackend::apikey::get_api_key_list_or_create,
            aibackend::apikey::try_save_api_key_list,
            setting::setting::get_settings,
            setting::setting::save_settings,
            setting::setting::get_default_settings,
            setting::setting::select_save_directory,
        ])
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // allowed the given directory
            let scope = app.fs_scope();
            let path = app.path();
            let mut checked_app_local_data_dir = None;
            let mut checked_app_config_dir = None;
            if let Ok(app_local_data_dir) = path.app_local_data_dir() {
                println!("app_local_data_dir: {:?}", app_local_data_dir);
                let result = scope.allow_directory(&app_local_data_dir, false);
                if let Err(e) = result {
                    eprintln!("Failed to allow directory: {}", e);
                }
                checked_app_local_data_dir = Some(app_local_data_dir);
            } else {
                eprintln!("Failed to get app_local_data_dir");
            }

            if let Ok(app_config_dir) = path.app_config_dir() {
                println!("app_config_dir: {:?}", app_config_dir);
                let result = scope.allow_directory(&app_config_dir, false);
                if let Err(e) = result {
                    eprintln!("Failed to allow directory: {}", e);
                }
                checked_app_config_dir = Some(app_config_dir);
            } else {
                eprintln!("Failed to get app_config_dir");
            }

            let handle: Arc<Box<AppHandle>> = Arc::new(Box::new(app.handle().clone()));
            aibackend::apikey::init(
                handle.clone(),
                checked_app_local_data_dir.clone().unwrap(),
                checked_app_config_dir.clone().unwrap(),
            );

            setting::setting::init(handle.clone(), checked_app_config_dir.clone().unwrap());

            let app_local_data_dir = path.app_local_data_dir()?;
            history_msg::history::init(handle.clone(), app_local_data_dir.clone());
            initialize_history();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
