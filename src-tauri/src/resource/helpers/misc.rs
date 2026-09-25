use sjmcl_types::error::{SJMCLError, SJMCLResult};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::launcher_config::models::LauncherConfig;
use crate::resource::models::{ModUpdateQuery, OtherResourceInfo, SourceType};
use crate::utils::string::contains_chinese;

pub fn get_source_priority_list(launcher_config: &LauncherConfig) -> Vec<SourceType> {
  match launcher_config.download.source.strategy.as_str() {
    "official" => vec![SourceType::Official, SourceType::Bmclapi],
    "mirror" => vec![SourceType::Bmclapi, SourceType::Official],
    "auto" => match launcher_config.basic_info.is_china_mainland_ip {
      true => vec![SourceType::Bmclapi, SourceType::Official],
      false => vec![SourceType::Official, SourceType::Bmclapi],
    },
    _ => vec![SourceType::Bmclapi, SourceType::Official],
  }
}

#[cfg(windows)]
pub use minecraftd::resource::mirrors::convert_url_to_target_source;
pub use minecraftd::resource::mirrors::get_download_api;

pub(crate) fn levenshtein_distance(a: &str, b: &str) -> usize {
  let b_chars: Vec<char> = b.chars().collect();
  let mut prev: Vec<usize> = (0..=b_chars.len()).collect();

  for (i, a_ch) in a.chars().enumerate() {
    let mut current = Vec::with_capacity(b_chars.len() + 1);
    current.push(i + 1);

    for (j, b_ch) in b_chars.iter().enumerate() {
      let cost = if a_ch == *b_ch { 0 } else { 1 };
      let insertion = current[j] + 1;
      let deletion = prev[j + 1] + 1;
      let substitution = prev[j] + cost;
      current.push(insertion.min(deletion).min(substitution));
    }

    prev = current;
  }

  *prev.last().unwrap_or(&0)
}

pub fn sort_localized_search_results(list: &mut Vec<OtherResourceInfo>, search_query: &str) {
  const CONTAIN_CHINESE_WEIGHT: i64 = 10;

  let search_query = search_query
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ");

  if search_query.is_empty() {
    return;
  }

  let mut translated_results = Vec::new();
  let mut untranslated_results = Vec::new();

  for resource in list.drain(..) {
    let contains_chinese_title = resource
      .translated_name
      .as_deref()
      .map_or_else(|| contains_chinese(&resource.name), contains_chinese);

    if contains_chinese_title {
      translated_results.push(resource);
    } else {
      untranslated_results.push(resource);
    }
  }

  translated_results.sort_by_key(|resource| {
    let display_name = resource
      .translated_name
      .as_deref()
      .unwrap_or(resource.name.as_str());

    let mut diff = levenshtein_distance(&search_query, display_name) as i64;
    for ch in search_query.chars() {
      if display_name.contains(ch) {
        diff -= CONTAIN_CHINESE_WEIGHT;
      }
    }

    diff
  });

  list.extend(translated_results);
  list.extend(untranslated_results);
}

pub fn resolve_mod_update_paths(
  mods_dir: &Path,
  queries: &[ModUpdateQuery],
) -> SJMCLResult<Vec<(PathBuf, PathBuf)>> {
  let canonical_mods_dir = mods_dir.canonicalize().map_err(|error| {
    SJMCLError(format!(
      "Failed to resolve mods directory {}: {}",
      mods_dir.display(),
      error
    ))
  })?;
  let mut targets = HashSet::new();
  let mut paths = Vec::with_capacity(queries.len());

  for query in queries {
    let old_file_path = PathBuf::from(&query.old_file_path);
    if !old_file_path.is_file() {
      return Err(SJMCLError(format!(
        "Old mod file does not exist: {}",
        old_file_path.display()
      )));
    }

    let old_parent = old_file_path.parent().ok_or_else(|| {
      SJMCLError(format!(
        "Old mod file has no parent directory: {}",
        old_file_path.display()
      ))
    })?;
    let canonical_old_parent = old_parent.canonicalize().map_err(|error| {
      SJMCLError(format!(
        "Failed to resolve old mod directory {}: {}",
        old_parent.display(),
        error
      ))
    })?;
    if !canonical_old_parent.starts_with(&canonical_mods_dir) {
      return Err(SJMCLError(format!(
        "Old mod file is outside the instance mods directory: {}",
        old_file_path.display()
      )));
    }

    let new_file_name = Path::new(&query.file_name);
    if new_file_name.file_name() != Some(new_file_name.as_os_str()) {
      return Err(SJMCLError(format!(
        "Invalid mod update file name: {}",
        query.file_name
      )));
    }

    let new_file_path = old_parent.join(new_file_name);
    let canonical_target = canonical_old_parent.join(new_file_name);
    if !targets.insert(canonical_target) {
      return Err(SJMCLError(format!(
        "Duplicate mod update target: {}",
        new_file_path.display()
      )));
    }
    if new_file_path != old_file_path && new_file_path.exists() {
      return Err(SJMCLError(format!(
        "Mod update target already exists: {}",
        new_file_path.display()
      )));
    }

    paths.push((old_file_path, new_file_path));
  }

  Ok(paths)
}
