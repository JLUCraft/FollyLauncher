use crate::resource::models::{GameClientResourceInfo, SourceType};
use minecraftd::minecraft::version::VersionMeta;
use sjmcl_types::error::SJMCLResult;
use std::fs;
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;
pub async fn get_game_version_manifest(
  app: &AppHandle,
  priority_list: &[SourceType],
) -> SJMCLResult<Vec<GameClientResourceInfo>> {
  let client = app.state::<reqwest::Client>();
  let manifest = minecraftd::minecraft::manifest::fetch_with(
    &crate::resource::helpers::http::Transport(&client),
    priority_list,
  )
  .await?;
  save_version_list_to_cache(app, &manifest.versions);
  Ok(
    manifest
      .versions
      .into_iter()
      .map(GameClientResourceInfo::from)
      .collect(),
  )
}
fn save_version_list_to_cache(app: &AppHandle, versions: &[VersionMeta]) {
  let cache_dir = match app.path().app_cache_dir().ok() {
    Some(dir) => dir,
    None => return,
  };

  if !cache_dir.exists() && fs::create_dir_all(&cache_dir).is_err() {
    return;
  }

  let file_path = cache_dir.join("game_versions.txt");
  let mut ids: Vec<String> = versions.iter().map(|v| v.id.clone()).collect();
  ids.reverse(); // reverse order

  let content = ids.join("\n");
  let _ = fs::write(file_path, content);
}
