use std::error::Error;

use crate::ChatHistory;

use super::{apikey::ApiKey, deepseek::DeepSeekChat, gemini::GeminiChat};

#[allow(dead_code)]
pub(crate) trait AIChat {
    /// 使用流式方式生成响应，接收一个回调函数处理返回的文本片段
    async fn generate_response_stream<F>(
        &mut self,
        api_key: ApiKey,
        prompt: String,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static;

    async fn regenerate_response_stream<F>(
        &mut self,
        api_key: ApiKey,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static;

    fn withdraw_response(&mut self) -> Result<String, Box<dyn Error>>;

    fn clear_context(&mut self) -> Result<String, Box<dyn Error>>;

    fn set_system_prompt(&mut self, prompt: String) -> Result<String, Box<dyn Error>>;

    // 设置模型参数，如temperature、max_tokens等
    fn set_parameter(&mut self, key: String, value: String) -> Result<(), Box<dyn Error>>;

    // 序列化聊天状态
    fn serialize(&self) -> String;

    // 反序列化聊天状态
    fn deserialize(&mut self, data: String) -> Result<(), Box<dyn Error>>;

    fn load_from(&mut self, chat_history: &ChatHistory) -> Result<(), Box<dyn Error>>;

    fn save_to(&self) -> Result<ChatHistory, Box<dyn Error>>;

    // 执行工具调用
    async fn execute_tool_call(
        &mut self,
        tool_name: String,
        args: String,
    ) -> Result<String, Box<dyn Error>>;
}

pub enum AIChatType {
    Gemini(GeminiChat),
    DeepSeek(DeepSeekChat),
}

impl AIChat for AIChatType {
    async fn generate_response_stream<F>(
        &mut self,
        api_key: ApiKey,
        prompt: String,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static,
    {
        match self {
            AIChatType::Gemini(chat) => {
                chat.generate_response_stream(api_key, prompt, callback)
                    .await
            }
            AIChatType::DeepSeek(chat) => {
                chat.generate_response_stream(api_key, prompt, callback)
                    .await
            }
        }
    }

    async fn regenerate_response_stream<F>(
        &mut self,
        api_key: ApiKey,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static,
    {
        match self {
            AIChatType::Gemini(chat) => chat.regenerate_response_stream(api_key, callback).await,
            AIChatType::DeepSeek(chat) => chat.regenerate_response_stream(api_key, callback).await,
        }
    }

    fn withdraw_response(&mut self) -> Result<String, Box<dyn Error>> {
        match self {
            AIChatType::Gemini(chat) => chat.withdraw_response(),
            AIChatType::DeepSeek(chat) => chat.withdraw_response(),
        }
    }

    fn clear_context(&mut self) -> Result<String, Box<dyn Error>> {
        match self {
            AIChatType::Gemini(chat) => chat.clear_context(),
            AIChatType::DeepSeek(chat) => chat.clear_context(),
        }
    }

    fn set_system_prompt(&mut self, prompt: String) -> Result<String, Box<dyn Error>> {
        match self {
            AIChatType::Gemini(chat) => chat.set_system_prompt(prompt),
            AIChatType::DeepSeek(chat) => chat.set_system_prompt(prompt),
        }
    }

    fn set_parameter(&mut self, key: String, value: String) -> Result<(), Box<dyn Error>> {
        match self {
            AIChatType::Gemini(chat) => chat.set_parameter(key, value),
            AIChatType::DeepSeek(chat) => chat.set_parameter(key, value),
        }
    }

    fn serialize(&self) -> String {
        match self {
            AIChatType::Gemini(chat) => chat.serialize(),
            AIChatType::DeepSeek(chat) => chat.serialize(),
        }
    }
    fn deserialize(&mut self, data: String) -> Result<(), Box<dyn Error>> {
        match self {
            AIChatType::Gemini(chat) => chat.deserialize(data),
            AIChatType::DeepSeek(chat) => chat.deserialize(data),
        }
    }
    fn load_from(&mut self, chat_history: &ChatHistory) -> Result<(), Box<dyn Error>> {
        match self {
            AIChatType::Gemini(chat) => chat.load_from(chat_history),
            AIChatType::DeepSeek(chat) => chat.load_from(chat_history),
        }
    }
    fn save_to(&self) -> Result<ChatHistory, Box<dyn Error>> {
        match self {
            AIChatType::Gemini(chat) => chat.save_to(),
            AIChatType::DeepSeek(chat) => chat.save_to(),
        }
    }
    async fn execute_tool_call(
        &mut self,
        tool_name: String,
        args: String,
    ) -> Result<String, Box<dyn Error>> {
        match self {
            AIChatType::Gemini(chat) => chat.execute_tool_call(tool_name, args).await,
            AIChatType::DeepSeek(chat) => chat.execute_tool_call(tool_name, args).await,
        }
    }
}
