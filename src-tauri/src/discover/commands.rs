use crate::discover::helpers::mc_news::{fetch_mc_news_page, MC_NEWS_ENDPOINT};
use crate::discover::models::{NewsPostRequest, NewsPostResponse, NewsSourceInfo};
use std::collections::HashMap;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn fetch_news_sources_info(
    app: AppHandle,
) -> Result<Vec<NewsSourceInfo>, String> {
    let client = app.state::<reqwest::Client>().inner().clone();

    let config = app
        .try_state::<std::sync::Arc<tokio::sync::Mutex<crate::api::LauncherConfig>>>()
        .map(|s| s.blocking_lock().clone());

    let endpoints = config
        .as_ref()
        .map(|c| c.discover_source_endpoints.clone())
        .unwrap_or_default();

    let futs: Vec<_> = endpoints
        .iter()
        .map(|(url, _enabled)| {
            let client = client.clone();
            let url = url.clone();
            tokio::spawn(async move {
                let probe_url = if url.starts_with(MC_NEWS_ENDPOINT) {
                    url.clone()
                } else {
                    format!("{}?pageSize=0", url)
                };
                let response = client.get(&probe_url).send().await.ok()?;
                if !response.status().is_success() {
                    return Some(NewsSourceInfo {
                        name: String::new(),
                        full_name: String::new(),
                        endpoint_url: url,
                        icon_src: String::new(),
                    });
                }
                let json: serde_json::Value = response.json().await.ok()?;
                let source_info = json.get("sourceInfo")?;
                Some(NewsSourceInfo {
                    name: source_info.get("name")?.as_str()?.to_string(),
                    full_name: source_info.get("fullName")?.as_str()?.to_string(),
                    endpoint_url: url,
                    icon_src: source_info.get("iconSrc")?.as_str()?.to_string(),
                })
            })
        })
        .collect();

    let results = futures::future::join_all(futs).await;
    let sources: Vec<NewsSourceInfo> = results
        .into_iter()
        .filter_map(|r| r.ok().flatten())
        .collect();

    Ok(sources)
}

#[tauri::command]
pub async fn fetch_news_post_summaries(
    app: AppHandle,
    requests: Vec<NewsPostRequest>,
) -> Result<NewsPostResponse, String> {
    let client = app.state::<reqwest::Client>().inner().clone();

    let futs: Vec<_> = requests
        .into_iter()
        .map(|req| {
            let client = client.clone();
            tokio::spawn(async move {
                if req.url.starts_with(MC_NEWS_ENDPOINT) {
                    fetch_mc_news_page(&client, &req.url, req.cursor).await
                } else {
                    fetch_generic_news_page(&client, &req.url, req.cursor).await
                }
            })
        })
        .collect();

    let results = futures::future::join_all(futs).await;

    let mut all_posts = Vec::new();
    let mut cursors = HashMap::new();

    for result in results {
        if let Ok(Some((url, response))) = result {
            all_posts.extend(response.posts);
            if let Some(next) = response.next {
                cursors.insert(url, next);
            }
            if let Some(more_cursors) = response.cursors {
                cursors.extend(more_cursors);
            }
        }
    }

    all_posts.sort_by(|a, b| b.create_at.cmp(&a.create_at));

    Ok(NewsPostResponse {
        posts: all_posts,
        next: None,
        cursors: if cursors.is_empty() { None } else { Some(cursors) },
    })
}

async fn fetch_generic_news_page(
    client: &reqwest::Client,
    url: &str,
    cursor: Option<u64>,
) -> Option<(String, NewsPostResponse)> {
    let url_str = if let Some(c) = cursor {
        let mut parsed = url::Url::parse(url).ok()?;
        parsed.query_pairs_mut().append_pair("cursor", &c.to_string());
        parsed.to_string()
    } else {
        url.to_string()
    };

    let response = client
        .get(&url_str)
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let mut response_data: NewsPostResponse = response.json().await.ok()?;

    for post in &mut response_data.posts {
        post.source.endpoint_url = url.to_string();
    }

    Some((url.to_string(), response_data))
}
