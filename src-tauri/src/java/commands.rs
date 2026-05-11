use crate::error::LauncherError;
use crate::java::models::JavaRuntime;
use crate::launch::jre_selector::{scan_java_runtimes, validate_java_path};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[tauri::command]
pub async fn retrieve_java_list(
    java_state: State<'_, Arc<Mutex<Vec<JavaRuntime>>>>,
) -> Result<Vec<JavaRuntime>, LauncherError> {
    let runtimes = scan_java_runtimes().await;
    let mut state = java_state.lock().await;
    *state = runtimes.clone();
    Ok(runtimes)
}

#[tauri::command]
pub async fn validate_java(java_path: String) -> Result<bool, LauncherError> {
    match validate_java_path(&java_path).await {
        Some(_rt) => Ok(true),
        None => Ok(false),
    }
}
