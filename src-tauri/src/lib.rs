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
            let _ = window_clone.emit("stream-message", "未找到API密钥，请先在设置中添加Gemini API密钥");
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
        let current_chat_context = {
            let history = CHAT_HISTORY.lock().unwrap();
            if let Some(chat) = history.get(&CURRENT_CHAT_ID.lock().unwrap()) {
                chat.clone()
            } else {
                ChatHistory {
                    id: 0,
                    title: String::new(),
                    time: String::new(),
                    content: vec![],
                }
            }
        };
        
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
        let accumulated_markdown_clone = accumulated_markdown.clone();
        
        // 创建一个回调函数，用于处理流式响应的每个部分
        let callback = {
            let window_clone = window_clone.clone();
            let mut cloned_context = cloned_context.clone();
            
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
                let mut accumulated = accumulated_markdown_clone.lock().unwrap();
                accumulated.push_str(&text);
                
                // 更新最后一条消息的内容
                let last_idx = cloned_context.content.len() - 1;
                cloned_context.content[last_idx].content = accumulated.clone();
                
                // 转换为HTML并发送到前端
                let content: &ChatHistory = &ChatHistory::markdown_to_html(&cloned_context);
                let _ = window_clone.emit("stream-message", content);
            }
        };
        
        // 使用tokio运行时执行异步任务
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let response_result = runtime.block_on(chat.generate_response_stream(
            api_key, 
            message.clone(),
            callback
        ));
        
        match response_result {
            Ok(final_response) => {
                // 储存当前对话的内容
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
                        content: final_response,
                    });
                    chat.time = chrono::Local::now().format("%H:%M").to_string();
                    // 保存历史记录
                    save_history(&history).unwrap_or_else(|e| {
                        println!("Failed to save history: {}", e);
                    });
                }
            },
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
    // 克隆一份当前对话的内容
    let current_id = *CURRENT_CHAT_ID.lock().unwrap();
    let mut history = CHAT_HISTORY.lock().unwrap();

    let chat_history = match history.get_mut(&current_id) {
        Some(history) => history,
        None => return Err("当前对话不存在".to_string()),
    };

    // 检查索引是否有效（必须是Assistant消息）
    if message_index >= chat_history.content.len() {
        return Err("消息索引无效".to_string());
    }

    // 找到指定索引的消息
    let message = &chat_history.content[message_index];

    // 只允许重做AI的消息
    if message.msgtype != ChatMessageType::Assistant {
        return Err("只能重新生成AI助手的消息".to_string());
    }

    // 找到该消息对应的用户消息（通常是上一条）
    let user_message = if message_index > 0
        && chat_history.content[message_index - 1].msgtype == ChatMessageType::User
    {
        chat_history.content[message_index - 1].content.clone()
    } else {
        // 如果没有找到对应的用户消息，返回错误
        return Err("未找到对应的用户消息".to_string());
    };

    // 移除当前消息以及之后的所有消息
    chat_history.content.truncate(message_index);
    
    // 更新前端显示为"正在重新生成..."
    let window_clone = window.clone();
    
    let cloned_context = chat_history.clone();
    let mut display_context = cloned_context.clone();
    
    display_context.content.push(ChatMessage {
        msgtype: ChatMessageType::Assistant,
        time: chrono::Local::now().format("%H:%M").to_string(),
        content: "正在重新生成...".to_string(),
    });
    
    let content: &ChatHistory = &ChatHistory::markdown_to_html(&display_context);
    let _ = window_clone.emit("stream-message", content);

    // 获取API密钥
    let api_key_list = aibackend::apikey::get_api_key_list_or_create("api_keys.json");
    let gemini_keys = api_key_list.filter_by_type(aibackend::apikey::ApiKeyType::Gemini);
    
    if gemini_keys.keys.is_empty() {
        // 如果没有API密钥，发送错误消息
        chat_history.content.push(ChatMessage {
            msgtype: ChatMessageType::Assistant,
            time: chrono::Local::now().format("%H:%M").to_string(),
            content: "未找到API密钥，请先在设置中添加Gemini API密钥".to_string(),
        });
        
        let content: &ChatHistory = &ChatHistory::markdown_to_html(&chat_history);
        let _ = window_clone.emit("stream-message", content);
        let _ = window_clone.emit("stream-complete", "");
        
        // 保存历史记录
        save_history(&history).unwrap_or_else(|e| {
            println!("Failed to save history: {}", e);
        });
        
        return Err("未找到API密钥".to_string());
    }
    
    // 随机选择一个API密钥
    let api_key = gemini_keys.keys[0].clone(); // 或者使用random_key()随机选择
    
    // 创建一个新线程处理消息
    let window_clone = window.clone();
    let chat_id = current_id;
    
    // 释放锁以便新线程可以获取它
    drop(history);
    
    std::thread::spawn(move || {
        // 初始化AI聊天实例
        let mut chat = aibackend::gemini::GeminiChat::new();
        
        // 设置系统提示语
        let _ = chat.set_system_prompt("你是NPULearn应用的AI助手，请尽可能提供专业、准确的回答。支持使用Markdown语法丰富你的回答。".to_string());
        
        // 调用AI生成回复
        let response_future = chat.generate_response(api_key, user_message.clone());
        
        // 使用tokio运行时执行异步任务
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let response_result = runtime.block_on(response_future);
        
        // 获取锁并更新历史记录
        let mut history = CHAT_HISTORY.lock().unwrap();
        let chat_history = match history.get_mut(&chat_id) {
            Some(history) => history,
            None => return, // 对话不存在，直接返回
        };
        
        match response_result {
            Ok(response) => {
                // 添加AI回复
                chat_history.content.push(ChatMessage {
                    msgtype: ChatMessageType::Assistant,
                    time: chrono::Local::now().format("%H:%M").to_string(),
                    content: response.clone(),
                });
                
                let content: &ChatHistory = &ChatHistory::markdown_to_html(&chat_history);
                
                // 更新前端显示
                let _ = window_clone.emit("stream-message", content);
            },
            Err(e) => {
                // 处理错误情况
                let error_message = format!("重新生成回复时出错: {}", e);
                chat_history.content.push(ChatMessage {
                    msgtype: ChatMessageType::Assistant,
                    time: chrono::Local::now().format("%H:%M").to_string(),
                    content: error_message.clone(),
                });
                
                let content: &ChatHistory = &ChatHistory::markdown_to_html(&chat_history);
                let _ = window_clone.emit("stream-message", content);
            }
        }
        
        // 更新对话时间
        chat_history.time = chrono::Local::now().format("%H:%M").to_string();
        
        // 保存历史记录
        save_history(&history).unwrap_or_else(|e| {
            println!("Failed to save history: {}", e);
        });
        
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
