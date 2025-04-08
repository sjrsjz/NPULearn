use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tauri::Manager;
use std::collections::HashMap;
use std::sync::Mutex;

use tauri_plugin_fs::FsExt;


mod document_renderer;
use document_renderer::renderer::convert_markdown_with_latex;
use document_renderer::style::MarkdownStyle;

mod aibackend;

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

/**
获取当前聊天内容
*/
#[tauri::command]
fn get_chat_html() -> String {
    let current_id = *CURRENT_CHAT_ID.lock().unwrap();
    let history = CHAT_HISTORY.lock().unwrap();

    if let Some(chat) = history.get(&current_id) {
        chat.content.clone()
    } else {
        let style_css = MarkdownStyle::Default.to_css();
        // 默认欢迎消息
        let html = format!(
            r#"

        <div class="chat-message system">
            <div class="message-content">
                <p>👋 你好！我是 AI 助手。请问有什么我可以帮助你的？</p>
            </div>
            <div class="message-time">今天 12:00</div>
        </div>

        {}
        "#,
            style_css
        );
        html.to_string()
    }
}

/*
创建新对话
*/
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
# Markdown 综合测试文档

这个文档涵盖了 Markdown 的各种元素和格式，同时也包含了数学公式渲染测试。

## 基本文本格式

普通文本段落。这是正常的文本内容，不带任何特殊格式。

*这是斜体文本* 和 _这也是斜体文本_

**这是粗体文本** 和 __这也是粗体文本__

***这是粗斜体文本*** 和 ___这也是粗斜体文本___

~~这是删除线文本~~

`这是行内代码`

这是<sub>下标</sub>和<sup>上标</sup>文本

## 引用

> 这是一个普通引用
> 
> 引用可以包含多行文本
>
>> 这是嵌套引用
>
> 引用回到第一层

## 列表

### 无序列表

* 列表项 1
* 列表项 2
  * 嵌套列表项 2.1
  * 嵌套列表项 2.2
* 列表项 3

### 有序列表

1. 第一项
2. 第二项
   1. 嵌套有序列表项 2.1
   2. 嵌套有序列表项 2.2
3. 第三项

### 任务列表

- [x] 已完成任务
- [ ] 未完成任务
- [ ] 另一个未完成任务

## 代码块

```javascript
// 这是一个JavaScript代码块
function helloWorld() {
  console.log("Hello, world!");
  return true;
}

// 包含一些特殊字符: /*${}+*/&^%#@!
const obj = { key: "value" };
```

```python
# 这是一个Python代码块
def fibonacci(n):
    if n <= 1:
        return n
    else:
        return fibonacci(n-1) + fibonacci(n-2)
        
# 测试一些中文注释和特殊符号
print("Hello, 世界!") # 输出问候语
```

```css
/* CSS 示例 */
.container {
  display: flex;
  flex-direction: column;
  align-items: center;
  background-color: #f0f0f0;
}

body {
  font-family: 'Arial', sans-serif;
  line-height: 1.6;
}
```

## 表格

| 左对齐 | 居中对齐 | 右对齐 |
| :--- | :---: | ---: |
| 单元格 1 | 单元格 2 | 单元格 3 |
| 较长的文本 | 文本 | 123.45 |
| 中文内容测试 | こんにちは | Привет |

## 水平线

---

## 链接

[Markdown 指南](https://www.markdownguide.org)

自动链接: https://example.com

## 图片

![示例图片](https://th.bing.com/th/id/OIP.oY0A5dYBc71GSk8z4gHMrAHaHa?rs=1&pid=ImgDetMain)

## 数学公式测试

### 行内公式

质能方程: $E=mc^2$

欧拉公式: $e^{i\pi} + 1 = 0$

求和公式: $\sum_{i=1}^{n} i = \frac{n(n+1)}{2}$

### 块级公式

贝叶斯公式:

$$P(A|B) = \frac{P(B|A) \cdot P(A)}{P(B)}$$

泰勒展开式:

$$f(x) = \sum_{n=0}^{\infty} \frac{f^{(n)}(a)}{n!} (x-a)^n$$

高斯分布公式:

$$f(x) = \frac{1}{\sigma\sqrt{2\pi}} e^{-\frac{1}{2}\left(\frac{x-\mu}{\sigma}\right)^2}$$

矩阵示例:

$$
\begin{bmatrix}
a & b & c \\
d & e & f \\
g & h & i
\end{bmatrix}
$$

连分数:

$$
x = a_0 + \cfrac{1}{a_1 + \cfrac{1}{a_2 + \cfrac{1}{a_3 + \cdots}}}
$$

## 嵌套元素测试

> 这是一个包含**粗体文本**和*斜体文本*的引用
> 
> - 引用中的列表项 1
> - 引用中的列表项 2
>
> ```python
> # 引用中的代码块
> print("Hello from quote")
> ```
>
> 引用中的数学公式: $\int_{a}^{b} f(x) dx$

## 特殊字符测试

HTML字符实体: &copy; &trade; &reg; &euro; &yen; &pound;

特殊符号: © ™ ® € ¥ £ § ¶ † ‡ ¤ ☺ ☻ ♠ ♣ ♥ ♦ ★ ☆

## 混合中英文测试

这是一段中英文混合的文本，Testing mixed Chinese and English text。包含一些**粗体**和*斜体*格式。

这里是一个公式 $f(x) = \sin(x) + \cos(x)$ 混合在中文段落中。

## 脚注测试

这是一个包含脚注的段落[^1]。

[^1]: 这是脚注的内容。> # 引用中的代码块
> print("Hello from quote")
> ```
>
> 引用中的数学公式: $\int_{a}^{b} f(x) dx$

## 特殊字符测试

HTML字符实体: &copy; &trade; &reg; &euro; &yen; &pound;

特殊符号: © ™ ® € ¥ £ § ¶ † ‡ ¤ ☺ ☻ ♠ ♣ ♥ ♦ ★ ☆

## 混合中英文测试

这是一段中英文混合的文本，Testing mixed Chinese and English text。包含一些**粗体**和*斜体*格式。

这里是一个公式 $f(x) = \sin(x) + \cos(x)$ 混合在中文段落中。

## 脚注测试

这是一个包含脚注的段落[^1]。

[^1]: 这是脚注的内容。
    
    "#;

    let converted_markdown = convert_markdown_with_latex(test_markdown);

    let style_css = MarkdownStyle::Default.to_css();

    // 构建用户消息和AI回复的HTML
    let html = format!(
        r#"

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

        {}

    "#,
        message, today, converted_markdown, today, style_css,
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
            process_message,
            aibackend::apikey::get_api_key_list_or_create,
            aibackend::apikey::try_save_api_key_list,
        ])
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // allowed the given directory
            let scope = app.fs_scope();
            let path = app.path();
            let mut checked_app_local_data_dir = None;
            let mut checked_app_config_dir = None;
            if let Ok(app_local_data_dir) = path.app_local_data_dir() {
                println!("app_local_data_dir: {:?}", app_local_data_dir);
                let result = scope.allow_directory(&app_local_data_dir, false);
                if let Err(e) = result {
                    eprintln!("Failed to allow directory: {}", e);
                }
                checked_app_local_data_dir = Some(app_local_data_dir);
            } else {
                eprintln!("Failed to get app_local_data_dir");
            }
            
            if let Ok(app_config_dir) = path.app_config_dir() {
                println!("app_config_dir: {:?}", app_config_dir);
                let result = scope.allow_directory(&app_config_dir, false);
                if let Err(e) = result {
                    eprintln!("Failed to allow directory: {}", e);
                }
                checked_app_config_dir = Some(app_config_dir);
            } else {
                eprintln!("Failed to get app_config_dir");
            }

            aibackend::apikey::init(app.handle().clone(), checked_app_local_data_dir.unwrap(), checked_app_config_dir.unwrap());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
