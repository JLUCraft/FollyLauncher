use crate::instance::models::misc::Instance;
pub use minecraftd::minecraft::profile::*;
use sjmcl_types::error::SJMCLResult;
use tauri::AppHandle;
pub fn rule_platform() -> RulePlatform {
  RulePlatform {
    os: tauri_plugin_os::type_().to_string(),
    arch: tauri_plugin_os::arch().into(),
    version: tauri_plugin_os::version().to_string(),
  }
}
#[cfg(target_os = "windows")]
pub fn load_native_libraries_replace_map(
  _app: &AppHandle,
) -> SJMCLResult<minecraftd::minecraft::natives::NativeReplacements> {
  Ok(minecraftd::minecraft::natives::native_replacements()?)
}
pub async fn replace_native_libraries(
  app: &AppHandle,
  client_info: &mut McClientInfo,
  instance: &Instance,
) -> SJMCLResult<()> {
  let platform = rule_platform();
  let newer = if platform.os == "macos" && platform.arch == "aarch64" {
    crate::instance::helpers::game_version::compare_game_versions(
      app,
      &instance.version,
      "1.20.1",
      true,
    )
    .await
      == std::cmp::Ordering::Greater
  } else {
    false
  };
  if minecraftd::minecraft::natives::needs_native_replacements(&platform, newer) {
    minecraftd::minecraft::natives::replace_native_libraries(
      client_info,
      &minecraftd::minecraft::natives::native_replacements()?,
      &platform,
    );
  }
  Ok(())
}
