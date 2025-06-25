use chrono;
use futures_util::StreamExt;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

use crate::aibackend::apikey::{ApiKey, ApiKeyType};
use crate::aibackend::interface::AIChat;
use crate::aibackend::template::{self, cot_template, TypesetInfo};
use crate::ChatHistory;

const COZE_API_URL: &str = "https://api.coze.cn/v3/chat";
const COZE_API_KEY: &str =
    "Bearer pat_ZlIJuCqHN4RZwpfZv3dVSBfi9bbZrDXJ7P5Kp1j4GI2Vk5IQSfN3r8wH9FeULFyl";
const BOT_ID: &str = "7517194614005055523";
const USER_ID: &str = "7510127542079569960";

// 定义 Coze API 请求结构
#[derive(Debug, Serialize, Deserialize)]
pub struct CozeRequest {
    bot_id: String,
    user_id: String,
    stream: bool,
    auto_save_history: bool,
    additional_messages: Vec<CozeMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CozeMessage {
    role: String,
    content: String,
    content_type: String,
}

// 定义 Coze API 响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct CozeResponse {
    code: i32,
    msg: String,
    data: CozeData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CozeData {
    id: String,
    conversation_id: String,
    bot_id: String,
    created_at: i64,
    completed_at: Option<i64>,
    failed_at: Option<i64>,
    meta_data: Option<Value>,
    last_error: Option<Value>,
    status: String,
    required_action: Option<Value>,
    usage: Option<Value>,
}

// 定义 Coze 流式响应的消息 delta 结构
#[derive(Debug, Serialize, Deserialize)]
pub struct CozeMessageDelta {
    id: String,
    conversation_id: String,
    bot_id: String,
    role: String,
    r#type: String,
    content: String,
    content_type: String,
    chat_id: String,
    section_id: String,
}

// 定义消息列表响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageListResponse {
    code: i32,
    msg: String,
    data: Vec<MessageData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageData {
    id: String,
    conversation_id: String,
    bot_id: String,
    chat_id: String,
    meta_data: Option<Value>,
    role: String,
    content: String,
    content_type: String,
    created_at: i64,
    updated_at: i64,
    r#type: String,
}

// 实现 CozeChat 结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct CozeChat {
    #[serde(skip)]
    client: reqwest::Client,
    api_key: String,
    // AIChat trait 需要的字段
    conversation_history: Vec<CozeMessage>,
    last_prompt: Option<String>,
    system_prompt: Option<String>,
    parameters: HashMap<String, String>,
    chat_id: u32,
    title: Option<String>,
    time: String,
}

impl CozeChat {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: COZE_API_KEY.to_string(),
            conversation_history: Vec::new(),
            last_prompt: None,
            system_prompt: None,
            parameters: HashMap::new(),
            chat_id: 0,
            title: None,
            time: chrono::Local::now().format("%H:%M").to_string(),
        }
    }

    /// 构建系统指令，包含排版格式提示词
    fn build_system_instruction(&self) -> String {
        let base_prompt = self.system_prompt.clone().unwrap_or_else(|| "You are a helpful assistant".to_string());
        
        // 使用 COT 模板，包含所有排版功能
        cot_template(&[
            TypesetInfo {
                name: "mermaid_render".to_string(),
                description: "render mermaid graph".to_string(),
                detail: "render mermaid graph by using mermaid.js renderer, should write down CORRECT mermaid code for sucessfully rendering".to_string(),
                args: {
                    let mut args = HashMap::new();
                    args.insert("mermaid_code".to_string(), Value::String("mermaid code which you what to render".to_string()));
                    args
                },
            },
            TypesetInfo {
                name: "pintora_render".to_string(),
                description: "render pintora graph".to_string(),
                detail: "render pintora graph by using pintora.js renderer, should write down CORRECT pintora code for sucessfully rendering".to_string(),
                args: {
                    let mut args = HashMap::new();
                    args.insert("diagram".to_string(), Value::String("pintora code which you what to render".to_string()));
                    args.insert("scale".to_string(), Value::Number(1.into()));
                    args
                },
            },
            TypesetInfo {
                name: "interactive_button".to_string(),
                description: "show a interactive button signed `message`, when user clicks on it, then you will receive `command` text".to_string(),
                detail: r#"show a interactive button signed `message`, when user clicks on it, then you will receive `command` text
    It is a good way for you to show a button for user to click when user learns something new
    - `message`: the text which you want to show on the button
    - `command`: the text which will be sent when user clicks the button
    > You can use it to give some hints to user, like "click me to send `Hello!`" or "click me to send `Bye!`"
    "#.to_string(),
                args: {
                    let mut args = HashMap::new();
                    args.insert("message".to_string(), Value::String("click me to send `Hello!`".to_string()));
                    args.insert("command".to_string(), Value::String("Hello!".to_string()));
                    args
                },
            },
            TypesetInfo {
                name: "typst_render".to_string(),
                description: "render typst document".to_string(),
                detail: "render typst document by using typst.ts renderer, should write down CORRECT typst code for successfully rendering mathematical formulas, diagrams, and professional documents".to_string(),
                args: {
                    let mut args = HashMap::new();
                    args.insert("typst_code".to_string(), Value::String("typst code which you want to render".to_string()));
                    args
                },
            },
            TypesetInfo {
                name: "html_render".to_string(),
                description: "render HTML content in a sandboxed environment".to_string(),
                detail: r#"render HTML content safely in a sandboxed iframe, which tolerates malformed HTML without affecting the page layout
    - `html`: the HTML content to render
    - `title`: optional title for the HTML container (default: "HTML内容")
    - `show_border`: optional boolean to show/hide border (default: true)"#.to_string(),
                args: {
                    let mut args = HashMap::new();
                    args.insert("html".to_string(), Value::String("<div>Your HTML content here</div>".to_string()));
                    args.insert("title".to_string(), Value::String("HTML内容".to_string()));
                    args.insert("show_border".to_string(), Value::Bool(true));
                    args
                },
            },
            TypesetInfo {
                name: "katex_render".to_string(),
                description: "render mathematical formulas".to_string(),
                detail: "render mathematical formulas by using katex renderer, should write down CORRECT latex code for successfully rendering mathematical formulas. No need to wrap by `$`. Be careful with backslashes: use double backslashes (\\\\) for commands like \\\\alpha instead of \\alpha, as single backslashes may be interpreted as escape characters (e.g., \\n becomes a newline).".to_string(),
                args: {
                    let mut args = HashMap::new();
                    args.insert("katex_code".to_string(), Value::String("Katex code which you want to render. No need to wrap by `$`. Remember to escape backslashes properly.".to_string()));
                    args
                },
            },
            TypesetInfo {
                name: "wolfram_alpha_compute".to_string(),
                description: "compute queries using Wolfram Alpha".to_string(),
                detail: r#"compute mathematical expressions, solve equations, convert units, and answer factual questions using Wolfram Alpha's computational engine
    - `query`: the query to compute (e.g., mathematical expressions, word problems, unit conversions)
    - `image_only`: optional boolean to return only image result (default: false)
    - `format`: optional format for results, only `html` avaliable"#.to_string(),
                args: {
                    let mut args = HashMap::new();
                    args.insert("query".to_string(), Value::String("1+1".to_string()));
                    args.insert("image_only".to_string(), Value::Bool(false));
                    args.insert("format".to_string(), Value::String("html".to_string()));
                    args
                },
            },
        ], &base_prompt)
    }    // 发送流式对话请求
    pub async fn send_stream_request<F>(
        &self,
        message: &str,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static,
    {
        // 构建包含系统指令的消息数组
        let mut messages = vec![];
        
        // 添加系统指令
        if !self.conversation_history.is_empty() || self.system_prompt.is_some() {
            messages.push(CozeMessage {
                role: "assistant".to_string(),
                content: format!(
                    "# I have double checked that my basic system settings are as follows, I will never disobey them:\n{}\n",
                    self.build_system_instruction()
                ),
                content_type: "text".to_string(),
            });
        }
          // 添加历史对话
        for msg in &self.conversation_history {
            messages.push(msg.clone());
        }
        
        // 添加当前用户消息
        messages.push(CozeMessage {
            role: "user".to_string(),
            content: message.to_string(),
            content_type: "text".to_string(),
        });

        let request_body = CozeRequest {
            bot_id: BOT_ID.to_string(),
            user_id: USER_ID.to_string(),
            stream: true, // 启用流式请求
            auto_save_history: false,
            additional_messages: messages,
        };

        let response = self
            .client
            .post(COZE_API_URL)
            .header("Content-Type", "application/json")
            .header("Authorization", &self.api_key)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(format!("Request failed with status {}: {}", status, error_text).into());
        }

        // 使用新的 Coze SSE 流式处理函数
        let response_text = process_coze_stream_response(response, callback).await?;
        
        // 应用模板提取
        let final_text = if let Some(extracted) = template::extract_response(&response_text) {
            extracted
        } else {
            response_text
        };
        
        Ok(final_text)
    }// 获取对话结果
}

// AIChat trait 将在 interface.rs 中通过其他方式实现

/// 过滤系统元数据，只保留用户友好的内容
fn filter_system_metadata(content: &str) -> String {
    // 检查是否包含系统元数据 JSON
    if content.contains("{\"msg_type\":") {
        // 尝试分离用户内容和系统元数据
        if let Some(json_start) = content.find("{\"msg_type\":") {
            // 取JSON之前的内容作为用户回答
            let user_content = &content[..json_start];
            return user_content.trim().to_string();
        }
    }

    // 检查是否是纯系统元数据（以 { 开头的JSON）
    let trimmed = content.trim();
    if trimmed.starts_with("{") && trimmed.contains("\"msg_type\"") {
        // 这是纯系统元数据，返回空字符串
        return String::new();
    }

    // 没有检测到系统元数据，返回原内容
    content.to_string()
}

/// 解析 Coze SSE 流式响应
async fn process_coze_stream_response<F>(
    response: reqwest::Response,
    mut callback: F,
) -> Result<String, Box<dyn Error>>
where
    F: FnMut(String) + Send + 'static,
{
    let mut stream = response.bytes_stream();
    let mut full_response = String::new();
    let mut has_received_data = false;
    let mut current_event_type = String::new();

    println!("Starting Coze SSE stream processing...");

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                has_received_data = true;
                let chunk_str = String::from_utf8_lossy(&chunk);
                println!("Received raw chunk: {}", chunk_str);

                // 处理 SSE 格式的流式响应
                for line in chunk_str.lines() {
                    let line = line.trim();

                    // 跳过空行和注释
                    if line.is_empty() || line.starts_with(':') {
                        continue;
                    }

                    // 处理事件类型
                    if line.starts_with("event:") {
                        current_event_type = line[6..].trim().to_string();
                        println!("Event type: {}", current_event_type);
                        continue;
                    }

                    // 处理数据
                    if line.starts_with("data:") {
                        let data = line[5..].trim();

                        // 跳过空数据和结束标记
                        if data.is_empty() || data == "[DONE]" {
                            continue;
                        }

                        println!(
                            "Processing data for event '{}': {}",
                            current_event_type, data
                        );
                        // 根据事件类型处理数据
                        match current_event_type.as_str() {
                            "conversation.chat.created" => {
                                println!("Chat created");
                            }
                            "conversation.chat.in_progress" => {
                                println!("Chat in progress");
                            }
                            "conversation.message.delta" => {
                                // 处理消息增量
                                if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                                    // 只处理 type 为 "answer" 的消息，过滤掉 "verbose" 等系统消息
                                    if let Some(msg_type) =
                                        json_data.get("type").and_then(|t| t.as_str())
                                    {
                                        if msg_type != "answer" {
                                            println!(
                                                "Skipping non-answer message type: {}",
                                                msg_type
                                            );
                                            continue; // 跳过非回答类型的消息
                                        }
                                    }

                                    // 提取增量内容
                                    if let Some(content) =
                                        json_data.get("content").and_then(|c| c.as_str())
                                    {
                                        if !content.is_empty() {
                                            // 过滤掉包含系统元数据的内容
                                            let filtered_content = filter_system_metadata(content);
                                            if !filtered_content.is_empty() {
                                                println!(
                                                    "Extracted delta content: {}",
                                                    filtered_content
                                                );
                                                callback(filtered_content.clone());
                                                full_response.push_str(&filtered_content);
                                            }
                                        }
                                    }
                                    // 也可能在 data 字段中
                                    else if let Some(data_obj) = json_data.get("data") {
                                        if let Some(content) =
                                            data_obj.get("content").and_then(|c| c.as_str())
                                        {
                                            if !content.is_empty() {
                                                let filtered_content =
                                                    filter_system_metadata(content);
                                                if !filtered_content.is_empty() {
                                                    println!(
                                                        "Extracted nested delta content: {}",
                                                        filtered_content
                                                    );
                                                    callback(filtered_content.clone());
                                                    full_response.push_str(&filtered_content);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "conversation.message.completed" => {
                                println!("Message completed - stopping content output to frontend");
                                // 消息完成后停止输出到前端，但继续处理直到聊天完成
                                break;
                            }
                            "conversation.chat.completed" => {
                                println!("Chat completed");
                                // 可能包含最终状态信息
                                if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                                    if let Some(status) =
                                        json_data.get("status").and_then(|s| s.as_str())
                                    {
                                        println!("Final status: {}", status);
                                    }
                                }
                                break;
                            }
                            "conversation.chat.failed" => {
                                println!("Chat failed");
                                if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                                    if let Some(error) = json_data.get("last_error") {
                                        return Err(format!("Chat failed: {}", error).into());
                                    }
                                }
                                break;
                            }
                            _ => {
                                // 尝试通用解析
                                if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                                    // 检查是否包含内容
                                    if let Some(content) =
                                        json_data.get("content").and_then(|c| c.as_str())
                                    {
                                        if !content.is_empty() {
                                            println!("Extracted generic content: {}", content);
                                            callback(content.to_string());
                                            full_response.push_str(content);
                                        }
                                    }

                                    // 检查状态变化（仅处理失败状态）
                                    if let Some(status) =
                                        json_data.get("status").and_then(|s| s.as_str())
                                    {
                                        match status {
                                            "failed" => {
                                                println!("Status failed");
                                                if let Some(error) = json_data.get("last_error") {
                                                    return Err(format!(
                                                        "Status failed: {}",
                                                        error
                                                    )
                                                    .into());
                                                }
                                                break;
                                            }
                                            _ => {
                                                println!("Status: {}", status);
                                            }
                                        }
                                    }
                                } else {
                                    println!(
                                        "Failed to parse JSON data for event '{}': {}",
                                        current_event_type, data
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("Stream error: {}", e);
                return Err(format!("Stream error: {}", e).into());
            }
        }
    }

    if full_response.is_empty() && has_received_data {
        println!("Warning: Received data but couldn't extract text");
        return Ok("(Response received but requires different format parsing)".to_string());
    } else if full_response.is_empty() {
        return Err("No text generated from the stream".into());
    }
    println!("Completed Coze stream response: {}", full_response);
    Ok(full_response)
}

// AIChat trait implementation for CozeChat
impl AIChat for CozeChat {
    async fn generate_response_stream<F>(
        &mut self,
        api_key: ApiKey,
        prompt: String,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static,
    {
        if api_key.key_type != ApiKeyType::Coze {
            return Err("Invalid API key type for Coze".into());
        }

        // 保存当前提示
        self.last_prompt = Some(prompt.clone());

        // 添加到对话历史
        self.conversation_history.push(CozeMessage {
            role: "user".to_string(),
            content: prompt.clone(),
            content_type: "text".to_string(),
        });

        // 使用流式API
        let response = self.send_stream_request(&prompt, callback).await?;

        // 添加助手响应到历史
        self.conversation_history.push(CozeMessage {
            role: "assistant".to_string(),
            content: response.clone(),
            content_type: "text".to_string(),
        });

        Ok(response)
    }

    async fn regenerate_response_stream<F>(
        &mut self,
        api_key: ApiKey,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static,
    {
        if api_key.key_type != ApiKeyType::Coze {
            return Err("Invalid API key type for Coze".into());
        }

        let last_prompt = self.withdraw_response()?;
        self.generate_response_stream(api_key, last_prompt, callback)
            .await
    }

    fn withdraw_response(&mut self) -> Result<String, Box<dyn Error>> {
        // 移除最后的助手响应，返回最后的用户提示
        if let Some(last_message) = self.conversation_history.last() {
            if last_message.role == "assistant" {
                self.conversation_history.pop();
            }
        }

        // 返回最后保存的提示或从历史中提取
        if let Some(prompt) = &self.last_prompt {
            Ok(prompt.clone())
        } else if let Some(last_user_msg) = self
            .conversation_history
            .iter()
            .rev()
            .find(|msg| msg.role == "user")
        {
            Ok(last_user_msg.content.clone())
        } else {
            Err("No previous prompt found".into())
        }
    }

    fn clear_context(&mut self) -> Result<String, Box<dyn Error>> {
        self.conversation_history.clear();
        self.last_prompt = None;
        Ok("Context cleared".to_string())
    }

    fn set_system_prompt(&mut self, prompt: String) -> Result<String, Box<dyn Error>> {
        self.system_prompt = Some(prompt.clone());
        Ok(format!("System prompt set: {}", prompt))
    }

    fn set_parameter(&mut self, key: String, value: String) -> Result<(), Box<dyn Error>> {
        self.parameters.insert(key, value);
        Ok(())
    }
    fn serialize(&self) -> String {
        // 使用显式的trait调用避免与serde的Serialize trait冲突
        serde_json::to_string(self).unwrap_or_else(|e| {
            eprintln!("Coze serialization error: {}", e);
            "{}".to_string()
        })
    }

    fn deserialize(&mut self, data: String) -> Result<(), Box<dyn Error>> {
        let chat: CozeChat = serde_json::from_str(&data)?;
        *self = chat;
        Ok(())
    }

    fn load_from(&mut self, chat_history: &ChatHistory) -> Result<(), Box<dyn Error>> {
        self.chat_id = chat_history.id;
        self.title = chat_history.title.clone();
        self.time = chat_history.time.clone();

        // 清空现有历史
        self.conversation_history.clear(); // 转换ChatHistory中的消息为CozeMessage
        for message in &chat_history.content {
            let role = match message.msgtype {
                crate::ChatMessageType::User => "user",
                crate::ChatMessageType::Assistant => "assistant",
                crate::ChatMessageType::System => "system", // 添加对系统消息的支持
            };

            self.conversation_history.push(CozeMessage {
                role: role.to_string(),
                content: message.content.clone(),
                content_type: "text".to_string(),
            });
        }

        Ok(())
    }

    fn save_to(&self) -> Result<ChatHistory, Box<dyn Error>> {
        let mut chat_messages = Vec::new(); // 转换CozeMessage为ChatMessage
        for message in &self.conversation_history {
            let msgtype = match message.role.as_str() {
                "user" => crate::ChatMessageType::User,
                "assistant" => crate::ChatMessageType::Assistant,
                "system" => crate::ChatMessageType::System,
                _ => continue, // 跳过未知类型
            };

            chat_messages.push(crate::ChatMessage {
                msgtype,
                time: chrono::Local::now().format("%H:%M").to_string(),
                content: message.content.clone(),
            });
        }

        Ok(ChatHistory {
            id: self.chat_id,
            title: self.title.clone(),
            time: self.time.clone(),
            content: chat_messages,
        })
    }

    async fn execute_tool_call(
        &mut self,
        _tool_name: String,
        _args: String,
    ) -> Result<String, Box<dyn Error>> {
        // Coze bot可能有内置工具，但这里暂时不支持直接工具调用
        Err("Direct tool execution is not supported for Coze. Tools are handled by the bot.".into())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    #[test]
    fn test_network_request_serialization() {
        // 测试网络请求体序列化
        let request_body = CozeRequest {
            bot_id: BOT_ID.to_string(),
            user_id: USER_ID.to_string(),
            stream: true,
            auto_save_history: false,
            additional_messages: vec![CozeMessage {
                role: "user".to_string(),
                content: "Test network request".to_string(),
                content_type: "text".to_string(),
            }],
        };

        let serialized = serde_json::to_string(&request_body).unwrap();
        assert!(serialized.contains("Test network request"));
        assert!(serialized.contains(BOT_ID));
        assert!(serialized.contains(USER_ID));
        assert!(serialized.contains("\"stream\":true"));

        println!("Network request serialization test passed");
    }

    #[test]
    fn test_http_headers_configuration() {
        // 测试 HTTP 请求头配置
        let chat = CozeChat::new();
        assert!(!chat.api_key.is_empty());
        assert!(chat.api_key.starts_with("Bearer "));

        // 验证 API URL 格式
        assert!(COZE_API_URL.starts_with("https://"));
        assert!(COZE_API_URL.contains("api.coze.cn"));

        println!("HTTP headers configuration test passed");
    }
    #[test]
    fn test_network_response_parsing() {
        // 测试网络响应解析 - Coze API 响应格式
        let mock_response = r#"{
            "code": 0,
            "msg": "success",
            "data": {
                "id": "test_id",
                "conversation_id": "test_conv_id",
                "bot_id": "test_bot_id",
                "created_at": 1640995200,
                "completed_at": null,
                "failed_at": null,
                "meta_data": null,
                "last_error": null,
                "status": "created",
                "required_action": null,
                "usage": null
            }
        }"#;

        let parsed: Result<CozeResponse, _> = serde_json::from_str(mock_response);
        assert!(parsed.is_ok());

        let response = parsed.unwrap();
        assert_eq!(response.code, 0);
        assert_eq!(response.msg, "success");
        assert_eq!(response.data.id, "test_id");
        assert_eq!(response.data.status, "created");

        println!("Network response parsing test passed");
    }

    #[test]
    fn test_message_list_network_response() {
        // 测试消息列表网络响应解析
        let mock_response = r#"{
            "code": 0,
            "msg": "success",
            "data": [{
                "id": "msg_id",
                "conversation_id": "conv_id",
                "bot_id": "bot_id",
                "chat_id": "chat_id",
                "meta_data": null,
                "role": "assistant",
                "content": "Hello, how can I help you?",
                "content_type": "text",
                "created_at": 1640995200,
                "updated_at": 1640995200,
                "type": "answer"
            }]
        }"#;

        let parsed: Result<MessageListResponse, _> = serde_json::from_str(mock_response);
        assert!(parsed.is_ok());

        let response = parsed.unwrap();
        assert_eq!(response.code, 0);
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].role, "assistant");
        assert_eq!(response.data[0].content, "Hello, how can I help you?");
        assert_eq!(response.data[0].r#type, "answer");
        println!("Message list network response parsing test passed");
    }

    #[tokio::test]
    async fn test_raw_network_response() {
        // 测试原始网络响应，查看具体的返回内容
        println!("开始测试原始网络响应...");

        let chat = CozeChat::new();
        let test_message = "Hello";

        let request_body = CozeRequest {
            bot_id: BOT_ID.to_string(),
            user_id: USER_ID.to_string(),
            stream: true,
            auto_save_history: false,
            additional_messages: vec![CozeMessage {
                role: "user".to_string(),
                content: test_message.to_string(),
                content_type: "text".to_string(),
            }],
        };

        println!("发送请求...");
        let response = chat
            .client
            .post(COZE_API_URL)
            .header("Content-Type", "application/json")
            .header("Authorization", &chat.api_key)
            .json(&request_body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                println!("✅ HTTP 请求成功!");
                println!("HTTP 状态码: {}", resp.status());
                println!("HTTP 响应头:");
                for (key, value) in resp.headers() {
                    println!("  {}: {:?}", key, value);
                }

                match resp.text().await {
                    Ok(body) => {
                        println!("✅ 响应体内容:");
                        println!("{}", body);

                        // 尝试解析为 JSON
                        match serde_json::from_str::<serde_json::Value>(&body) {
                            Ok(json) => {
                                println!("✅ JSON 解析成功:");
                                println!("{:#}", json);
                            }
                            Err(e) => {
                                println!("❌ JSON 解析失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ 读取响应体失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("❌ HTTP 请求失败: {}", e);
            }
        }

        println!("原始网络响应测试完成!");
    }

    #[test]
    fn test_url_construction() {
        // 测试 URL 构造
        let conversation_id = "test_conv_123";
        let chat_id = "test_chat_456";

        let message_list_url = format!(
            "https://api.coze.cn/v1/conversation/message/list?conversation_id={}",
            conversation_id
        );
        let status_check_url = format!(
            "https://api.coze.cn/v1/conversation/retrieve?conversation_id={}&chat_id={}",
            conversation_id, chat_id
        );

        println!("Message List URL: {}", message_list_url);
        println!("Status Check URL: {}", status_check_url);

        assert!(message_list_url.contains("conversation_id=test_conv_123"));
        assert!(status_check_url.contains("conversation_id=test_conv_123"));
        assert!(status_check_url.contains("chat_id=test_chat_456"));
        println!("URL construction test passed");
    }

    #[test]
    fn test_coze_aichat_trait_implementation() {
        // 测试 CozeChat 实现 AIChat trait 的基本功能
        let mut chat = CozeChat::new();

        // 测试系统提示设置
        let result = chat.set_system_prompt("You are a helpful assistant.".to_string());
        assert!(result.is_ok());
        assert!(chat.system_prompt.is_some());

        // 测试参数设置
        let param_result = chat.set_parameter("temperature".to_string(), "0.7".to_string());
        assert!(param_result.is_ok());
        assert!(chat.parameters.contains_key("temperature"));
        // 测试序列化和反序列化
        let serialized = AIChat::serialize(&chat);
        assert!(!serialized.is_empty());

        let mut new_chat = CozeChat::new();
        let deserialize_result = new_chat.deserialize(serialized);
        assert!(deserialize_result.is_ok());

        // 测试上下文清除
        let clear_result = chat.clear_context();
        assert!(clear_result.is_ok());
        assert!(chat.conversation_history.is_empty());

        println!("Coze AIChat trait implementation test passed");
    }
}
