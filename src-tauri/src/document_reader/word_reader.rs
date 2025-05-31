use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

/// 读取Word文档内容
pub async fn read_word_document(file_path: &str) -> Result<String, String> {
    let file_path = file_path.to_string();
    
    tokio::task::spawn_blocking(move || {
        // 尝试打开文件
        let file = File::open(&file_path)
            .map_err(|e| format!("无法打开文件: {}", e))?;
        
        // 检查文件扩展名
        if file_path.to_lowercase().ends_with(".docx") {
            read_docx_content(file)
        } else if file_path.to_lowercase().ends_with(".doc") {
            // 对于.doc文件，我们暂时只返回提示信息
            Ok("检测到.doc格式文件。建议将文件转换为.docx格式以获得更好的支持。".to_string())
        } else if file_path.to_lowercase().ends_with(".rtf") {
            read_rtf_content(file)
        } else {
            Err("不支持的Word文档格式".to_string())
        }
    }).await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

/// 读取DOCX文件内容
fn read_docx_content(file: File) -> Result<String, String> {
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("无法解析DOCX文件: {}", e))?;
    
    // 查找document.xml文件
    let mut document_xml = archive.by_name("word/document.xml")
        .map_err(|_| "无法找到文档内容文件")?;
    
    let mut xml_content = String::new();
    document_xml.read_to_string(&mut xml_content)
        .map_err(|e| format!("无法读取文档内容: {}", e))?;
    
    // 解析XML并提取文本
    extract_text_from_docx_xml(&xml_content)
}

/// 从DOCX XML中提取文本内容
fn extract_text_from_docx_xml(xml_content: &str) -> Result<String, String> {
    use xml::reader::{EventReader, XmlEvent};
    use std::io::Cursor;
    
    let cursor = Cursor::new(xml_content);
    let parser = EventReader::new(cursor);
    let mut text_content = String::new();
    let mut in_text = false;
    
    for event in parser {
        match event {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "t" {
                    in_text = true;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "t" {
                    in_text = false;
                } else if name.local_name == "p" {
                    // 段落结束，添加换行
                    text_content.push('\n');
                }
            }
            Ok(XmlEvent::Characters(text)) => {
                if in_text {
                    text_content.push_str(&text);
                }
            }
            Err(e) => {
                return Err(format!("XML解析错误: {}", e));
            }
            _ => {}
        }
    }
    
    Ok(clean_document_content(text_content))
}

/// 读取RTF文件内容（简单实现）
fn read_rtf_content(mut file: File) -> Result<String, String> {
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("无法读取RTF文件: {}", e))?;
    
    // 简单的RTF文本提取（移除控制字符）
    let text = extract_text_from_rtf(&content);
    Ok(clean_document_content(text))
}

/// 从RTF内容中提取纯文本
fn extract_text_from_rtf(rtf_content: &str) -> String {
    let mut result = String::new();
    let mut chars = rtf_content.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                // 跳过控制字符直到空格或其他分隔符
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphabetic() {
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            '{' | '}' => {
                // 忽略大括号
            }
            _ if ch.is_ascii() && !ch.is_control() => {
                result.push(ch);
            }
            _ => {}
        }
    }
    
    result
}

/// 清理文档内容，移除多余的空白和格式字符
fn clean_document_content(content: String) -> String {
    // 移除多余的空行
    let lines: Vec<&str> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    
    let cleaned = lines.join("\n");
    
    // 如果内容为空或只有空白字符，返回提示信息
    if cleaned.trim().is_empty() {
        "文档内容为空或无法读取有效文本".to_string()
    } else {
        cleaned
    }
}
