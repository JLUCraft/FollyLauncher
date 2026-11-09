use crate::launcher_config::models::LauncherConfig;
use sjmcl_types::storage::Storage;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex as AsyncMutex;

#[derive(Default)]
pub struct RelayState(AsyncMutex<Option<node::Node>>);

async fn apply(app: &AppHandle, enabled: bool) -> Result<(), String> {
  let state = app.state::<RelayState>();
  let mut active = state.0.lock().await;
  if enabled {
    if active.is_none() {
      let node = node::Node::start(
        crate::union::identity(app).await?,
        node::NodeConfig {
          relay: Some(node::RelayConfig::default()),
          discovery_server: true,
          ..node::NodeConfig::new(protocol::Namespace::builtin())
        },
      )
      .await
      .map_err(|error| error.to_string())?;
      *active = Some(node);
    }
  } else if let Some(node) = active.take() {
    node.shutdown().await.map_err(|error| error.to_string())?;
  }
  Ok(())
}

pub async fn start_configured(app: &AppHandle) {
  let enabled = app
    .state::<Mutex<LauncherConfig>>()
    .lock()
    .map(|config| config.federation.relay_enabled)
    .unwrap_or(false);
  if enabled && let Err(error) = apply(app, true).await {
    log::error!("Failed to start federation relay: {error}");
  }
}

#[tauri::command]
pub async fn union_set_relay_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
  apply(&app, enabled).await?;
  let config = app.state::<Mutex<LauncherConfig>>();
  let mut config = config.lock().map_err(|error| error.to_string())?;
  config
    .partial_update(&app, "federation.relay_enabled", &enabled.to_string())
    .map_err(|error| error.to_string())?;
  config.save().map_err(|error| error.to_string())
}
