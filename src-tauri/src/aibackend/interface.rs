pub trait AIChat {
    async fn generate_response(
        &mut self,
        prompt: String,
    ) -> Result<String, String>;

    async fn regenerate_response(
        &mut self,
    ) -> Result<String, String>;

    async fn withdraw_response(
        &mut self,
    ) -> Result<String, String>;

    async fn clear_context(
        &mut self,
    ) -> Result<String, String>;

    async fn set_system_prompt(
        &mut self,
        prompt: String,
    ) -> Result<String, String>;

    // 新增：设置模型参数 (示例)
    async fn set_parameter(&mut self, key: String, value: String) -> Result<(), String>;

    fn serialize(&self) -> String;

    fn deserialize(&mut self, data: String) -> Result<(), String>;

}