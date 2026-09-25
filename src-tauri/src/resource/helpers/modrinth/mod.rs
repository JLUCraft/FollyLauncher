pub mod misc;

use crate::resource::helpers::http::Transport;
use minecraftd::resource::modrinth::ModrinthClient;
use sjmcl_types::error::SJMCLResult;
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;

use crate::resource::helpers::misc::sort_localized_search_results;
use crate::resource::helpers::mod_db::{HandledSearchQuery, handle_localized_search_query};
use crate::resource::helpers::translation::{
  apply_other_resource_enhancements, apply_other_resource_enhancements_concurrently,
};
use crate::resource::models::{
  OtherResourceFileInfo, OtherResourceInfo, OtherResourceSearchQuery, OtherResourceSearchRes,
  OtherResourceVersionPack, OtherResourceVersionPackQuery,
};
use crate::utils::string::contains_chinese;

pub async fn fetch_resource_list_by_name_modrinth(
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
  let is_default_sort = sort_by == "relevance";
  let client = app.state::<reqwest::Client>();
  let backend = ModrinthClient::with_client(Transport(&client));
  let mut remote_query = query.clone();
  remote_query.search_query = handled_search_query.query.clone();
  let mut search_result = backend.search_resources(&remote_query).await?;

  apply_other_resource_enhancements_concurrently(app, &mut search_result.list).await;

  if handled_search_query.is_chinese && is_default_sort {
    sort_localized_search_results(&mut search_result.list, search_query);
  }

  Ok(search_result)
}

pub async fn fetch_resource_version_packs_modrinth(
  app: &AppHandle,
  query: &OtherResourceVersionPackQuery,
) -> SJMCLResult<Vec<OtherResourceVersionPack>> {
  let client = app.state::<reqwest::Client>();
  Ok(
    ModrinthClient::with_client(Transport(&client))
      .version_packs(query)
      .await?,
  )
}

pub async fn fetch_remote_resource_by_local_modrinth(
  app: &AppHandle,
  file_path: &str,
) -> SJMCLResult<OtherResourceFileInfo> {
  let client = app.state::<reqwest::Client>();
  Ok(
    ModrinthClient::with_client(Transport(&client))
      .identify_file(std::path::Path::new(file_path))
      .await?,
  )
}

pub async fn fetch_remote_resource_by_id_modrinth(
  app: &AppHandle,
  resource_id: &str,
) -> SJMCLResult<OtherResourceInfo> {
  let client = app.state::<reqwest::Client>();
  let mut resource_info = ModrinthClient::with_client(Transport(&client))
    .get_project(resource_id)
    .await?
    .into();
  let _ = apply_other_resource_enhancements(app, &mut resource_info).await;

  Ok(resource_info)
}
