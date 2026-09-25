pub mod misc;

use crate::resource::helpers::http::Transport;
use minecraftd::resource::curseforge::CurseForgeClient;
use sjmcl_types::error::SJMCLResult;
use std::collections::HashMap;
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;

use crate::resource::helpers::misc::{levenshtein_distance, sort_localized_search_results};
use crate::resource::helpers::mod_db::{HandledSearchQuery, handle_localized_search_query};
use crate::resource::helpers::translation::{
  apply_other_resource_enhancements, apply_other_resource_enhancements_concurrently,
};
use crate::resource::models::{
  OtherResourceFileInfo, OtherResourceInfo, OtherResourceSearchQuery, OtherResourceSearchRes,
  OtherResourceVersionPack, OtherResourceVersionPackQuery,
};
use crate::utils::string::contains_chinese;

const WORD_PERFECT_MATCH_WEIGHT: usize = 5;

fn tokenize_words(text: &str) -> impl Iterator<Item = &str> {
  text.split_whitespace()
}

pub async fn fetch_resource_list_by_name_curseforge(
  app: &AppHandle,
  query: &OtherResourceSearchQuery,
) -> SJMCLResult<OtherResourceSearchRes> {
  let search_query = &query.search_query;
  let sort_by = &query.sort_by;
  let handled_search_query = handle_localized_search_query(app, search_query)
    .await
    .unwrap_or_else(|_| HandledSearchQuery {
      query: search_query.clone(),
      is_chinese: contains_chinese(search_query),
    });

  let is_default_sort = sort_by == "Popularity";
  let client = app.state::<reqwest::Client>();
  let backend =
    CurseForgeClient::with_client(Transport(&client), misc::CURSEFORGE_API_KEY.to_string());
  let mut remote_query = query.clone();
  remote_query.search_query = handled_search_query.query.clone();
  let mut search_result = backend.search_resources(&remote_query).await?;

  let has_search_filter = !handled_search_query.query.trim().is_empty();
  // Empty search is browsing by the selected API sort; avoid re-ranking by title length.
  // When the user selects a non-default sort, keep the API order instead of applying relevance.
  let should_rerank_by_search_filter =
    !handled_search_query.is_chinese && is_default_sort && has_search_filter;

  if should_rerank_by_search_filter {
    let lower_case_search_filter = handled_search_query.query.to_lowercase();
    let mut search_filter_words = HashMap::new();
    for token in tokenize_words(&lower_case_search_filter) {
      *search_filter_words
        .entry(token.to_string())
        .or_insert(0usize) += 1;
    }

    let mut scored_results: Vec<(OtherResourceInfo, i64)> = search_result
      .list
      .into_iter()
      .map(|resource| {
        let title = resource
          .translated_name
          .as_deref()
          .unwrap_or(resource.name.as_str());
        let lower_case_result = title.to_lowercase();

        let mut diff = levenshtein_distance(&lower_case_search_filter, &lower_case_result) as i64;

        for token in tokenize_words(&lower_case_result) {
          if let Some(count) = search_filter_words.get(token) {
            diff -= (WORD_PERFECT_MATCH_WEIGHT * *count * token.len()) as i64;
          }
        }

        (resource, diff)
      })
      .collect();

    scored_results.sort_by_key(|(_, diff)| *diff);
    search_result.list = scored_results
      .into_iter()
      .map(|(resource, _)| resource)
      .collect();
  }

  apply_other_resource_enhancements_concurrently(app, &mut search_result.list).await;

  if handled_search_query.is_chinese && is_default_sort {
    sort_localized_search_results(&mut search_result.list, search_query);
  }

  Ok(search_result)
}

pub async fn fetch_resource_version_packs_curseforge(
  app: &AppHandle,
  query: &OtherResourceVersionPackQuery,
) -> SJMCLResult<Vec<OtherResourceVersionPack>> {
  let client = app.state::<reqwest::Client>();
  Ok(
    CurseForgeClient::with_client(Transport(&client), misc::CURSEFORGE_API_KEY.to_string())
      .version_packs(query)
      .await?,
  )
}

pub async fn fetch_remote_resource_by_local_curseforge(
  app: &AppHandle,
  file_path: &str,
) -> SJMCLResult<OtherResourceFileInfo> {
  let client = app.state::<reqwest::Client>();
  Ok(
    CurseForgeClient::with_client(Transport(&client), misc::CURSEFORGE_API_KEY.to_string())
      .identify_file(std::path::Path::new(file_path))
      .await?,
  )
}

pub async fn fetch_remote_resource_by_id_curseforge(
  app: &AppHandle,
  resource_id: &str,
) -> SJMCLResult<OtherResourceInfo> {
  let client = app.state::<reqwest::Client>();
  let mut resource_info =
    CurseForgeClient::with_client(Transport(&client), misc::CURSEFORGE_API_KEY.to_string())
      .get_mod(resource_id.parse::<u64>()?)
      .await?
      .into();
  let _ = apply_other_resource_enhancements(app, &mut resource_info).await;

  Ok(resource_info)
}
