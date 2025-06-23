use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::AppHandle;
use tauri_plugin_fs::{FilePath, FsExt, OpenOptions};

// 为settings模块创建自己的静态变量
static SETTINGS_APP_HANDLE: Lazy<Mutex<Option<Arc<Box<AppHandle>>>>> =
    Lazy::new(|| Mutex::new(None));
static SETTINGS_CONFIG_DIR: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

// 初始化settings模块所需的全局变量
pub fn init(handle: Arc<Box<AppHandle>>, config_dir: PathBuf) {
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
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AppSettings {
    pub theme: String,             // 主题: system, light, dark
    pub font_size: String,         // 字体大小: small, medium, large
    pub auto_save: bool,           // 自动保存对话
    pub save_path: String,         // 保存路径
    pub api_model: String,         // 模型选择
    pub model_config: ModelConfig, // 模型配置
    pub model_selection: HashMap<String, String>, // 每种API密钥类型的模型选择
}

// 模型配置结构体
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ModelConfig {
    pub temperature: f32, // 温度
    pub max_tokens: i32,  // 最大生成令牌数
}

impl Default for AppSettings {
    fn default() -> Self {        let mut model_selection = HashMap::new();
        model_selection.insert("Gemini".to_string(), "gemini-2.0-flash".to_string());
        model_selection.insert("DeepSeek".to_string(), "deepseek-chat".to_string());
        model_selection.insert("Coze".to_string(), "coze-bot".to_string());
        
        AppSettings {
            theme: "system".to_string(),
            font_size: "medium".to_string(),
            auto_save: true,
            save_path: "".to_string(),
            api_model: "gemini".to_string(),
            model_config: ModelConfig {
                temperature: 0.7,
                max_tokens: 2048,
            },
            model_selection,
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
        }        // 尝试解析 JSON
        match serde_json::from_str::<AppSettings>(&contents) {
            Ok(mut settings) => {
                println!("成功加载设置，模型选择: {:?}", settings.model_selection);
                // 确保所有API类型都有对应的模型选择
                if !settings.model_selection.contains_key("Gemini") {
                    settings.model_selection.insert("Gemini".to_string(), "gemini-2.0-flash".to_string());
                }
                if !settings.model_selection.contains_key("DeepSeek") {
                    settings.model_selection.insert("DeepSeek".to_string(), "deepseek-chat".to_string());
                }
                if !settings.model_selection.contains_key("Coze") {
                    settings.model_selection.insert("Coze".to_string(), "coze-bot".to_string());
                }
                println!("修复后的模型选择: {:?}", settings.model_selection);
                Ok(settings)
            },
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
        println!("Settings content: {:?}", self);

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
    let settings = AppSettings::load_from("settings.json");
    if let Ok(ref s) = settings {
        println!("返回给前端的设置，模型选择: {:?}", s.model_selection);
    }
    settings
}

//     Tauri 命令：保存设置
#[tauri::command]
pub fn save_settings(settings: AppSettings) -> Result<(), String> {
    println!("收到前端保存设置请求，模型选择: {:?}", settings.model_selection);
    let result = settings.save_to("settings.json");
    if let Ok(_) = result {
        println!("设置保存成功");
    } else {
        println!("设置保存失败: {:?}", result);
    }
    result
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
    let folder = Arc::new(Mutex::new(Err("".to_string())));
    let folder_clone = folder.clone();
    app_handle.dialog().file().pick_file(move |folder_path| {
        let mut folder_guard = folder_clone.lock().unwrap();
        match folder_path {
            Some(path) => *folder_guard = Ok(path.to_string()),
            None => *folder_guard = Err("No directory selected".to_string()),
        };
    });

    return folder.lock().unwrap().clone();
}
