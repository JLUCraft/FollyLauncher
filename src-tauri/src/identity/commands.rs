use crate::error::LauncherError;
use crate::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use super::{VcImportResult, VcStatus};

#[derive(Debug, Serialize)]
pub struct IdentityResponse {
    pub peer_id: String,
    pub public_key: String,
    pub club: Option<String>,
}

#[tauri::command]
pub async fn get_identity(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<IdentityResponse, LauncherError> {
    let state = state.lock().await;
    let id = state.identity.identity();
    Ok(IdentityResponse {
        peer_id: id.peer_id.clone(),
        public_key: id.public_key.clone(),
        club: id.club.clone(),
    })
}

#[tauri::command]
pub async fn get_vc_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<VcStatus, LauncherError> {
    let state = state.lock().await;
    Ok(state.identity.vc_status().await)
}

#[tauri::command]
pub async fn import_vc(
    state: State<'_, Arc<Mutex<AppState>>>,
    vc_json: String,
) -> Result<VcImportResult, LauncherError> {
    let mut state = state.lock().await;
    state
        .identity
        .import_vc(&vc_json)
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
pub async fn clear_vc(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), LauncherError> {
    let mut state = state.lock().await;
    state.identity.clear_vc().await.map_err(LauncherError::from)
}
