#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Federation commands shared with the Android manager through union-core.
use client::application as client;
const MAX_GAME_ADDRESSES: usize = 8;
const MAX_GAME_ADDRESS_BYTES: usize = 2048;
use identity::Identity;
use tauri::{AppHandle, Manager};

pub mod rooms;
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
  let signer = identity(&app)?;
  let genesis = input["genesis_hash"].as_str().unwrap_or("").to_owned();
  let response = if input["action"]["type"] == "local.points.request_ai" {
    ::client::points::request_text(signer.clone(), input).await
  } else {
    let input: client::Input = serde_json::from_value(input).map_err(|e| e.to_string())?;
    client::execute(signer.clone(), input).await
  }
  .map_err(|e| e.failure())?;
  let mut rankings = serde_json::Map::new();
  let mut activity_briefs = serde_json::Map::new();
  if let Some(state) = &response.state {
    for (id, event) in &state.domain.events {
      let Ok(cid) = event.brief_hash.parse() else {
        continue;
      };
      let Ok(bytes) = state.domain.inner.files.read(&cid) else {
        continue;
      };
      let Ok(brief) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        continue;
      };
      if brief
        .get("description")
        .is_some_and(serde_json::Value::is_string)
      {
        activity_briefs.insert(id.clone(), brief);
      }
    }
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
  let point_view = response
    .state
    .as_ref()
    .zip(response.identity_view.as_ref())
    .zip(response.proof.as_ref())
    .map(|((state, view), proof)| {
      ::client::points::present(state, view, &signer, &genesis, proof.observed_ms)
    })
    .transpose()
    .map_err(|e| e.failure())?
    .flatten();
  let mut result = serde_json::to_value(response).map_err(|e| e.to_string())?;
  if let Some(points) = point_view {
    result["state"]["points_view"] = serde_json::to_value(points).map_err(|e| e.to_string())?;
  }
  result["season_rankings"] = serde_json::Value::Object(rankings);
  result["activity_briefs"] = serde_json::Value::Object(activity_briefs);
  Ok(result)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoverInput {
  pub expected_genesis: Option<String>,
  pub wait_ms: Option<u64>,
}

/// LAN auto-discovery from union-core; when `expected_genesis` is the built-in pin,
/// only the matching union's servers are returned.
#[tauri::command]
pub async fn union_discover(
  app: AppHandle,
  input: Option<DiscoverInput>,
) -> Result<serde_json::Value, String> {
  let input = input.unwrap_or(DiscoverInput {
    expected_genesis: None,
    wait_ms: None,
  });
  let mut servers = ::client::discovery::discover(
    identity(&app)?,
    input.wait_ms.map(std::time::Duration::from_millis),
  )
  .await
  .map_err(|e| e.to_string())?;
  if let Some(expected) = input.expected_genesis {
    servers.retain(|server| server.genesis_hash == expected);
  }
  serde_json::to_value(serde_json::json!({
    "servers": servers.into_iter().map(|server| serde_json::json!({
      "peer": server.peer.to_string(),
      "address": server.address,
      "federation": server.federation,
      "genesis": server.genesis,
      "genesis_hash": server.genesis_hash,
      "application": server.application,
      "services": server.services,
    })).collect::<Vec<_>>()
  }))
  .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn union_points_output(
  snapshot: serde_json::Value,
  instance: String,
  stage: usize,
) -> Result<String, String> {
  ::client::points::output_value(&snapshot, &instance, stage)
    .map_err(|_| "AI output unavailable".to_owned())
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
