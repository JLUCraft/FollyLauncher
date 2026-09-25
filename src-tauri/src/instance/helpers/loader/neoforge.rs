use crate::instance::helpers::client_json::McClientInfo;
use crate::instance::helpers::loader::common::prepare_libraries;
use crate::instance::models::misc::{Instance, ModLoader, ModLoaderType};
use crate::resource::models::SourceType;
use crate::tasks::PTaskParam;
use sjmcl_types::error::SJMCLResult;
use std::path::PathBuf;
use tauri::AppHandle;
pub async fn install_neoforge_loader(
  priority: &[SourceType],
  loader: &ModLoader,
  lib_dir: PathBuf,
  task_params: &mut Vec<PTaskParam>,
) -> SJMCLResult<()> {
  let request = minecraftd::minecraft::loader::processors::installer_download(
    priority,
    ModLoaderType::NeoForge,
    "",
    &loader.version,
    loader.branch.as_deref(),
    &lib_dir,
  )?;
  task_params.push(PTaskParam::Download(request));
  Ok(())
}
pub async fn download_neoforge_libraries(
  app: &AppHandle,
  priority: &[SourceType],
  instance: &Instance,
  client_info: &mut McClientInfo,
) -> SJMCLResult<()> {
  prepare_libraries(
    app,
    priority,
    instance,
    client_info,
    ModLoaderType::NeoForge,
  )
  .await
}
