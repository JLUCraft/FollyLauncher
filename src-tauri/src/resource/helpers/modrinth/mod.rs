pub mod misc;

use crate::resource::helpers::misc::apply_other_resource_enhancements;
use crate::resource::helpers::mod_db::handle_search_query;
use crate::resource::helpers::modrinth::misc::{
    get_modrinth_api, map_modrinth_file_to_version_pack, ModrinthProject, ModrinthSearchRes,
    ModrinthVersionPack,
};
use crate::resource::models::{
    OtherResourceApiEndpoint, OtherResourceFileInfo, OtherResourceInfo,
    OtherResourceSearchQuery, OtherResourceSearchRes, OtherResourceVersionPack,
    OtherResourceVersionPackQuery, ResourceError,
};
use std::collections::HashMap;
use tauri::{AppHandle, Manager};

const ALL_FILTER: &str = "All";

fn append_query_params(base_url: &str, params: &HashMap<String, String>) -> Result<String, ResourceError> {
    let mut url = url::Url::parse(base_url).map_err(|_| ResourceError::ParseError)?;
    {
        let mut pairs = url.query_pairs_mut();
        for (k, v) in params {
            pairs.append_pair(k, v);
        }
    }
    Ok(url.to_string())
}

pub async fn fetch_resource_list_by_name_modrinth(
    app: &AppHandle,
    query: &OtherResourceSearchQuery,
) -> Result<OtherResourceSearchRes, ResourceError> {
    let url = get_modrinth_api(OtherResourceApiEndpoint::Search, None)?;
    let OtherResourceSearchQuery { resource_type, search_query, game_version, selected_tag, sort_by, page, page_size } = query;
    let handled_search_query = handle_search_query(app, search_query).await.unwrap_or(search_query.clone());
    let mut facets = vec![vec![format!("project_type:{}", resource_type)]];
    if !game_version.is_empty() && game_version != ALL_FILTER { facets.push(vec![format!("versions:{}", game_version)]); }
    if !selected_tag.is_empty() && selected_tag != ALL_FILTER { facets.push(vec![format!("categories:{}", selected_tag)]); }
    let mut params = HashMap::new();
    params.insert("query".to_string(), handled_search_query);
    params.insert("facets".to_string(), serde_json::to_string(&facets).unwrap_or_default());
    params.insert("offset".to_string(), (page * page_size).to_string());
    params.insert("limit".to_string(), page_size.to_string());
    params.insert("index".to_string(), sort_by.to_string());
    let final_url = append_query_params(&url, &params)?;
    let client = app.state::<reqwest::Client>();
    let response = client.get(&final_url).send().await.map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() { return Err(ResourceError::NetworkError); }
    let results: ModrinthSearchRes = response.json().await.map_err(|_| ResourceError::ParseError)?;
    let mut search_result: OtherResourceSearchRes = results.into();
    for resource_info in &mut search_result.list { let _ = apply_other_resource_enhancements(app, resource_info).await; }
    Ok(search_result)
}

pub async fn fetch_resource_version_packs_modrinth(
    app: &AppHandle, query: &OtherResourceVersionPackQuery,
) -> Result<Vec<OtherResourceVersionPack>, ResourceError> {
    let OtherResourceVersionPackQuery { resource_id, mod_loader, game_versions } = query;
    let url = get_modrinth_api(OtherResourceApiEndpoint::VersionPack, Some(resource_id))?;
    let mut params = HashMap::new();
    if mod_loader != ALL_FILTER { params.insert("loaders".to_string(), format!("[\"{}\"]", mod_loader.to_lowercase())); }
    if let Some(first_version) = game_versions.first() {
        if first_version != ALL_FILTER {
            let versions_json = format!("[{}]", game_versions.iter().map(|v| format!("\"{}\"", v)).collect::<Vec<_>>().join(","));
            params.insert("game_versions".to_string(), versions_json);
        }
    }
    let final_url = append_query_params(&url, &params)?;
    let client = app.state::<reqwest::Client>();
    let response = client.get(&final_url).send().await.map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() { return Err(ResourceError::NetworkError); }
    let results: Vec<ModrinthVersionPack> = response.json().await.map_err(|_| ResourceError::ParseError)?;
    Ok(map_modrinth_file_to_version_pack(results))
}

pub async fn fetch_remote_resource_by_local_modrinth(
    app: &AppHandle, file_path: &str,
) -> Result<OtherResourceFileInfo, ResourceError> {
    let file_content = tokio::fs::read(file_path).await.map_err(|_| ResourceError::ParseError)?;
    let hash_string = hex::encode(sha1_smol::Sha1::from(&file_content).digest().bytes());
    let url = get_modrinth_api(OtherResourceApiEndpoint::FromLocal, Some(&hash_string))?;
    let mut params = HashMap::new();
    params.insert("algorithm".to_string(), "sha1".to_string());
    let final_url = append_query_params(&url, &params)?;
    let client = app.state::<reqwest::Client>();
    let response = client.get(&final_url).send().await.map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() { return Err(ResourceError::NetworkError); }
    let version_pack: ModrinthVersionPack = response.json().await.map_err(|_| ResourceError::ParseError)?;
    let file_info = version_pack.files.iter().find(|file| file.hashes.sha1 == hash_string).ok_or(ResourceError::ParseError)?;
    Ok((&version_pack, file_info, if version_pack.loaders.is_empty() { None } else { Some(version_pack.loaders[0].clone()) }).into())
}

pub async fn fetch_remote_resource_by_id_modrinth(
    app: &AppHandle, resource_id: &str,
) -> Result<OtherResourceInfo, ResourceError> {
    let url = get_modrinth_api(OtherResourceApiEndpoint::ById, Some(resource_id))?;
    let client = app.state::<reqwest::Client>();
    let response = client.get(&url).send().await.map_err(|_| ResourceError::NetworkError)?;
    if !response.status().is_success() { return Err(ResourceError::NetworkError); }
    let results: ModrinthProject = response.json().await.map_err(|_| ResourceError::ParseError)?;
    let mut resource_info: OtherResourceInfo = results.into();
    let _ = apply_other_resource_enhancements(app, &mut resource_info).await;
    Ok(resource_info)
}