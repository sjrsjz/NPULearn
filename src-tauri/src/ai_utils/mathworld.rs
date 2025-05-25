use reqwest;
use scraper::{Html, Selector};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::error::Error;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    title: String,
    link: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentResult {
    title: String,
    content: String,
}

pub async fn search(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let base_url = "https://mathworld.wolfram.com/search/?query=";
    let encoded_query = utf8_percent_encode(query, NON_ALPHANUMERIC).to_string();
    let search_url = format!("{}{}", base_url, encoded_query);
    
    let client = reqwest::Client::new();
    let response = client.get(&search_url).send().await?;
    
    if response.status().is_success() {
        let html = response.text().await?;
        let document = Html::parse_document(&html);
        
        let results_selector = Selector::parse("div.search-results").unwrap();
        let result_title_selector = Selector::parse("div.search-result-title").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        
        let mut results = Vec::new();
        
        if let Some(div) = document.select(&results_selector).next() {
            for item in div.select(&result_title_selector) {
                let title = item.text().collect::<Vec<_>>().join(" ").trim().to_string();
                
                if let Some(link_element) = item.select(&link_selector).next() {
                    if let Some(href) = link_element.value().attr("href") {
                        results.push(SearchResult {
                            title,
                            link: href.to_string(),
                        });
                    }
                }
            }
        }
        
        Ok(results)
    } else {
        Ok(Vec::new())
    }
}

/// 获取前n项搜索结果的纯文本内容
///
/// # Arguments
///
/// * `query` - 搜索查询
/// * `n` - 要获取的结果数量，默认为3
///
/// # Returns
///
/// 包含每个结果标题和内容的列表，如果发生错误则返回空列表
pub async fn get_content_from_results(query: &str, n: usize) -> Result<Vec<ContentResult>, Box<dyn Error>> {
    let results = search(query).await?;
    if results.is_empty() {
        return Ok(Vec::new());
    }

    let mut contents = Vec::new();
    let mut count = 0;
    let client = reqwest::Client::new();

    for result in results {
        if count >= n {
            break;
        }

        let url = if !result.link.starts_with("http") {
            format!("https://mathworld.wolfram.com{}", result.link)
        } else {
            result.link.clone()
        };

        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let html = response.text().await?;
                    let document = Html::parse_document(&html);
                    
                    let content_selector = Selector::parse("div.entry-content").unwrap();
                    
                    if let Some(content_div) = document.select(&content_selector).next() {
                        let content = content_div.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        contents.push(ContentResult {
                            title: result.title,
                            content,
                        });
                        count += 1;
                    }
                }
            },
            Err(_) => continue,
        }
    }

    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    
    #[tokio::test]
    async fn test_mathworld_search() {
        let query = "Pythagorean theorem";
        match get_content_from_results(query, 3).await {
            Ok(results) => {
                if !results.is_empty() {
                    for result in results {
                        println!("Title: {}", result.title);
                        println!("Content: {}...", &result.content[..std::cmp::min(200, result.content.len())]);
                        println!("{}", "-".repeat(80));
                    }
                } else {
                    println!("No results found.");
                }
            },
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}