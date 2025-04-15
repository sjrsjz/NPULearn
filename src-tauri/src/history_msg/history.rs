use serde::de::value::Error;
use std::collections::HashMap;
use std::fs::File;
use std::os::windows::io::HandleOrInvalid;
use std::{io::Read, path::PathBuf, sync::Mutex};

use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tauri::{utils::config, AppHandle};
use tauri_plugin_fs::{FilePath, FsExt, OpenOptions};
static APP_HANDLE: Lazy<Mutex<Option<AppHandle>>> = Lazy::new(|| Mutex::new(None));
static APP_DATA_DIR: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

// static FILE_PATH: &str = "./resources/chatHistory.json";
static FILE_NAME: &str = "chatHistory.json";

#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct ChatHistory {
    pub(crate) id: u32,
    pub(crate) title: String,
    pub(crate) time: String,
    pub(crate) content: String,
}

pub fn init(handle: AppHandle, app_data_dir: PathBuf) {
    let mut app_handle = APP_HANDLE.lock().unwrap();
    *app_handle = Some(handle);
    let mut app_data = APP_DATA_DIR.lock().unwrap();
    *app_data = Some(app_data_dir);
}

// #[tauri::command]
pub fn load_history() -> Result<HashMap<u32, ChatHistory>, String> {
    let app_handle_lock = APP_HANDLE.lock().unwrap();
    let app_handle = app_handle_lock
        .as_ref()
        .ok_or_else(|| "App handle not initialized".to_string())?;

    let fs = app_handle.fs();
    let app_data_dir = APP_DATA_DIR.lock().unwrap();
    let app_data_dir = app_data_dir
        .as_ref()
        .ok_or_else(|| "App data directory not initialized".to_string())?;
    let mut opt = OpenOptions::new();
    let path_buf = PathBuf::from(app_data_dir);
    let file_path = FilePath::Path(path_buf.join(FILE_NAME));

    let file = fs.open(file_path, opt.read(true).write(false).create(true).clone());

    if let std::io::Result::Err(e) = file {
        return Err(format!("Failed to open file: {}", e));
    }
    let file = file.unwrap();
    let chat_history: Result<HashMap<u32, ChatHistory>, String> =
        serde_json::from_reader(file).map_err(|e| format!("Failed to read file: {}", e));

    if let Ok(chat_history) = chat_history {
        return Ok(chat_history);
    } else {
        return Err("Failed to parse chat history".to_string());
    }
}

// #[tauri::command]
pub fn save_history(history: &HashMap<u32, ChatHistory>) -> Result<(), String> {
    let app_handle_lock = APP_HANDLE.lock().unwrap();
    let app_handle = app_handle_lock
        .as_ref()
        .ok_or_else(|| "App handle not initialized".to_string())?;
    let fs = app_handle.fs();

    let app_data_dir = APP_DATA_DIR.lock().unwrap();
    let app_data_dir = app_data_dir
        .as_ref()
        .ok_or_else(|| "App data directory not initialized".to_string())?;
    let mut opt = OpenOptions::new();

    let path_buf = PathBuf::from(app_data_dir);
    let file_path = FilePath::Path(path_buf.join(FILE_NAME));

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
