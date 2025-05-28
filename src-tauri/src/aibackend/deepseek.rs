use crate::aibackend::interface::AIChat;
use crate::aibackend::openai_types::{
    ChatCompletionMessage, Content, MessageRole, Tool, ToolCall, 
    ChatCompletionResponse, ChatCompletionStreamResponse,
};
use crate::aibackend::template::{self, cot_template, COT, TypesetInfo};
use crate::{ChatHistory, ChatMessage, ChatMessageType};
use futures_util::StreamExt;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::apikey::{ApiKey, ApiKeyType};

// --- Constants ---
const DEEPSEEK_API_BASE_URL: &str = "https://api.deepseek.com";

// 添加简化的消息结构用于 DeepSeek API
#[derive(Clone, Debug, Serialize)]
struct DeepSeekMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

// 添加简化的请求结构用于 DeepSeek API
#[derive(Clone, Debug, Serialize)]
struct DeepSeekRequest {
    model: String,
    messages: Vec<DeepSeekMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

// --- DeepSeek Chat Structure ---

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeepSeekChat {
    base_url: String,
    model: String,
    system_prompt: String,
    messages: Vec<ChatCompletionMessage>,
    temperature: f32,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    last_prompt: Option<String>,
    tools: Vec<Tool>,

    chat_id: u32,
    title: String,
    time: String,
}

// --- Helper Functions ---

/// 构建 DeepSeek API URL
fn build_deepseek_url(base_url: &str, endpoint: &str) -> String {
    format!("{}/{}", base_url, endpoint)
}

/// 将 ChatCompletionMessage 转换为 DeepSeekMessage
fn convert_to_deepseek_message(msg: &ChatCompletionMessage) -> DeepSeekMessage {
    let role_str = match msg.role {
        MessageRole::user => "user",
        MessageRole::assistant => "assistant",
        MessageRole::system => "system",
        MessageRole::function => "function",
        MessageRole::tool => "tool",
    }.to_string();

    let content_str = match &msg.content {
        Content::Text(text) => text.clone(),
    };

    DeepSeekMessage {
        role: role_str,
        content: content_str,
        name: msg.name.clone(),
        tool_calls: msg.tool_calls.clone(),
        tool_call_id: msg.tool_call_id.clone(),
    }
}

/// 解析流式响应块并通过回调函数返回文本
async fn process_deepseek_stream_response<F>(
    response: reqwest::Response,
    mut callback: F,
) -> Result<String, Box<dyn Error>>
where
    F: FnMut(String) + Send + 'static,
{
    let mut stream = response.bytes_stream();
    let mut full_response = String::new();
    let mut has_received_data = false;

    println!("Starting DeepSeek stream processing...");

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                has_received_data = true;
                let chunk_str = String::from_utf8_lossy(&chunk);
                println!("Received raw chunk: {}", chunk_str);

                // 处理 SSE 格式的流式响应
                for line in chunk_str.lines() {
                    if line.starts_with("data: ") {
                        let data = &line[6..]; // 移除 "data: " 前缀
                        
                        if data == "[DONE]" {
                            break;
                        }

                        if let Ok(json_data) = serde_json::from_str::<ChatCompletionStreamResponse>(data) {
                            if let Some(choice) = json_data.choices.first() {
                                if let Some(delta) = &choice.delta {
                                    if let Some(content) = &delta.content {
                                        if !content.is_empty() {
                                            println!("Extracted content: {}", content);
                                            callback(content.clone());
                                            full_response.push_str(content);
                                        }
                                    }
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

    println!("Completed DeepSeek stream response: {}", full_response);
    Ok(full_response)
}

#[allow(dead_code)]
impl DeepSeekChat {
    pub fn new() -> Self {
        DeepSeekChat {
            base_url: DEEPSEEK_API_BASE_URL.to_string(),
            model: "deepseek-chat".to_string(),
            system_prompt: "You are a helpful assistant".to_string(),
            messages: Vec::new(),
            temperature: 1.0,
            max_tokens: Some(4096),
            top_p: Some(0.95),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            last_prompt: None,
            tools: Vec::new(),
            chat_id: 0,
            title: "New Chat".to_string(),
            time: "".to_string(),
        }
    }

    fn build_system_instruction(&self) -> String {
        return cot_template(&[
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
        ], &self.system_prompt);
    }

    /// 构建请求体 - 修改为使用 DeepSeekRequest
    fn build_request_body(
        &self,
        messages: &[ChatCompletionMessage],
        tools: Option<&[Tool]>,
        stream: bool,
    ) -> DeepSeekRequest {
        let mut all_messages = Vec::new();

        // 添加系统消息
        if !self.system_prompt.is_empty() {
            all_messages.push(DeepSeekMessage {
                role: "system".to_string(),
                content: format!(
                    "# I have double checked that my basic system settings are as follows, I will never disobey them:\n{}\n",
                    self.build_system_instruction()
                ),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // 添加用户消息 - 转换为 DeepSeekMessage
        for msg in messages {
            all_messages.push(convert_to_deepseek_message(msg));
        }

        // 添加COT指令
        all_messages.push(DeepSeekMessage {
            role: "system".to_string(),
            content: format!(
                "# I have double checked that my basic COT settings are as follows:\n{}\nNow I will answer the user's request.\n",
                COT
            ),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        DeepSeekRequest {
            model: self.model.clone(),
            messages: all_messages,
            temperature: Some(self.temperature),
            max_tokens: self.max_tokens,
            top_p: self.top_p,
            frequency_penalty: self.frequency_penalty,
            presence_penalty: self.presence_penalty,
            tools: tools.map(|t| t.to_vec()),
            tool_choice: if tools.is_some() && !tools.unwrap().is_empty() {
                Some("auto".to_string())
            } else {
                None
            },
            stream: Some(stream),
        }
    }

    /// 核心流式处理函数 - 修改为使用 DeepSeekRequest
    async fn stream_request<F>(
        &self,
        request_body: DeepSeekRequest,
        api_key: &str,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static,
    {
        let client = reqwest::Client::new();
        let url = build_deepseek_url(&self.base_url, "chat/completions");
        println!("request_body: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default());        

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(format!("API request failed ({}): {}", status, error_text).into());
        }

        if request_body.stream == Some(true) {
            process_deepseek_stream_response(response, callback).await
        } else {
            let response_json: ChatCompletionResponse = response.json().await?;
            let text = response_json
                .choices
                .first()
                .and_then(|choice| match &choice.message.content {
                    Content::Text(text) => Some(text.clone()),
                })
                .unwrap_or_default();

            // 应用模板提取
            let final_text = if let Some(extracted) = template::extract_response(&text) {
                extracted
            } else {
                text
            };

            let mut callback_clone = callback;
            callback_clone(final_text.clone());
            Ok(final_text)
        }
    }

    /// 聊天请求 - 更新签名
    async fn chat_deepseek<F>(
        &self,
        api_key: &str,
        messages: &[ChatCompletionMessage],
        tools: Option<&[Tool]>,
        use_streaming: bool,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static,
    {
        let request_body = self.build_request_body(messages, tools, use_streaming);
        self.stream_request(request_body, api_key, callback).await
    }

    /// 简化的非流式聊天接口
    async fn chat_deepseek_simple(
        &self,
        api_key: &str,
        messages: &[ChatCompletionMessage],
        tools: Option<&[Tool]>,
    ) -> Result<String, Box<dyn Error>> {
        let full_response = Arc::new(std::sync::Mutex::new(String::new()));
        let response_clone = full_response.clone();

        let _ = self
            .chat_deepseek(
                api_key,
                messages,
                tools,
                false,
                move |chunk| {
                    let mut locked_response = response_clone.lock().unwrap();
                    locked_response.push_str(&chunk);
                },
            )
            .await?;

        let final_response = full_response.lock().unwrap().clone();
        Ok(final_response)
    }

    /// 解析工具调用响应 - 使用 DeepSeekRequest
    async fn parse_tool_calls(
        &self,
        api_key: &str,
        messages: &[ChatCompletionMessage],
        tools: &[Tool],
    ) -> Result<(Option<String>, Vec<ToolCall>), Box<dyn Error>> {
        let request_body = self.build_request_body(messages, Some(tools), false);
        let client = reqwest::Client::new();
        let url = build_deepseek_url(&self.base_url, "chat/completions");

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(format!("API request failed ({}): {}", status, error_text).into());
        }

        let response_json: ChatCompletionResponse = response.json().await?;
        
        if let Some(choice) = response_json.choices.first() {
            let text_response = match &choice.message.content {
                Content::Text(text) => Some(text.clone()),
            };
            let tool_calls = choice.message.tool_calls.clone().unwrap_or_default();
            
            Ok((text_response, tool_calls))
        } else {
            Err("No response from DeepSeek API".into())
        }
    }

    /// 通过工具调用实现聊天功能
    async fn chat_deepseek_with_tools(
        &self,
        api_key: &str,
        messages: &[ChatCompletionMessage],
        tools: &[Tool],
        tool_call_processor: impl Fn(
                String,
                HashMap<String, Value>,
            ) -> Pin<Box<dyn Future<Output = Result<String, Box<dyn Error>>> + Send>>
            + Send
            + Sync,
    ) -> Result<String, Box<dyn Error>> {
        let (initial_response_text, tool_calls) =
            self.parse_tool_calls(api_key, messages, tools).await?;

        if !tool_calls.is_empty() && !self.check_if_skip_tool_call(&tool_calls) {
            let mut current_messages = messages.to_vec();

            // 添加模型的回复
            if let Some(text) = initial_response_text {
                current_messages.push(ChatCompletionMessage {
                    role: MessageRole::assistant,
                    content: Content::Text(text),
                    name: None,
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                });
            }

            // 执行工具调用
            for tool_call in &tool_calls {
                let args: HashMap<String, Value> = if let Some(args_str) = &tool_call.function.arguments {
                    serde_json::from_str(args_str).unwrap_or_default()
                } else {
                    HashMap::new()
                };

                let result = tool_call_processor(tool_call.function.name.clone(), args).await?;

                current_messages.push(ChatCompletionMessage {
                    role: MessageRole::tool,
                    content: Content::Text(result),
                    name: Some(tool_call.function.name.clone()),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            }

            return self
                .chat_deepseek_simple(api_key, &current_messages, Some(tools))
                .await;
        } else if let Some(text) = initial_response_text {
            return Ok(text);
        } else {
            return Err("DeepSeek response contained neither text nor tool calls.".into());
        }
    }

    /// 检查是否需要跳过工具调用
    fn check_if_skip_tool_call(&self, tool_calls: &[ToolCall]) -> bool {
        tool_calls.iter().any(|call| call.function.name == "skip_tool_call")
    }
}

// --- AIChat Trait Implementation ---

impl AIChat for DeepSeekChat {
    fn withdraw_response(&mut self) -> Result<String, Box<dyn Error>> {
        // 移除所有尾部的非用户消息
        while let Some(message) = self.messages.last() {
            if message.role != MessageRole::user {
                self.messages.pop();
            } else {
                break;
            }
        }

        // 检查是否还有用户消息可以移除
        if let Some(last_message) = self.messages.last() {
            if last_message.role == MessageRole::user {
                let content = match &last_message.content {
                    Content::Text(text) => text.clone(),
                };

                self.messages.pop();

                // 更新 last_prompt
                self.last_prompt = self
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == MessageRole::user)
                    .and_then(|m| match &m.content {
                        Content::Text(text) => Some(text.clone()),
                    });

                return Ok(content);
            }
        }

        Err("No user message to withdraw.".into())
    }

    fn clear_context(&mut self) -> Result<String, Box<dyn Error>> {
        self.messages.clear();
        self.last_prompt = None;
        Ok("Context cleared".to_string())
    }

    fn set_system_prompt(&mut self, prompt: String) -> Result<String, Box<dyn Error>> {
        self.system_prompt = prompt;
        Ok("System prompt set. Consider clearing context if needed.".to_string())
    }

    fn set_parameter(&mut self, key: String, value: String) -> Result<(), Box<dyn Error>> {
        match key.as_str() {
            "temperature" => {
                self.temperature = value
                    .parse::<f32>()
                    .map_err(|e| format!("Invalid temperature value: {}", e))?
            }
            "max_tokens" => {
                self.max_tokens = Some(
                    value
                        .parse::<u32>()
                        .map_err(|e| format!("Invalid max_tokens value: {}", e))?,
                )
            }
            "top_p" => {
                self.top_p = Some(
                    value
                        .parse::<f32>()
                        .map_err(|e| format!("Invalid top_p value: {}", e))?,
                )
            }
            "frequency_penalty" => {
                self.frequency_penalty = Some(
                    value
                        .parse::<f32>()
                        .map_err(|e| format!("Invalid frequency_penalty value: {}", e))?,
                )
            }
            "presence_penalty" => {
                self.presence_penalty = Some(
                    value
                        .parse::<f32>()
                        .map_err(|e| format!("Invalid presence_penalty value: {}", e))?,
                )
            }
            "model" => self.model = value,
            _ => return Err(format!("Unknown parameter: {}", key).into()),
        }
        Ok(())
    }

    fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            eprintln!("Serialization error: {}", e);
            "{}".to_string()
        })
    }

    fn deserialize(&mut self, data: String) -> Result<(), Box<dyn Error>> {
        let chat: DeepSeekChat = serde_json::from_str(&data)?;
        *self = chat;
        Ok(())
    }

    async fn execute_tool_call(
        &mut self,
        _tool_name: String,
        _args: String,
    ) -> Result<String, Box<dyn Error>> {
        Err(
            "Direct tool execution via this method is not implemented for the current DeepSeek flow."
                .into(),
        )
    }

    async fn generate_response_stream<F>(
        &mut self,
        api_key: ApiKey,
        prompt: String,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static,
    {
        if api_key.key_type != ApiKeyType::DeepSeek {
            return Err("Invalid API key type for DeepSeek".into());
        }

        let user_message = ChatCompletionMessage {
            role: MessageRole::user,
            content: Content::Text(prompt.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        let mut current_messages = self.messages.clone();
        current_messages.push(user_message);

        self.last_prompt = Some(prompt.clone());

        let response = self
            .chat_deepseek(
                &api_key.key,
                &current_messages,
                None,
                true,
                callback,
            )
            .await?;

        let assistant_message = ChatCompletionMessage {
            role: MessageRole::assistant,
            content: Content::Text(response.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        current_messages.push(assistant_message);
        self.messages = current_messages;

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
        if api_key.key_type != ApiKeyType::DeepSeek {
            return Err("Invalid API key type for DeepSeek".into());
        }

        let last_prompt = self.withdraw_response()?;
        self.generate_response_stream(api_key, last_prompt, callback)
            .await
    }

    fn load_from(&mut self, chat_history: &ChatHistory) -> Result<(), Box<dyn Error>> {
        self.messages = chat_history
            .content
            .iter()
            .map(|msg| ChatCompletionMessage {
                role: match msg.msgtype {
                    ChatMessageType::User => MessageRole::user,
                    ChatMessageType::Assistant => MessageRole::assistant,
                    ChatMessageType::System => MessageRole::system,
                },
                content: Content::Text(msg.content.clone()),
                name: Some(msg.time.clone()),
                tool_calls: None,
                tool_call_id: None,
            })
            .collect();
        self.time = chat_history.time.clone();
        self.title = chat_history.title.clone();
        self.chat_id = chat_history.id;
        Ok(())
    }

    fn save_to(&self) -> Result<ChatHistory, Box<dyn Error>> {
        let chat_history = ChatHistory {
            content: self
                .messages
                .iter()
                .map(|msg| ChatMessage {
                    msgtype: match msg.role {
                        MessageRole::user => ChatMessageType::User,
                        MessageRole::assistant => ChatMessageType::Assistant,
                        MessageRole::system => ChatMessageType::System,
                        _ => ChatMessageType::User,
                    },
                    content: match &msg.content {
                        Content::Text(text) => text.clone(),
                    },
                    time: msg.name.clone().unwrap_or_default(),
                })
                .collect(),
            time: self.time.clone(),
            title: self.title.clone(),
            id: self.chat_id,
        };
        Ok(chat_history)
    }
}