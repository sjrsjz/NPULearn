use encoding_rs::*;
use std::fs;

/// 读取文本文件内容，自动检测编码
pub async fn read_text_file(file_path: &str) -> Result<String, String> {
    let file_path = file_path.to_string();
    
    tokio::task::spawn_blocking(move || {
        // 读取文件字节
        let bytes = fs::read(&file_path)
            .map_err(|e| format!("无法读取文件: {}", e))?;
        
        // 检测文件编码
        let content = detect_and_decode(&bytes)?;
        
        Ok(content)
    }).await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

/// 检测并解码文件内容
fn detect_and_decode(bytes: &[u8]) -> Result<String, String> {
    // 首先尝试UTF-8
    if let Ok(content) = std::str::from_utf8(bytes) {
        return Ok(content.to_string());
    }
    
    // 使用chardet检测编码
    let charset = chardet::detect(bytes);
    let encoding_name = charset.0;
    
    // 尝试使用检测到的编码
    if let Some(encoding) = Encoding::for_label(encoding_name.as_bytes()) {
        let (decoded, _, had_errors) = encoding.decode(bytes);
        if !had_errors {
            return Ok(decoded.into_owned());
        }
    }
    
    // 如果检测失败，尝试常见的编码
    try_common_encodings(bytes)
}

/// 尝试常见的编码格式
fn try_common_encodings(bytes: &[u8]) -> Result<String, String> {
    let encodings = [
        UTF_8,
        GBK,
        GB18030,
        UTF_16LE,
        UTF_16BE,
        WINDOWS_1252,
    ];
    
    for encoding in &encodings {
        let (decoded, _, had_errors) = encoding.decode(bytes);
        if !had_errors {
            return Ok(decoded.into_owned());
        }
    }
    
    // 如果所有编码都失败，使用UTF-8强制解码，替换无效字符
    Ok(String::from_utf8_lossy(bytes).into_owned())
}
