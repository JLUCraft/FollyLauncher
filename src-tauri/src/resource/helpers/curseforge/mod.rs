pub mod misc;

use crate::resource::helpers::curseforge::misc::{
    cvt_category_to_id, cvt_mod_loader_to_id, cvt_sort_by_to_id, cvt_type_to_class_id,
    cvt_version_to_type_id, get_curseforge_api,
    map_curseforge_file_to_version_pack, CurseForgeFileInfo, CurseForgeFingerprintRes,
    CurseForgeGetProjectRes, CurseForgeSearchRes, CurseForgeVersionPackSearchRes,
};
use crate::resource::helpers::misc::apply_other_resource_enhancements;
use crate::resource::helpers::mod_db::handle_search_query;
use crate::resource::models::{
    OtherResourceApiEndpoint, OtherResourceFileInfo, OtherResourceInfo,
    OtherResourceSearchQuery, OtherResourceSearchRes, OtherResourceVersionPack,
    OtherResourceVersionPackQuery, ResourceError,
};
use std::collections::HashMap;
use tauri::{AppHandle, Manager};
use url::Url;

const MINECRAFT_GAME_ID: &str = "432";
const ALL_FILTER: &str = "All";
const WORD_PERFECT_MATCH_WEIGHT: usize = 10;

fn tokenize_words(text: &str) -> impl Iterator<Item = &str> {
    text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let b_chars: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
    for (i, a_ch) in a.chars().enumerate() {
        let mut current = Vec::with_capacity(b_chars.len() + 1);
        current.push(i + 1);
        for (j, b_ch) in b_chars.iter().enumerate() {
            let cost = if a_ch == *b_ch { 0 } else { 1 };
            let insertion = current[j] + 1;
            let deletion = prev[j + 1] + 1;
            let substitution = prev[j] + cost;
            current.push(insertion.min(deletion).min(substitution));
        }
        prev = current;
    }
    *prev.last().unwrap_or(&0)
}

fn get_cf_api_key() -> String {
    std::env::var("FOLLY_CURSEFORGE_API_KEY").unwrap_or_default()
}

async fn cf_get<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    params: Option<&HashMap<String, String>>,
) -> Result<T, ResourceError> {
    let key = get_cf_api_key();
    let full_url = if let Some(p) = params {
        let mut parsed = Url::parse(url).map_err(|_| ResourceError::NetworkError)?;
        parsed.query_pairs_mut().extend_pairs(p.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        parsed.to_string()
    } else {
        url.to_string()
    };
    let response = client.get(&full_url).header("x-api-key", &key).send().await.map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() {
        return Err(ResourceError::NetworkError);
    }
    response.json::<T>().await.map_err(|_| ResourceError::ParseError)
}

async fn cf_post<T: serde::de::DeserializeOwned, P: serde::Serialize>(
    client: &reqwest::Client,
    url: &str,
    payload: &P,
) -> Result<T, ResourceError> {
    let key = get_cf_api_key();
    let response = client
        .post(url)
        .header("x-api-key", &key)
        .json(payload)
        .send()
        .await
        .map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() {
        return Err(ResourceError::NetworkError);
    }
    response.json::<T>().await.map_err(|_| ResourceError::ParseError)
}

pub async fn fetch_resource_list_by_name_curseforge(
    app: &AppHandle,
    query: &OtherResourceSearchQuery,
) -> Result<OtherResourceSearchRes, ResourceError> {
    let url = get_curseforge_api(OtherResourceApiEndpoint::Search, None)?;
    let OtherResourceSearchQuery { resource_type, search_query, game_version, selected_tag, sort_by, page, page_size } = query;
    let handled_search_query = handle_search_query(app, search_query).await.unwrap_or(search_query.clone());
    let class_id = cvt_type_to_class_id(resource_type);
    let sort_field = cvt_sort_by_to_id(sort_by);
    let sort_order = match sort_field { 4 => "asc", _ => "desc" };
    let mut params = HashMap::new();
    params.insert("gameId".to_string(), MINECRAFT_GAME_ID.to_string());
    params.insert("classId".to_string(), class_id.to_string());
    params.insert("searchFilter".to_string(), handled_search_query.clone());
    if game_version != ALL_FILTER { params.insert("gameVersion".to_string(), game_version.to_string()); }
    if selected_tag != ALL_FILTER { params.insert("categoryId".to_string(), cvt_category_to_id(selected_tag, class_id).to_string()); }
    params.insert("sortField".to_string(), sort_field.to_string());
    params.insert("sortOrder".to_string(), sort_order.to_string());
    params.insert("index".to_string(), (page * page_size).to_string());
    params.insert("pageSize".to_string(), page_size.to_string());
    let client = app.state::<reqwest::Client>();
    let results: CurseForgeSearchRes = cf_get(&client, &url, Some(&params)).await?;
    let mut search_result: OtherResourceSearchRes = results.into();
    let lower_case_search_filter = handled_search_query.to_lowercase();
    let mut search_filter_words = HashMap::new();
    for token in tokenize_words(&lower_case_search_filter) {
        *search_filter_words.entry(token.to_string()).or_insert(0usize) += 1;
    }
    let mut scored_results: Vec<(OtherResourceInfo, i64)> = search_result.list.into_iter().map(|resource| {
        let title = resource.translated_name.as_deref().unwrap_or(resource.name.as_str());
        let lower_case_result = title.to_lowercase();
        let mut diff = levenshtein_distance(&lower_case_search_filter, &lower_case_result) as i64;
        for token in tokenize_words(&lower_case_result) {
            if let Some(count) = search_filter_words.get(token) { diff -= (WORD_PERFECT_MATCH_WEIGHT * *count * token.len()) as i64; }
        }
        (resource, diff)
    }).collect();
    scored_results.sort_by_key(|(_, diff)| *diff);
    search_result.list = scored_results.into_iter().map(|(resource, _)| resource).collect();
    for resource_info in &mut search_result.list { let _ = apply_other_resource_enhancements(app, resource_info).await; }
    Ok(search_result)
}

pub async fn fetch_resource_version_packs_curseforge(
    app: &AppHandle, query: &OtherResourceVersionPackQuery,
) -> Result<Vec<OtherResourceVersionPack>, ResourceError> {
    let mut aggregated_files: Vec<CurseForgeFileInfo> = Vec::new();
    let mut page_idx: u32 = 0;
    let page_size: u32 = 50;
    let OtherResourceVersionPackQuery { resource_id, mod_loader, game_versions } = query;
    loop {
        let url = get_curseforge_api(OtherResourceApiEndpoint::VersionPack, Some(resource_id))?;
        let mut params = HashMap::new();
        if mod_loader != ALL_FILTER { params.insert("modLoaderType".to_string(), cvt_mod_loader_to_id(mod_loader).to_string()); }
        if let Some(version) = game_versions.first() { if version != ALL_FILTER { params.insert("gameVersionTypeId".to_string(), cvt_version_to_type_id(version).to_string()); } }
        params.insert("index".to_string(), (page_idx * page_size).to_string());
        params.insert("pageSize".to_string(), page_size.to_string());
        let client = app.state::<reqwest::Client>();
        let results: CurseForgeVersionPackSearchRes = cf_get(&client, &url, Some(&params)).await?;
        let has_more = results.pagination.total_count > ((page_idx + 1) * page_size) as u64;
        aggregated_files.extend(results.data);
        if !has_more { break; }
        page_idx += 1;
    }
    Ok(map_curseforge_file_to_version_pack(aggregated_files))
}

pub async fn fetch_remote_resource_by_local_curseforge(
    app: &AppHandle, file_path: &str,
) -> Result<OtherResourceFileInfo, ResourceError> {
    let file_content = tokio::fs::read(file_path).await.map_err(|_| ResourceError::ParseError)?;
    let local_sha1 = hex::encode(sha1_smol::Sha1::from(&file_content).digest().bytes());
    let filtered_bytes: Vec<u8> = file_content.into_iter().filter(|&byte| !matches!(byte, 0x09 | 0x0a | 0x0d | 0x20)).collect();
    let hash = murmur2::murmur2(&filtered_bytes, 1) as u64;
    let url = get_curseforge_api(OtherResourceApiEndpoint::FromLocal, None)?;
    let payload = serde_json::json!({ "fingerprints": [hash] });
    let client = app.state::<reqwest::Client>();
    let fp_res: CurseForgeFingerprintRes = cf_post(&client, &url, &payload).await?;
    if let Some(exact_match) = fp_res.data.exact_matches.first() {
        let cf_file = &exact_match.file;
        if let Some(remote_sha1) = cf_file.hashes.iter().find(|h| h.algo == 1) {
            if remote_sha1.value.to_lowercase() == local_sha1.to_lowercase() { Ok((cf_file, None).into()) } else { Err(ResourceError::ParseError) }
        } else { Err(ResourceError::ParseError) }
    } else { Err(ResourceError::ParseError) }
}

pub async fn fetch_remote_resource_by_id_curseforge(
    app: &AppHandle, resource_id: &str,
) -> Result<OtherResourceInfo, ResourceError> {
    let url = get_curseforge_api(OtherResourceApiEndpoint::ById, Some(resource_id))?;
    let client = app.state::<reqwest::Client>();
    let result: CurseForgeGetProjectRes = cf_get(&client, &url, None).await?;
    let mut resource_info: OtherResourceInfo = result.data.into();
    let _ = apply_other_resource_enhancements(app, &mut resource_info).await;
    Ok(resource_info)
}