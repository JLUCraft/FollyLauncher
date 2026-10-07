#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Federation commands shared with the Android manager through union-core.
use client::client;
const MAX_GAME_ADDRESSES: usize = 8;
const MAX_GAME_ADDRESS_BYTES: usize = 2048;
use identity::Identity;
use tauri::{AppHandle, Manager};

pub mod rooms;
mod session;
pub mod trust;
mod vault;
fn identity(app: &AppHandle) -> Result<Identity, String> {
  let directory = app.path().app_data_dir().map_err(|e| e.to_string())?;
  vault::identity(&directory)
}
#[tauri::command]
pub fn union_identity(app: AppHandle) -> Result<String, String> {
  Ok(identity(&app)?.peer_id().to_string())
}
#[tauri::command]
pub async fn union_governance(
  app: AppHandle,
  mut input: serde_json::Value,
) -> Result<serde_json::Value, ::client::protocol::ClientFailure> {
  input["identity"] = serde_json::to_value(trust::configuration(&app)?.ok_or_else(|| {
    ::client::protocol::ClientFailure {
      code: ::client::protocol::ClientErrorCode::IdentityNotConfigured,
      message: "Configure unified identity first".into(),
    }
  })?)
  .map_err(|e| e.to_string())?;
  let input: client::Input = serde_json::from_value(input).map_err(|e| e.to_string())?;
  let response = client::execute(identity(&app)?, input)
    .await
    .map_err(|e| e.failure())?;
  let mut rankings = serde_json::Map::new();
  if let Some(state) = &response.state {
    for id in state.domain.seasons.keys() {
      let scores = state
        .domain
        .season_rankings(id)
        .map_err(|e| e.to_string())?;
      // Preserve exact i64 totals across JSON/JavaScript; sorting belongs to league.
      let rows = |values: Vec<(String, i64)>| {
        values
          .into_iter()
          .map(|(id, score)| (id, score.to_string()))
          .collect::<Vec<_>>()
      };
      rankings.insert(
        id.clone(),
        serde_json::json!({"players": rows(scores.players), "teams": rows(scores.teams)}),
      );
    }
  }
  let mut result = serde_json::to_value(response).map_err(|e| e.to_string())?;
  result["season_rankings"] = serde_json::Value::Object(rankings);
  Ok(result)
}

#[derive(Default)]
pub struct ProxyState(pub tokio::sync::Mutex<Option<GameConnection>>);
pub struct GameConnection {
  proxy: terminal_runtime::LocalProxy,
  status: GameConnectionStatus,
}
#[derive(Clone, serde::Serialize)]
pub struct GameConnectionStatus {
  address: String,
  peer: String,
  service: String,
  origin: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyInput {
  pub peer: String,
  pub addresses: Vec<String>,
  pub service: String,
  pub origin: String,
}

#[tauri::command]
pub async fn union_proxy_start(
  app: AppHandle,
  state: tauri::State<'_, ProxyState>,
  input: ProxyInput,
) -> Result<GameConnectionStatus, String> {
  if input.addresses.is_empty()
    || input.addresses.len() > MAX_GAME_ADDRESSES
    || input
      .addresses
      .iter()
      .any(|a| a.len() > MAX_GAME_ADDRESS_BYTES)
  {
    return Err("Invalid game addresses".into());
  }
  if !matches!(input.origin.as_str(), "room" | "match") {
    return Err("Invalid game connection origin".into());
  }
  let mut active = state.0.lock().await;
  if active.as_ref().is_some_and(|p| p.proxy.is_running()) {
    return Err("Disconnect the current game connection first".into());
  }
  let target = terminal_runtime::TargetArgs {
    peer: input
      .peer
      .parse::<node::PeerId>()
      .map_err(|e| e.to_string())?,
    address: input
      .addresses
      .iter()
      .map(|a| a.parse::<node::Multiaddr>())
      .collect::<Result<_, _>>()
      .map_err(|e| e.to_string())?,
  };
  let service = input
    .service
    .parse()
    .map_err(|e: protocol::Error| e.to_string())?;
  let device = identity(&app)?;
  trust::configuration(&app)?.ok_or("Configure unified identity before joining rooms")?;
  let node = node::Node::start(device, node::NodeConfig::default())
    .await
    .map_err(|e| e.to_string())?;
  if let Err(error) = trust::synchronize(&app, &node).await {
    node.shutdown().await.map_err(|e| e.to_string())?;
    return Err(error);
  }
  let proxy = terminal_runtime::LocalProxy::start(
    node,
    target,
    service,
    std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
  )
  .await
  .map_err(|e| e.to_string())?;
  let status = GameConnectionStatus {
    address: proxy.address().to_string(),
    peer: input.peer,
    service: input.service,
    origin: input.origin,
  };
  *active = Some(GameConnection {
    proxy,
    status: status.clone(),
  });
  Ok(status)
}
#[tauri::command]
pub async fn union_proxy_stop(state: tauri::State<'_, ProxyState>) -> Result<(), String> {
  state.0.lock().await.take();
  Ok(())
}
#[tauri::command]
pub async fn union_proxy_status(
  state: tauri::State<'_, ProxyState>,
) -> Result<Option<GameConnectionStatus>, String> {
  Ok(
    state
      .0
      .lock()
      .await
      .as_ref()
      .filter(|p| p.proxy.is_running())
      .map(|p| p.status.clone()),
  )
}

pub async fn active_game_address(app: &AppHandle) -> Option<String> {
  app
    .state::<ProxyState>()
    .0
    .lock()
    .await
    .as_ref()
    .filter(|c| c.proxy.is_running())
    .map(|c| c.proxy.address().to_string())
}
