use crate::error::LauncherError;
use crate::AppState;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use super::{ClusterMessage, InstanceInfo, NetworkDiagnostics, ResolvedInstance};

#[tauri::command]
pub async fn list_peers(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<String>, LauncherError> {
    state
        .lock()
        .await
        .network
        .get_peers()
        .await
        .map_err(LauncherError::from)
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
        .map_err(LauncherError::from)
}

#[tauri::command]
pub async fn list_instances(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<InstanceInfo>, LauncherError> {
    state
        .lock()
        .await
        .network
        .list_instances()
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
pub async fn measure_latency(
    state: State<'_, Arc<Mutex<AppState>>>,
    peer_id: String,
) -> Result<Option<u32>, LauncherError> {
    state
        .lock()
        .await
        .network
        .measure_latency(peer_id)
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
pub async fn get_cluster_messages(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<ClusterMessage>, LauncherError> {
    state
        .lock()
        .await
        .network
        .get_messages()
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
pub async fn get_network_diagnostics(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<NetworkDiagnostics, LauncherError> {
    state
        .lock()
        .await
        .network
        .get_diagnostics()
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
pub async fn subscribe_instance_events(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
) -> Result<(), LauncherError> {
    state
        .lock()
        .await
        .network
        .subscribe_topic(format!("mc.events.instance.{instance_id}"))
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
pub async fn unsubscribe_instance_events(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
) -> Result<(), LauncherError> {
    state
        .lock()
        .await
        .network
        .unsubscribe_topic(format!("mc.events.instance.{instance_id}"))
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
pub async fn subscribe_tournament_events(
    state: State<'_, Arc<Mutex<AppState>>>,
    tournament_id: String,
) -> Result<(), LauncherError> {
    state
        .lock()
        .await
        .network
        .subscribe_topic(format!("mc.events.tournament.{tournament_id}"))
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
pub async fn unsubscribe_tournament_events(
    state: State<'_, Arc<Mutex<AppState>>>,
    tournament_id: String,
) -> Result<(), LauncherError> {
    state
        .lock()
        .await
        .network
        .unsubscribe_topic(format!("mc.events.tournament.{tournament_id}"))
        .await
        .map_err(LauncherError::from)
}
