use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsSourceInfo {
    pub name: String,
    pub full_name: String,
    pub endpoint_url: String,
    pub icon_src: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsPostSummary {
    pub title: String,
    #[serde(rename = "abstract")]
    pub abstracts: Option<String>,
    pub keywords: Option<String>,
    pub image_src: Option<Vec<serde_json::Value>>,
    pub source: NewsSourceInfo,
    pub create_at: String,
    pub link: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsPostRequest {
    pub url: String,
    pub cursor: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsPostResponse {
    pub posts: Vec<NewsPostSummary>,
    pub next: Option<u64>,
    pub cursors: Option<HashMap<String, u64>>,
}
