pub enum MarkdownStyle {
    Default,
}

impl MarkdownStyle {
    pub fn to_css(&self) -> String {
        match self {
            MarkdownStyle::Default => r#"
<style>
    body {
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans", Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji";
        line-height: 1.5;
        color: #24292f;
        background-color: #ffffff;
        margin: 16px;
    }
    
    h1, h2, h3, h4, h5, h6 {
        margin-top: 24px;
        margin-bottom: 16px;
        font-weight: 600;
    }
    
    h1 { font-size: 2em; padding-bottom: .3em; border-bottom: 1px solid #eaecef; }
    h2 { font-size: 1.5em; padding-bottom: .3em; border-bottom: 1px solid #eaecef; }
    h3 { font-size: 1.25em; }
    h4 { font-size: 1em; }
    h5 { font-size: .875em; }
    h6 { font-size: .85em; color: #57606a; }
    
    a {
        color: #0969da;
        text-decoration: none;
    }
    
    a:hover {
        text-decoration: underline;
    }
    
    table {
        border-spacing: 0;
        border-collapse: collapse;
        margin: 16px 0;
        width: 100%;
    }
    
    th, td {
        padding: 6px 13px;
        border: 1px solid #d0d7de;
    }
    
    th {
        font-weight: 600;
        background-color: #f6f8fa;
    }
    
    tr:nth-child(2n) {
        background-color: #f6f8fa;
    }
    
    pre {
        padding: 16px;
        overflow: auto;
        font-size: 85%;
        line-height: 1.45;
        background-color: #f6f8fa;
        border-radius: 6px;
        margin: 16px 0;
        font-family: ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace;
    }
    
    code {
        padding: .2em .4em;
        margin: 0;
        font-size: 85%;
        background-color: rgba(175, 184, 193, 0.2);
        border-radius: 6px;
        font-family: ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace;
    }
    
    pre code {
        padding: 0;
        margin: 0;
        background-color: transparent;
    }
    
    blockquote {
        padding: 0 1em;
        color: #57606a;
        border-left: .25em solid #d0d7de;
        margin: 16px 0;
    }
    
    ul, ol {
        padding-left: 2em;
        margin: 16px 0;
    }
    
    img {
        max-width: 100%;
        height: auto;
        border-style: none;
        margin: 16px 0;
    }
    
    hr {
        height: .25em;
        padding: 0;
        margin: 24px 0;
        background-color: #d0d7de;
        border: 0;
    }
    
    input[type="checkbox"] {
        margin: 0 .2em .25em -1.4em;
    }
    
    del {
        color: #cf222e;
    }
    
    ins {
        color: #116329;
        text-decoration: none;
        background-color: #dafbe1;
    }
    
    mark {
        background-color: #fff8c5;
        color: #24292f;
    }
    
    .alert {
        padding: 16px;
        margin: 16px 0;
        border-radius: 6px;
        border: 1px solid;
    }
    
    .alert-info {
        color: #0969da;
        background-color: #ddf4ff;
        border-color: #54aeff;
    }
    
    .alert-warning {
        color: #9a6700;
        background-color: #fff8c5;
        border-color: #f3c666;
    }
    
    .alert-danger {
        color: #cf222e;
        background-color: #ffebe9;
        border-color: #ff8182;
    }
    
    .alert-success {
        color: #116329;
        background-color: #dafbe1;
        border-color: #4ac26b;
    }


    .chat-message {
        margin-bottom: 20px;
        animation: fadeIn 0.3s ease;
    }
    
    @keyframes fadeIn {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: translateY(0); }
    }
    
    .system {
        background-color: #f2f2f2;
        border-radius: 12px;
        padding: 12px 16px;
        max-width: 85%;
    }
    
    .user {
        background-color: #e1f5fe;
        border-radius: 12px;
        padding: 12px 16px;
        max-width: 85%;
        margin-left: auto;
    }
    
    .message-content {
        margin-bottom: 5px;
    }
    
    .message-time {
        font-size: 12px;
        color: #666;
        text-align: right;
    }
    
    @media (prefers-color-scheme: dark) {
        .system {
            background-color: #2d333b;
        }
        
        .user {
            background-color: #254254;
        }
        
        .message-time {
            color: #aaa;
        }
    }
</style>
            "#.to_string(),
        }
    }
}
