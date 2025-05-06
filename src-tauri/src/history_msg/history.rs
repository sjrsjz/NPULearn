use std::collections::HashMap;
use std::sync::Arc;
use std::{io::Read, path::PathBuf, sync::Mutex};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_fs::{FilePath, FsExt, OpenOptions};
use base64::{Engine as _, engine::general_purpose};

use crate::document_renderer::renderer::convert_markdown_with_latex;
static APP_HANDLE: Lazy<Mutex<Option<Arc<Box<AppHandle>>>>> = Lazy::new(|| Mutex::new(None));
static APP_DATA_DIR: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

static FILE_NAME: &str = "chat_history.json";

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub(crate) enum ChatMessageType {
    User,
    System,
    Assistant,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct ChatMessage {
    pub(crate) msgtype: ChatMessageType,
    pub(crate) time: String,
    pub(crate) content: String,
}
#[allow(dead_code)]
impl ChatMessage {
    // HTML 转义函数，保留换行符为 <br> 标签
    fn escape_html(content: &str) -> String {
        content
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&#39;")
            .replace("\n", "<br>")
    }

    pub(crate) fn markdown_to_html(&self) -> Self {
        // 使用 UTF-8 编码确保中文等非ASCII字符能正确编码
        let original_bytes = self.content.as_bytes();
        let original_base64 = general_purpose::STANDARD.encode(original_bytes);
        
        let new_content = match self.msgtype {
            ChatMessageType::Assistant => {
                let html_content = convert_markdown_with_latex(&self.content);
                // 在 Assistant 消息中添加不可见标签保存原始消息
                format!("{}<div class=\"original-message\" style=\"display:none;\" data-content=\"{}\"></div>", html_content, original_base64)
            },
            _ => {
                let escaped_content = Self::escape_html(&self.content);
                // 在其他类型消息中也添加不可见标签
                format!("{}<div class=\"original-message\" style=\"display:none;\" data-content=\"{}\"></div>", escaped_content, original_base64)
            }
        };
        
        return Self {
            msgtype: self.msgtype.clone(),
            time: self.time.clone(),
            content: new_content,
        };
    }

    pub(crate) fn markdown_to_html_vec(messages : &Vec<Self>) -> Vec<Self> {
        println!("messages: {:?}", messages);
        let mut html_messages = Vec::new();
        for message in messages {
            html_messages.push(message.markdown_to_html());
        }
        return html_messages;
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct ChatHistory {
    pub(crate) id: u32,
    pub(crate) title: String,
    pub(crate) time: String,
    pub(crate) content: Vec<ChatMessage>,
}

#[allow(dead_code)]
impl ChatHistory {
    pub(crate) fn markdown_to_html(&self) -> Self {
        let mut content = self.content.clone();
        for i in 0..content.len() {
            content[i] = content[i].markdown_to_html();
        }
        return Self {
            id: self.id,
            title: self.title.clone(),
            time: self.time.clone(),
            content,
        };
    }
}

pub fn init(handle: Arc<Box<AppHandle>>, app_data_dir: PathBuf) {
    let mut app_handle = APP_HANDLE.lock().unwrap();
    *app_handle = Some(handle);
    let mut app_data = APP_DATA_DIR.lock().unwrap();
    *app_data = Some(app_data_dir.clone());
    if !app_data_dir.exists() {
        std::fs::create_dir_all(&app_data_dir).unwrap();
    }
}

// #[tauri::command]
pub fn load_history() -> Result<HashMap<u32, ChatHistory>, String> {
    let app_handle_lock = APP_HANDLE.lock().unwrap();
    let app_handle = app_handle_lock
        .as_ref()
        .ok_or_else(|| "App handle not initialized".to_string())?;

    let fs = app_handle.fs();

    let app_data_dir_lock = APP_DATA_DIR.lock().unwrap();
    let app_data_dir = app_data_dir_lock
        .as_ref()
        .ok_or_else(|| "App data directory not initialized".to_string())?;

    // 确保配置目录存在
    let path_buf = PathBuf::from(app_data_dir);
    if !path_buf.exists() {
        std::fs::create_dir_all(&path_buf)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    let mut opt = OpenOptions::new();
    let file_path = FilePath::Path(path_buf.join(FILE_NAME));
    println!("file_path: {:?}", file_path);

    let file = fs.open(file_path, opt.read(true).write(false).create(false).clone());

    if let std::io::Result::Err(e) = file {
        return Err(format!("Failed to open file: {}", e));
    }
    let mut file = file.unwrap();
    let mut contents = String::new();

    if let Err(e) = file.read_to_string(&mut contents) {
        return Err(format!("Failed to read file content: {}", e));
    }

    if contents.trim().is_empty() {
        return Ok(HashMap::new());
    }

    match serde_json::from_str::<HashMap<u32, ChatHistory>>(&contents) {
        Ok(chat_history) => Ok(chat_history),
        Err(e) => {
            println!("Failed to parse JSON: {}", e);
            Err(format!("Failed to parse chat history: {}", e))
        }
    }
}

// #[tauri::command]
pub fn save_history(history: &HashMap<u32, ChatHistory>) -> Result<(), String> {
    let app_handle_lock = APP_HANDLE.lock().unwrap();
    let app_handle = app_handle_lock
        .as_ref()
        .ok_or_else(|| "App handle not initialized".to_string())?;
    let fs = app_handle.fs();

    let app_data_dir_lock = APP_DATA_DIR.lock().unwrap();
    let app_data_dir = app_data_dir_lock
        .as_ref()
        .ok_or_else(|| "App data directory not initialized".to_string())?;
    let mut opt = OpenOptions::new();

    let path_buf = PathBuf::from(app_data_dir);
    let file_path = FilePath::Path(path_buf.join(FILE_NAME));
    println!("file_path: {:?}", file_path);

    let file = fs.open(
        file_path,
        opt.read(false)
            .write(true)
            .create(true)
            .truncate(true)
            .clone(),
    );

    if let std::io::Result::Err(e) = file {
        return Err(format!("Failed to open file: {}", e));
    }

    let file = file.unwrap();

    serde_json::to_writer_pretty(file, history)
        .map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}
