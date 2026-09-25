use crate::instance::models::world::{base::WorldInfo, level::LevelData};
use sjmcl_types::error::SJMCLResult;
use std::path::{Path, PathBuf};
pub async fn load_world_info_from_dir(
  path: &Path,
  has_difficulty_support: bool,
) -> SJMCLResult<WorldInfo> {
  Ok(minecraftd::minecraft::world::load_world_info_from_dir(path, has_difficulty_support).await?)
}
pub async fn load_level_data_from_nbt(path: &PathBuf) -> SJMCLResult<LevelData> {
  Ok(minecraftd::minecraft::world::load_level_data_from_nbt(path).await?)
}
