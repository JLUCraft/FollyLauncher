use sjmcl_types::error::SJMCLResult;
use std::path::PathBuf;
use tauri::AppHandle;

use crate::instance::helpers::client_json::McClientInfo;
use crate::instance::helpers::loader::cleanroom::install_cleanroom_loader;
use crate::instance::helpers::loader::fabric::install_fabric_loader;
use crate::instance::helpers::loader::forge::{InstallProfile, install_forge_loader};
use crate::instance::helpers::loader::neoforge::install_neoforge_loader;
use crate::instance::helpers::loader::quilt::install_quilt_loader;
use crate::instance::helpers::misc::get_instance_game_config;
use crate::instance::models::misc::{Instance, InstanceError, ModLoader, ModLoaderType};
use crate::launch::helpers::jre_selector::select_java_runtime;
use crate::resource::models::SourceType;
use crate::tasks::PTaskParam;

pub async fn install_mod_loader(
  app: AppHandle,
  priority: &[SourceType],
  game_version: &str,
  loader: &ModLoader,
  lib_dir: PathBuf,
  mods_dir: PathBuf,
  client_info: &mut McClientInfo,
  task_params: &mut Vec<PTaskParam>,
  is_install_fabric_api: Option<bool>,
  is_install_qf_api: Option<bool>,
) -> SJMCLResult<()> {
  match loader.loader_type {
    ModLoaderType::Fabric => {
      install_fabric_loader(
        app,
        priority,
        game_version,
        loader,
        lib_dir,
        mods_dir,
        client_info,
        task_params,
        is_install_fabric_api,
      )
      .await
    }
    ModLoaderType::Quilt => {
      install_quilt_loader(
        app,
        priority,
        game_version,
        loader,
        lib_dir,
        mods_dir,
        client_info,
        task_params,
        is_install_qf_api,
      )
      .await
    }
    ModLoaderType::Forge => {
      install_forge_loader(priority, game_version, loader, lib_dir.clone(), task_params).await
    }
    ModLoaderType::Cleanroom => {
      install_cleanroom_loader(priority, loader, lib_dir.clone(), task_params).await
    }
    ModLoaderType::NeoForge => {
      install_neoforge_loader(priority, loader, lib_dir, task_params).await
    }
    _ => Err(InstanceError::UnsupportedModLoader.into()),
  }
}

pub async fn execute_processors(
  app: &AppHandle,
  instance: &Instance,
  client_info: &McClientInfo,
  install_profile: &InstallProfile,
) -> SJMCLResult<()> {
  let game_config = get_instance_game_config(app, instance);

  let selected_java = select_java_runtime(
    app,
    Some(&game_config.game_java),
    instance,
    client_info
      .java_version
      .as_ref()
      .map_or(0i32, |v| v.major_version),
  )
  .await?;

  Ok(
    minecraftd::minecraft::loader::processors::execute_processors(
      std::path::Path::new(&selected_java.exec_path),
      &instance.version_path,
      install_profile,
    )
    .await?,
  )
}

pub struct MetadataClient<'a>(pub &'a tauri_plugin_http::reqwest::Client);
#[async_trait::async_trait]
impl minecraftd::minecraft::loader::libraries::MetadataClient for MetadataClient<'_> {
  async fn get_json(
    &self,
    url: url::Url,
  ) -> Result<serde_json::Value, minecraftd::minecraft::loader::installer::LoaderInstallError> {
    let response = self
      .0
      .get(url)
      .send()
      .await
      .and_then(|response| response.error_for_status())
      .map_err(|error| {
        minecraftd::minecraft::loader::installer::LoaderInstallError::Network(error.to_string())
      })?;
    response.json().await.map_err(|error| {
      minecraftd::minecraft::loader::installer::LoaderInstallError::Parse(error.to_string())
    })
  }
}

pub async fn prepare_libraries(
  app: &AppHandle,
  priority: &[SourceType],
  instance: &Instance,
  client_info: &mut McClientInfo,
  loader: ModLoaderType,
) -> SJMCLResult<()> {
  use crate::instance::helpers::misc::get_instance_subdir_paths;
  use crate::instance::models::misc::InstanceSubdirType;
  let paths = get_instance_subdir_paths(
    app,
    instance,
    &[&InstanceSubdirType::Root, &InstanceSubdirType::Libraries],
  )
  .ok_or(InstanceError::InvalidSourcePath)?;
  let [root, libraries] = paths.as_slice() else {
    return Err(InstanceError::InvalidSourcePath.into());
  };
  let target = minecraftd::minecraft::loader::processors::InstallTarget {
    loader,
    loader_version: &instance.mod_loader.version,
    name: &instance.name,
    version_dir: &instance.version_path,
    root_dir: root,
    library_dir: libraries,
  };
  let downloads =
    minecraftd::minecraft::loader::processors::prepare_libraries(priority, &target, client_info)?;
  crate::tasks::commands::schedule_progressive_task_group(
    app.clone(),
    format!(
      "{}-libraries?{}",
      loader.to_string().to_lowercase(),
      instance.id
    ),
    downloads.into_iter().map(PTaskParam::Download).collect(),
    true,
  )
  .await?;
  Ok(())
}
