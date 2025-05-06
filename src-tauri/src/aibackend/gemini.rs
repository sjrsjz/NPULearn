use crate::aibackend::interface::AIChat;
use crate::aibackend::openai_types::{
    ChatCompletionMessage, Content, JSONSchemaType, MessageRole, Tool,
};
use crate::aibackend::template::{self, gemini_chat_instruction};
use crate::{ChatHistory, ChatMessage, ChatMessageType};
use base64::Engine;
use futures_util::StreamExt;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::apikey::{ApiKey, ApiKeyType};
use super::template::{cot_template, TypesetInfo};

// --- Enums and Structs ---

/// 用于Gemini API安全设置的枚举
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum HarmCategory {
    HateSpeech,
    Harassment,
    SexuallyExplicit,
    DangerousContent,
}

/// 用于Gemini API安全阈值的枚举
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum HarmBlockThreshold {
    BlockNone,
    BlockOnly,
    BlockMost,
    BlockSome,
}

/// 用于Gemini API完成原因的枚举
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum FinishReason {
    Stop,
    Length,
    Content,
    Safety,
}

/// 表示工具调用的结构
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ToolCall {
    name: String,
    args: HashMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeminiChat {
    base_url: String, // Note: This base_url seems unused for direct Gemini calls
    model: String,
    system_prompt: String,
    messages: Vec<ChatCompletionMessage>,
    temperature: f32,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    last_prompt: Option<String>,
    tools: Vec<Tool>, // Consider if this needs to be stored if tools are passed per call

    chat_id: u32,  // 用于唯一标识聊天会话
    title: String, // 聊天标题
    time: String,  // 聊天时间
}

// --- Constants ---
const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

// --- Helper Functions ---

/// 构建 Gemini API URL
fn build_gemini_url(model: &str, api_key: &str, action: &str) -> String {
    format!(
        "{}/{}:{}?key={}",
        GEMINI_API_BASE_URL, model, action, api_key
    )
}

/// 构建 Gemini API URL，支持流式传输
fn build_gemini_stream_url(model: &str, api_key: &str) -> String {
    format!(
        "{}/{}:streamGenerateContent?key={}",
        GEMINI_API_BASE_URL, model, api_key
    )
}

/// 转换 OpenAI 工具为 Gemini 格式
fn convert_tools_to_gemini_format(tools: &[Tool]) -> Value {
    let function_declarations: Vec<Value> = tools
        .iter()
        .map(|tool| {
            let func = &tool.function;
            let mut properties = json!({});

            if let Some(props) = &func.parameters.properties {
                for (param_name, param_def) in props {
                    let param_type = match param_def.schema_type {
                        Some(JSONSchemaType::String) => "STRING",
                        Some(JSONSchemaType::Number) => "NUMBER",
                        Some(JSONSchemaType::Boolean) => "BOOLEAN",
                        Some(JSONSchemaType::Object) => "OBJECT",
                        Some(JSONSchemaType::Array) => "ARRAY",
                        _ => "UNKNOWN", // Or handle more gracefully
                    };

                    properties[param_name] = json!({
                        "type": param_type,
                        // Add description if available in param_def
                        // "description": param_def.description.clone().unwrap_or_default()
                    });
                }
            }

            json!({
                "name": func.name,
                "description": func.description.clone().unwrap_or_default(),
                "parameters": {
                    "type": "OBJECT", // Assuming top-level parameters are always an object
                    "properties": properties,
                    "required": func.parameters.required.clone().unwrap_or_default()
                }
            })
        })
        .collect();

    json!({
        "tools": [{ "functionDeclarations": function_declarations }],
        "toolConfig": { "functionCallingConfig": { "mode": "ANY" } } // Or "AUTO", "NONE"
    })
}

/// 解析 Gemini API 响应，检查安全并提取文本
fn parse_gemini_response(response_json: &Value) -> Result<String, Box<dyn Error>> {
    if let Some(candidates) = response_json.get("candidates").and_then(|c| c.as_array()) {
        if let Some(candidate) = candidates.get(0) {
            // 检查安全过滤
            if let Some(finish_reason) = candidate.get("finishReason").and_then(|fr| fr.as_str()) {
                if finish_reason == "SAFETY" {
                    let mut reasons = String::new();
                    if let Some(safety_ratings) =
                        candidate.get("safetyRatings").and_then(|sr| sr.as_array())
                    {
                        for rating in safety_ratings {
                            if rating
                                .get("blocked")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false)
                            {
                                if let Some(category) =
                                    rating.get("category").and_then(|c| c.as_str())
                                {
                                    reasons.push_str(&format!("{}: blocked\n", category));
                                }
                            }
                        }
                    }
                    return Err(
                        format!("Content blocked due to safety concerns:\n{}", reasons).into(),
                    );
                }
            }

            // 提取文本内容
            if let Some(content) = candidate.get("content") {
                if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                    if let Some(part) = parts.get(0) {
                        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                            // 使用模板提取有效响应
                            if let Some(extracted) = template::extract_response(text) {
                                return Ok(extracted);
                            }
                            return Ok(text.to_string());
                        }
                    }
                }
            }
        }
    }
    Err("Failed to extract response text from Gemini API".into())
}

/// 解析 Gemini API 响应以获取工具调用或文本
#[allow(dead_code)]
fn parse_gemini_tool_call_response(
    response_json: &Value,
) -> Result<(Option<String>, Vec<ToolCall>), Box<dyn Error>> {
    let mut function_calls = Vec::new();
    let mut response_text: Option<String> = None;

    if let Some(candidates) = response_json.get("candidates").and_then(|c| c.as_array()) {
        if let Some(candidate) = candidates.get(0) {
            // 检查安全过滤 (可以复用 parse_gemini_response 的安全检查逻辑)
            if let Some(finish_reason) = candidate.get("finishReason").and_then(|fr| fr.as_str()) {
                if finish_reason == "SAFETY" {
                    // ... (安全检查逻辑同 parse_gemini_response) ...
                    return Err("Content blocked due to safety concerns.".into());
                    // 简化错误信息
                }
            }

            // 提取内容部分
            if let Some(content) = candidate.get("content") {
                if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                    for part in parts {
                        // 检查函数调用
                        if let Some(function_call) = part.get("functionCall") {
                            let name = function_call
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or_default()
                                .to_string();

                            let args = function_call
                                .get("args")
                                .and_then(|a| a.as_object())
                                .map(|obj| {
                                    obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                                })
                                .unwrap_or_default();

                            if !name.is_empty() {
                                function_calls.push(ToolCall { name, args });
                            }
                        }
                        // 检查文本部分 (即使有函数调用，也可能包含文本)
                        else if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                            response_text = Some(text.to_string());
                            // 如果需要模板提取，在这里处理
                            // if let Some(extracted) = template::extract_response(text) {
                            //     response_text = Some(extracted);
                            // }
                        }
                    }
                }
            }
        }
    }

    // 如果没有函数调用且没有文本，则视为错误
    if function_calls.is_empty() && response_text.is_none() {
        Err("Failed to extract response or tool calls from Gemini API".into())
    } else {
        Ok((response_text, function_calls))
    }
}

/// 解析流式响应块并通过回调函数返回文本
async fn process_stream_response<F>(
    response: reqwest::Response,
    mut callback: F,
) -> Result<String, Box<dyn Error>>
where
    F: FnMut(String) + Send + 'static,
{
    let mut stream = response.bytes_stream();
    let mut full_response = String::new();
    let mut has_received_data = false;

    // 字符级解析变量
    let mut buffer = String::new();
    let mut buffer_lv = 0; // 跟踪JSON嵌套深度: 0=最外层, 1=在数组内但未进入对象, >1=在对象内
    let mut in_string = false; // 是否在字符串内
    let mut escape_char = false; // 是否在转义字符后

    println!("Starting stream processing...");

    // 处理流式响应
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                has_received_data = true; // 标记已收到数据
                let chunk_str = String::from_utf8_lossy(&chunk);
                println!("Received raw chunk: {}", chunk_str); // 调试输出

                // 逐字符处理，模拟Python示例中的逻辑
                for c in chunk_str.chars() {
                    // 处理转义
                    if in_string && !escape_char && c == '\\' {
                        escape_char = true;
                        buffer.push(c);
                        continue;
                    }

                    if in_string && escape_char {
                        escape_char = false;
                        buffer.push(c);
                        continue;
                    }

                    // 字符串边界处理
                    if c == '"' && !escape_char {
                        in_string = !in_string;
                    }
                    // 增加嵌套深度 (仅在非字符串内)
                    else if (c == '{' || c == '[') && !in_string {
                        buffer_lv += 1;
                    }
                    // 减少嵌套深度 (仅在非字符串内)
                    else if (c == '}' || c == ']') && !in_string {
                        buffer_lv -= 1;
                    }

                    // 当深度>1，即进入JSON对象内时，记录字符
                    if buffer_lv > 1 {
                        if in_string && c == '\n' {
                            buffer.push_str("\\n"); // 处理字符串内的换行符
                        } else {
                            buffer.push(c);
                        }
                    }
                    // 当回到深度1(对象结束)且buffer非空，说明完成了一个对象的处理
                    else if buffer_lv == 1 && !buffer.is_empty() {
                        // 补充右花括号，因为右花括号已被读取但未加入buffer
                        buffer.push('}');
                        println!("Completed buffer: {}", buffer);

                        // 解析整个对象
                        match serde_json::from_str::<Value>(&buffer) {
                            Ok(json_value) => {
                                // 提取文本内容
                                if let Some(candidates) =
                                    json_value.get("candidates").and_then(|c| c.as_array())
                                {
                                    if let Some(candidate) = candidates.get(0) {
                                        if let Some(content) = candidate.get("content") {
                                            if let Some(parts) =
                                                content.get("parts").and_then(|p| p.as_array())
                                            {
                                                if let Some(part) = parts.get(0) {
                                                    if let Some(text) =
                                                        part.get("text").and_then(|t| t.as_str())
                                                    {
                                                        if !text.is_empty() {
                                                            println!("Extracted text: {}", text);
                                                            callback(text.to_string());
                                                            full_response.push_str(text);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Failed to parse JSON object: {} - Buffer: {}", e, buffer);
                            }
                        }

                        // 清空buffer，准备下一个对象
                        buffer.clear();
                    }
                    // 处理深度0或1的其他字符(如逗号、空格等)，直接忽略
                }
            }
            Err(e) => {
                println!("Stream error: {}", e);
                return Err(format!("Stream error: {}", e).into());
            }
        }
    }

    // 处理最后可能未处理完的buffer
    if !buffer.is_empty() {
        println!("Processing remaining buffer: {}", buffer);
        // 如果buffer不为空，尝试修复并解析
        if buffer.starts_with('{') && !buffer.ends_with('}') {
            buffer.push('}');
        }

        match serde_json::from_str::<Value>(&buffer) {
            Ok(json_value) => {
                // 提取文本与前面相同
                if let Some(candidates) = json_value.get("candidates").and_then(|c| c.as_array()) {
                    if let Some(candidate) = candidates.get(0) {
                        if let Some(content) = candidate.get("content") {
                            if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                                if let Some(part) = parts.get(0) {
                                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                        if !text.is_empty() {
                                            callback(text.to_string());
                                            full_response.push_str(text);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {} // 忽略最后一个不完整对象的解析错误
        }
    }

    // 检查响应是否为空，但之前收到过数据
    if full_response.is_empty() && has_received_data {
        println!("Warning: Received data but couldn't extract text");
        // 查看是否是特殊情况：所有数据都收到但无法解析为标准格式
        return Ok("(Response received but requires different format parsing)".to_string());
    } else if full_response.is_empty() {
        return Err("No text generated from the stream".into());
    }

    // 返回完整响应
    println!("Completed stream response: {}", full_response);
    Ok(full_response)
}

#[allow(dead_code)]
impl GeminiChat {
    pub fn new() -> Self {
        GeminiChat {
            // base_url 仍然保留，以防未来需要与其他兼容 OpenAI 的 API 交互
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai/".to_string(),
            model: "gemini-2.5-flash-preview-04-17".to_string(), // 更新为推荐模型
            system_prompt: "You are a helpful assistant".to_string(),
            messages: Vec::new(),
            temperature: 0.95,
            max_tokens: Some(8192), // 设置默认值
            top_p: Some(0.95),      // 设置默认值
            top_k: Some(40),        // 设置默认值
            last_prompt: None,
            tools: Vec::new(),
            chat_id: 0,                    // 初始化为0或其他默认值
            title: "New Chat".to_string(), // 初始化标题
            time: "".to_string(),          // 初始化时间
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
                name: "interactive_button".to_string(),
                description: "show a interactive button signed `message`, when user clicks on it, then you will receive `command` text".to_string(),
                detail: r#"show a interactive button signed `message`, when user clicks on it, then you will receive `command` text
    - `message`: the text which you want to show on the button
    - `command`: the text which will be sent when user clicks the button
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
    /// 转换OpenAI格式的消息为Gemini格式的请求体
    fn build_gemini_request_body(
        &self,
        messages: &[ChatCompletionMessage],
        tools: Option<&[Tool]>,
    ) -> Result<Value, Box<dyn Error>> {
        let mut gemini_messages: Vec<Value> = messages
            .iter()
            .filter_map(|message| {
                let Content::Text(content) = &message.content;
                {
                    if !content.is_empty() {
                        let role = match message.role {
                            MessageRole::assistant => "model",
                            MessageRole::user => "user",
                            // Gemini 不直接支持 system 角色在 contents 中，通常通过指令或首条消息模拟
                            MessageRole::system => return None, // 或者转换为 user/model 消息
                            MessageRole::function | MessageRole::tool => "function", // Gemini 使用 function 角色表示工具结果
                        };

                        // 处理工具调用和结果的特殊格式
                        if message.role == MessageRole::tool {
                            Some(json!({
                                "role": role,
                                "parts": [{
                                    "functionResponse": {
                                        "name": message.name.clone().unwrap_or_default(), // 需要工具调用的名称
                                        "response": {
                                            // 假设 content 是 JSON 字符串或其他可序列化的结果
                                            "content": content
                                        }
                                    }
                                }]
                            }))
                        } else if message.role == MessageRole::assistant
                            && message.tool_calls.is_some()
                        {
                            // 处理模型发起的工具调用请求
                            let function_calls: Vec<Value> = message
                                .tool_calls
                                .as_ref()
                                .unwrap_or(&vec![])
                                .iter()
                                .map(|tc| {
                                    let args_value = tc
                                        .function
                                        .arguments
                                        .as_deref() // Get Option<&str>
                                        .and_then(|s| serde_json::from_str::<Value>(s).ok()) // Try parsing if Some, get Option<Value>
                                        .unwrap_or(json!({})); // Default to {} if None or parse error
                                    json!({
                                        "name": tc.function.name,
                                        "args": args_value
                                    })
                                })
                                .collect();
                            Some(json!({
                                "role": role,
                                "parts": [{"functionCall": function_calls[0]}] // Gemini 当前似乎只支持单个 functionCall part
                            }))
                        } else {
                            Some(json!({
                                "role": role,
                                "parts": [{ "text": content }]
                            }))
                        }
                    } else {
                        None
                    }
                }
            })
            .collect();

        gemini_messages.insert(
            0,
            json!({
                "role": "model",
                "parts": [{ "text": format!("# I have double checked that my basic system settings are as follows, I will never disobey them:\n{}\n", self.build_system_instruction()) }]
            }),
        ); // 添加系统指令

        let mut request_body = json!({
            "contents": gemini_messages,
            "generationConfig": {
                "temperature": self.temperature,
                "topP": self.top_p, //.unwrap_or(0.95), // 使用 Option 类型
                "topK": self.top_k, //.unwrap_or(40), // 使用 Option 类型
                "maxOutputTokens": self.max_tokens, //.unwrap_or(8192), // 使用 Option 类型
                //"responseMimeType": "text/plain", // 通常不需要

            },
            "safetySettings": [
                { "category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_NONE" },
                { "category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE" },
                { "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_NONE" },
                { "category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_NONE" }
            ]
            // systemInstruction 可以在这里添加，如果模型支持
            // "systemInstruction": { "parts": [{"text": self.system_prompt}]}
        });

        if let Some(active_tools) = tools {
            if !active_tools.is_empty() {
                let tool_config = convert_tools_to_gemini_format(active_tools);
                // 合并 tool_config 到 request_body
                if let Some(obj) = request_body.as_object_mut() {
                    if let Some(tools_val) = tool_config.get("tools") {
                        obj.insert("tools".to_string(), tools_val.clone());
                    }
                    if let Some(config_val) = tool_config.get("toolConfig") {
                        obj.insert("toolConfig".to_string(), config_val.clone());
                    }
                }
            }
        }
        if !self.system_prompt.is_empty() {
            // Use as_object_mut to modify the existing JSON Value
            if let Some(obj) = request_body.as_object_mut() {
                obj.insert(
                    "systemInstruction".to_string(),
                    json!({ "parts": [{"text": gemini_chat_instruction()}] }), // Correct Gemini format
                );
            }
        }
        Ok(request_body)
    }

    /// 核心流式处理函数 - 所有Gemini API调用都通过这个接口
    async fn stream_request<F>(
        &self,
        request_body: Value,
        url: String,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static,
    {
        let client = reqwest::Client::new();

        let response = client.post(&url).json(&request_body).send().await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(format!("API request failed ({}): {}", status, error_text).into());
        }

        // 检查是否是流式响应或普通响应
        if url.contains("stream") {
            // 处理流式响应，使用全局函数而不是重复实现
            process_stream_response(response, callback).await
        } else {
            // 处理普通响应（转换为单个回调）
            let response_json: Value = response.json().await?;
            match parse_gemini_response(&response_json) {
                Ok(text) => {
                    let mut callback_clone = callback;
                    callback_clone(text.clone());
                    Ok(text)
                }
                Err(e) => Err(e),
            }
        }
    }

    /// 以流式方式发送聊天请求到Gemini API - 高级接口
    async fn chat_stream<F>(
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
        // 构建请求体
        let request_body = self.build_gemini_request_body(messages, tools)?;

        // 根据是否使用流式传输选择URL
        let url = if use_streaming {
            build_gemini_stream_url(&self.model, api_key)
        } else {
            build_gemini_url(&self.model, api_key, "generateContent")
        };

        // 使用统一的流处理接口
        self.stream_request(request_body, url, callback).await
    }

    /// 简化的非流式聊天接口 (内部复用流式接口)
    async fn chat_gemini(
        &self,
        api_key: &str,
        messages: &[ChatCompletionMessage],
        tools: Option<&[Tool]>,
    ) -> Result<String, Box<dyn Error>> {
        // 使用流式接口，但禁用实际的流式传输
        let full_response = Arc::new(std::sync::Mutex::new(String::new()));
        let response_clone = full_response.clone();

        let _ = self
            .chat_stream(
                api_key,
                messages,
                tools,
                false, // 不使用流式传输
                move |chunk| {
                    let mut locked_response = response_clone.lock().unwrap();
                    locked_response.push_str(&chunk);
                },
            )
            .await?;

        // 如果需要，应用模板提取
        let final_response = full_response.lock().unwrap().clone();
        if let Some(extracted) = template::extract_response(&final_response) {
            return Ok(extracted);
        }

        Ok(final_response)
    }

    /// 解析工具调用响应
    async fn parse_tool_calls(
        &self,
        api_key: &str,
        messages: &[ChatCompletionMessage],
        tools: &[Tool],
    ) -> Result<(Option<String>, Vec<ToolCall>), Box<dyn Error>> {
        // 使用chat_gemini获取响应，然后解析工具调用
        let request_body = self.build_gemini_request_body(messages, Some(tools))?;
        let url = build_gemini_url(&self.model, api_key, "generateContent");

        // 发送请求
        let client = reqwest::Client::new();
        let response = client.post(&url).json(&request_body).send().await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(format!("API request failed ({}): {}", status, error_text).into());
        }

        let response_json: Value = response.json().await?;
        parse_gemini_tool_call_response(&response_json)
    }

    /// 通过工具调用实现聊天功能
    async fn chat_gemini_with_tools(
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
        // 1. 第一次调用：发送包含工具的请求，解析工具调用
        let (initial_response_text, tool_calls) =
            self.parse_tool_calls(api_key, messages, tools).await?;

        // 2. 检查是否有工具调用需要执行
        if !tool_calls.is_empty() && !self.check_if_skip_tool_call(&tool_calls) {
            let mut tool_result_messages = Vec::new();

            // 添加模型的回复（可能包含思考过程或函数调用请求）
            let mut current_messages = messages.to_vec(); // 克隆消息列表
            if let Some(text) = initial_response_text {
                current_messages.push(ChatCompletionMessage {
                    role: MessageRole::assistant, // 'model'
                    content: Content::Text(text), // 模型可能的回应文本
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                });
            }

            // 3. 执行工具调用
            for tool_call in &tool_calls {
                let result =
                    tool_call_processor(tool_call.name.clone(), tool_call.args.clone()).await?;

                // 4. 构建工具结果消息 (Function Response)
                tool_result_messages.push(ChatCompletionMessage {
                    role: MessageRole::tool,            // 'function' role in Gemini
                    content: Content::Text(result),     // 工具执行结果
                    name: Some(tool_call.name.clone()), // 必须提供工具名称
                    tool_calls: None,
                    tool_call_id: None,
                });
            }

            // 5. 第二次调用：发送包含工具结果的请求
            current_messages.extend(tool_result_messages);
            return self
                .chat_gemini(api_key, &current_messages, Some(tools))
                .await;
        } else if let Some(text) = initial_response_text {
            // 如果没有工具调用或跳过，直接返回初始文本响应
            return Ok(text);
        } else {
            // 如果既没有工具调用也没有文本响应（理论上不应发生）
            return Err("Gemini response contained neither text nor tool calls.".into());
        }
    }

    /// 检查是否需要跳过工具调用 (假设有个名为 skip_tool_call 的特殊工具)
    fn check_if_skip_tool_call(&self, tool_calls: &[ToolCall]) -> bool {
        tool_calls.iter().any(|call| call.name == "skip_tool_call")
    }
}

// --- AIChat Trait Implementation ---

impl AIChat for GeminiChat {
    fn withdraw_response(&mut self) -> Result<String, Box<dyn Error>> {
        // 首先移除所有尾部的非用户消息
        while let Some(message) = self.messages.last() {
            if message.role != MessageRole::user {
                self.messages.pop();
            } else {
                break;
            }
        }

        // 然后检查是否还有用户消息可以移除
        if let Some(last_message) = self.messages.last() {
            if last_message.role == MessageRole::user {
                // 获取用户消息的内容
                let content = match &last_message.content {
                    Content::Text(text) => text.clone(),
                };

                // 移除这条用户消息
                self.messages.pop();

                // 更新 last_prompt 为当前最后一条用户消息的内容（如果存在）
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

        // 没有找到用户消息
        Err("No user message to withdraw.".into())
    }

    fn clear_context(&mut self) -> Result<String, Box<dyn Error>> {
        self.messages.clear();
        self.last_prompt = None;
        Ok("Context cleared".to_string())
    }

    fn set_system_prompt(&mut self, prompt: String) -> Result<String, Box<dyn Error>> {
        self.system_prompt = prompt;
        // 可能需要清除现有消息，因为系统提示已更改
        // self.messages.clear();
        // self.last_prompt = None;
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
            "top_k" => {
                self.top_k = Some(
                    value
                        .parse::<u32>()
                        .map_err(|e| format!("Invalid top_k value: {}", e))?,
                )
            }
            "model" => self.model = value,
            // 可以添加 top_k 等其他参数
            _ => return Err(format!("Unknown parameter: {}", key).into()),
        }
        Ok(())
    }

    fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            eprintln!("Serialization error: {}", e); // 添加错误日志
            "{}".to_string() // 返回空 JSON 对象或错误指示
        })
    }

    fn deserialize(&mut self, data: String) -> Result<(), Box<dyn Error>> {
        let chat: GeminiChat = serde_json::from_str(&data)?;
        *self = chat;
        Ok(())
    }

    async fn execute_tool_call(
        &mut self,
        _tool_name: String, // 参数名加下划线表示未使用
        _args: String,
    ) -> Result<String, Box<dyn Error>> {
        // 这个方法在 Gemini 的流程中可能不太直接适用
        // 因为工具调用是在 chat_gemini_with_tools 内部处理的
        // 如果需要外部触发工具调用，需要不同的设计
        Err(
            "Direct tool execution via this method is not implemented for the current Gemini flow."
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
        if api_key.key_type != ApiKeyType::Gemini {
            return Err("Invalid API key type for Gemini".into());
        }

        // 创建用户消息
        let user_message = ChatCompletionMessage {
            role: MessageRole::user,
            content: Content::Text(prompt.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        // 更新消息历史 (只添加用户消息，助手消息在获取响应后添加)
        let mut current_messages = self.messages.clone();
        current_messages.push(user_message);

        // 保存最后的提示
        self.last_prompt = Some(prompt.clone());

        // 使用流式API调用Gemini
        let response = self
            .chat_stream(
                &api_key.key,
                &current_messages,
                None,
                true, // 使用真正的流式传输
                callback,
            )
            .await?;

        // 创建助手消息并添加到历史
        let assistant_message = ChatCompletionMessage {
            role: MessageRole::assistant,
            content: Content::Text(response.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        // 更新完整消息历史
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
        if api_key.key_type != ApiKeyType::Gemini {
            return Err("Invalid API key type for Gemini".into());
        }
        // 获取最后一条用户消息内容
        let last_prompt = self.withdraw_response()?;
        // 重新生成响应
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
                name: Some(msg.time.clone()), // 假设时间戳作为名称
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
        // 将消息转换为 ChatHistory 格式
        let chat_history = ChatHistory {
            content: self
                .messages
                .iter()
                .map(|msg| ChatMessage {
                    msgtype: match msg.role {
                        MessageRole::user => ChatMessageType::User,
                        MessageRole::assistant => ChatMessageType::Assistant,
                        MessageRole::system => ChatMessageType::System,
                        _ => ChatMessageType::User, // 默认处理
                    },
                    content: match &msg.content {
                        Content::Text(text) => text.clone(),
                    },
                    time: msg.name.clone().unwrap_or_default(), // 假设名称作为时间戳
                })
                .collect(),
            time: self.time.clone(),
            title: self.title.clone(),
            id: self.chat_id,
        };
        Ok(chat_history)
    }
}

// --- Standalone Functions ---

/// 图像到文本转换函数 (保持不变，但使用辅助函数构建 URL)
#[allow(dead_code)]
pub async fn image_to_text(api_key: &str, image_data: &[u8]) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let base64_image = base64::engine::general_purpose::STANDARD.encode(image_data);

    let request_json = json!({
        "contents": [{
            "parts": [
                { "text": "# You are an image desciptor, Only output what the Image is, if the image contains TEXT, you should use Markdown to output the text" },
                { "inlineData": { "mimeType": "image/jpeg", "data": base64_image } }
            ]
        }]
        // 可以添加 generationConfig 和 safetySettings
    });

    let model = "gemini-2.0-flash";
    let url = build_gemini_url(model, api_key, "generateContent");

    let response = client.post(&url).json(&request_json).send().await?;

    let status = response.status(); // Store status before consuming response
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("API request failed ({}): {}", status, error_text).into());
    }

    let response_json: Value = response.json().await?;
    parse_gemini_response(&response_json) // 复用解析逻辑
}
