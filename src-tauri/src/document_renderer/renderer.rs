use comrak::{markdown_to_html, ComrakOptions};

pub fn convert_markdown_with_latex(markdown: &str) -> String {
    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.render.unsafe_ = true; // 允许原始 HTML

    markdown_to_html(markdown, &options)
}
