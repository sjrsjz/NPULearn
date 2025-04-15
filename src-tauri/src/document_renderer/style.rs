pub enum MarkdownStyle {
    Default,
}

impl MarkdownStyle {
    pub fn to_css(&self) -> String {
        match self {
            // 返回一个空的样式标签，样式将由前端提供
            MarkdownStyle::Default => String::from("<style>/* 样式由前端提供 */</style>"),
        }
    }
}
