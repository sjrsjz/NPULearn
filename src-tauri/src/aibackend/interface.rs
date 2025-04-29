use std::error::Error;

use super::apikey::ApiKey;

pub(crate) trait AIChat {
    async fn generate_response(
        &mut self,
        api_key: ApiKey,
        prompt: String,
    ) -> Result<String, Box<dyn Error>>;

    /// 使用流式方式生成响应，接收一个回调函数处理返回的文本片段
    async fn generate_response_stream<F>(
        &mut self,
        api_key: ApiKey,
        prompt: String,
        callback: F,
    ) -> Result<String, Box<dyn Error>>
    where
        F: FnMut(String) + Send + 'static;

    async fn regenerate_response(&mut self, api_key: ApiKey) -> Result<String, Box<dyn Error>>;

    fn withdraw_response(&mut self) -> Result<String, Box<dyn Error>>;

    fn clear_context(&mut self) -> Result<String, Box<dyn Error>>;

    fn set_system_prompt(&mut self, prompt: String) -> Result<String, Box<dyn Error>>;

    // 设置模型参数，如temperature、max_tokens等
    fn set_parameter(&mut self, key: String, value: String) -> Result<(), Box<dyn Error>>;

    // 序列化聊天状态
    fn serialize(&self) -> String;

    // 反序列化聊天状态
    fn deserialize(&mut self, data: String) -> Result<(), Box<dyn Error>>;

    // 执行工具调用
    async fn execute_tool_call(
        &mut self,
        tool_name: String,
        args: String,
    ) -> Result<String, Box<dyn Error>>;
}
