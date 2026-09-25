use crate::instance::helpers::client_json::McClientInfo;
use crate::instance::helpers::misc::{get_instance_game_config, get_instance_subdir_paths};
use crate::instance::models::misc::{Instance, InstanceError, InstanceSubdirType};
use crate::launch::helpers::jre_selector::select_java_runtime;
use crate::resource::models::{OptiFineResourceInfo, SourceType};
use crate::tasks::{PTaskParam, commands::schedule_progressive_task_group};
use minecraftd::minecraft::loader::optifine;
use sjmcl_types::error::SJMCLResult;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
pub async fn download_optifine_installer(
  game_version: &str,
  info: &OptiFineResourceInfo,
  lib_dir: PathBuf,
  tasks: &mut Vec<PTaskParam>,
) -> SJMCLResult<()> {
  tasks.push(PTaskParam::Download(optifine::installer_download(
    game_version,
    info,
    &lib_dir,
  )?));
  Ok(())
}
fn library_dir(app: &AppHandle, instance: &Instance) -> SJMCLResult<PathBuf> {
  get_instance_subdir_paths(app, instance, &[&InstanceSubdirType::Libraries])
    .and_then(|paths| paths.into_iter().next())
    .ok_or_else(|| InstanceError::InvalidSourcePath.into())
}
pub async fn download_optifine_libraries(
  app: &AppHandle,
  priority: &[SourceType],
  instance: &Instance,
  client_info: &mut McClientInfo,
) -> SJMCLResult<()> {
  let info = instance
    .optifine
    .as_ref()
    .ok_or(InstanceError::ClientJsonParseError)?;
  let downloads = optifine::prepare_libraries(
    priority,
    &info.filename,
    &info.version,
    instance.mod_loader.loader_type,
    &library_dir(app, instance)?,
    client_info,
  )?;
  schedule_progressive_task_group(
    app.clone(),
    format!("optifine-libraries?{}", instance.id),
    downloads.into_iter().map(PTaskParam::Download).collect(),
    true,
  )
  .await?;
  Ok(())
}
pub async fn finish_optifine_install(
  app: &AppHandle,
  instance: &Instance,
  client_info: &McClientInfo,
) -> SJMCLResult<()> {
  let libraries = library_dir(app, instance)?;
  let info = instance
    .optifine
    .as_ref()
    .ok_or(InstanceError::ClientJsonParseError)?;
  let java = if optifine::requires_patcher(&libraries, &info.filename)? {
    let config = get_instance_game_config(app, instance);
    Some(
      select_java_runtime(
        app,
        Some(&config.game_java),
        instance,
        client_info
          .java_version
          .as_ref()
          .map_or(0, |java| java.major_version),
      )
      .await?,
    )
  } else {
    None
  };
  Ok(
    optifine::finish_install(
      java.as_ref().map(|java| Path::new(&java.exec_path)),
      &libraries,
      &info.filename,
      &instance.version_path.join(format!("{}.jar", instance.name)),
    )
    .await?,
  )
}
