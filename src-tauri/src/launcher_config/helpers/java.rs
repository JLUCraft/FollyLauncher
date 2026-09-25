use sjmcl_types::error::{SJMCLError, SJMCLResult};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;

use crate::launcher_config::models::{JavaInfo, LauncherConfig};
use crate::resource::helpers::misc::get_source_priority_list;
use crate::tasks::PTaskParam;
use crate::utils::fs::{PermissionOperation, manage_permissions_unix};

#[cfg(target_os = "windows")]
use crate::utils::sys_info::get_all_drive_mount_points;

pub async fn refresh_and_update_javas(app: &AppHandle) {
  // get java paths from system PATH, etc.
  let mut java_paths = get_java_paths(app);

  // add user-added paths from config state.
  let config_binding = app.state::<Mutex<LauncherConfig>>();
  let mut config_state = config_binding.lock().unwrap();
  let extra_java_paths = config_state.extra_java_paths.clone();
  java_paths.extend(extra_java_paths.clone());

  let mut seen_paths: HashMap<String, JavaInfo> = HashMap::new();

  for java_exec_path in java_paths {
    // ensure execute permissions (for Linux and macOS)
    let _ = manage_permissions_unix(&java_exec_path, 0o111, PermissionOperation::Upgrade);

    let Some(runtime) = minecraftd::minecraft::java::discovery::inspect(
      &java_exec_path,
      extra_java_paths.contains(&java_exec_path),
    ) else {
      continue;
    };
    let java_info = JavaInfo {
      name: runtime.name,
      exec_path: runtime.exec_path.to_string_lossy().into_owned(),
      vendor: runtime.vendor,
      major_version: runtime.major_version,
      is_lts: runtime.is_lts,
      is_user_added: runtime.is_user_added,
    };

    seen_paths.entry(java_exec_path).or_insert(java_info);
  }

  let mut java_list: Vec<JavaInfo> = seen_paths.into_values().collect();
  java_list.sort_by(|a, b| {
    b.major_version
      .cmp(&a.major_version)
      .then_with(|| a.exec_path.len().cmp(&b.exec_path.len()))
  });

  // check selected java in global game config, if not exist, remove it.
  let current_selected_java = &config_state.global_game_config.game_java.exec_path;
  let is_valid_java = java_list
    .iter()
    .any(|java| &java.exec_path == current_selected_java);
  if !is_valid_java {
    config_state.global_game_config.game_java.exec_path = "".to_string();
  }

  let javas_binding = app.state::<Mutex<Vec<JavaInfo>>>();
  let mut javas_state = javas_binding.lock().unwrap();
  *javas_state = java_list;
}

pub use minecraftd::minecraft::java::discovery::{
  get_java_info_from_command, get_java_info_from_release_file,
};
pub fn get_java_paths(app: &AppHandle) -> Vec<String> {
  let game_directories = app
    .state::<Mutex<LauncherConfig>>()
    .lock()
    .map(|config| {
      config
        .local_game_directories
        .iter()
        .map(|directory| directory.dir.clone())
        .collect()
    })
    .unwrap_or_default();
  minecraftd::minecraft::java::discovery::get_java_paths(
    &minecraftd::minecraft::java::discovery::JavaLocations {
      home: app.path().home_dir().ok(),
      data: app.path().app_data_dir().ok(),
      game_directories,
      #[cfg(windows)]
      drives: get_all_drive_mount_points(),
      #[cfg(not(windows))]
      drives: Vec::new(),
    },
  )
}

pub async fn build_mojang_java_download_params(
  app: &AppHandle,
  version: &str,
) -> SJMCLResult<Vec<PTaskParam>> {
  let config = app.state::<Mutex<LauncherConfig>>().lock()?.clone();
  let client = app.state::<reqwest::Client>();

  let runtime_dir = app.path().resolve(
    format!("runtime/java-{version}"),
    tauri::path::BaseDirectory::AppData,
  )?;
  minecraftd::minecraft::java::runtime::download_plan(
    &crate::resource::helpers::http::Transport(&client),
    &get_source_priority_list(&config),
    &config.basic_info.os_type,
    &config.basic_info.arch,
    version,
    &runtime_dir,
  )
  .await
  .map(|downloads| downloads.into_iter().map(PTaskParam::Download).collect())
  .map_err(|error| SJMCLError(error.to_string()))
}
