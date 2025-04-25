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
fn process_message_stream(window: Window, message: &str) {
    // 模拟AI响应
    // 在实际应用中，这里应该是从AI API获取的流式响应
    let test_markdown = r#"
# 流式传输测试

这是一个测试流式传输功能的响应。

## 段落1

这是第一段内容，用于测试流式传输。文本将逐步显示在界面上。

## 段落2

这是第二段内容，包含一些*格式化*的**文本**。

## 数学公式测试

行内公式: $E=mc^2$

块级公式:

$$f(x) = \frac{1}{\sigma\sqrt{2\pi}} e^{-\frac{1}{2}\left(\frac{x-\mu}{\sigma}\right)^2}$$

## 代码示例

```python
def hello_world():
    print("Hello, streaming world!")
```
    "#;

    // 在前端累积构建的HTML内容
    let mut accumulated_markdown = String::new();

    // 将markdown分成有意义的块，模拟流式传输
    // 这里用段落或小节作为分隔点，而不是简单按行分割
    let sections = test_markdown.split("\n").collect::<Vec<&str>>();

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

    for section in sections {
        // 累积构建Markdown
        accumulated_markdown.push_str(section);
        accumulated_markdown.push_str("\n");

        // 转换累积的Markdown为HTML

        let mut cloned_context = current_chat_context.clone();

        cloned_context.content.push(ChatMessage {
            msgtype: ChatMessageType::User,
            time: chrono::Local::now().format("%H:%M").to_string(),
            content: message.to_string(),
        });

        cloned_context.content.push(ChatMessage {
            msgtype: ChatMessageType::Assistant,
            time: chrono::Local::now().format("%H:%M").to_string(),
            content: accumulated_markdown.clone(),
        });

        let content: &ChatHistory = &ChatHistory::markdown_to_html(&ChatHistory {
            id: current_chat_context.id,
            title: current_chat_context.title.clone(),
            time: current_chat_context.time.clone(),
            content: cloned_context.content.clone(),
        });

        // 向前端发送完整的HTML内容（而不是增量部分）
        let _ = window.emit("stream-message", content);

        // 模拟网络延迟
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // 储存当前对话的内容
    let current_id = *CURRENT_CHAT_ID.lock().unwrap();
    let mut history = CHAT_HISTORY.lock().unwrap();
    if let Some(chat) = history.get_mut(&current_id) {
        chat.content.push(ChatMessage {
            msgtype: ChatMessageType::User,
            time: chrono::Local::now().format("%H:%M").to_string(),
            content: message.to_string(),
        });
        chat.content.push(ChatMessage {
            msgtype: ChatMessageType::Assistant,
            time: chrono::Local::now().format("%H:%M").to_string(),
            content: accumulated_markdown.clone(),
        });
        chat.time = chrono::Local::now().format("%H:%M").to_string();
        // 保存
        save_history(&history).unwrap_or_else(|e| {
            println!("Failed to save history: {}", e);
        });
    }

    // 通知前端流式传输完成
    let _ = window.emit("stream-complete", "");
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
