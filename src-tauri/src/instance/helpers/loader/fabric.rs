use crate::instance::helpers::client_json::McClientInfo;
use crate::instance::models::misc::{ModLoader, ModLoaderType};
use crate::resource::models::SourceType;
use crate::tasks::PTaskParam;
use sjmcl_types::error::SJMCLResult;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;
pub async fn install_fabric_loader(
  app: AppHandle,
  priority: &[SourceType],
  game_version: &str,
  loader: &ModLoader,
  lib_dir: PathBuf,
  mods_dir: PathBuf,
  client_info: &mut McClientInfo,
  task_params: &mut Vec<PTaskParam>,
  is_install_fabric_api: Option<bool>,
) -> SJMCLResult<()> {
  let client = app.state::<reqwest::Client>();
  let plan = minecraftd::minecraft::loader::libraries::plan(
    &crate::instance::helpers::loader::common::MetadataClient(&client),
    priority,
    ModLoaderType::Fabric,
    game_version,
    &loader.version,
    &lib_dir,
  )
  .await?;
  plan.apply(client_info);
  task_params.extend(plan.downloads.into_iter().map(PTaskParam::Download));
  if is_install_fabric_api.unwrap_or(true)
    && let Ok(Some(download)) = minecraftd::minecraft::loader::libraries::companion(
      &crate::resource::helpers::http::Transport(&client),
      ModLoaderType::Fabric,
      game_version,
      &mods_dir,
    )
    .await
  {
    task_params.push(PTaskParam::Download(download));
  }
  Ok(())
}
pub async fn remove_fabric_api_mods<P: AsRef<std::path::Path>>(mods_dir: P) -> SJMCLResult<()> {
  Ok(minecraftd::minecraft::metadata::remove_fabric_api_mods(mods_dir.as_ref()).await?)
}
