use crate::instance::helpers::loader::common::MetadataClient;
use crate::resource::models::{OptiFineResourceInfo, SourceType};
use sjmcl_types::error::SJMCLResult;
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;
pub async fn get_optifine_meta_by_game_version(
  app: &AppHandle,
  priority: &[SourceType],
  game_version: &str,
) -> SJMCLResult<Vec<OptiFineResourceInfo>> {
  let client = app.state::<reqwest::Client>();
  Ok(
    minecraftd::minecraft::loader::catalog::optifine_versions(
      &MetadataClient(&client),
      priority,
      game_version,
    )
    .await?,
  )
}
