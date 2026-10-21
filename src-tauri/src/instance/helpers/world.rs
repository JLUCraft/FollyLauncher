use crate::instance::models::misc::InstanceError;
use crate::instance::models::world::{WorldDetails, WorldInfo};
use sjmcl_types::error::SJMCLResult;
use std::path::Path;

pub async fn load_world_info_from_dir(
  path: &Path,
  has_difficulty_support: bool,
) -> SJMCLResult<WorldInfo> {
  Ok(minecraftd::minecraft::world::load_world_info_from_dir(path, has_difficulty_support).await?)
}

pub async fn load_world_data_from_dir(path: &Path) -> SJMCLResult<WorldDetails> {
  minecraftd::minecraft::world::load_world_data_from_dir(path)
    .await
    .map_err(|error| {
      log::warn!(
        "Failed to read world metadata at {}: {}",
        path.display(),
        error
      );
      match error {
        minecraftd::minecraft::world::WorldError::Io(error)
          if error.kind() == std::io::ErrorKind::NotFound =>
        {
          InstanceError::LevelNotExistError.into()
        }
        _ => InstanceError::LevelParseError.into(),
      }
    })
}
