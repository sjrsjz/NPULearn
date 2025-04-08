use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{io::Read, path::PathBuf, sync::Mutex};
use tauri::AppHandle;
use tauri_plugin_fs::{FilePath, FsExt, OpenOptions};


// 为settings模块创建自己的静态变量
static SETTINGS_APP_HANDLE: Lazy<Mutex<Option<AppHandle>>> = Lazy::new(|| Mutex::new(None));
static SETTINGS_CONFIG_DIR: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

// 初始化settings模块所需的全局变量
pub fn init(handle: AppHandle, config_dir: PathBuf) {
    let mut app_handle = SETTINGS_APP_HANDLE.lock().unwrap();
    *app_handle = Some(handle);
    let mut config_dir_lock = SETTINGS_CONFIG_DIR.lock().unwrap();
    *config_dir_lock = Some(config_dir.clone());

    // 确保配置目录存在
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).unwrap();
    }
}

// 应用设置结构体
#[derive(Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,             // 主题: system, light, dark
    pub font_size: String,         // 字体大小: small, medium, large
    pub auto_save: bool,           // 自动保存对话
    pub save_path: String,         // 保存路径
    pub api_model: String,         // 模型选择
    pub model_config: ModelConfig, // 模型配置
}

// 模型配置结构体
#[derive(Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub max_tokens: i32, // 最大生成令牌数
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            theme: "system".to_string(),
            font_size: "medium".to_string(),
            auto_save: true,
            save_path: "".to_string(),
            api_model: "gemini".to_string(),
            model_config: ModelConfig { max_tokens: 2048 },
        }
    }
}

impl AppSettings {
    // 从文件加载设置
    pub fn load_from(config_name: &str) -> Result<Self, String> {
        let app_handle_lock = SETTINGS_APP_HANDLE.lock().unwrap();
        let app_handle = app_handle_lock
            .as_ref()
            .ok_or_else(|| "Settings app handle not initialized".to_string())?;

        let fs = app_handle.fs();

        let config_dir_lock = SETTINGS_CONFIG_DIR.lock().unwrap();
        let config_dir = config_dir_lock
            .as_ref()
            .ok_or_else(|| "Settings config directory not initialized".to_string())?;

        // 确保配置目录存在
        let path_buf = PathBuf::from(config_dir);
        if !path_buf.exists() {
            std::fs::create_dir_all(&path_buf)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        // 打印路径信息以便调试
        println!(
            "Loading settings from path: {:?}",
            path_buf.join(config_name)
        );

        let mut opt = OpenOptions::new();
        let file_path = FilePath::Path(path_buf.join(config_name));

        // 尝试打开文件
        let file = fs.open(file_path, opt.read(true).write(false).create(false).clone());

        // 如果文件不存在或打开失败，返回默认设置
        if let std::io::Result::Err(e) = file {
            println!(
                "Settings file not found or cannot be opened, using defaults: {}",
                e
            );
            return Ok(AppSettings::default());
        }

        let mut file = file.unwrap();

        // 尝试读取文件内容
        let mut contents = String::new();
        if let Err(e) = file.read_to_string(&mut contents) {
            println!("Failed to read settings file, using defaults: {}", e);
            return Ok(AppSettings::default());
        }

        // 如果文件为空，返回默认设置
        if contents.trim().is_empty() {
            println!("Settings file is empty, using defaults");
            return Ok(AppSettings::default());
        }

        // 尝试解析 JSON
        match serde_json::from_str::<AppSettings>(&contents) {
            Ok(settings) => Ok(settings),
            Err(e) => {
                println!("Failed to parse settings JSON, using defaults: {}", e);
                Ok(AppSettings::default())
            }
        }
    }

    // 保存设置到文件
    pub fn save_to(&self, config_name: &str) -> Result<(), String> {
        let app_handle_lock = SETTINGS_APP_HANDLE.lock().unwrap();
        let app_handle = app_handle_lock
            .as_ref()
            .ok_or_else(|| "Settings app handle not initialized".to_string())?;

        let fs = app_handle.fs();

        let config_dir_lock = SETTINGS_CONFIG_DIR.lock().unwrap();
        let config_dir = config_dir_lock
            .as_ref()
            .ok_or_else(|| "Settings config directory not initialized".to_string())?;

        let path_buf = PathBuf::from(config_dir);
        let file_path = FilePath::Path(path_buf.join(config_name));

        println!("Saving settings to: {:?}", file_path);

        let mut opt = OpenOptions::new();
        let file = fs.open(
            file_path,
            opt.read(false)
                .write(true)
                .create(true)
                .truncate(true)
                .clone(),
        );

        if let std::io::Result::Err(e) = file {
            return Err(format!("Failed to open settings file for writing: {}", e));
        }

        let file = file.unwrap();

        serde_json::to_writer_pretty(file, self)
            .map_err(|e| format!("Failed to write settings to file: {}", e))?;

        Ok(())
    }
}

// Tauri 命令：获取设置
#[tauri::command]
pub fn get_settings() -> Result<AppSettings, String> {
    AppSettings::load_from("settings.json")
}

// Tauri 命令：保存设置
#[tauri::command]
pub fn save_settings(settings: AppSettings) -> Result<(), String> {
    settings.save_to("settings.json")
}

// Tauri 命令：获取默认设置
#[tauri::command]
pub fn get_default_settings() -> AppSettings {
    AppSettings::default()
}

// Tauri 命令：选择保存目录
#[tauri::command]
pub async fn select_save_directory(app_handle: AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    
    // 非阻塞方式不适合我们的用例，因为我们需要返回结果
    // 使用阻塞式文件夹选择对话框
    let folder_path = app_handle.dialog().file().blocking_pick_folder();
    
    match folder_path {
        Some(path) => Ok(path.to_string()),
        None => Err("No directory selected".to_string()),
    }
}