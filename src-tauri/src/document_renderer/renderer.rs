use comrak::{markdown_to_html, ComrakOptions};
use ammonia::clean;

pub fn convert_markdown_with_latex(markdown: &str) -> String {
    // 将输入的 markdown 按照特殊标记分割成思考过程和最终回答
    let parts: Vec<&str> = markdown.split("<|start_header|>typeset_and_respond<|end_header|>").collect();
    
    // 设置 Comrak Markdown 转换选项
    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.render.unsafe_ = true; // 允许原始 HTML

    // 如果没有特殊标记或只有一个部分，直接转换整个 markdown
    if parts.len() <= 1 {
        return markdown_to_html(markdown, &options);
    }
    
    // 处理思考过程部分（除了最后一部分的所有内容）
    let mut result = String::new();
    
    // 收集所有中间思考过程
    let thinking_parts = &parts[0..parts.len() - 1];
    if !thinking_parts.is_empty() {
        let thinking_content = thinking_parts.join("<|start_header|>typeset_and_respond<|end_header|>");
        let html_thinking = markdown_to_html(&thinking_content, &options);
        let sanitized_html_thinking = clean(&html_thinking); // 清理HTML，进行转义

        result.push_str("<details class=\"thinking-details\">\n");
        result.push_str("<summary class=\"thinking-summary\">点击查看思考过程</summary>\n");
        result.push_str("<div class=\"thinking-content\">\n");
        result.push_str(&sanitized_html_thinking); // 使用清理后的HTML
        result.push_str("\n</div>\n");
        result.push_str("</details>\n\n");

    }
    
    // 处理最终回答（最后一部分）
    let final_answer = parts[parts.len() - 1];
    let html_answer = markdown_to_html(final_answer, &options);
    result.push_str(&html_answer);
    
    result
}
