use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    assistant,
    user,
    system,
    function,
    tool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Content {
    Text(String),
    // 可以根据需要扩展其他内容类型
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    pub role: MessageRole,
    pub content: Content,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub function: Function,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Parameters,
    pub arguments: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Parameters {
    pub properties: Option<HashMap<String, PropertyValue>>,
    pub required: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertyValue {
    pub schema_type: Option<JSONSchemaType>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JSONSchemaType {
    String,
    Number,
    Boolean,
    Object,
    Array,
    Null,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tool {
    pub function: FunctionDefinition,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Parameters,
}

// --- Chat Completion Request/Response Types ---

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatCompletionMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<String>,
    pub stream: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: Option<Usage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatCompletionMessage,
    pub finish_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// --- Streaming Types ---

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatCompletionStreamResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionStreamChoice>,
    pub usage: Option<Usage>, // 添加 usage 字段，用于推理模型的最后响应
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatCompletionStreamChoice {
    pub index: u32,
    pub delta: Option<ChatCompletionDelta>,
    pub finish_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatCompletionDelta {
    pub role: Option<MessageRole>,
    pub content: Option<String>,
    pub reasoning_content: Option<String>, // 添加推理内容字段
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    FunctionCall,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}