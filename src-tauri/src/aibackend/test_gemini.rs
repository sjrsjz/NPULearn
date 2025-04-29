use super::{apikey::{ApiKey, ApiKeyType}, gemini::GeminiChat};
use std::env;
use crate::aibackend::interface::AIChat;
use tokio::runtime::Runtime;

/// 从环境变量中获取Gemini API密钥
#[allow(dead_code)]
fn get_gemini_api_key() -> String {
    env::var("GEMINI_API_KEY").unwrap_or_else(|_| {
        println!("警告: 未设置GEMINI_API_KEY环境变量，使用空字符串");
        String::new()
    })
}

#[allow(dead_code)]
#[test]
fn test_gemini_chat() {
    // 测试Gemini聊天功能
    let mut gemini = GeminiChat::new();
    let api_key = ApiKey {
        key: get_gemini_api_key(),
        key_type: ApiKeyType::Gemini,
        name: "Gemini".to_string(),
    };
    
    // 创建Tokio运行时来运行异步测试
    let rt = Runtime::new().unwrap();
    
    // 只有当API密钥不为空时才进行实际测试
    if !api_key.key.is_empty() {
        rt.block_on(async {
            // 设置系统提示
            gemini.set_system_prompt("你是一个友好的助手".to_string()).unwrap();
            
            // 生成响应
            let response = gemini.generate_response(api_key.clone(), "你好，请介绍一下自己".to_string()).await;
            
            match response {
                Ok(text) => {
                    println!("Gemini响应: {}", text);
                    assert!(!text.is_empty(), "响应不应为空");
                },
                Err(e) => {
                    println!("测试失败: {}", e);
                    assert!(false, "API调用失败: {}", e);
                }
            }
        });
    } else {
        println!("跳过Gemini API测试，因为未提供API密钥");
    }
}