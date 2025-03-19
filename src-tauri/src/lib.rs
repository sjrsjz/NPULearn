use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

mod document_renderer;
use document_renderer::renderer::convert_markdown_with_latex;
use document_renderer::style::MarkdownStyle;

// 定义一个全局状态来存储聊天历史
static CHAT_HISTORY: Lazy<Mutex<HashMap<u32, ChatHistory>>> = Lazy::new(|| {
    let mut map = HashMap::new();

    Mutex::new(map)
});

// 定义当前活跃的对话ID
static CURRENT_CHAT_ID: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(1)); // 默认为对话1
static NEXT_CHAT_ID: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(4)); // 下一个新建对话的ID

// 聊天历史结构体
#[derive(Clone, Serialize, Deserialize)]
struct ChatHistory {
    id: u32,
    title: String,
    time: String,
    content: String,
}

// 聊天历史项目（不包含内容，用于列表展示）
#[derive(Clone, Serialize, Deserialize)]
struct ChatHistoryItem {
    id: u32,
    title: String,
    time: String,
}

// 获取聊天历史列表
#[tauri::command]
fn get_chat_history() -> Vec<ChatHistoryItem> {
    println!("{}", convert_markdown_with_latex("**Hello World!**"));
    let history = CHAT_HISTORY.lock().unwrap();
    let mut history_items: Vec<ChatHistoryItem> = history
        .values()
        .map(|h| ChatHistoryItem {
            id: h.id,
            title: h.title.clone(),
            time: h.time.clone(),
        })
        .collect();

    // 按ID排序，最新的在前面
    history_items.sort_by(|a, b| b.id.cmp(&a.id));
    history_items
}

// 获取指定ID的聊天内容
#[tauri::command]
fn get_chat_by_id(id: u32) -> String {
    let mut current_id = CURRENT_CHAT_ID.lock().unwrap();
    *current_id = id; // 更新当前对话ID

    let history = CHAT_HISTORY.lock().unwrap();
    if let Some(chat) = history.get(&id) {
        chat.content.clone()
    } else {
        "<div class=\"chat-message system\"><div class=\"message-content\"><p>未找到对话内容</p></div></div>".to_string()
    }
}

// 获取当前聊天内容
#[tauri::command]
fn get_chat_html() -> String {
    let current_id = *CURRENT_CHAT_ID.lock().unwrap();
    let history = CHAT_HISTORY.lock().unwrap();

    if let Some(chat) = history.get(&current_id) {
        chat.content.clone()
    } else {
        // 默认欢迎消息
        let html = r#"
        <div class="chat-message system">
            <div class="message-content">
                <p>👋 你好！我是 AI 助手。请问有什么我可以帮助你的？</p>
            </div>
            <div class="message-time">今天 12:00</div>
        </div>
        
        <style>
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
        "#;
        html.to_string()
    }
}

// 创建新对话
#[tauri::command]
fn create_new_chat() -> String {
    // 获取新ID
    let mut next_id = NEXT_CHAT_ID.lock().unwrap();
    let new_id = *next_id;
    *next_id += 1;

    // 更新当前对话ID
    let mut current_id = CURRENT_CHAT_ID.lock().unwrap();
    *current_id = new_id;

    // 创建新对话
    let now = chrono::Local::now();
    let today = now.format("%H:%M").to_string();

    let new_chat = ChatHistory {
        id: new_id,
        title: format!("新对话 {}", new_id),
        time: "刚刚".to_string(),
        content: format!(
            r#"
        <div class="chat-message system">
            <div class="message-content">
                <p>👋 你好！这是一个新对话。请问有什么我可以帮助你的？</p>
            </div>
            <div class="message-time">今天 {}</div>
        </div>
        
        <style>
            .chat-message {{
                margin-bottom: 20px;
                animation: fadeIn 0.3s ease;
            }}
            
            @keyframes fadeIn {{
                from {{ opacity: 0; transform: translateY(10px); }}
                to {{ opacity: 1; transform: translateY(0); }}
            }}
            
            .system {{
                background-color: #f2f2f2;
                border-radius: 12px;
                padding: 12px 16px;
                max-width: 85%;
            }}
            
            .user {{
                background-color: #e1f5fe;
                border-radius: 12px;
                padding: 12px 16px;
                max-width: 85%;
                margin-left: auto;
            }}
            
            .message-content {{
                margin-bottom: 5px;
            }}
            
            .message-time {{
                font-size: 12px;
                color: #666;
                text-align: right;
            }}
            
            @media (prefers-color-scheme: dark) {{
                .system {{
                    background-color: #2d333b;
                }}
                
                .user {{
                    background-color: #254254;
                }}
                
                .message-time {{
                    color: #aaa;
                }}
            }}
        </style>
        "#,
            today
        ),
    };

    let content = new_chat.content.clone();

    // 添加到历史记录
    let mut history = CHAT_HISTORY.lock().unwrap();
    history.insert(new_id, new_chat);

    content
}

// 处理用户消息
#[tauri::command]
fn process_message(message: &str) -> String {
    // 获取当前时间
    let now = chrono::Local::now();
    let today = now.format("%H:%M").to_string();

    // 获取当前对话ID
    let current_id = *CURRENT_CHAT_ID.lock().unwrap();

    let test_markdown = r#"
# 这是一个测试标题

## 这是一个测试副标题

这是一个测试列表：
- 这是一个测试列表项1
- 这是一个测试列表项2

```python
def hello_world():
    print("Hello, world!")
```

这是一个测试段落，包含一些**加粗文本**和*斜体文本*。

这是一个测试链接：[点击这里](https://example.com)。

这是一个测试图片：![测试图片](https://th.bing.com/th/id/OIP.oY0A5dYBc71GSk8z4gHMrAHaHa?rs=1&pid=ImgDetMain)

这是一个测试表格：

| 列1 | 列2 |
| ---- | ---- |
| 数据1 | 数据2 |

这是一个测试数学公式：$E=mc^2$。

这是一个测试数学块：

$$
E=mc^2
$$

这是一个测试数学块2：

$$
E=mc^2
$$

    
    "#;

    let converted_markdown = convert_markdown_with_latex(test_markdown);

    let style_css = MarkdownStyle::Default.to_css();

    // 构建用户消息和AI回复的HTML
    let html = format!(
        r#"
    {}

    <div class="chat-message user">
        <div class="message-content">
            <p>{}</p>
        </div>
        <div class="message-time">今天 {}</div>
    </div>
    
    <div class="chat-message system">
        <div class="message-content">
            {}
        </div>
        <div class="message-time">今天 {}</div>
    </div>
    
    <style>
        .chat-message {{
            margin-bottom: 20px;
            animation: fadeIn 0.3s ease;
        }}
        
        @keyframes fadeIn {{
            from {{ opacity: 0; transform: translateY(10px); }}
            to {{ opacity: 1; transform: translateY(0); }}
        }}
        
        .system {{
            background-color: #f2f2f2;
            border-radius: 12px;
            padding: 12px 16px;
            max-width: 85%;
        }}
        
        .user {{
            background-color: #e1f5fe;
            border-radius: 12px;
            padding: 12px 16px;
            max-width: 85%;
            margin-left: auto;
        }}
        
        .message-content {{
            margin-bottom: 5px;
        }}
        
        .message-time {{
            font-size: 12px;
            color: #666;
            text-align: right;
        }}
        
        @media (prefers-color-scheme: dark) {{
            .system {{
                background-color: #2d333b;
            }}
            
            .user {{
                background-color: #254254;
            }}
            
            .message-time {{
                color: #aaa;
            }}
        }}
    </style>
    "#,
        style_css, message, today, converted_markdown, today
    );

    // 更新当前对话的内容
    let mut history = CHAT_HISTORY.lock().unwrap();
    if let Some(chat) = history.get_mut(&current_id) {
        chat.content = html.clone();
    } else {
        // 如果当前对话不存在，创建一个新对话
        let new_chat = ChatHistory {
            id: current_id,
            title: format!("对话 {}", message.chars().take(10).collect::<String>()),
            time: "刚刚".to_string(),
            content: html.clone(),
        };
        history.insert(current_id, new_chat);
    }

    html
}

// 确保在 run 函数中注册所有命令
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_chat_html,
            get_chat_history,
            get_chat_by_id,
            create_new_chat,
            process_message
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
