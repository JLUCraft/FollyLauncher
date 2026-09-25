#[cfg(target_os = "windows")]
use crate::resource::helpers::misc::convert_url_to_target_source;
#[cfg(target_os = "windows")]
use crate::resource::models::ResourceType;
#[cfg(target_os = "windows")]
use crate::tasks::download::DownloadParam;
#[cfg(target_os = "windows")]
use crate::utils::fs::validate_sha1;
use sjmcl_types::error::SJMCLResult;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
#[cfg(target_os = "windows")]
use tokio::fs;

#[cfg(target_os = "windows")]
use std::sync::Mutex;
#[cfg(target_os = "windows")]
use tauri::Manager;

use crate::instance::helpers::asset_index::load_asset_index;
use crate::instance::helpers::client_json::McClientInfo;
use crate::instance::models::misc::ModLoaderType;
use crate::resource::models::SourceType;
use crate::tasks::PTaskParam;

#[cfg(target_os = "windows")]
use crate::instance::helpers::client_json::{LibrariesValue, load_native_libraries_replace_map};
#[cfg(target_os = "windows")]
use crate::launcher_config::helpers::graphics::mesa_driver_name;
#[cfg(target_os = "windows")]
use crate::launcher_config::models::{GameConfig, LauncherConfig};

pub async fn get_invalid_library_files(
  source: SourceType,
  library_path: &Path,
  client_info: &McClientInfo,
  check_hash: bool,
) -> SJMCLResult<Vec<PTaskParam>> {
  Ok(
    minecraftd::minecraft::dependencies::download_plan(
      source,
      library_path,
      client_info,
      &crate::instance::helpers::client_json::rule_platform(),
      check_hash,
    )
    .await?
    .into_iter()
    .map(PTaskParam::Download)
    .collect(),
  )
}

#[cfg(target_os = "windows")]
pub fn get_windows_mesa_loader_library(app: &AppHandle) -> SJMCLResult<Option<LibrariesValue>> {
  let all_replace_map = load_native_libraries_replace_map(app)?;
  let (os, arch) = {
    let launcher_config_state = app.state::<Mutex<LauncherConfig>>();
    let cfg = launcher_config_state.lock().unwrap();
    (cfg.basic_info.os_type.clone(), cfg.basic_info.arch.clone())
  };
  let platform_key = format!("{}-{}", os.to_lowercase(), arch.to_lowercase());
  Ok(
    all_replace_map
      .get(platform_key.as_str())
      .and_then(|platform_map| platform_map.get("mesa-loader"))
      .cloned()
      .flatten(),
  )
}

#[cfg(target_os = "windows")]
pub fn get_windows_mesa_loader_path(
  app: &AppHandle,
  library_path: &Path,
) -> SJMCLResult<Option<PathBuf>> {
  Ok(get_windows_mesa_loader_library(app)?.and_then(|lib| {
    lib
      .downloads
      .and_then(|downloads| downloads.artifact)
      .map(|artifact| library_path.join(artifact.path))
  }))
}

#[cfg(target_os = "windows")]
pub async fn get_invalid_windows_mesa_loader_file(
  app: &AppHandle,
  source: SourceType,
  library_path: &Path,
  game_config: &GameConfig,
  check_hash: bool,
) -> SJMCLResult<Vec<PTaskParam>> {
  if mesa_driver_name(
    &game_config.advanced.graphics.api,
    &game_config.advanced.graphics.renderer,
  )
  .is_none()
  {
    return Ok(Vec::new());
  }

  let Some(lib) = get_windows_mesa_loader_library(app)? else {
    return Ok(Vec::new());
  };
  let Some(artifact) = lib.downloads.and_then(|downloads| downloads.artifact) else {
    return Ok(Vec::new());
  };

  let file_path = library_path.join(&artifact.path);
  let exists = fs::try_exists(&file_path).await?;
  if exists && (!check_hash || validate_sha1(file_path.clone(), artifact.sha1.clone()).is_ok()) {
    return Ok(Vec::new());
  }

  let src = convert_url_to_target_source(
    &url::Url::parse(&artifact.url)?,
    &[ResourceType::Libraries],
    &source,
  )?;
  Ok(vec![PTaskParam::Download(DownloadParam {
    src,
    dest: file_path,
    filename: None,
    sha1: Some(artifact.sha1),
  })])
}

pub fn get_nonnative_library_paths(
  client_info: &McClientInfo,
  library_path: &Path,
  mod_loader_type: ModLoaderType,
) -> SJMCLResult<Vec<PathBuf>> {
  Ok(
    minecraftd::minecraft::dependencies::get_nonnative_library_paths(
      client_info,
      library_path,
      mod_loader_type,
      &crate::instance::helpers::client_json::rule_platform(),
    )?,
  )
}
pub async fn extract_native_libraries(
  client_info: &McClientInfo,
  library_path: &Path,
  natives_dir: &Path,
  use_native_glfw: bool,
  use_native_openal: bool,
) -> SJMCLResult<()> {
  Ok(
    minecraftd::minecraft::dependencies::extract_native_libraries(
      client_info,
      library_path,
      natives_dir,
      use_native_glfw,
      use_native_openal,
      &crate::instance::helpers::client_json::rule_platform(),
    )
    .await?,
  )
}
pub async fn get_invalid_assets(
  app: &AppHandle,
  client_info: &McClientInfo,
  source: SourceType,
  asset_path: &Path,
  check_hash: bool,
) -> SJMCLResult<Vec<PTaskParam>> {
  let asset_index_path = asset_path.join(format!("indexes/{}.json", client_info.asset_index.id));
  let index = load_asset_index(app, &asset_index_path, &client_info.asset_index.url).await?;
  Ok(
    minecraftd::minecraft::assets::plan_assets(&index, asset_path, source, check_hash)
      .await?
      .into_iter()
      .map(PTaskParam::Download)
      .collect(),
  )
}

pub async fn prepare_legacy_assets(
  root_dir: &Path,
  assets_dir: &Path,
  assets_index_name: &str,
) -> SJMCLResult<()> {
  Ok(
    minecraftd::minecraft::assets::prepare_legacy_assets(root_dir, assets_dir, assets_index_name)
      .await?,
  )
}
