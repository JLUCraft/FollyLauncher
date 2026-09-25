//! Desktop assembly of the shared member room runtime.
const MAX_ROOM_CONFIG_BYTES: u64 = 64 * 1024;
use crate::union::identity;
use federated_server::rooms::HostStatus;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

#[derive(Default)]
pub struct RoomState(
  pub Mutex<Option<RoomHandle>>,
  pub std::sync::atomic::AtomicBool,
);

pub struct RoomHandle(federated_server::runtime::Host);
impl RoomHandle {
  pub fn status(&self) -> Option<HostStatus> {
    self.0.room_status()
  }
  pub async fn stop(self) -> node::Result<()> {
    self.0.close().await
  }
}

fn addresses(values: Vec<String>) -> Result<Vec<node::Multiaddr>, String> {
  if values.len() > node::protocol::MAX_DIRECTORY_ADDRESSES
    || values
      .iter()
      .any(|v| v.len() > node::protocol::MAX_DIRECTORY_ADDRESS_BYTES)
  {
    return Err("Invalid room routes".into());
  }
  values
    .into_iter()
    .map(|v| {
      let address: node::Multiaddr = v.parse().map_err(|_| "Invalid room route")?;
      if !matches!(address.iter().last(), Some(node::AddressProtocol::P2p(_))) {
        return Err("Room route must include peer identity".into());
      }
      Ok(address)
    })
    .collect()
}

pub(crate) fn host_identity(app: &AppHandle) -> Result<identity::Identity, String> {
  let directory = app
    .path()
    .app_data_dir()
    .map_err(|e| e.to_string())?
    .join("room-host");
  crate::union::vault::identity_for(&directory, "room-host")
}
#[tauri::command]
pub fn union_room_identity(app: AppHandle) -> Result<String, String> {
  Ok(host_identity(&app)?.peer_id().to_string())
}

#[tauri::command]
pub async fn union_rooms(
  app: AppHandle,
  seeds: Vec<String>,
) -> Result<client::rooms::Discovery, String> {
  let seeds = addresses(seeds)?;
  let node = node::Node::start(
    identity(&app)?,
    node::NodeConfig {
      bootstrap: seeds.clone(),
      ..Default::default()
    },
  )
  .await
  .map_err(|e| e.to_string())?;
  let result = client::rooms::discover(&node, &seeds)
    .await
    .map_err(|e| e.to_string());
  node.shutdown().await.map_err(|e| e.to_string())?;
  result
}

#[tauri::command]
pub async fn union_room_host(
  app: AppHandle,
  state: tauri::State<'_, RoomState>,
  config_path: String,
  seeds: Vec<String>,
  relays: Vec<String>,
) -> Result<HostStatus, String> {
  let mut active = state.0.lock().await;
  if active.as_ref().and_then(RoomHandle::status).is_some() {
    return Err("A room is already running".into());
  }
  active.take();
  let seeds = addresses(seeds)?;
  let relays = addresses(relays)?;
  let metadata = tokio::fs::metadata(&config_path)
    .await
    .map_err(|e| e.to_string())?;
  if metadata.len() > MAX_ROOM_CONFIG_BYTES {
    return Err("Room configuration is too large".into());
  }
  let bytes = tokio::fs::read(config_path)
    .await
    .map_err(|e| e.to_string())?;
  let identity_config = crate::union::trust::configuration(&app)?
    .ok_or("Configure unified identity before hosting rooms")?;
  let room: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
  let genesis_path = identity_config.log.with_extension("genesis.json");
  std::fs::write(
    &genesis_path,
    serde_json::to_vec(&identity_config.genesis).map_err(|e| e.to_string())?,
  )
  .map_err(|e| e.to_string())?;
  let replica = identity_config.log.with_extension("host.jsonl");
  let config:federated_server::runtime::Config=serde_json::from_value(serde_json::json!({
      "node":{"key":"","listen":["/ip4/0.0.0.0/tcp/0"],"bootstrap":seeds.iter().map(ToString::to_string).collect::<Vec<_>>(),"reserve":relays.iter().map(ToString::to_string).collect::<Vec<_>>()},
      "services":{"identity":{"genesis":genesis_path,"genesis_hash":identity_config.genesis_hash,"log":replica,"writer":false,"source":{"peer":identity_config.peer,"address":identity_config.address},"max_staleness_seconds":60},"member_rooms":[room]}
    })).map_err(|e|e.to_string())?;
  let host = federated_server::runtime::Host::start_with_identity(config, host_identity(&app)?)
    .await
    .map_err(|e| e.to_string())?;
  let status = host.room_status().ok_or("Room stopped during startup")?;
  *active = Some(RoomHandle(host));
  Ok(status)
}
#[tauri::command]
pub async fn union_room_stop(state: tauri::State<'_, RoomState>) -> Result<(), String> {
  let mut active = state.0.lock().await;
  if let Some(room) = active.take() {
    room.stop().await.map_err(|e| e.to_string())?;
  }
  Ok(())
}
#[tauri::command]
pub async fn union_room_status(
  state: tauri::State<'_, RoomState>,
) -> Result<Option<HostStatus>, String> {
  Ok(state.0.lock().await.as_ref().and_then(RoomHandle::status))
}
