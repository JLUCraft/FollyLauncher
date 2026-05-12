use crate::account::commands::{account_token_for_local_launch, selected_account_in};
use crate::error::LauncherError;
use crate::instance::commands::{get_instance_in, mark_instance_played_in};
use crate::launch::file_validator::save_crash_report;
use crate::launch::jre_selector::{scan_java_runtimes, select_java_runtime};
use crate::launch::models::{LaunchStep, LaunchingState};
use crate::launch::process_monitor::{kill_process, monitor_process};
use crate::AppState;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};
use tokio::sync::Mutex;


#[derive(Debug, Clone, serde::Serialize)]
pub struct LaunchLocalInstanceResult {
    pub launching_id: u64,
    pub pid: u32,
    pub instance_id: String,
    pub instance_name: String,
    pub game_version: String,
    pub username: String,
    pub uuid: String,
    pub java_path: String,
    pub game_dir: String,
}





pub fn local_version_json_path(game_dir: &Path, version: &str) -> PathBuf {
    game_dir
        .join("versions")
        .join(version)
        .join(format!("{version}.json"))
}








pub fn resolve_local_java_path(
    configured_java: &str,
    selected_java: Option<String>,
) -> Result<String, crate::error::LauncherError> {
    if !configured_java.trim().is_empty() {
        return Ok(configured_java.to_string());
    }
    selected_java.ok_or_else(|| {
        crate::error::LauncherError::from("未找到合适的 Java 运行时，请在设置中选择 Java")
    })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LaunchStateResponse {
    pub id: u64,
    pub step: String,
    pub instance_id: String,
    pub version: String,
    pub pid: u32,
    pub exit_code: Option<i32>,
    pub exit_ok: Option<bool>,
    pub game_ready: bool,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub recent_logs: Vec<String>,
}

impl From<&LaunchingState> for LaunchStateResponse {
    fn from(state: &LaunchingState) -> Self {
        Self {
            id: state.id,
            step: format!("{:?}", LaunchStep::from(state.current_step)),
            instance_id: state.instance_id.clone(),
            version: state.version.clone(),
            pid: state.pid,
            exit_code: state.exit_code,
            exit_ok: state.exit_ok,
            game_ready: state.game_ready,
            start_time: state.start_time,
            end_time: state.end_time,
            recent_logs: state.recent_logs.clone(),
        }
    }
}

#[tauri::command]
pub async fn launch_local_instance(
    app: AppHandle,
    launch_states: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
    app_state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
) -> Result<LaunchLocalInstanceResult, LauncherError> {

    let (data_dir, game_settings) = {
        let s = app_state.lock().await;
        (s.data_dir.clone(), s.game_settings.clone())
    };


    game_settings.validate().map_err(|e| e.to_string())?;


    let instance = get_instance_in(&data_dir, &instance_id)?;


    let account = selected_account_in(&data_dir)?;

    let instance_game_dir = instance.game_dir.clone();
    let instance_game_version = instance.game_version.clone();
    let instance_name = instance.name.clone();


    let selected_java = if game_settings.java_path.trim().is_empty() {
        let runtimes = scan_java_runtimes().await;
        Some(
            select_java_runtime(&runtimes, &instance_game_version, None, None)
                .ok_or_else(|| "未找到合适的 Java 运行时，请在设置中选择 Java".to_string())?
                .exec_path,
        )
    } else {
        None
    };
    let java_path = resolve_local_java_path(&game_settings.java_path, selected_java)?;


    let game_dir_path = std::path::PathBuf::from(&instance_game_dir);
    let version_json = local_version_json_path(&game_dir_path, &instance_game_version);
    if !version_json.exists() {
        return Err(LauncherError::new(
            "VERSION_MISSING",
            format!("版本文件缺失: {}", version_json.display()),
        ));
    }


    let native_readiness =
        crate::resource::validator::check_native_readiness(&game_dir_path, &instance_game_version);
    if !native_readiness.ready {
        return Err(LauncherError::new(
            "NATIVES_NOT_READY",
            format!(
                "原生库未就绪 ({}): {}. 请先修复客户端文件再启动。",
                native_readiness.natives_dir_path,
                native_readiness.reasons.join("; ")
            ),
        ));
    }


    let jvm_args = game_settings.build_jvm_args();
    let token = account_token_for_local_launch(&data_dir, &account)?;

    let options = crate::launch::minecraft_command::MinecraftOptions {
        username: Some(account.username.clone()),
        uuid: Some(account.uuid.clone()),
        token: Some(token),
        server: None,
        port: None,
        launcher_name: Some("FollyLauncher".to_string()),
        executable_path: Some(java_path.clone()),
        jvm_arguments: Some(jvm_args),
        game_directory: Some(instance_game_dir.clone()),
        custom_resolution: Some(
            game_settings.resolution_width > 0 && game_settings.resolution_height > 0,
        ),
        resolution_width: Some(game_settings.resolution_width.to_string()),
        resolution_height: Some(game_settings.resolution_height.to_string()),
    };

    let command = crate::launch::minecraft_command::get_minecraft_command(
        &instance_game_version,
        &game_dir_path,
        &options,
    )
    .map_err(|e| {
        format!(
            "构建启动命令失败 (版本 {} 可能未安装): {e}",
            instance_game_version
        )
    })?;

    use crate::launch::process::LogOutput;
    let log_output = if game_settings.show_game_log {
        LogOutput::Piped
    } else {
        LogOutput::Null
    };
    let child =
        crate::launch::process::spawn_minecraft_process(&command, &game_dir_path, log_output)?;

    let pid = child.id().unwrap_or(0);
    if pid == 0 {
        tracing::warn!("child process exited before PID could be captured");
    }


    let launching_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    {
        let mut states = launch_states.lock().await;
        states.insert(
            launching_id,
            LaunchingState {
                id: launching_id,
                current_step: 4,
                instance_id: instance_id.clone(),
                version: instance_game_version.clone(),
                java_path: Some(java_path.clone()),
                java_major_version: None,
                game_directory: instance_game_dir.clone(),
                pid,
                full_command: command.join(" "),
                start_time: Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                ),
                exit_code: None,
                exit_ok: None,
                game_ready: false,
                end_time: None,
                recent_logs: Vec::new(),
            },
        );
    }


    let arc_states = (*launch_states).clone();
    monitor_process(
        app,
        launching_id,
        child,
        instance_id.clone(),
        game_settings.show_game_log,
        arc_states,
    )
    .await;


    let _ = mark_instance_played_in(&data_dir, &instance_id);

    Ok(LaunchLocalInstanceResult {
        launching_id,
        pid,
        instance_id,
        instance_name,
        game_version: instance_game_version,
        username: account.username,
        uuid: account.uuid,
        java_path,
        game_dir: instance_game_dir,
    })
}






#[tauri::command]
pub async fn launch_cancel(
    state: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
    launching_id: u64,
) -> Result<(), LauncherError> {
    let mut states = state.lock().await;
    if let Some(launching) = states.get_mut(&launching_id) {
        if launching.pid != 0 {
            let _ = kill_process(launching.pid);
        }
        launching.current_step = LaunchStep::Exited as usize;
        launching.exit_ok = Some(false);
        launching.end_time = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn launch_get_state(
    state: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
    launching_id: u64,
) -> Result<LaunchStateResponse, LauncherError> {
    let states = state.lock().await;
    let launching = states.get(&launching_id).ok_or("启动状态未找到")?;
    Ok(LaunchStateResponse::from(launching))
}

#[tauri::command]
pub async fn launch_list_states(
    state: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
) -> Result<Vec<LaunchStateResponse>, LauncherError> {
    let states = state.lock().await;
    Ok(states.values().map(LaunchStateResponse::from).collect())
}

#[tauri::command]
pub async fn launch_export_crash(
    state: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
    launching_id: u64,
    save_path: String,
) -> Result<String, LauncherError> {
    let states = state.lock().await;
    let launching = states.get(&launching_id).ok_or("启动状态未找到")?;

    let game_dir = std::path::PathBuf::from(&launching.game_directory);
    let save = std::path::PathBuf::from(&save_path);

    if let Err(e) = save_crash_report(&launching.instance_id, &game_dir, &save).await {
        return Err(LauncherError::new(
            "CRASH_REPORT_ERROR",
            format!("创建崩溃报告失败: {e}"),
        ));
    }

    Ok(save_path)
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::models::{LauncherAccount, LauncherAccountKind};



    #[test]
    fn version_json_path_builds_correctly() {
        let game_dir = Path::new("/games/minecraft");
        let path = local_version_json_path(game_dir, "1.21.4");
        assert_eq!(
            path,
            PathBuf::from("/games/minecraft/versions/1.21.4/1.21.4.json")
        );
    }

    #[test]
    fn version_json_path_handles_snapshot_version() {
        let expected = Path::new("/mc")
            .join("versions")
            .join("25w16a")
            .join("25w16a.json");
        let path = local_version_json_path(Path::new("/mc"), "25w16a");
        assert_eq!(path, expected);
    }



    #[test]
    fn resolve_uses_configured_java_when_present() {
        let result =
            resolve_local_java_path("/usr/bin/java21", Some("/usr/bin/java17".to_string()));
        assert_eq!(result.unwrap(), "/usr/bin/java21");
    }

    #[test]
    fn resolve_uses_scanned_java_when_configured_java_is_empty() {
        let result = resolve_local_java_path("", Some("/usr/bin/java21".to_string()));
        assert_eq!(result.unwrap(), "/usr/bin/java21");
    }

    #[test]
    fn resolve_errors_when_both_empty() {
        let result = resolve_local_java_path("", None);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("未找到合适的 Java"),
            "error should mention Java"
        );
    }

    #[test]
    fn resolve_trims_whitespace_configured_java() {
        let result = resolve_local_java_path("  ", Some("/usr/bin/java".to_string()));
        assert_eq!(result.unwrap(), "/usr/bin/java");
    }




    use crate::account::commands::account_token_for_local_launch;
    use std::path::Path;

    #[test]
    fn offline_account_token_is_zero() {
        let account = LauncherAccount {
            id: "test-id".to_string(),
            kind: LauncherAccountKind::Offline,
            username: "Steve".to_string(),
            uuid: "5627dd98-e6be-3c21-b8a8-e92344183641".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            last_validated_at: None,
            token_expires_at: None,
        };

        let dummy_dir = Path::new("/nonexistent");
        assert_eq!(
            account_token_for_local_launch(dummy_dir, &account).unwrap(),
            "0"
        );
    }

    #[test]
    fn microsoft_account_missing_token_errors() {
        let account = LauncherAccount {
            id: "ms-id".to_string(),
            kind: LauncherAccountKind::Microsoft,
            username: "Steve".to_string(),
            uuid: "5627dd98-e6be-3c21-b8a8-e92344183641".to_string(),
            selected: true,
            auth_server_url: None,
            avatar_url: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            last_validated_at: None,
            token_expires_at: None,
        };
        let data_dir = Path::new("target/test-data/microsoft_account_missing_token_errors");
        std::fs::create_dir_all(data_dir.join("accounts")).unwrap();
        let err = account_token_for_local_launch(data_dir, &account).unwrap_err();
        assert!(err.contains("已失效"), "unexpected error: {err}");
    }





    fn version_json_with_natives() -> serde_json::Value {
        let os = crate::resource::validator::current_os_name();
        serde_json::json!({
            "libraries": [
                {
                    "name": "org.lwjgl:lwjgl:3.3.1",
                    "natives": {
                        os: "natives-${arch}"
                    },
                    "downloads": {
                        "artifact": {
                            "sha1": "abc123",
                            "size": 100,
                            "url": "https://example.com/lwjgl.jar"
                        },
                        "classifiers": {
                            "natives-windows": {
                                "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar"
                            }
                        }
                    }
                }
            ],
            "mainClass": "net.minecraft.client.main.Main",
            "downloads": {
                "client": {
                    "sha1": "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d",
                    "size": 1000,
                    "url": "https://example.com/client.jar"
                }
            }
        })
    }

    #[test]
    fn launch_validation_refuses_missing_natives_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();


        let version_json = version_json_with_natives();
        let version_dir = game_dir.join("versions").join("1.21");
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::write(
            version_dir.join("1.21.json"),
            serde_json::to_string(&version_json).unwrap(),
        )
        .unwrap();

        std::fs::write(version_dir.join("1.21.jar"), b"hello").unwrap();

        let readiness = crate::resource::validator::check_native_readiness(game_dir, "1.21");
        assert!(
            !readiness.ready,
            "launch validation must refuse when natives dir is missing"
        );
        assert!(
            readiness
                .reasons
                .iter()
                .any(|r| r.contains("does not exist")),
            "should explain why natives are not ready: {:?}",
            readiness.reasons
        );
    }

    #[test]
    fn launch_validation_refuses_empty_natives_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();

        let version_json = version_json_with_natives();
        let version_dir = game_dir.join("versions").join("1.21");
        let natives_dir = version_dir.join("natives");
        std::fs::create_dir_all(&natives_dir).unwrap();
        std::fs::write(
            version_dir.join("1.21.json"),
            serde_json::to_string(&version_json).unwrap(),
        )
        .unwrap();
        std::fs::write(version_dir.join("1.21.jar"), b"hello").unwrap();

        let readiness = crate::resource::validator::check_native_readiness(game_dir, "1.21");
        assert!(
            !readiness.ready,
            "launch validation must refuse when natives dir is empty"
        );
        assert!(
            readiness
                .reasons
                .iter()
                .any(|r| r.contains("no native files")),
            "should mention empty dir: {:?}",
            readiness.reasons
        );
    }

    #[test]
    fn launch_validation_accepts_populated_natives_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();

        let version_json = version_json_with_natives();
        let version_dir = game_dir.join("versions").join("1.21");
        let natives_dir = version_dir.join("natives");
        std::fs::create_dir_all(&natives_dir).unwrap();
        std::fs::write(natives_dir.join("lwjgl.dll"), b"mock-native").unwrap();
        std::fs::write(
            version_dir.join("1.21.json"),
            serde_json::to_string(&version_json).unwrap(),
        )
        .unwrap();
        std::fs::write(version_dir.join("1.21.jar"), b"hello").unwrap();

        let readiness = crate::resource::validator::check_native_readiness(game_dir, "1.21");
        assert!(
            readiness.ready,
            "launch validation must accept when natives dir is populated"
        );
        assert!(
            readiness.reasons.is_empty(),
            "no reasons expected when ready, got: {:?}",
            readiness.reasons
        );
    }
}
