use base64::{engine::general_purpose, Engine as _};
use futures_util::{SinkExt, StreamExt};
use html_escape;
use log::{debug, error, info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

// 缓存结构，用于存储查询结果
static WOLFRAM_CACHE: Lazy<Mutex<HashMap<String, Vec<WolframResult>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// 缓存大小限制
const CACHE_SIZE_LIMIT: usize = 100;

// 日志函数定义
fn log_message(level: &str, module: &str, message: &str, data: Option<&serde_json::Value>) {
    match level {
        "INFO" => info!("[{}] {} {:?}", module, message, data),
        "ERROR" => error!("[{}] {} {:?}", module, message, data),
        "WARN" => warn!("[{}] {} {:?}", module, message, data),
        "DEBUG" => debug!("[{}] {} {:?}", module, message, data),
        _ => info!("[{}] {} {:?}", module, message, data),
    }
}
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WolframResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plaintext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub img_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub img_contenttype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minput: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moutput: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relatedQueries: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WolframQuery {
    t: u8,
    v: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WolframInitMessage {
    category: String,
    #[serde(rename = "type")]
    message_type: String,
    lang: String,
    wa_pro_s: String,
    wa_pro_t: String,
    wa_pro_u: String,
    exp: u64,
    displayDebuggingInfo: bool,
    messages: Vec<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WolframQueryMessage {
    #[serde(rename = "type")]
    message_type: String,
    locationId: String,
    language: String,
    displayDebuggingInfo: bool,
    yellowIsError: bool,
    requestSidebarAd: bool,
    category: String,
    input: String, // Base64编码的查询
    i2d: bool,
    assumption: Vec<String>,
    apiParams: serde_json::Value,
    file: Option<String>,
    theme: String,
}

// 从Wolfram Alpha获取计算结果
pub async fn wolfram_alpha_compute(
    query: &str,
    image_only: bool,
) -> Result<Vec<WolframResult>, String> {
    // 检查缓存
    let cache_key = format!("{}-{}", query, image_only);
    {
        let cache = WOLFRAM_CACHE.lock().unwrap();
        if let Some(cached_results) = cache.get(&cache_key) {
            return Ok(cached_results.clone());
        }
    }

    // 创建查询
    let q = vec![WolframQuery {
        t: 0,
        v: query.to_string(),
    }];
    let mut results = Vec::new();

    // 连接到Wolfram Alpha WebSocket
    let (mut ws_stream, _) = connect_async("wss://gateway.wolframalpha.com/gateway")
        .await
        .map_err(|e| format!("无法连接到Wolfram Alpha: {}", e))?;

    // 发送初始化消息
    let init_message = WolframInitMessage {
        category: "results".to_string(),
        message_type: "init".to_string(),
        lang: "en".to_string(),
        wa_pro_s: "".to_string(),
        wa_pro_t: "".to_string(),
        wa_pro_u: "".to_string(),
        exp: chrono::Utc::now().timestamp_millis() as u64,
        displayDebuggingInfo: false,
        messages: vec![],
    };

    let init_json =
        serde_json::to_string(&init_message).map_err(|e| format!("序列化初始化消息失败: {}", e))?;

    ws_stream
        .send(Message::Text(init_json))
        .await
        .map_err(|e| format!("发送初始化消息失败: {}", e))?;

    // 接收响应
    let response = ws_stream
        .next()
        .await
        .ok_or_else(|| "没有收到响应".to_string())?
        .map_err(|e| format!("接收响应失败: {}", e))?;

    if let Message::Text(text) = response {
        let response_json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

        if response_json["type"] != "ready" {
            log_message("ERROR", "WolframAlpha", "Error:", Some(&response_json));
            return Err("Wolfram Alpha服务未就绪".to_string());
        }

        log_message("INFO", "WolframAlpha", "Response:", Some(&response_json));
    } else {
        return Err("收到的响应不是文本格式".to_string());
    }

    // 准备查询
    let input_json = serde_json::to_string(&q).map_err(|e| format!("序列化查询失败: {}", e))?;
    let input_base64 = general_purpose::STANDARD.encode(input_json);

    let query_message = WolframQueryMessage {
        message_type: "newQuery".to_string(),
        locationId: "oi8ft_en_light".to_string(),
        language: "en".to_string(),
        displayDebuggingInfo: false,
        yellowIsError: false,
        requestSidebarAd: false,
        category: "results".to_string(),
        input: input_base64,
        i2d: true,
        assumption: vec![],
        apiParams: serde_json::json!({}),
        file: None,
        theme: "light".to_string(),
    };

    let query_json =
        serde_json::to_string(&query_message).map_err(|e| format!("序列化查询消息失败: {}", e))?;

    log_message(
        "INFO",
        "WolframAlpha",
        "Sending Query:",
        Some(&serde_json::from_str(&query_json).unwrap_or(serde_json::json!({}))),
    );

    ws_stream
        .send(Message::Text(query_json))
        .await
        .map_err(|e| format!("发送查询消息失败: {}", e))?;

    // 接收查询结果
    loop {
        if let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let response_json: serde_json::Value =
                        serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

                    // 检查查询是否完成
                    if response_json["type"] == "queryComplete" {
                        break;
                    }

                    // 处理pods (若存在)
                    if let Some(pods) = response_json.get("pods") {
                        if let Some(pods_array) = pods.as_array() {
                            for pod in pods_array {
                                if let Some(subpods) = pod.get("subpods") {
                                    if let Some(subpods_array) = subpods.as_array() {
                                        let mut data = WolframResult {
                                            title: pod
                                                .get("title")
                                                .and_then(|t| t.as_str())
                                                .map(String::from),
                                            plaintext: None,
                                            img_base64: None,
                                            img_contenttype: None,
                                            minput: None,
                                            moutput: None,
                                            relatedQueries: None,
                                        };

                                        for subpod in subpods_array {
                                            // 提取文本内容
                                            if !image_only {
                                                if let Some(plaintext) =
                                                    subpod.get("plaintext").and_then(|t| t.as_str())
                                                {
                                                    data.plaintext = Some(plaintext.to_string());
                                                }

                                                if let Some(minput) =
                                                    subpod.get("minput").and_then(|t| t.as_str())
                                                {
                                                    data.minput = Some(minput.to_string());
                                                }

                                                if let Some(moutput) =
                                                    subpod.get("moutput").and_then(|t| t.as_str())
                                                {
                                                    data.moutput = Some(moutput.to_string());
                                                }
                                            }

                                            // 提取图片
                                            if let Some(img) = subpod.get("img") {
                                                if let Some(img_data) =
                                                    img.get("data").and_then(|d| d.as_str())
                                                {
                                                    data.img_base64 = Some(img_data.to_string());
                                                }
                                                if let Some(content_type) =
                                                    img.get("contenttype").and_then(|t| t.as_str())
                                                {
                                                    data.img_contenttype =
                                                        Some(content_type.to_string());
                                                }
                                            }
                                        }

                                        results.push(data);
                                    }
                                }
                            }
                        }
                    }

                    // 处理相关查询 (若存在)
                    if let Some(related_queries) = response_json.get("relatedQueries") {
                        if let Some(queries_array) = related_queries.as_array() {
                            let queries: Vec<String> = queries_array
                                .iter()
                                .filter_map(|q| q.as_str().map(String::from))
                                .collect();

                            if !queries.is_empty() {
                                results.push(WolframResult {
                                    title: None,
                                    plaintext: None,
                                    img_base64: None,
                                    img_contenttype: None,
                                    minput: None,
                                    moutput: None,
                                    relatedQueries: Some(queries),
                                });
                            }
                        }
                    }
                }
                Ok(_) => {} // 忽略非文本消息
                Err(e) => return Err(format!("接收响应失败: {}", e)),
            }
        } else {
            break; // 连接关闭
        }
    }

    log_message(
        "INFO",
        "WolframAlpha",
        "Results count:",
        Some(&serde_json::json!(results.len())),
    );

    // 更新缓存
    {
        let mut cache = WOLFRAM_CACHE.lock().unwrap();
        // 检查缓存大小，如果超过限制，移除最旧的项目
        if cache.len() >= CACHE_SIZE_LIMIT && !cache.contains_key(&cache_key) {
            // 简单地移除任意一个条目 (在实际应用中可以使用LRU缓存)
            if let Some(old_key) = cache.keys().next().cloned() {
                cache.remove(&old_key);
            }
        }
        cache.insert(cache_key, results.clone());
    }

    Ok(results)
}

// 将Wolfram结果转换为HTML格式
pub fn format_to_html(results: &[WolframResult]) -> String {
    if results.is_empty() {
        return r#"<div class="alert alert-warning" role="alert">没有找到结果</div>"#.to_string();
    }

    let mut html = r#"<div class="wolfram-results">"#.to_string();

    for result in results {
        html.push_str(r#"<div class="wolfram-result-item">"#);

        // 添加标题
        if let Some(title) = &result.title {
            html.push_str(&format!(
                r#"<h3 class="wolfram-item-title">{}</h3>"#,
                html_escape::encode_text(title)
            ));
        }

        // 添加文本内容
        if let Some(plaintext) = &result.plaintext {
            html.push_str(&format!(
                r#"<p class="wolfram-item-text"><strong>Expr:</strong> {}</p>"#,
                html_escape::encode_text(plaintext)
            ));
        }

        // 添加图片
        if let Some(img_base64) = &result.img_base64 {
            let content_type = result.img_contenttype.as_deref().unwrap_or("image/png");
            html.push_str(&format!(
                r#"<p class="wolfram-item-image"><img src="data:{};base64,{}" alt="Wolfram Alpha result" /></p>"#,
                content_type, img_base64
            ));
        }

        // 添加Mathematica输入
        if let Some(minput) = &result.minput {
            html.push_str(&format!(r#"<p class="wolfram-item-code"><strong>Mathematica Input:</strong> <code>{}</code></p>"#, html_escape::encode_text(minput)));
        }

        // 添加Mathematica输出
        if let Some(moutput) = &result.moutput {
            html.push_str(&format!(r#"<p class="wolfram-item-code"><strong>Mathematica Output:</strong> <code>{}</code></p>"#, html_escape::encode_text(moutput)));
        }

        // 添加相关查询
        if let Some(queries) = &result.relatedQueries {
            html.push_str(
                r#"<div class="wolfram-related-queries"><strong>Related Queries:</strong><ul>"#,
            );
            for query in queries {
                html.push_str(&format!(r#"<li>{}</li>"#, html_escape::encode_text(query)));
            }
            html.push_str(r#"</ul></div>"#);
        }

        html.push_str(r#"</div><hr />"#);
    }

    // 移除最后一个分隔线
    if html.ends_with("<hr />") {
        html.truncate(html.len() - 7);
    }

    html.push_str(r#"</div>"#);
    html
}

// 将Wolfram结果转换为Markdown格式
pub fn format_to_markdown(results: &[WolframResult]) -> String {
    if results.is_empty() {
        return r#"<div class="alert alert-warning" role="alert">没有找到结果</div>"#.to_string();
    }

    let mut md = String::new();

    for result in results {
        // 添加标题
        if let Some(title) = &result.title {
            md.push_str(&format!("## {}\n\n", title));
        }

        // 添加文本内容
        if let Some(plaintext) = &result.plaintext {
            md.push_str(&format!("**Expr:** {}\n\n", plaintext));
        }

        // 添加图片
        if let Some(img_base64) = &result.img_base64 {
            let content_type = result.img_contenttype.as_deref().unwrap_or("image/png");
            md.push_str(&format!(
                "![Image](data:{};base64,{})\n\n",
                content_type, img_base64
            ));
        }

        // 添加Mathematica输入
        if let Some(minput) = &result.minput {
            md.push_str(&format!("**Mathematica Input:** `{}`\n\n", minput));
        }

        // 添加Mathematica输出
        if let Some(moutput) = &result.moutput {
            md.push_str(&format!("**Mathematica Output:** `{}`\n\n", moutput));
        }

        // 添加相关查询
        if let Some(queries) = &result.relatedQueries {
            md.push_str("**Related Queries:**\n\n");
            for query in queries {
                md.push_str(&format!("- {}\n", query));
            }
            md.push_str("\n");
        }

        md.push_str("---\n\n");
    }

    // 移除最后一个分隔线
    if md.ends_with("---\n\n") {
        md.truncate(md.len() - 6);
    }

    md
}
