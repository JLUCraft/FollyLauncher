use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use crate::launcher_config::models::LauncherConfig;
use crate::resource::helpers::misc::get_source_priority_list;
use crate::resource::helpers::version_manifest::get_game_version_manifest;
use crate::utils::fs::get_app_resource_filepath;

fn load_versions(app: &AppHandle, path: &str, from_cache: bool) -> Vec<String> {
  let list_file_path: Option<PathBuf> = if from_cache {
    app.path().resolve(path, BaseDirectory::AppCache).ok()
  } else {
    get_app_resource_filepath(app, path).ok()
  };

  // Read & split lines, or return an empty vec
  list_file_path
    .and_then(|p| fs::read_to_string(p).ok())
    .map(|content| content.lines().map(|l| l.trim().to_string()).collect())
    .unwrap_or_default()
}

/// Compare two Minecraft version IDs.
/// The order is determined by `assets/game/versions.txt`,
/// or fallback to cache, and optionally refresh from remote manifest if not found.
///
/// Ordering is supplied by minecraftd's manifest-based VersionOrder.
///
/// # Examples
/// ```rust,ignore
/// if compare_game_versions(&app, version, "1.19", true).await.le(&Ordering::Equal) {
///     println!("{} <= 1.19", version);
/// }
/// ```
///
/// # Parameters
/// - `fallback_fetch_remote`: Whether to refresh the version list from remote if not found in built-in or cache.
///   ⚠️ If enabled, this may take extra time due to network request.
///
/// # Expected result
/// - Returns `Ordering::Less` if `version_a` < `version_b`
/// - Returns `Ordering::Equal` if `version_a` == `version_b`
/// - Returns `Ordering::Greater` if `version_a` > `version_b`, or if `version_a` not found
/// - If both versions are not found, returns `Ordering::Equal`
///
async fn load_order(
  app: &AppHandle,
  needed: &[&str],
  refresh: bool,
) -> minecraftd::minecraft::order::VersionOrder {
  use minecraftd::minecraft::order::VersionOrder;
  let mut order = VersionOrder::new(load_versions(app, "assets/game/versions.txt", false));
  if !needed.iter().all(|version| order.contains(version)) {
    order = VersionOrder::new(load_versions(app, "game_versions.txt", true));
  }
  if refresh
    && !needed.iter().all(|version| order.contains(version))
    && let Some(state) = app.try_state::<Mutex<LauncherConfig>>()
  {
    let priority = state
      .lock()
      .map(|config| get_source_priority_list(&config))
      .unwrap_or_default();
    let _ = get_game_version_manifest(app, &priority).await;
    order = VersionOrder::new(load_versions(app, "game_versions.txt", true));
  }
  order
}
pub async fn compare_game_versions(app: &AppHandle, a: &str, b: &str, refresh: bool) -> Ordering {
  load_order(app, &[a, b], refresh).await.compare(a, b)
}

/// Build a comparator function for Minecraft version IDs based on
/// the built-in or cached version list files.
///
/// The returned comparator can be used directly in sorting operations, e.g.:
///
/// # Examples
/// ```rust,ignore
/// let cmp_fn = build_game_version_cmp_fn(&app);
/// summary_list.sort_by(|a, b| cmp_fn(&a.version, &b.version));
/// ```
///
/// # Returns
/// A closure suitable for `.sort_by()` or `.sort_by_key()` usage.
///
pub fn build_game_version_cmp_fn(app: &AppHandle) -> impl Fn(&str, &str) -> Ordering + use<> {
  let mut versions = load_versions(app, "assets/game/versions.txt", false);

  if versions.is_empty() {
    versions = load_versions(app, "game_versions.txt", true);
  }

  let order = minecraftd::minecraft::order::VersionOrder::new(versions);
  move |a, b| order.compare(a, b)
}

/// Find the major game version ("1.x" or "YY.x") corresponding to the provided game version.
/// If the version starts with a dot-separated number (e.g., 1.7, 1.18-pre1, 26.1-snapshot),
/// it returns the parsed major version (e.g., 1.7, 1.18, 26.1).
/// Otherwise (e.g., snapshot format like "24w02a"), it will return the corresponding major version found from the list.
///
/// It is noted that the version number system of Minecraft has a breaking change starting in 2026 ("1.x" to "YY.x")
/// ref: https://www.minecraft.net/en-us/article/minecraft-new-version-numbering-system
///
/// # Examples
/// ```rust,ignore
/// let version = get_major_game_version(&app, "1.18-pre1", true).await;
/// println!("Major version of 1.18-pre1: {}", version); // => "1.18"
///
/// let version = get_major_game_version(&app, "26.1.1", false).await;
/// println!("Major version of 26.1.1: {}", version); // => "26.1"
///
/// let version = get_major_game_version(&app, "24w09a", false).await;
/// println!("Major version of 24w09a: {}", version); // => "1.20.4" => "1.20"
/// ```
///
/// # Parameters
/// - `fallback_fetch_remote`: Whether to refresh the version list from remote if not found in built-in or cache.
///   ⚠️ If enabled, this may take extra time due to network request.
///
/// # Expected result
/// - Returns the major version if the input version starts with "1.x" or "YY.x"
/// - Returns the closest major version from the list otherwise
/// - Returns empty string as fallback.
pub async fn get_major_game_version(
  app: &AppHandle,
  version: &str,
  fallback_fetch_remote: bool,
) -> String {
  if let Some(major) = minecraftd::minecraft::order::release_major(version) {
    return major;
  }
  load_order(app, &[version], fallback_fetch_remote)
    .await
    .major(version)
    .unwrap_or_default()
}
