use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use futures::{SinkExt, StreamExt};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::error::Error as StdError;
use tokio::runtime::Runtime;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_tungstenite::tungstenite::http::StatusCode; // 明确使用 tungstenite 的 StatusCode
use url::Url;

// 修改错误枚举中的 HttpError 变体
#[derive(Debug)]
pub enum WolframAlphaError {
    WebSocketError(String),
    JsonError(String),
    UrlError(String),
    Base64EncodeError(String),
    InitNotReady(JsonValue),
    UnexpectedMessageType,
    NoPodsFound,
    MissingData,
    HttpError(StatusCode), // 修改为 tungstenite 的 StatusCode
    TimeoutError(String),
}

impl std::fmt::Display for WolframAlphaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WolframAlphaError::WebSocketError(e) => write!(f, "WebSocket connection error: {}", e),
            WolframAlphaError::JsonError(e) => write!(f, "JSON (de)serialization error: {}", e),
            WolframAlphaError::UrlError(e) => write!(f, "URL parsing error: {}", e),
            WolframAlphaError::Base64EncodeError(e) => write!(f, "Base64 encoding error: {}", e),
            WolframAlphaError::InitNotReady(val) => write!(f, "Initial connection response not ready: {:?}", val),
            WolframAlphaError::UnexpectedMessageType => write!(f, "Received unexpected message type"),
            WolframAlphaError::NoPodsFound => write!(f, "Query failed or returned no pods"),
            WolframAlphaError::MissingData => write!(f, "Missing required data in response"),
            WolframAlphaError::HttpError(status) => write!(f, "HTTP error during WebSocket connection: {}", status),
            WolframAlphaError::TimeoutError(e) => write!(f, "Operation timed out: {}", e),
        }
    }
}

impl StdError for WolframAlphaError {}

// --- Data Structures (Mirroring JSON) ---

// Generic message envelope (simplified)
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)] // Allow deserializing to different types based on content
enum WaMessage {
    Init(InitMessage),
    NewQuery(NewQueryMessage),
    Response(Response),
    QueryComplete(QueryCompleteMessage),
    // Add other message types if needed
    Other(JsonValue), // Catch any unknown messages
}

// --- Messages Sent to Gateway ---

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct InitMessage {
    category: String,
    #[serde(rename = "type")] // Renaming for "type" keyword
    msg_type: String,
    lang: String,
    wa_pro_s: String,
    wa_pro_t: String,
    wa_pro_u: String,
    exp: u64, // Or i64 depending on epoch format
    display_debugging_info: bool,
    messages: Vec<JsonValue>, // Use Value for potentially mixed types
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct NewQueryMessage {
    #[serde(rename = "type")] // Renaming for "type" keyword
    msg_type: String,
    location_id: String,
    language: String,
    display_debugging_info: bool,
    yellow_is_error: bool,
    request_sidebar_ad: bool,
    category: String,
    input: String, // Base64 encoded JSON string
    i2d: bool,
    assumption: Vec<JsonValue>, // Use Value for flexibility
    api_params: JsonValue,      // API parameters as a JSON value
    file: Option<String>,       // Or Option<JsonValue>
    theme: String,
}

// --- Messages Received from Gateway ---

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Response {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)] // Field might be missing
    pods: Vec<Pod>,
    #[serde(default)] // Field might be missing
    related_queries: Vec<String>,
    // Add other fields from response if necessary (e.g., timing, messages)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Pod {
    title: String,
    #[serde(default)]
    subpods: Vec<Subpod>,
    // Add other pod fields like position, scanner etc. if needed
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Subpod {
    #[serde(default)]
    title: String, // Subpod title might be different or empty
    plaintext: Option<String>,
    minput: Option<String>,
    moutput: Option<String>,
    img: Option<Image>,
    // Add other subpod fields like id, primary etc. if needed
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Image {
    src: String,   // URL of the image
    alt: String,   // Alt text
    title: String, // Image title
    width: u32,
    height: u32,
    data: Option<String>, // Base64 image data (might be present depending on request/response)
    contenttype: Option<String>, // Image content type (e.g., "image/png")
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct QueryCompleteMessage {
    #[serde(rename = "type")]
    msg_type: String,
    // Add other fields if needed
}

// --- Parsed Result Structure ---
// This is what our compute function will return, simplified from the raw response
#[derive(Debug, Clone)]
pub struct WolframAlphaResult {
    pub title: Option<String>,
    pub plaintext: Option<String>,
    pub minput: Option<String>,
    pub moutput: Option<String>,
    pub img_base64: Option<String>,
    pub img_contenttype: Option<String>,
    pub related_queries: Vec<String>, // Related queries might be top-level
}

/// 实际的 WebSocket 通信实现，异步版本
pub async fn wolfram_alpha_compute_async(
    query: &str,
    image_only: bool,
) -> Result<Vec<WolframAlphaResult>, WolframAlphaError> {
    info!("开始查询: {}", query);
    
    // 准备查询数据
    let q_data = vec![serde_json::json!({"t": 0, "v": query})];
    let q_json = serde_json::to_string(&q_data)
        .map_err(|e| WolframAlphaError::JsonError(e.to_string()))?;
    let q_base64 = STANDARD.encode(q_json.as_bytes());
    
    // 建立WebSocket连接
    let url = Url::parse("wss://gateway.wolframalpha.com/gateway")
        .map_err(|e| WolframAlphaError::UrlError(e.to_string()))?;
    
    info!("正在连接到 {}", url);
    let (mut ws_stream, response) = connect_async(url).await
        .map_err(|e| WolframAlphaError::WebSocketError(e.to_string()))?;
    
    info!("已连接. HTTP状态码: {}", response.status());
    
    // 检查连接状态
    if response.status() != StatusCode::SWITCHING_PROTOCOLS && !response.status().is_success() {
        return Err(WolframAlphaError::HttpError(response.status()));
    }
    
    // 1. 发送初始化消息
    let init_msg = InitMessage {
        category: "results".to_string(),
        msg_type: "init".to_string(),
        lang: "en".to_string(),
        wa_pro_s: "".to_string(),
        wa_pro_t: "".to_string(),
        wa_pro_u: "".to_string(),
        exp: 1714399254570, // 注意: 硬编码过期时间，可能需要动态生成
        display_debugging_info: false,
        messages: vec![],
    };
    
    let init_json = serde_json::to_string(&init_msg)
        .map_err(|e| WolframAlphaError::JsonError(e.to_string()))?;
    
    info!("发送初始化消息: {}", init_json);
    ws_stream.send(Message::Text(init_json)).await
        .map_err(|e| WolframAlphaError::WebSocketError(e.to_string()))?;
    
    // 2. 接收初始化响应
    match timeout(Duration::from_secs(15), ws_stream.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => {
            let resp_val: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| WolframAlphaError::JsonError(e.to_string()))?;
            
            info!("收到初始化响应: {:?}", resp_val);
            
            if resp_val["type"] != "ready" {
                error!("初始化响应未就绪: {:?}", resp_val);
                return Err(WolframAlphaError::InitNotReady(resp_val));
            }
        },
        Ok(Some(Ok(_))) => return Err(WolframAlphaError::UnexpectedMessageType),
        Ok(Some(Err(e))) => return Err(WolframAlphaError::WebSocketError(e.to_string())),
        Ok(None) => return Err(WolframAlphaError::UnexpectedMessageType),
        Err(e) => return Err(WolframAlphaError::TimeoutError(e.to_string())),
    }
    
    // 3. 发送查询消息
    let new_query_msg = NewQueryMessage {
        msg_type: "newQuery".to_string(),
        location_id: "oi8ft_en_light".to_string(),
        language: "en".to_string(),
        display_debugging_info: false,
        yellow_is_error: false,
        request_sidebar_ad: false,
        category: "results".to_string(),
        input: q_base64,
        i2d: true,
        assumption: vec![],
        api_params: serde_json::json!({}),
        file: None,
        theme: "light".to_string(),
    };
    
    let new_query_json = match serde_json::to_string(&new_query_msg) {
        Ok(json) => json,
        Err(e) => return Err(WolframAlphaError::JsonError(e.to_string())),
    };
    
    info!("发送查询消息: {}", new_query_json);
    ws_stream.send(Message::Text(new_query_json)).await
        .map_err(|e| WolframAlphaError::WebSocketError(e.to_string()))?;
    
    // 4. 接收和处理结果
    let mut results: Vec<WolframAlphaResult> = Vec::new();
    let mut message_count = 0;
    const MAX_MESSAGES: usize = 50; // 设置一个合理的最大消息数量限制
    
    while let Ok(Some(msg_result)) = timeout(Duration::from_secs(30), ws_stream.next()).await {
        // 检查是否超过最大迭代次数
        message_count += 1;
        if message_count > MAX_MESSAGES {
            warn!("达到最大消息数量限制 ({})，终止接收", MAX_MESSAGES);
            break;
        }
        
        match msg_result {
            Ok(Message::Text(text)) => {
                info!("接收到消息 #{} (长度={})", message_count, text.len());
                
                // 解析为JSON
                let json_val: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(val) => val,
                    Err(e) => {
                        error!("消息解析失败: {}. 消息内容: {}", e, text);
                        continue;
                    }
                };
                
                // 基于 type 字段处理不同类型的消息
                match json_val.get("type").and_then(|t| t.as_str()) {
                    Some("queryCompleted") => {
                        info!("查询完成");
                        break;
                    },
                    Some("pods") => {
                        info!("接收到结果消息");
                        
                        // 处理相关查询
                        if json_val.get("relatedQueries").is_some() {
                            let related_queries: Vec<String> = serde_json::from_value(
                                json_val["relatedQueries"].clone()
                            ).unwrap_or_default();
                            
                            if !related_queries.is_empty() {
                                if results.is_empty() {
                                    results.push(WolframAlphaResult {
                                        title: None,
                                        plaintext: None, 
                                        minput: None,
                                        moutput: None,
                                        img_base64: None,
                                        img_contenttype: None,
                                        related_queries: related_queries,
                                    });
                                } else if let Some(last) = results.last_mut() {
                                    last.related_queries.extend(related_queries);
                                }
                            }
                        }
                        
                        // 处理pods
                        // 修复第一处错误 - 在处理 pods 时
                        // 原代码:
                        // let pods = json_val["pods"].as_array().unwrap_or(&Vec::new());
                        // for pod in pods { ... }
                        
                        // 修改为:
                        let empty_pods = Vec::new();
                        let pods = json_val["pods"].as_array().unwrap_or(&empty_pods);
                        for pod in pods {
                            if !pod["subpods"].is_array() {
                                continue;
                            }
                            
                            let mut result = WolframAlphaResult {
                                title: pod["title"].as_str().map(|s| s.to_string()),
                                plaintext: None,
                                minput: None, 
                                moutput: None,
                                img_base64: None,
                                img_contenttype: None,
                                related_queries: Vec::new(),
                            };
                            
                            // 修复第二处错误 - 在处理 subpods 时
                            // 原代码:
                            // let subpods = pod["subpods"].as_array().unwrap_or(&Vec::new());
                            // for subpod in subpods { ... }
                            
                            // 修改为:
                            let empty_subpods = Vec::new();
                            let subpods = pod["subpods"].as_array().unwrap_or(&empty_subpods);
                            for subpod in subpods {
                                // 处理文本内容（如果不是仅图像模式）
                                if !image_only {
                                    if let Some(text) = subpod["plaintext"].as_str() {
                                        result.plaintext = Some(text.to_string());
                                    }
                                    
                                    if let Some(minput) = subpod["minput"].as_str() {
                                        result.minput = Some(minput.to_string());
                                    }
                                    
                                    if let Some(moutput) = subpod["moutput"].as_str() {
                                        result.moutput = Some(moutput.to_string());
                                    }
                                }
                                
                                // 处理图像
                                if let Some(img) = subpod.get("img") {
                                    if let Some(data) = img["data"].as_str() {
                                        result.img_base64 = Some(data.to_string());
                                    }
                                    
                                    if let Some(ctype) = img["contenttype"].as_str() {
                                        result.img_contenttype = Some(ctype.to_string());
                                    }
                                }
                            }
                            
                            results.push(result);
                        }
                        
                    },
                    Some(other) => {
                        warn!("接收到类型为 '{}' 的未处理消息", other);
                    },
                    None => {
                        warn!("接收到没有type字段的消息");
                    }
                }
            },
            Ok(Message::Binary(_)) => {
                info!("接收到二进制消息（已忽略）");
            },
            Ok(Message::Ping(_)) => {
                info!("接收到ping消息");
            },
            Ok(Message::Pong(_)) => {
                info!("接收到pong消息");
            },
            Ok(Message::Close(_)) => {
                info!("接收到关闭消息，退出循环");
                break;
            },
            Ok(Message::Frame(_)) => {
                info!("接收到帧消息（已忽略）");
            },
            Err(e) => {
                error!("WebSocket消息错误: {}", e);
                return Err(WolframAlphaError::WebSocketError(e.to_string()));
            }
        }
    }
    
    info!("结果收集完成: {} 项", results.len());
    
    if results.is_empty() {
        return Err(WolframAlphaError::NoPodsFound);
    }
    
    Ok(results)
}

/// 同步包装函数，用于在非异步上下文中调用
pub fn wolfram_alpha_compute(
    query: &str,
    image_only: bool,
) -> Result<Vec<WolframAlphaResult>, WolframAlphaError> {
    // 创建tokio运行时
    let rt = Runtime::new()
        .map_err(|e| WolframAlphaError::WebSocketError(e.to_string()))?;
    
    // 在运行时中执行异步函数
    rt.block_on(wolfram_alpha_compute_async(query, image_only))
}

/// 不包含图像的查询，异步版本
pub async fn wolfram_alpha_compute_without_image_async(
    query: &str,
) -> Result<Vec<WolframAlphaResult>, WolframAlphaError> {
    wolfram_alpha_compute_async(query, true).await
}

/// 不包含图像的查询，同步版本
pub fn wolfram_alpha_compute_without_image(
    query: &str,
) -> Result<Vec<WolframAlphaResult>, WolframAlphaError> {
    wolfram_alpha_compute(query, true)
}

// --- Formatting Functions ---

/// Formats results into a structure suitable for Mirai Console WebSocket API.
pub fn format_to_mirai_ws(results: &[WolframAlphaResult]) -> Option<Vec<JsonValue>> {
    if results.is_empty() {
        return None;
    }

    let mut msg_list: Vec<JsonValue> = Vec::new();
    let n = results.len();

    for (i, result) in results.iter().enumerate() {
        if let Some(title) = &result.title {
            msg_list.push(serde_json::json!({"type": "Plain", "text": format!("{}\n", title)}));
        }
        if let Some(plaintext) = &result.plaintext {
            msg_list.push(serde_json::json!({"type": "Plain", "text": format!("Expr:{}\n", plaintext)}));
        }
        if let Some(img_base64) = &result.img_base64 {
            msg_list.push(serde_json::json!({"type": "Image", "base64": img_base64}));
        }
        if let Some(minput) = &result.minput {
            msg_list.push(serde_json::json!({"type": "Plain", "text": format!("Mathematica Input:{}\n", minput)}));
        }
        if let Some(moutput) = &result.moutput {
            msg_list.push(serde_json::json!({"type": "Plain", "text": format!("Mathematica Output:{}\n", moutput)}));
        }
        if !result.related_queries.is_empty() {
            msg_list.push(serde_json::json!({"type": "Plain", "text": "Related Queries:\n"}));
            for query in &result.related_queries {
                msg_list.push(serde_json::json!({"type": "Plain", "text": format!("{}\n", query)}));
            }
        }

        if i < n - 1 {
            msg_list.push(serde_json::json!({"type": "Plain", "text": "----------------------\n"}));
        }
    }

    // Remove trailing newlines from the last plain text element
    if let Some(last_elem) = msg_list.last_mut() {
        if let Some(text) = last_elem.get_mut("text").and_then(|t| t.as_str()) {
            let trimmed_text = text.trim_end_matches('\n');
            if trimmed_text != text {
                *last_elem = serde_json::json!({"type": "Plain", "text": trimmed_text});
            }
        }
    }

    if msg_list.is_empty() {
        None
    } else {
        Some(msg_list)
    }
}

/// Formats results into a CQ code string.
pub fn format_to_cq(results: &[WolframAlphaResult]) -> Option<String> {
    if results.is_empty() {
        return None;
    }

    let mut ret = String::new();
    let n = results.len();

    for (i, result) in results.iter().enumerate() {
        if let Some(title) = &result.title {
            ret.push_str(&format!("{}\n", title));
        }
        if let Some(plaintext) = &result.plaintext {
            ret.push_str(&format!("Expr:{}\n", plaintext));
        }
        if let Some(img_base64) = &result.img_base64 {
            ret.push_str(&format!("[CQ:image,file=base64://{}]", img_base64));
        }
        if let Some(minput) = &result.minput {
            ret.push_str(&format!("Mathematica Input:{}\n", minput));
        }
        if let Some(moutput) = &result.moutput {
            ret.push_str(&format!("Mathematica Output:{}\n", moutput));
        }
        if !result.related_queries.is_empty() {
            ret.push_str("Related Queries:\n");
            for query in &result.related_queries {
                ret.push_str(&format!("{}\n", query));
            }
        }

        if i < n - 1 {
            ret.push_str("----------------------\n");
        }
    }

    if ret.is_empty() {
        None
    } else {
        Some(ret)
    }
}

/// Formats results into a Markdown string.
pub fn format_to_markdown(results: &[WolframAlphaResult]) -> String {
    if results.is_empty() {
        // Following Python's no-result behavior, return HTML warning
        return "<div class=\"alert alert-warning\" role=\"alert\">No results</div>".to_string();
    }

    let mut ret = String::new();
    // Python didn't add separator for Markdown, replicating that
    // let n = results.len();

    for result in results.iter() {
        if let Some(title) = &result.title {
            ret.push_str(&format!("{}\n", title));
        }
        if let Some(plaintext) = &result.plaintext {
            ret.push_str(&format!("Expr:{}\n", plaintext));
        }
        if let Some(img_base64) = &result.img_base64 {
            let content_type = result.img_contenttype.as_deref().unwrap_or("image/png");
            ret.push_str(&format!(
                "![Image](data:{};base64,{})\n",
                content_type, img_base64
            ));
        }
        if let Some(minput) = &result.minput {
            ret.push_str(&format!("Mathematica Input:{}\n", minput));
        }
        if let Some(moutput) = &result.moutput {
            ret.push_str(&format!("Mathematica Output:{}\n", moutput));
        }
        if !result.related_queries.is_empty() {
            ret.push_str("Related Queries:\n");
            for query in &result.related_queries {
                ret.push_str(&format!("{}\n", query));
            }
        }
    }

    ret
}

/// Formats results into an HTML string.
pub fn format_to_html(results: &[WolframAlphaResult]) -> String {
    if results.is_empty() {
        return r#"<div class="alert alert-warning" role="alert">No results</div>"#.to_string();
    }

    let mut ret = String::new();
    ret.push_str(
        r#"<div style="border: 1px solid #ccc; padding: 10px; margin: 10px; border-radius: 5px;">"#,
    );
    let n = results.len();

    for (i, result) in results.iter().enumerate() {
        if let Some(title) = &result.title {
            ret.push_str(&format!("<h2>{}</h2>\n", escape_html(title)));
        }
        if let Some(plaintext) = &result.plaintext {
            ret.push_str(&format!(
                "<p><strong>Expr:</strong> {}</p>\n",
                escape_html(plaintext)
            ));
        }
        if let Some(img_base64) = &result.img_base64 {
            let content_type = result.img_contenttype.as_deref().unwrap_or("image/png");
            ret.push_str(&format!(
                r#"<p><img src='data:{};base64,{}' alt='Image' /></p>"#,
                escape_html(content_type),
                img_base64
            ));
        }
        if let Some(minput) = &result.minput {
            ret.push_str(&format!(
                "<p><strong>Mathematica Input:</strong> {}</p>\n",
                escape_html(minput)
            ));
        }
        if let Some(moutput) = &result.moutput {
            ret.push_str(&format!(
                "<p><strong>Mathematica Output:</strong> {}</p>\n",
                escape_html(moutput)
            ));
        }
        if !result.related_queries.is_empty() {
            ret.push_str("<p><strong>Related Queries:</strong></p>\n<ul>\n");
            for query in &result.related_queries {
                ret.push_str(&format!("<li>{}</li>\n", escape_html(query)));
            }
            ret.push_str("</ul>\n");
        }

        if i < n - 1 {
            ret.push_str("<hr />\n");
        }
    }

    ret.push_str("</div>\n");
    ret
}

/// Simple HTML escaping
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Formats results into an XML string.
pub fn format_to_xml(results: &[WolframAlphaResult]) -> String {
    if results.is_empty() {
        return "<alert>No results</alert>".to_string();
    }

    let mut ret = String::new();
    ret.push_str("<results>\n");
    let n = results.len();

    for (i, result) in results.iter().enumerate() {
        // Wrap each logical result block? Python didn't, just added <hr/>
        // Let's add a root element for each result for better XML structure
        ret.push_str("<result>\n");
        if let Some(title) = &result.title {
            ret.push_str(&format!("<title>{}</title>\n", escape_xml(title)));
        }
        if let Some(plaintext) = &result.plaintext {
            ret.push_str(&format!(
                "<plaintext>{}</plaintext>\n",
                escape_xml(plaintext)
            ));
        }
        if let Some(img_base64) = &result.img_base64 {
            let content_type = result.img_contenttype.as_deref().unwrap_or("image/png");
            ret.push_str(&format!(
                "<img contenttype='{}'>{}</img>\n",
                escape_xml_attr(content_type),
                img_base64
            ));
        }
        if let Some(minput) = &result.minput {
            ret.push_str(&format!("<minput>{}</minput>\n", escape_xml(minput)));
        }
        if let Some(moutput) = &result.moutput {
            ret.push_str(&format!("<moutput>{}</moutput>\n", escape_xml(moutput)));
        }
        if !result.related_queries.is_empty() {
            ret.push_str("<relatedQueries>\n");
            for query in &result.related_queries {
                ret.push_str(&format!("<query>{}</query>\n", escape_xml(query)));
            }
            ret.push_str("</relatedQueries>\n");
        }
        ret.push_str("</result>\n");

        // Python added <hr/>, which isn't standard XML. Add a comment or separator tag instead?
        // Replicating Python's literal output:
        if i < n - 1 {
            ret.push_str("<hr />\n");
        }
    }

    ret.push_str("</results>\n");
    ret
}

/// Simple XML element content escaping
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Simple XML attribute value escaping
fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;") // Use &quot; for attributes quoted with "
        .replace('\'', "&#39;") // Use &#39; for attributes quoted with '
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wolfram_alpha_compute_basic() {
        let result = wolfram_alpha_compute("integral of x^2", false);
        println!("\nresult: {:?}", result);
        assert!(result.is_ok(), "Should successfully compute the query");
        let results = result.unwrap();
        assert!(!results.is_empty(), "Should return non-empty results");
    }

    #[test]
    fn test_wolfram_alpha_compute_image_only() {
        let result = wolfram_alpha_compute("solve x^2 + 3x + 2 = 0", true);
        println!("result: {:?}", result);
        assert!(
            result.is_ok(),
            "Should successfully compute with image_only=true"
        );
        let results = result.unwrap();
        assert!(!results.is_empty(), "Should return non-empty results");

        // Verify image_only mode doesn't return plaintext data
        for result in &results {
            if result.title.is_some() && result.img_base64.is_some() {
                assert!(
                    result.plaintext.is_none() || result.plaintext.as_ref().unwrap().is_empty(),
                    "Image-only mode should not include plaintext"
                );
            }
        }
    }

    #[test]
    fn test_format_to_markdown() {
        let results = vec![
            WolframAlphaResult {
                title: Some("Test Result".to_string()),
                plaintext: Some("x^2 + 2x + 1".to_string()),
                minput: Some("x^2 + 2x + 1".to_string()),
                moutput: Some("(x + 1)^2".to_string()),
                img_base64: Some("MOCK_BASE64".to_string()),
                img_contenttype: Some("image/png".to_string()),
                related_queries: vec!["related query".to_string()],
            }
        ];
        
        let md = format_to_markdown(&results);
        assert!(md.contains("Test Result"));
        assert!(md.contains("Expr:x^2 + 2x + 1"));
        assert!(md.contains("![Image](data:image/png;base64,MOCK_BASE64)"));
    }
}