use crate::discover::models::{NewsPostResponse, NewsPostSummary, NewsSourceInfo};
use serde::Deserialize;

pub const MC_NEWS_ENDPOINT: &str = "https://net-secondary.web.minecraft-services.net/api/v1.0";
pub const MC_NEWS_DEFAULT_PAGE_SIZE: u32 = 12;
pub const MC_NET_ICON: &str = "https://www.minecraft.net/favicon.ico";

#[derive(Deserialize, Debug)]
struct McNewsSearchResponse {
    result: Option<McNewsResult>,
}

#[derive(Deserialize, Debug)]
struct McNewsResult {
    results: Vec<McNewsItem>,
    num_found: u32,
}

#[derive(Deserialize, Debug)]
struct McNewsItem {
    title: String,
    url: String,
    description: Option<String>,
    image: Option<McNewsImage>,
    time: i64,
}

#[derive(Deserialize, Debug)]
struct McNewsImage {
    url: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

fn parse_mc_timestamp(ts: i64) -> String {
    if ts > 10_000_000_000 {
        let secs = ts / 1000;
        let nsecs = ((ts % 1000) * 1_000_000) as u32;
        match chrono::DateTime::from_timestamp(secs, nsecs) {
            Some(dt) => dt.to_rfc3339(),
            None => ts.to_string(),
        }
    } else {
        match chrono::DateTime::from_timestamp(ts, 0) {
            Some(dt) => dt.to_rfc3339(),
            None => ts.to_string(),
        }
    }
}

pub fn mc_news_source_info(endpoint_url: &str) -> NewsSourceInfo {
    NewsSourceInfo {
        name: "Minecraft".to_string(),
        full_name: "Minecraft Official News".to_string(),
        endpoint_url: endpoint_url.to_string(),
        icon_src: MC_NET_ICON.to_string(),
    }
}

impl From<(McNewsItem, NewsSourceInfo)> for NewsPostSummary {
    fn from((item, source): (McNewsItem, NewsSourceInfo)) -> Self {
        let image_src = item.image.map(|img| {
            vec![
                serde_json::Value::String(img.url.unwrap_or_default()),
                serde_json::Value::Number(serde_json::Number::from(img.width.unwrap_or(0))),
                serde_json::Value::Number(serde_json::Number::from(img.height.unwrap_or(0))),
            ]
        });

        NewsPostSummary {
            title: item.title,
            abstracts: item.description,
            keywords: None,
            image_src,
            source,
            create_at: parse_mc_timestamp(item.time),
            link: item.url,
        }
    }
}

pub async fn fetch_mc_news_page(
    client: &reqwest::Client,
    base_url: &str,
    cursor: Option<u64>,
) -> Option<(String, NewsPostResponse)> {
    let page: u32 = cursor.map(|c| c as u32).unwrap_or(1);
    let url_str = format!(
        "{}/zh-cn/search?page={}&pageSize={}&sortType=Recent&category=News&newsOnly=true",
        base_url, page, MC_NEWS_DEFAULT_PAGE_SIZE
    );

    let response = client.get(&url_str).send().await.ok()?;

    if !response.status().is_success() {
        return None;
    }

    let search_res: McNewsSearchResponse = response.json().await.ok()?;
    let result = search_res.result?;

    let source_info = mc_news_source_info(base_url);
    let posts: Vec<NewsPostSummary> = result
        .results
        .into_iter()
        .map(|item| (item, source_info.clone()).into())
        .collect();

    let has_more = (page * MC_NEWS_DEFAULT_PAGE_SIZE) < result.num_found;
    let next = if has_more {
        Some(page as u64 + 1)
    } else {
        None
    };

    Some((
        base_url.to_string(),
        NewsPostResponse {
            posts,
            next,
            cursors: None,
        },
    ))
}
