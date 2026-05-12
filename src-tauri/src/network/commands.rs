use crate::error::LauncherError;
use crate::AppState;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use super::{InstanceInfo, NetworkDiagnostics, ResolvedInstance};

#[tauri::command]
pub async fn list_peers(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<String>, LauncherError> {
    state.lock().await.network.get_peers().await
}

#[tauri::command]
pub async fn resolve_instance(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
) -> Result<Option<ResolvedInstance>, LauncherError> {
    state
        .lock()
        .await
        .network
        .resolve_instance(instance_id)
        .await
}

#[tauri::command]
pub async fn list_instances(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<InstanceInfo>, LauncherError> {
    let control = {
        let s = state.lock().await;
        s.control.clone()
    };

    match control.list_instances("", "", "").await {
        Ok(api_instances) => Ok(api_instances
            .into_iter()
            .map(|api| InstanceInfo {
                id: api.id,
                name: api.name,
                kind: api.kind,
                status: api.status,
                host: api.host.clone(),
                mode: api.mode,
                club: api.club,
                players: api.player_count,
                max_players: api.max_players,
                version: api.version,
                peer_id: api.host,
                discovered_at: api.created_at,
                updated_at: api.updated_at,
            })
            .collect()),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn measure_latency(
    state: State<'_, Arc<Mutex<AppState>>>,
    peer_id: String,
) -> Result<Option<u32>, LauncherError> {
    state.lock().await.network.measure_latency(peer_id).await
}

#[tauri::command]
pub async fn get_network_diagnostics(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<NetworkDiagnostics, LauncherError> {
    state.lock().await.network.get_diagnostics().await
}
