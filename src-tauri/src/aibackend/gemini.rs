use crate::aibackend::interface::AIChat;
use crate::aibackend::template;
use base64::Engine;
use futures_util::StreamExt;
use openai_api_rs::v1::chat_completion::{self, Content, MessageRole};
use openai_api_rs::v1::types;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::pin::Pin;
use std::future::Future;

use super::apikey::{ApiKey, ApiKeyType};

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
    messages: Vec<chat_completion::ChatCompletionMessage>,
    temperature: f32,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    last_prompt: Option<String>,
    tools: Vec<chat_completion::Tool>, // Consider if this needs to be stored if tools are passed per call
}

// --- Constants ---
const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

// --- Stream 相关常量 ---
const STREAM_DELIM: &str = "\n\n";
const STREAM_DATA_PREFIX: &str = "data: ";
const STREAM_DONE_MARKER: &str = "[DONE]";

// --- Helper Functions ---

/// 构建 Gemini API URL
fn build_gemini_url(model: &str, api_key: &str, action: &str) -> String {
    format!("{}/{}:{}?key={}", GEMINI_API_BASE_URL, model, action, api_key)
}

/// 转换 OpenAI 工具为 Gemini 格式
fn convert_tools_to_gemini_format(tools: &[chat_completion::Tool]) -> Value {
    let function_declarations: Vec<Value> = tools.iter().map(|tool| {
        let func = &tool.function;
        let mut properties = json!({});
        
        if let Some(props) = &func.parameters.properties {
            for (param_name, param_def) in props {
                let param_type = match param_def.schema_type {
                    Some(types::JSONSchemaType::String) => "STRING",
                    Some(types::JSONSchemaType::Number) => "NUMBER",
                    Some(types::JSONSchemaType::Boolean) => "BOOLEAN",
                    Some(types::JSONSchemaType::Object) => "OBJECT",
                    Some(types::JSONSchemaType::Array) => "ARRAY",
                    _ => "UNKNOWN" // Or handle more gracefully
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
    }).collect();

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
                    if let Some(safety_ratings) = candidate.get("safetyRatings").and_then(|sr| sr.as_array()) {
                        for rating in safety_ratings {
                            if rating.get("blocked").and_then(|v| v.as_bool()).unwrap_or(false) {
                                if let Some(category) = rating.get("category").and_then(|c| c.as_str()) {
                                    reasons.push_str(&format!("{}: blocked\n", category));
                                }
                            }
                        }
                    }
                    return Err(format!("Content blocked due to safety concerns:\n{}", reasons).into());
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
fn parse_gemini_tool_call_response(response_json: &Value) -> Result<(Option<String>, Vec<ToolCall>), Box<dyn Error>> {
    let mut function_calls = Vec::new();
    let mut response_text: Option<String> = None;

    if let Some(candidates) = response_json.get("candidates").and_then(|c| c.as_array()) {
        if let Some(candidate) = candidates.get(0) {
            // 检查安全过滤 (可以复用 parse_gemini_response 的安全检查逻辑)
            if let Some(finish_reason) = candidate.get("finishReason").and_then(|fr| fr.as_str()) {
                 if finish_reason == "SAFETY" {
                     // ... (安全检查逻辑同 parse_gemini_response) ...
                     return Err("Content blocked due to safety concerns.".into()); // 简化错误信息
                 }
            }

            // 提取内容部分
            if let Some(content) = candidate.get("content") {
                if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                    for part in parts {
                        // 检查函数调用
                        if let Some(function_call) = part.get("functionCall") {
                            let name = function_call.get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or_default()
                                .to_string();
                            
                            let args = function_call.get("args")
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

#[allow(dead_code)]
impl GeminiChat {
    pub fn new() -> Self {
        GeminiChat {
            // base_url 仍然保留，以防未来需要与其他兼容 OpenAI 的 API 交互
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai/".to_string(), 
            model: "gemini-1.5-flash-latest".to_string(), // 更新为推荐模型
            system_prompt: "You are a helpful assistant".to_string(),
            messages: Vec::new(),
            temperature: 0.7,
            max_tokens: Some(8192), // 设置默认值
            top_p: Some(0.95),      // 设置默认值
            last_prompt: None,
            tools: Vec::new(),
        }
    }

    /// 转换OpenAI格式的消息为Gemini格式的请求体
    fn build_gemini_request_body(&self, messages: &[chat_completion::ChatCompletionMessage], tools: Option<&[chat_completion::Tool]>) -> Result<Value, Box<dyn Error>> {
        let gemini_messages: Vec<Value> = messages.iter()
            .filter_map(|message| {
                if let Content::Text(content) = &message.content {
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
                        } else if message.role == MessageRole::assistant && message.tool_calls.is_some() {
                            // 处理模型发起的工具调用请求
                            let function_calls: Vec<Value> = message.tool_calls.as_ref().unwrap_or(&vec![]).iter().map(|tc| {
                                let args_value = tc.function.arguments.as_deref() // Get Option<&str>
                                    .and_then(|s| serde_json::from_str::<Value>(s).ok()) // Try parsing if Some, get Option<Value>
                                    .unwrap_or(json!({})); // Default to {} if None or parse error
                                json!({
                                    "name": tc.function.name,
                                    "args": args_value
                                })
                            }).collect();
                             Some(json!({
                                "role": role,
                                "parts": [{"functionCall": function_calls[0]}] // Gemini 当前似乎只支持单个 functionCall part
                            }))
                        }
                        else {
                            Some(json!({
                                "role": role,
                                "parts": [{ "text": content }]
                            }))
                        }
                    } else {
                        None
                    }
                } else {
                    None // 忽略非文本内容的消息
                }
            })
            .collect();

        let mut request_body = json!({
            "contents": gemini_messages,
            "generationConfig": {
                "temperature": self.temperature,
                "topP": self.top_p, //.unwrap_or(0.95), // 使用 Option 类型
                "topK": 40, // 保留或设为可选
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
        
        Ok(request_body)
    }


    /// 发送请求到Gemini API 并解析纯文本响应
    async fn send_request_and_parse_text(&self, api_key: &str, request_body: Value) -> Result<String, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let url = build_gemini_url(&self.model, api_key, "generateContent");
        
        let response = client.post(&url)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status(); // Store status before consuming response
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(format!("API request failed ({}): {}", status, error_text).into());
        }
        
        let response_json: Value = response.json().await?;
        parse_gemini_response(&response_json)
    }

     /// 发送请求到Gemini API 并解析工具调用或文本响应
    async fn send_request_and_parse_tool_call(&self, api_key: &str, request_body: Value) -> Result<(Option<String>, Vec<ToolCall>), Box<dyn Error>> {
        let client = reqwest::Client::new();
        let url = build_gemini_url(&self.model, api_key, "generateContent");

        let response = client.post(&url)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status(); // Store status before consuming response
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("API request failed ({}): {}", status, error_text).into());
        }

        let response_json: Value = response.json().await?;
        parse_gemini_tool_call_response(&response_json)
    }
    
    /// 准备包含系统指令的消息列表
    fn prepare_messages_with_system_prompt(&self, messages: Vec<chat_completion::ChatCompletionMessage>, system_instruction: &str) -> Vec<chat_completion::ChatCompletionMessage> {
        // Gemini 推荐将系统指令放在第一个用户消息之前，或者使用特定的 systemInstruction 字段（如果模型支持）
        // 这里我们模拟将其放在开头，但标记为 'user' 或 'model' 角色，具体取决于 API 要求
        // 最简单的方式是将其合并到第一个用户消息中，或者作为单独的 model 消息开始
        let mut all_messages = vec![
             // 方式一：作为 model 消息开始 (模拟思考过程)
            // chat_completion::ChatCompletionMessage {
            //     role: MessageRole::assistant, // 'model' in Gemini
            //     content: Content::Text(format!(
            //         "<|start_header|>think<|end_header|>My instructons are as follows:\n--- [System Instructions] ---\n{}\n--- [System Instructions End] ---<|start_header|>gather_information_and_respond_by_using_typesetting_format<|end_header|>ready",
            //         system_instruction
            //     )),
            //     name: None, tool_calls: None, tool_call_id: None,
            // }
            // 方式二：将系统指令预置到第一个用户消息（如果存在）
            // 方式三：使用顶层的 systemInstruction 字段（在 build_gemini_request_body 中处理）
        ];

        // 如果使用方式一或二，需要调整消息列表
        // 当前实现假设 system_instruction 通过顶层字段传递或模型默认处理
        all_messages.extend(messages); 
        all_messages
    }

    /// 从Gemini使用系统指令构建聊天请求 (简化版，依赖 build_gemini_request_body)
    async fn chat_gemini(&mut self, api_key: &str, messages: Vec<chat_completion::ChatCompletionMessage>, system_instruction: &str) -> Result<String, Box<dyn Error>> {
        // 注意：system_instruction 的处理方式取决于 build_gemini_request_body 的实现
        // let prepared_messages = self.prepare_messages_with_system_prompt(messages, system_instruction);
        let request_body = self.build_gemini_request_body(&messages, None)?; // 假设系统指令在顶层处理
        self.send_request_and_parse_text(api_key, request_body).await
    }
    
    /// 通过工具调用实现聊天功能
    async fn chat_gemini_with_tools(
        &mut self,
        api_key: &str,
        messages: Vec<chat_completion::ChatCompletionMessage>,
        tools: &[chat_completion::Tool],
        system_instruction: &str,
        tool_call_processor: impl Fn(String, HashMap<String, Value>) -> Pin<Box<dyn Future<Output = Result<String, Box<dyn Error>>> + Send>> + Send + Sync,
    ) -> Result<String, Box<dyn Error>> {
        
        // 1. 第一次调用：发送包含工具的请求
        // let prepared_messages = self.prepare_messages_with_system_prompt(messages.clone(), system_instruction);
        let initial_request_body = self.build_gemini_request_body(&messages, Some(tools))?;
        let (initial_response_text, tool_calls) = self.send_request_and_parse_tool_call(api_key, initial_request_body).await?;

        // 2. 检查是否有工具调用需要执行
        if !tool_calls.is_empty() && !self.check_if_skip_tool_call(&tool_calls) {
            let mut tool_results_map = HashMap::new();
            let mut tool_result_messages = Vec::new();

            // 添加模型的回复（可能包含思考过程或函数调用请求）
             let mut current_messages = messages.clone(); // 开始构建下一次请求的消息历史
             if let Some(text) = initial_response_text {
                 current_messages.push(chat_completion::ChatCompletionMessage {
                     role: MessageRole::assistant, // 'model'
                     content: Content::Text(text), // 模型可能的回应文本
                     name: None, tool_calls: None, tool_call_id: None, // 简化处理，实际应包含 tool_calls
                 });
             }
             // 添加模型请求的工具调用消息 (转换 ToolCall 回 ChatCompletionMessage)
             // TODO: 需要更精确地将 Gemini 的 FunctionCall 转换为 OpenAI 的 ToolCall 格式存储
             // current_messages.push( ... 模型请求工具调用的消息 ... );


            // 3. 执行工具调用
            for tool_call in &tool_calls {
                let result = tool_call_processor(tool_call.name.clone(), tool_call.args.clone()).await?;
                tool_results_map.insert(tool_call.name.clone(), result.clone());
                
                // 4. 构建工具结果消息 (Function Response)
                tool_result_messages.push(chat_completion::ChatCompletionMessage {
                    role: MessageRole::tool, // 'function' role in Gemini
                    content: Content::Text(result), // 工具执行结果
                    name: Some(tool_call.name.clone()), // 必须提供工具名称
                    tool_calls: None,
                    tool_call_id: None, // TODO: 需要关联到原始的 tool_call_id (如果 Gemini 提供)
                });
            }

            // 5. 第二次调用：发送包含工具结果的请求
            current_messages.extend(tool_result_messages);
            let final_request_body = self.build_gemini_request_body(&current_messages, Some(tools))?; // 仍然传递 tools 定义
            return self.send_request_and_parse_text(api_key, final_request_body).await;

        } else if let Some(text) = initial_response_text {
             // 如果没有工具调用或跳过，直接返回初始文本响应
            return Ok(text);
        } else {
             // 如果既没有工具调用也没有文本响应（理论上不应发生，除非API错误或解析问题）
             return Err("Gemini response contained neither text nor tool calls.".into());
        }
    }
    
    // chat_gemini_tool_call 函数被 send_request_and_parse_tool_call 替代
    
    /// 检查是否需要跳过工具调用 (假设有个名为 skip_tool_call 的特殊工具)
    fn check_if_skip_tool_call(&self, tool_calls: &[ToolCall]) -> bool {
        tool_calls.iter().any(|call| call.name == "skip_tool_call")
    }
    
    // format_tool_call_results 不再需要，因为结果直接作为消息发送
    /// 创建流式请求体
    fn build_gemini_stream_request_body(&self, messages: &[chat_completion::ChatCompletionMessage]) -> Result<Value, Box<dyn Error>> {
        // 获取基础请求体 
        let mut request_body = self.build_gemini_request_body(messages, None)?;
        
        // 添加流式处理标志
        if let Some(obj) = request_body.as_object_mut() {
            obj.insert("stream".to_string(), json!(true));
        }
        
        Ok(request_body)
    }

    /// 以流式方式发送请求到Gemini API并处理响应块
    async fn stream_gemini_chat<F>(&mut self, api_key: &str, messages: Vec<chat_completion::ChatCompletionMessage>, callback: F) -> Result<String, Box<dyn Error>> 
    where
        F: FnMut(String) + Send + 'static
    {
        use futures_util::StreamExt; // Import StreamExt trait locally to bring `next()` into scope
        
        let client = reqwest::Client::new();
        let url = build_gemini_url(&self.model, api_key, "generateContent");
        let request_body = self.build_gemini_stream_request_body(&messages)?;
        
        let mut callback = callback;
        let mut full_response = String::new();
        
        let response = client.post(&url)
            .json(&request_body)
            .send()
            .await?;
            
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(format!("API request failed ({}): {}", status, error_text).into());
        }
        
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    // 将字节转换为字符串并添加到缓冲区
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    buffer.push_str(&chunk_str);
                    
                    // 处理缓冲区中的所有完整消息
                    while let Some(pos) = buffer.find(STREAM_DELIM) {
                        let message = buffer[..pos].trim().to_string();
                        buffer = buffer[pos + STREAM_DELIM.len()..].to_string();
                        
                        // 跳过空消息
                        if message.is_empty() {
                            continue;
                        }
                        
                        // 处理数据前缀
                        let content = if message.starts_with(STREAM_DATA_PREFIX) {
                            &message[STREAM_DATA_PREFIX.len()..]
                        } else {
                            message.as_str()
                        };
                        
                        // 检查流是否结束
                        if content == STREAM_DONE_MARKER {
                            break;
                        }
                        
                        // 解析JSON响应
                        match serde_json::from_str::<Value>(content) {
                            Ok(json_value) => {
                                if let Some(text) = extract_text_from_chunk(&json_value) {
                                    if !text.is_empty() {
                                        // 将文本块传递给回调函数
                                        callback(text.clone());
                                        full_response.push_str(&text);
                                    }
                                }
                            },
                            Err(e) => {
                                eprintln!("Failed to parse JSON: {}", e);
                                eprintln!("Invalid JSON: {}", content);
                            }
                        }
                    }
                },
                Err(e) => {
                    return Err(format!("Stream error: {}", e).into());
                }
            }
        }
        
        // 如果完整响应为空，可能会出现错误
        if full_response.is_empty() {
            return Err("No text generated from the stream".into());
        }
        
        // 返回完整的响应文本
        Ok(full_response)
    }
}

// --- AIChat Trait Implementation ---

impl AIChat for GeminiChat {
    async fn generate_response(
        &mut self,
        api_key: ApiKey,
        prompt: String,
    ) -> Result<String, Box<dyn Error>> {
        if api_key.key_type != ApiKeyType::Gemini {
            return Err("Invalid API key type for Gemini".into());
        }

        // 创建用户消息
        let user_message = chat_completion::ChatCompletionMessage {
            role: MessageRole::user,
            content: Content::Text(prompt.clone()),
            name: None, tool_calls: None, tool_call_id: None,
        };
        
        // 更新消息历史 (只添加用户消息，助手消息在获取响应后添加)
        let mut current_messages = self.messages.clone();
        current_messages.push(user_message);
        
        // 保存最后的提示
        self.last_prompt = Some(prompt.clone());
        
        // 构建聊天指令 (使用 self.system_prompt)
        // let chat_instruction = template::gemini_chat_instruction(); // 或者直接用 self.system_prompt
        
        // 调用Gemini API
        // 注意：这里简化为不使用工具。如果需要工具，应调用 chat_gemini_with_tools
        let response = self.chat_gemini(&api_key.key, current_messages.clone(), self.system_prompt.clone().as_str()).await?;
        
        // 创建助手消息
        let assistant_message = chat_completion::ChatCompletionMessage {
            role: MessageRole::assistant,
            content: Content::Text(response.clone()),
            name: None, tool_calls: None, tool_call_id: None,
        };

        // 更新完整消息历史
        current_messages.push(assistant_message);
        self.messages = current_messages;
        
        Ok(response)
    }

    async fn regenerate_response(&mut self, api_key: ApiKey) -> Result<String, Box<dyn Error>> {
         if api_key.key_type != ApiKeyType::Gemini {
            return Err("Invalid API key type for Gemini".into());
        }
        if self.messages.len() < 2 { // 需要至少一条用户消息和一条助手消息才能重新生成
             return Err("Not enough context to regenerate response".into());
        }

        // 移除最后一条助手消息
        if self.messages.last().map_or(false, |m| m.role == MessageRole::assistant) {
            self.messages.pop();
        } else {
             return Err("Last message is not an assistant response, cannot regenerate.".into());
        }
        
        // 获取移除助手消息后的当前消息列表
        let current_messages = self.messages.clone();
        
        // 重新调用 Gemini API
        let response = self.chat_gemini(&api_key.key, current_messages.clone(), self.system_prompt.clone().as_str()).await?;

        // 添加新的助手消息
        let assistant_message = chat_completion::ChatCompletionMessage {
            role: MessageRole::assistant,
            content: Content::Text(response.clone()),
            name: None, tool_calls: None, tool_call_id: None,
        };
        self.messages.push(assistant_message);

        // 更新 last_prompt (如果需要，但通常重新生成不需要改 last_prompt)
        // self.last_prompt = self.messages.iter().rev().find(|m| m.role == MessageRole::user)...

        Ok(response)
    }

    fn withdraw_response(&mut self) -> Result<String, Box<dyn Error>> {
        let mut removed_count = 0;
        // 移除最后一条助手消息
        if self.messages.last().map_or(false, |m| m.role == MessageRole::assistant) {
            self.messages.pop();
            removed_count += 1;
        }
        // 移除最后一条用户消息
        if self.messages.last().map_or(false, |m| m.role == MessageRole::user) {
            self.messages.pop();
            removed_count += 1;
        }

        if removed_count > 0 {
            // 更新 last_prompt 为撤回后的最后一条用户消息内容（如果存在）
            self.last_prompt = self.messages.iter().rev()
                .find(|m| m.role == MessageRole::user)
                .and_then(|m| match &m.content {
                    Content::Text(text) => Some(text.clone()),
                    _ => None,
                });
            Ok(format!("Removed last {} message(s).", removed_count))
        } else {
            Err("No response to withdraw.".into())
        }
    }

    // ... (clear_context, set_system_prompt, set_parameter, serialize, deserialize 保持不变) ...
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
            "temperature" => self.temperature = value.parse::<f32>().map_err(|e| format!("Invalid temperature value: {}", e))?,
            "max_tokens" => self.max_tokens = Some(value.parse::<u32>().map_err(|e| format!("Invalid max_tokens value: {}", e))?),
            "top_p" => self.top_p = Some(value.parse::<f32>().map_err(|e| format!("Invalid top_p value: {}", e))?),
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
        Err("Direct tool execution via this method is not implemented for the current Gemini flow.".into())
    }

    async fn generate_response_stream<F>(
        &mut self,
        api_key: ApiKey,
        prompt: String,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static
    {
        if api_key.key_type != ApiKeyType::Gemini {
            return Err("Invalid API key type for Gemini".into());
        }

        // 创建用户消息
        let user_message = chat_completion::ChatCompletionMessage {
            role: MessageRole::user,
            content: Content::Text(prompt.clone()),
            name: None, tool_calls: None, tool_call_id: None,
        };
        
        // 更新消息历史 (只添加用户消息，助手消息在获取响应后添加)
        let mut current_messages = self.messages.clone();
        current_messages.push(user_message);
        
        // 保存最后的提示
        self.last_prompt = Some(prompt.clone());
        
        // 使用流式API调用Gemini
        let response = self.stream_gemini_chat(&api_key.key, current_messages.clone(), callback).await?;
        
        // 创建助手消息并添加到历史
        let assistant_message = chat_completion::ChatCompletionMessage {
            role: MessageRole::assistant,
            content: Content::Text(response.clone()),
            name: None, tool_calls: None, tool_call_id: None,
        };

        // 更新完整消息历史
        current_messages.push(assistant_message);
        self.messages = current_messages;
        
        Ok(response)
    }
}

// --- Standalone Functions ---

/// 图像到文本转换函数 (保持不变，但使用辅助函数构建 URL)
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
    
    // 使用 gemini-1.5-flash 或其他支持视觉的模型
    let model = "gemini-1.5-flash-latest"; 
    let url = build_gemini_url(model, api_key, "generateContent"); 

    let response = client.post(&url)
        .json(&request_json)
        .send()
        .await?;
        
    let status = response.status(); // Store status before consuming response
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("API request failed ({}): {}", status, error_text).into());
    }
    
    let response_json: Value = response.json().await?;
    parse_gemini_response(&response_json) // 复用解析逻辑
}

/// 从JSON响应流中提取文本
fn extract_text_from_chunk(chunk: &Value) -> Option<String> {
    // 提取Gemini响应中的文本部分
    if let Some(candidates) = chunk.get("candidates").and_then(|c| c.as_array()) {
        if let Some(candidate) = candidates.get(0) {
            if let Some(content) = candidate.get("content") {
                if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                    if let Some(part) = parts.get(0) {
                        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                            return Some(text.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

