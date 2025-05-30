/// 读取PDF文档内容（暂时不支持）
pub async fn read_pdf_document(_file_path: &str) -> Result<String, String> {
    Ok("PDF文件支持正在开发中，请将PDF转换为文本文件后上传。".to_string())
}
