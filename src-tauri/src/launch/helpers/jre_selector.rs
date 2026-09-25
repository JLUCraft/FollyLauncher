use sjmcl_types::error::SJMCLResult;
use std::cmp::Ordering;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

use crate::instance::helpers::game_version::compare_game_versions;
use crate::instance::models::misc::Instance;
use crate::launch::models::LaunchError;
use crate::launcher_config::models::{GameJava, JavaInfo};

pub async fn select_java_runtime(
  app: &AppHandle,
  game_java: Option<&GameJava>, // None means force auto selection for some installation flows.
  instance: &Instance,
  client_json_req: i32,
  // TODO: pass client and mod loader info to calculate version with more rules, instead of passing require version
  // ref: https://github.com/Hex-Dragon/PCL2/blob/16e09c792ce8c13435fc6827e6da54170aaa3bc0/Plain%20Craft%20Launcher%202/Modules/Minecraft/ModLaunch.vb#L1130
) -> SJMCLResult<JavaInfo> {
  let java_list = app.state::<Mutex<Vec<JavaInfo>>>().lock()?.clone();

  let preferred = game_java
    .filter(|java| !java.auto)
    .map(|java| java.exec_path.as_str());
  let required = if preferred.is_some() {
    0
  } else {
    get_minimum_java_version_by_game(app, instance, client_json_req, true).await
  };
  let entries = java_list
    .iter()
    .map(|java| (java.exec_path.as_str(), java.major_version))
    .collect::<Vec<_>>();
  minecraftd::minecraft::java::select_runtime_index(&entries, preferred, required)
    .and_then(|index| java_list.get(index).cloned())
    .ok_or_else(|| {
      if preferred.is_some() {
        LaunchError::SelectedJavaUnavailable.into()
      } else {
        LaunchError::NoSuitableJava.into()
      }
    })
}

/// Get minimum java version requirement by game client version.
/// ref: https://zh.minecraft.wiki/w/Java%E7%89%88?variant=zh-cn#%E8%BD%AF%E4%BB%B6%E9%9C%80%E6%B1%82
async fn get_minimum_java_version_by_client(
  app: &AppHandle,
  instance: &Instance,
  fallback_fetch_remote: bool,
) -> i32 {
  for (index, &(threshold, major)) in minecraftd::minecraft::java::JAVA_THRESHOLDS
    .iter()
    .enumerate()
  {
    if compare_game_versions(
      app,
      &instance.version,
      threshold,
      fallback_fetch_remote && index == 0,
    )
    .await
      >= Ordering::Equal
    {
      return major;
    }
  }
  0
}

pub async fn get_minimum_java_version_by_game(
  app: &AppHandle,
  instance: &Instance,
  client_json_req: i32,
  fallback_fetch_remote: bool,
) -> i32 {
  // by default, the minimum java version requirement is determined by the game client version.
  let client_version_req =
    get_minimum_java_version_by_client(app, instance, fallback_fetch_remote).await;

  // override by the mod loader
  let mod_loader_version_req = minecraftd::minecraft::java::loader_java_requirement(
    instance.mod_loader.loader_type,
    &instance.mod_loader.version,
  );

  // override by the client.json requirement
  client_version_req
    .max(mod_loader_version_req)
    .max(client_json_req)
}
