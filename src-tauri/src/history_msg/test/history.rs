use super::super::history::{load_history, save_history};
use crate::history_msg::history::ChatHistory;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modify() {
        let mut history = load_history();
        println!("{:?}", history);

        let new_id = 2;
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
        history.insert(new_id, new_chat);
        save_history(history);
    }
}
