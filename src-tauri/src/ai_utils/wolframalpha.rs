use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use futures::{SinkExt, StreamExt};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tauri::http;
use thiserror::Error;
use tokio::runtime::Runtime;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

// --- Error Handling ---
#[derive(Error, Debug)]
enum WolframAlphaError {
    #[error("WebSocket connection error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("JSON (de)serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("URL parsing error: {0}")]
    UrlError(#[from] url::ParseError),
    #[error("Base64 encoding error: {0}")]
    Base64EncodeError(#[from] base64::DecodeError), // base64::EncodeError is not an explicit type, uses DecodeError
    #[error("Initial connection response not ready: {0:?}")]
    InitNotReady(serde_json::Value),
    #[error("Received unexpected message type")]
    UnexpectedMessageType,
    #[error("Query failed or returned no pods")]
    NoPodsFound,
    #[error("Missing required data in response")]
    MissingData,
    #[error("HTTP error during WebSocket connection: {0}")]
    HttpError(http::StatusCode), // http crate is a dependency of tokio-tungstenite
}

// --- Data Structures (Mirroring JSON) ---

// Generic message envelope (simplified)
#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(untagged)] // Allow deserializing to different types based on content
enum WaMessage {
    Init(InitMessage),
    NewQuery(NewQueryMessage),
    Response(Response),
    QueryComplete(QueryCompleteMessage),
    // Add other message types if needed
    Other(serde_json::Value), // Catch any unknown messages
}

// --- Messages Sent to Gateway ---

#[derive(serde::Serialize, serde::Deserialize, Debug)]
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
    messages: Vec<serde_json::Value>, // Use Value for potentially mixed types
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
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
    assumption: Vec<serde_json::Value>, // Use Value for flexibility
    api_params: serde_json::Value,      // API parameters as a JSON value
    file: Option<String>,               // Or Option<serde_json::Value>
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
                          // Add other image fields if needed
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

// --- Core Compute Function ---

/// Connects to Wolfram Alpha WebSocket gateway, sends a query, and parses results.
///
/// If `image_only` is true, only includes title, image data, and related queries
/// in the results where available. Otherwise, includes all available data.
async fn wolfram_alpha_compute_async(
    query: &str,
    image_only: bool,
) -> Result<Vec<WolframAlphaResult>, WolframAlphaError> {
    println!("Query: {}", query);
    let url = Url::parse("wss://gateway.wolframalpha.com/gateway")?;

    println!("Connecting to {}", url);
    let (mut stream, response) = connect_async(url).await?;
    println!("Connected. HTTP status: {}", response.status());

    if response.status() != http::StatusCode::SWITCHING_PROTOCOLS && !response.status().is_success()
    {
        return Err(WolframAlphaError::HttpError(response.status()));
    }

    // 1. Send Init Message
    let init_msg = InitMessage {
        category: "results".to_string(),
        msg_type: "init".to_string(),
        lang: "en".to_string(),
        wa_pro_s: "".to_string(), // Provide actual if available
        wa_pro_t: "".to_string(), // Provide actual if available
        wa_pro_u: "".to_string(), // Provide actual if available
        exp: 1714399254570,       // !! WARNING: Hardcoded expiry, may need dynamic generation !!
        display_debugging_info: false,
        messages: vec![],
    };
    let init_json = serde_json::to_string(&init_msg)?;
    info!("Sending init message: {}", init_json);
    stream.send(Message::Text(init_json)).await?;

    // 2. Receive Init Response
    match stream.next().await {
        Some(Ok(Message::Text(text))) => {
            let resp_val: serde_json::Value = serde_json::from_str(&text)?;
            info!("Received init response: {:?}", resp_val);
            if resp_val["type"] != "ready" {
                error!("Init response not ready: {:?}", resp_val);
                return Err(WolframAlphaError::InitNotReady(resp_val));
            }
        }
        Some(Ok(_)) => return Err(WolframAlphaError::UnexpectedMessageType),
        Some(Err(e)) => return Err(WolframAlphaError::WebSocketError(e)),
        None => return Err(WolframAlphaError::UnexpectedMessageType), // Connection closed prematurely
    }

    // 3. Send New Query Message
    let q_data = vec![serde_json::json!({"t": 0, "v": query})];
    let q_json = serde_json::to_string(&q_data)?;
    let q_base64 = STANDARD.encode(q_json.as_bytes());

    let new_query_msg = NewQueryMessage {
        msg_type: "newQuery".to_string(),
        location_id: "oi8ft_en_light".to_string(), // Example ID, may vary
        language: "en".to_string(),
        display_debugging_info: false,
        yellow_is_error: false,
        request_sidebar_ad: false,
        category: "results".to_string(),
        input: q_base64,
        i2d: true, // Input 2D?
        assumption: vec![],
        api_params: serde_json::json!({}), // Use empty JSON object for api_params
        file: None,
        theme: "light".to_string(),
    };

    let new_query_json = serde_json::to_string(&new_query_msg)?;
    info!(
        "Sending query message (input base64-encoded): {}",
        new_query_json
    );
    stream.send(Message::Text(new_query_json)).await?;

    // 4. Receive and Process Results
    let mut results: Vec<WolframAlphaResult> = Vec::new();

    while let Some(msg_result) = stream.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                let wa_msg: WaMessage = match serde_json::from_str(&text) {
                    Ok(msg) => msg,
                    Err(e) => {
                        // Log parsing errors but don't necessarily stop
                        error!("Failed to parse message JSON: {}. Message: {}", e, text);
                        continue;
                    }
                };

                match wa_msg {
                    WaMessage::QueryComplete(_) => {
                        println!("Query complete message received.");
                        println!("msg: {}", text);
                        break; // Exit loop when query is complete
                    }
                    WaMessage::Response(resp) => {
                        println!("Received response message with type: {}", resp.msg_type);
                        if !resp.related_queries.is_empty() {
                            // Handle top-level related queries separately or integrate
                            // Here we add them to the first result or as a standalone if no pods
                            if results.is_empty() {
                                results.push(WolframAlphaResult {
                                    title: None,
                                    plaintext: None,
                                    minput: None,
                                    moutput: None,
                                    img_base64: None,
                                    img_contenttype: None,
                                    related_queries: resp.related_queries,
                                });
                            } else {
                                // Append to the last result's related queries or a new entry
                                // Appending to last result for simplicity
                                if let Some(last_result) = results.last_mut() {
                                    last_result.related_queries.extend(resp.related_queries);
                                } else {
                                    results.push(WolframAlphaResult {
                                        title: None,
                                        plaintext: None,
                                        minput: None,
                                        moutput: None,
                                        img_base64: None,
                                        img_contenttype: None,
                                        related_queries: resp.related_queries,
                                    });
                                }
                            }
                        }

                        for pod in resp.pods {
                            let mut result_data = WolframAlphaResult {
                                title: Some(pod.title),
                                plaintext: None,
                                minput: None,
                                moutput: None,
                                img_base64: None,
                                img_contenttype: None,
                                related_queries: vec![], // Pod-level related queries are less common, main ones are top-level
                            };

                            for subpod in pod.subpods {
                                // Apply image_only filter here during parsing
                                if !image_only {
                                    result_data.plaintext =
                                        subpod.plaintext.or(result_data.plaintext); // Take first non-None
                                    result_data.minput = subpod.minput.or(result_data.minput);
                                    result_data.moutput = subpod.moutput.or(result_data.moutput);
                                }

                                if let Some(img) = subpod.img {
                                    // Include image data regardless of image_only flag here, similar to Python logic
                                    // If image_only was strict, this 'if' would be guarded by 'if image_only {'
                                    result_data.img_base64 = img.data.or(result_data.img_base64);
                                    result_data.img_contenttype =
                                        img.contenttype.or(result_data.img_contenttype);
                                }
                            }
                            results.push(result_data);
                        }
                    }
                    WaMessage::Other(val) => {
                        // Log messages that didn't match known types
                        warn!("Received unknown message type: {:?}", val);
                    }
                    _ => {
                        // Ignore other message types like Init, NewQuery if received back (unlikely)
                        info!("Ignoring message type: {:?}", wa_msg);
                    }
                }
            }
            Ok(Message::Binary(_)) => println!("Received binary message (ignored)."),
            Ok(Message::Ping(_)) => println!("Received ping."),
            Ok(Message::Pong(_)) => println!("Received pong."),
            Ok(Message::Close(_)) => {
                println!("Received close message. Exiting loop.");
                break; // Server initiated close
            }
            Ok(Message::Frame(_)) => println!("Received frame message (ignored)."),
            Err(e) => {
                println!("message error: {}", e);
                error!("WebSocket message error: {}", e);
                return Err(WolframAlphaError::WebSocketError(e));
            }
        }
    }

    println!("Final results collected: {:?}", results);
    Ok(results)
}

// --- Formatting Functions ---

// Note: These return String or Vec<Value> (for Mirai-like JSON), not async in Rust
// unless the formatting itself involved async operations (e.g., fetching external data)

/// Formats results into a structure suitable for Mirai Console WebSocket API.
pub fn format_to_mirai_ws(results: &[WolframAlphaResult]) -> Option<Vec<serde_json::Value>> {
    if results.is_empty() {
        return None;
    }

    let mut msg_list: Vec<serde_json::Value> = Vec::new();
    let n = results.len();

    for (i, result) in results.iter().enumerate() {
        if let Some(title) = &result.title {
            msg_list.push(json!({"type": "Plain", "text": format!("{}\n", title)}));
        }
        if let Some(plaintext) = &result.plaintext {
            msg_list.push(json!({"type": "Plain", "text": format!("Expr:{}\n", plaintext)}));
        }
        if let Some(img_base64) = &result.img_base64 {
            msg_list.push(json!({"type": "Image", "base64": img_base64}));
        }
        if let Some(minput) = &result.minput {
            msg_list
                .push(json!({"type": "Plain", "text": format!("Mathematica Input:{}\n", minput)}));
        }
        if let Some(moutput) = &result.moutput {
            msg_list.push(
                json!({"type": "Plain", "text": format!("Mathematica Output:{}\n", moutput)}),
            );
        }
        if !result.related_queries.is_empty() {
            msg_list.push(json!({"type": "Plain", "text": "Related Queries:\n"}));
            for query in &result.related_queries {
                msg_list.push(json!({"type": "Plain", "text": format!("{}\n", query)}));
            }
        }

        if i < n - 1 {
            msg_list.push(json!({"type": "Plain", "text": "----------------------\n"}));
        }
    }

    // Remove trailing newlines from the last plain text element
    if let Some(last_elem) = msg_list.last_mut() {
        if let Some(text) = last_elem.get_mut("text").and_then(|t| t.as_str()) {
            let trimmed_text = text.trim_end_matches('\n');
            if trimmed_text != text {
                last_elem["text"] = json!(trimmed_text);
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

        // Python's original Markdown had a commented out separator
        // if i < n - 1 {
        //    ret.push_str("\n---\n");
        // }
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

// --- Main Function ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wolfram_alpha_compute_basic() {
        let rt = Runtime::new().unwrap();
        let result =
            rt.block_on(async { wolfram_alpha_compute_async("integral of x^2", false).await });
        print!("\nresult: {:?}", result);
        assert!(result.is_ok(), "Should successfully compute the query");
        let results = result.unwrap();
        // assert!(!results.is_empty(), "Should return non-empty results");
    }

    #[test]
    fn test_wolfram_alpha_compute_image_only() {
        let rt = Runtime::new().unwrap();
        let result = rt
            .block_on(async { wolfram_alpha_compute_async("solve x^2 + 3x + 2 = 0", true).await });
        print!("result: {:?}", result);
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
    fn test_format_to_mirai_ws() {
        let results = vec![WolframAlphaResult {
            title: Some("Test Title".to_string()),
            plaintext: Some("Test Plaintext".to_string()),
            minput: Some("x^2".to_string()),
            moutput: Some("x^2".to_string()),
            img_base64: Some("base64data".to_string()),
            img_contenttype: Some("image/png".to_string()),
            related_queries: vec!["Related Query 1".to_string()],
        }];

        let formatted = format_to_mirai_ws(&results);
        assert!(formatted.is_some(), "Should format non-empty results");

        let formatted_vec = formatted.unwrap();
        assert!(
            !formatted_vec.is_empty(),
            "Formatted output should not be empty"
        );

        // Verify the JSON structure contains expected elements
        assert!(formatted_vec
            .iter()
            .any(|val| val["type"] == "Plain"
                && val["text"].as_str().unwrap().contains("Test Title")));
        assert!(formatted_vec
            .iter()
            .any(|val| val["type"] == "Image" && val["base64"] == "base64data"));
    }

    #[test]
    fn test_format_to_cq() {
        let results = vec![WolframAlphaResult {
            title: Some("Test Title".to_string()),
            plaintext: Some("Test Plaintext".to_string()),
            minput: None,
            moutput: None,
            img_base64: Some("base64data".to_string()),
            img_contenttype: None,
            related_queries: vec![],
        }];

        let formatted = format_to_cq(&results);
        assert!(formatted.is_some(), "Should format non-empty results");

        let cq_string = formatted.unwrap();
        assert!(cq_string.contains("Test Title"), "Should contain title");
        assert!(
            cq_string.contains("Test Plaintext"),
            "Should contain plaintext"
        );
        assert!(
            cq_string.contains("[CQ:image,file=base64://base64data]"),
            "Should contain CQ image format"
        );
    }

    #[test]
    fn test_format_to_markdown() {
        let results = vec![WolframAlphaResult {
            title: Some("Test Title".to_string()),
            plaintext: Some("Test Plaintext".to_string()),
            minput: None,
            moutput: None,
            img_base64: Some("base64data".to_string()),
            img_contenttype: Some("image/png".to_string()),
            related_queries: vec![],
        }];

        let markdown = format_to_markdown(&results);
        assert!(markdown.contains("Test Title"), "Should contain title");
        assert!(
            markdown.contains("![Image](data:image/png;base64,base64data)"),
            "Should format base64 image correctly"
        );
    }

    #[test]
    fn test_format_to_html() {
        let results = vec![WolframAlphaResult {
            title: Some("Test Title".to_string()),
            plaintext: Some("Test Plaintext".to_string()),
            minput: None,
            moutput: None,
            img_base64: Some("base64data".to_string()),
            img_contenttype: Some("image/png".to_string()),
            related_queries: vec!["Related Query 1".to_string()],
        }];

        let html = format_to_html(&results);
        assert!(
            html.contains("<h2>Test Title</h2>"),
            "Should format title as h2"
        );
        assert!(
            html.contains("<img src='data:image/png;base64,base64data'"),
            "Should format image data correctly"
        );
        assert!(
            html.contains("<li>Related Query 1</li>"),
            "Should format related queries as list items"
        );
    }

    #[test]
    fn test_format_to_xml() {
        let results = vec![WolframAlphaResult {
            title: Some("Test Title".to_string()),
            plaintext: Some("Test Plaintext".to_string()),
            minput: None,
            moutput: None,
            img_base64: None,
            img_contenttype: None,
            related_queries: vec![],
        }];

        let xml = format_to_xml(&results);
        assert!(
            xml.contains("<title>Test Title</title>"),
            "Should format title correctly"
        );
        assert!(
            xml.contains("<plaintext>Test Plaintext</plaintext>"),
            "Should format plaintext correctly"
        );
    }

    #[test]
    fn test_empty_results_formatting() {
        let empty_results: Vec<WolframAlphaResult> = vec![];

        assert!(
            format_to_mirai_ws(&empty_results).is_none(),
            "Should return None for empty results"
        );
        assert!(
            format_to_cq(&empty_results).is_none(),
            "Should return None for empty results"
        );

        let markdown = format_to_markdown(&empty_results);
        assert!(
            markdown.contains("No results"),
            "Should indicate no results in markdown"
        );

        let html = format_to_html(&empty_results);
        assert!(
            html.contains("No results"),
            "Should indicate no results in HTML"
        );

        let xml = format_to_xml(&empty_results);
        assert!(
            xml.contains("No results"),
            "Should indicate no results in XML"
        );
    }

    #[test]
    fn test_html_escaping() {
        let html_input = "x < 3 && y > 2";
        let escaped = escape_html(html_input);
        assert_eq!(escaped, "x &lt; 3 &amp;&amp; y &gt; 2");
    }

    #[test]
    fn test_xml_escaping() {
        let xml_input = "<function>x & y</function>";
        let escaped = escape_xml(xml_input);
        assert_eq!(escaped, "&lt;function&gt;x &amp; y&lt;/function&gt;");
    }
}
