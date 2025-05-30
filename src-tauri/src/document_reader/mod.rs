pub mod word_reader;
pub mod pdf_reader;
pub mod text_reader;

use std::path::Path;

#[cfg(target_os = "android")]
use std::pin::Pin;

#[cfg(target_os = "android")]
use std::future::Future;

#[cfg(target_os = "android")]
use tokio::fs;

#[derive(Debug)]
pub enum DocumentType {
    Text,
    Word,
    Pdf,
    Csv,
    Json,
    Xml,
    Html,    Code(String), // 编程语言类型
    #[allow(dead_code)]
    Other(String),
}

impl DocumentType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            // 文本文件
            "txt" | "md" | "markdown" | "log" | "cfg" | "conf" | "ini" | "env" => Self::Text,
            
            // Office 文档
            "doc" | "docx" | "rtf" => Self::Word,
            
            // PDF
            "pdf" => Self::Pdf,
            
            // 数据文件
            "csv" | "tsv" => Self::Csv,
            "json" => Self::Json,
            "xml" => Self::Xml,
            "html" | "htm" => Self::Html,
              // 编程语言文件
            "rs" | "rust" => Self::Code("rust".to_string()),
            "py" | "pyw" => Self::Code("python".to_string()),
            "js" => Self::Code("javascript".to_string()),
            "jsx" => Self::Code("jsx".to_string()),
            "ts" => Self::Code("typescript".to_string()),
            "tsx" => Self::Code("tsx".to_string()),
            "java" => Self::Code("java".to_string()),
            "c" => Self::Code("c".to_string()),
            "cpp" | "cxx" | "cc" => Self::Code("cpp".to_string()),
            "h" | "hpp" => Self::Code("c".to_string()),
            "cs" => Self::Code("csharp".to_string()),
            "go" => Self::Code("go".to_string()),
            "php" => Self::Code("php".to_string()),
            "rb" => Self::Code("ruby".to_string()),
            "swift" => Self::Code("swift".to_string()),
            "kt" => Self::Code("kotlin".to_string()),
            "scala" => Self::Code("scala".to_string()),
            "dart" => Self::Code("dart".to_string()),
            "lua" => Self::Code("lua".to_string()),
            "perl" | "pl" => Self::Code("perl".to_string()),
            "r" => Self::Code("r".to_string()),
            "sql" => Self::Code("sql".to_string()),
            "sh" | "bash" => Self::Code("bash".to_string()),
            "zsh" => Self::Code("zsh".to_string()),
            "ps1" | "psm1" => Self::Code("powershell".to_string()),
            "bat" | "cmd" => Self::Code("batch".to_string()),
            "vbs" => Self::Code("vbscript".to_string()),
            "yaml" | "yml" => Self::Code("yaml".to_string()),            "toml" => Self::Code("toml".to_string()),
            "css" => Self::Code("css".to_string()),
            "scss" | "sass" => Self::Code("scss".to_string()),
            "less" => Self::Code("less".to_string()),
            "vue" => Self::Code("vue".to_string()),
            "svelte" => Self::Code("svelte".to_string()),
            "makefile" | "cmake" => Self::Code("makefile".to_string()),
            "dockerfile" => Self::Code("dockerfile".to_string()),
            "gitignore" | "gitattributes" => Self::Code("gitconfig".to_string()),
            
            // 其他格式
            other => Self::Other(other.to_string()),
        }
    }
    
    pub fn is_supported(&self) -> bool {
        match self {
            Self::Other(_) => false,
            _ => true,
        }
    }
    
    pub fn get_language_hint(&self) -> String {
        match self {
            Self::Code(lang) => lang.clone(),
            Self::Json => "json".to_string(),
            Self::Xml => "xml".to_string(),
            Self::Html => "html".to_string(),
            Self::Csv => "csv".to_string(),
            Self::Text => "text".to_string(),
            Self::Word | Self::Pdf => "text".to_string(),
            Self::Other(_) => "text".to_string(),
        }
    }
}

/// 读取文档内容的统一接口
pub async fn read_document(file_path: &str) -> Result<String, String> {
    println!("Processing file path: {}", file_path);
    
    // 处理Android content URI
    #[cfg(target_os = "android")]
    {
        if file_path.starts_with("content://") {
            return read_android_content_uri(file_path).await;
        }
    }
    
    // 处理传统文件路径
    let path = Path::new(file_path);
    
    if !path.exists() {
        return Err("文件不存在".to_string());
    }
    
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    let doc_type = DocumentType::from_extension(&extension);
    
    if !doc_type.is_supported() {
        return Err(format!("不支持的文件类型: .{}", extension));
    }
    
    let content = match doc_type {
        DocumentType::Word => word_reader::read_word_document(file_path).await?,
        DocumentType::Pdf => pdf_reader::read_pdf_document(file_path).await?,
        _ => text_reader::read_text_file(file_path).await?,
    };
    
    // 获取文件名
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("未知文件");
    
    // 格式化文件内容
    let language_hint = doc_type.get_language_hint();
    let formatted_content = format!(
        "📎 **上传文件: {}**\n\n```{}\n{}\n```",
        file_name, language_hint, content
    );
    
    Ok(formatted_content)
}

#[cfg(target_os = "android")]
async fn read_android_content_uri(content_uri: &str) -> Result<String, String> {
    println!("Reading Android content URI: {}", content_uri);
    
    // 提取文件名（从URI的最后部分）并进行URL解码
    let encoded_file_name = content_uri
        .split('/')
        .last()
        .unwrap_or("上传的文件")
        .to_string();
    
    // URL解码文件名以正确处理中文字符
    let file_name = match urlencoding::decode(&encoded_file_name) {
        Ok(decoded) => decoded.to_string(),
        Err(_) => encoded_file_name, // 如果解码失败，使用原始名称
    };
    
    println!("Decoded file name: {}", file_name);
    
    // 尝试从URI推断文件扩展名
    let extension = if file_name.contains('.') {
        file_name.split('.').last().unwrap_or("txt").to_lowercase()
    } else {
        // 如果没有扩展名，默认为文本文件
        "txt".to_string()
    };
      let doc_type = DocumentType::from_extension(&extension);
    
    if !doc_type.is_supported() {
        return Err(format!("不支持的文件类型: .{}", extension));
    }
    
    // 根据文件类型选择合适的读取方法
    let content = match doc_type {
        DocumentType::Word => {
            // 对于Word文档，需要先尝试找到实际文件路径
            match find_android_file_path(content_uri, &file_name).await {
                Ok(actual_path) => word_reader::read_word_document(&actual_path).await?,
                Err(_) => return Err("无法找到Word文档的实际路径".to_string())
            }
        }
        DocumentType::Pdf => {
            // 对于PDF文档，需要先尝试找到实际文件路径  
            match find_android_file_path(content_uri, &file_name).await {
                Ok(actual_path) => pdf_reader::read_pdf_document(&actual_path).await?,
                Err(_) => return Err("无法找到PDF文档的实际路径".to_string())
            }
        }
        _ => {
            // 文本文件可以直接读取
            read_android_content_as_text(content_uri).await?
        }
    };
    
    let language_hint = doc_type.get_language_hint();
    let formatted_content = format!(
        "📎 **上传文件: {}**\n\n```{}\n{}\n```",
        file_name, language_hint, content    );
    Ok(formatted_content)
}

#[cfg(target_os = "android")]
async fn find_android_file_path(content_uri: &str, file_name: &str) -> Result<String, String> {
    println!("Searching for Android file: {}", file_name);
    
    // 尝试多个可能的路径
    let possible_paths = vec![
        format!("/storage/emulated/0/Download/{}", file_name),
        format!("/storage/emulated/0/Documents/{}", file_name),
        format!("/sdcard/Download/{}", file_name),
        format!("/sdcard/Documents/{}", file_name),
        format!("/sdcard/{}", file_name),
        format!("/storage/self/primary/Download/{}", file_name),
        format!("/storage/self/primary/Documents/{}", file_name),
        format!("/storage/self/primary/{}", file_name),
    ];
    
    for path in possible_paths {
        println!("Checking path: {}", path);
        if tokio::fs::metadata(&path).await.is_ok() {
            println!("Found file at: {}", path);
            return Ok(path);
        }
    }
    
    Err(format!("无法找到文件: {}", file_name))
}

#[cfg(target_os = "android")]
async fn read_android_content_as_text(content_uri: &str) -> Result<String, String> {
    println!("Attempting to read content URI as text: {}", content_uri);
    
    // 简单粗暴地解码 URL
    let file_name = if let Some(encoded_name) = content_uri.split('/').last() {
        println!("Encoded file name part: {}", encoded_name);
        
        // 直接 URL 解码
        match urlencoding::decode(encoded_name) {
            Ok(decoded) => {
                println!("URL decode successful: {}", decoded);
                decoded.to_string()
            }
            Err(_) => {
                println!("URL decode failed, using original: {}", encoded_name);
                encoded_name.to_string()
            }
        }
    } else {
        return Err("无法从URI提取文件名".to_string());
    };
    
    println!("Final decoded file name: {}", file_name);
    
    // 先尝试 Tauri 文件系统读取
    if let Ok(content) = try_read_with_tauri_fs(content_uri).await {
        return Ok(content);
    }
    
    // 再尝试直接路径读取
    let simple_paths = vec![
        format!("/storage/emulated/0/Download/{}", file_name),
        format!("/storage/emulated/0/Documents/{}", file_name),
        format!("/sdcard/{}", file_name),
        format!("/storage/self/primary/{}", file_name),
    ];
    
    for path in simple_paths {
        println!("Trying direct path: {}", path);
        if let Ok(content) = fs::read_to_string(&path).await {
            println!("Successfully read from: {}", path);
            return Ok(content);
        }
    }
    
    Err(format!("无法读取文件: {}", file_name))
}

#[cfg(target_os = "android")]
async fn try_read_with_tauri_fs(_content_uri: &str) -> Result<String, String> {
    // 简单的占位符，让其他方法继续尝试
    Err("Tauri FS not available".to_string())
}
