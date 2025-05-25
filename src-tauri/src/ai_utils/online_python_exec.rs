use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize)]
struct ExecuteRequest {
    language: String,
    source: String,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteResponse {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub output: Option<String>,
    pub code: Option<i32>,
    pub signal: Option<String>,
    pub message: Option<String>,
}

/// 在线执行Python代码
///
/// 使用 emkc.org 的 Piston API 执行 Python 代码
///
/// # Arguments
///
/// * `code` - 要执行的Python代码
///
/// # Returns
///
/// 返回执行结果，包含标准输出、标准错误等信息
pub async fn execute_python_code(code: &str) -> Result<ExecuteResponse, Box<dyn Error>> {
    let client = reqwest::Client::new();
    
    let request_data = ExecuteRequest {
        language: "python".to_string(),
        source: code.to_string(),
    };
    
    let response = client
        .post("https://emkc.org/api/v1/piston/execute")
        .json(&request_data)
        .send()
        .await?;
    
    let response_data = response.json::<ExecuteResponse>().await?;
    
    Ok(response_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    
    #[tokio::test]
    async fn test_execute_python_code() {
        let code = r#"
print("Hello, World!")
a = 5
b = 10
print(f"Sum: {a + b}")
"#;
        
        match execute_python_code(code).await {
            Ok(response) => {
                println!("Output: {:?}", response.output);
                println!("Stdout: {:?}", response.stdout);
                println!("Stderr: {:?}", response.stderr);
            },
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}