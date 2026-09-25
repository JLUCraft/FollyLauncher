use sjmcl_types::error::SJMCLResult;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

use crate::instance::helpers::misc::get_instance_subdir_path_by_id;
use crate::instance::models::misc::InstanceSubdirType;

pub use minecraftd::minecraft::servers::{GameServerInfo, SERVERS_DAT_FILENAME};

pub fn get_servers_nbt_path_by_instance_id(
  app: &AppHandle,
  instance_id: &String,
) -> Option<PathBuf> {
  let game_root_dir = get_instance_subdir_path_by_id(app, instance_id, &InstanceSubdirType::Root)?;
  Some(game_root_dir.join(SERVERS_DAT_FILENAME))
}

pub async fn load_servers_info_from_nbt(path: &Path) -> SJMCLResult<Vec<GameServerInfo>> {
  Ok(minecraftd::minecraft::servers::load_servers_info_from_nbt(path).await?)
}
pub async fn save_servers_to_nbt(path: &Path, servers: &[GameServerInfo]) -> SJMCLResult<()> {
  Ok(minecraftd::minecraft::servers::save_servers_to_nbt(path, servers).await?)
}

pub async fn query_servers_online(
  servers: Vec<GameServerInfo>,
) -> SJMCLResult<Vec<GameServerInfo>> {
  Ok(minecraftd::minecraft::servers::query_servers_online(servers).await)
}
