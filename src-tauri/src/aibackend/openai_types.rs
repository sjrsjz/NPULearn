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