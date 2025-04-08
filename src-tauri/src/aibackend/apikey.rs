use std::{io::Read, path::PathBuf, sync::Mutex};

use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tauri::{utils::config, AppHandle};
use tauri_plugin_fs::{FilePath, FsExt, OpenOptions};

// 全局存储 app_handle
static APP_HANDLE: Lazy<Mutex<Option<AppHandle>>> = Lazy::new(|| Mutex::new(None));
static APP_DATA_DIR: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));
static APP_CONFIG_DIR: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));
// 设置全局 app_handle
pub fn init(handle: AppHandle, app_data_dir: PathBuf, app_config_dir: PathBuf) {
    let mut app_handle = APP_HANDLE.lock().unwrap();
    *app_handle = Some(handle);
    let mut app_data = APP_DATA_DIR.lock().unwrap();
    *app_data = Some(app_data_dir);
    let mut app_config = APP_CONFIG_DIR.lock().unwrap();
    *app_config = Some(app_config_dir);
}

#[derive(Clone, PartialEq, Eq)]
pub enum ApiKeyType {
    Gemini,
}

impl ApiKeyType {
    pub fn to_string(&self) -> String {
        match self {
            ApiKeyType::Gemini => "Gemini".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Option<ApiKeyType> {
        match s {
            "Gemini" => Some(ApiKeyType::Gemini),
            _ => None,
        }
    }
}

impl Serialize for ApiKeyType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ApiKeyType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ApiKeyType::from_string(&s).ok_or_else(|| serde::de::Error::custom("Invalid ApiKeyType"))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key: String,
    pub name: String,
    pub key_type: ApiKeyType,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ApiKeyList {
    pub keys: Vec<ApiKey>,
}

impl ApiKeyList {
    pub fn new() -> Self {
        ApiKeyList { keys: vec![] }
    }

    pub fn add_key(&mut self, key: ApiKey) {
        self.keys.push(key);
    }

    pub fn remove_key(&mut self, key: &ApiKey) {
        self.keys.retain(|k| k.key != key.key);
    }

    pub fn load_from(config_name: &str) -> Result<Self, String> {
        let app_handle_lock = APP_HANDLE.lock().unwrap();
        let app_handle = app_handle_lock
            .as_ref()
            .ok_or_else(|| "App handle not initialized".to_string())?;

        let fs = app_handle.fs();

        let app_config_dir_lock = APP_CONFIG_DIR.lock().unwrap();
        let app_config_dir = app_config_dir_lock
            .as_ref()
            .ok_or_else(|| "App config directory not initialized".to_string())?;

        let mut opt = OpenOptions::new();

        let path_buf = PathBuf::from(app_config_dir);
        let file_path = FilePath::Path(path_buf.join(config_name));
        let file = fs.open(file_path, opt.read(true).write(false).create(true).clone());
        if let std::io::Result::Err(e) = file {
            return Err(format!("Failed to open file: {}", e));
        }
        let file = file.unwrap();
        let api_key_list: Result<ApiKeyList, String> =
            serde_json::from_reader(file).map_err(|e| format!("Failed to read file: {}", e));

        if let Ok(api_key_list) = api_key_list {
            return Ok(api_key_list);
        } else {
            return Err("Failed to parse API key list".to_string());
        }
    }

    pub fn save_to(&self, config_name: &str) -> Result<(), String> {
        let app_handle_lock = APP_HANDLE.lock().unwrap();
        let app_handle = app_handle_lock
            .as_ref()
            .ok_or_else(|| "App handle not initialized".to_string())?;

        let fs = app_handle.fs();

        let app_config_dir_lock = APP_CONFIG_DIR.lock().unwrap();
        let app_config_dir = app_config_dir_lock
            .as_ref()
            .ok_or_else(|| "App config directory not initialized".to_string())?;

        let mut opt = OpenOptions::new();

        let path_buf = PathBuf::from(app_config_dir);
        let file_path = FilePath::Path(path_buf.join(config_name));
        println!("file_path: {}", file_path);
        
        let file = fs.open(
            file_path,
            opt.read(false)
                .write(true)
                .create(true)
                .truncate(true)
                .clone(),
        );

        if let std::io::Result::Err(e) = file {
            return Err(format!("Failed to open file for writing: {}", e));
        }

        let file = file.unwrap();

        serde_json::to_writer_pretty(file, self)
            .map_err(|e| format!("Failed to write to file: {}", e))?;

        Ok(())
    }

    pub fn filter_by_type(&self, key_type: ApiKeyType) -> ApiKeyList {
        ApiKeyList {
            keys: self
                .keys
                .iter()
                .filter(|key| key.key_type == key_type)
                .cloned()
                .collect(),
        }
    }

    pub fn random_key(&self) -> Option<ApiKey> {
        if self.keys.is_empty() {
            return None;
        }
        let mut rng = rand::rng();
        let index = rand::Rng::random_range(&mut rng, 0..self.keys.len());
        Some(self.keys[index].clone())
    }
}

#[tauri::command]
pub fn get_api_key_list_or_create(config_name: &str) -> ApiKeyList {
    let list = ApiKeyList::load_from(config_name);
    match list {
        Ok(list) => list,
        Err(_) => ApiKeyList::new(),
    }
}

#[tauri::command]
pub fn try_save_api_key_list(config_name: &str, list: ApiKeyList) -> Result<(), String> {
    list.save_to(config_name)
}