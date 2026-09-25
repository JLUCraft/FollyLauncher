use crate::instance::helpers::loader::common::MetadataClient;
use crate::resource::models::{ModLoaderResourceInfo, SourceType};
use sjmcl_types::error::SJMCLResult;
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;
pub async fn get_neoforge_meta_by_game_version(
  app: &AppHandle,
  priority: &[SourceType],
  game_version: &str,
) -> SJMCLResult<Vec<ModLoaderResourceInfo>> {
  let client = app.state::<reqwest::Client>();
  Ok(
    minecraftd::minecraft::loader::catalog::versions(
      &MetadataClient(&client),
      priority,
      minecraftd::minecraft::loaders::ModLoaderType::NeoForge,
      game_version,
    )
    .await?,
  )
}
