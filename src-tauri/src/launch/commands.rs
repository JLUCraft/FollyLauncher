use crate::launch::helpers::file_validator::resolve_library_path;
use crate::launch::helpers::file_validator::save_crash_report;
use crate::launch::helpers::jre_selector::{scan_java_runtimes, select_java_runtime};
use crate::launch::helpers::process_monitor::{kill_process, monitor_process};
use crate::launch::models::{LaunchStep, LaunchingState};
use crate::AppState;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

#[derive(Debug, Clone, serde::Serialize)]
pub struct LaunchStateResponse {
    pub id: u64,
    pub step: String,
    pub instance_id: String,
    pub version: String,
    pub pid: u32,
}

impl From<&LaunchingState> for LaunchStateResponse {
    fn from(state: &LaunchingState) -> Self {
        Self {
            id: state.id,
            step: format!("{:?}", LaunchStep::from(state.current_step)),
            instance_id: state.instance_id.clone(),
            version: state.version.clone(),
            pid: state.pid,
        }
    }
}

#[tauri::command]
pub async fn launch_select_jre(
    state: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
    instance_id: String,
    version: String,
    game_dir: String,
) -> Result<u64, String> {
    let runtimes = scan_java_runtimes().await;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    let selected = select_java_runtime(&runtimes, &version, None)
        .ok_or_else(|| "未找到合适的 Java 运行时".to_string())?;

    let mut states = state.lock().await;
    let id = timestamp;
    states.insert(
        id,
        LaunchingState {
            id,
            current_step: 1,
            instance_id,
            version,
            java_path: Some(selected.exec_path.clone()),
            java_major_version: Some(selected.major_version),
            game_directory: game_dir,
            pid: 0,
            full_command: String::new(),
            start_time: None,
        },
    );

    Ok(id)
}

#[tauri::command]
pub async fn launch_validate_files(
    state: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
    launching_id: u64,
) -> Result<(), String> {
    let mut states = state.lock().await;
    let launching = states.get_mut(&launching_id).ok_or("启动状态未找到")?;
    launching.current_step = 2;

    let game_dir = std::path::PathBuf::from(&launching.game_directory);
    if !game_dir.exists() {
        return Err(format!("游戏目录不存在: {}", launching.game_directory));
    }

    let version_json = game_dir
        .join("versions")
        .join(&launching.version)
        .join(format!("{}.json", launching.version));
    if !version_json.exists() {
        return Err(format!("版本文件缺失: {}", version_json.display()));
    }

    let content = std::fs::read_to_string(&version_json)
        .map_err(|e| format!("无法读取版本文件: {}", e))?;
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("版本文件 JSON 无效: {}", e))?;

    if let Some(downloads) = parsed.get("downloads") {
        if let Some(client) = downloads.get("client") {
            if let Some(sha1) = client.get("sha1").and_then(|v| v.as_str()) {
                let jar_path = game_dir
                    .join("versions")
                    .join(&launching.version)
                    .join(format!("{}.jar", launching.version));
                if !jar_path.exists() {
                    return Err(format!("客户端 JAR 缺失: {}", jar_path.display()));
                }
                let jar_bytes = std::fs::read(&jar_path)
                    .map_err(|e| format!("无法读取客户端 JAR: {}", e))?;
                let actual = sha1_smol::Sha1::from(&jar_bytes).digest().to_string();
                if actual != sha1.to_lowercase() {
                    tracing::warn!(
                        expected = %sha1,
                        actual = %actual,
                        "client JAR SHA1 mismatch"
                    );
                }
            }
        }
    }

    let libraries_path = game_dir.join("libraries");
    if let Some(libraries) = parsed.get("libraries").and_then(|v| v.as_array()) {
        for lib in libraries {
            if let Some(name) = lib.get("name").and_then(|v| v.as_str()) {
                if let Some(ref lib_path) = resolve_library_path(name, &libraries_path) {
                    if !lib_path.exists() {
                        tracing::warn!(lib = %name, "library missing");
                    }
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn launch_game(
    app: AppHandle,
    state: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
    app_state: State<'_, Arc<Mutex<AppState>>>,
    launching_id: u64,
) -> Result<u32, String> {
    let (instance_id, version, java_path, game_dir) = {
        let mut states = state.lock().await;
        let launching = states.get_mut(&launching_id).ok_or("启动状态未找到")?;
        launching.current_step = 3;

        (
            launching.instance_id.clone(),
            launching.version.clone(),
            launching
                .java_path
                .clone()
                .unwrap_or_else(|| "java".to_string()),
            launching.game_directory.clone(),
        )
    };

    let settings = {
        let app_state = app_state.lock().await;
        app_state.game_settings.clone()
    };

    let game_dir_path = std::path::PathBuf::from(&game_dir);
    let jvm_args = settings.build_jvm_args();

    let options = mc_launcher_core::types::MinecraftOptions {
        username: Some("Player".to_string()),
        uuid: Some(uuid::Uuid::new_v4().to_string()),
        token: Some("0".to_string()),
        launcher_name: Some("FollyLauncher".to_string()),
        executable_path: Some(java_path.clone()),
        jvm_arguments: Some(jvm_args),
        game_directory: Some(game_dir.clone()),
        custom_resolution: Some(settings.resolution_width > 0 && settings.resolution_height > 0),
        resolution_width: Some(settings.resolution_width.to_string()),
        resolution_height: Some(settings.resolution_height.to_string()),
        ..Default::default()
    };

    let command =
        mc_launcher_core::command::get_minecraft_command(&version, &game_dir_path, &options)
            .map_err(|e| format!("构建启动命令失败: {e}"))?;

    let mut cmd = tokio::process::Command::new(&command[0]);
    cmd.args(&command[1..])
        .current_dir(&game_dir_path)
        .stdin(std::process::Stdio::null());

    if settings.show_game_log {
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
    } else {
        cmd.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
    }

    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("启动 Minecraft 进程失败: {e}"))?;

    let pid = child.id().unwrap_or(0);
    if pid == 0 {
        tracing::warn!("child process exited before PID could be captured");
    }

    monitor_process(
        app,
        launching_id,
        child,
        instance_id.clone(),
        settings.show_game_log,
    )
    .await;

    {
        let mut states = state.lock().await;
        if let Some(launching) = states.get_mut(&launching_id) {
            launching.pid = pid;
            launching.current_step = 4;
            launching.start_time = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        }
    }

    Ok(pid)
}

#[tauri::command]
pub async fn launch_cancel(
    state: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
    launching_id: u64,
) -> Result<(), String> {
    let mut states = state.lock().await;
    if let Some(launching) = states.get_mut(&launching_id) {
        if launching.pid != 0 {
            let _ = kill_process(launching.pid);
        }
        launching.current_step = 0;
    }
    Ok(())
}

#[tauri::command]
pub async fn launch_get_state(
    state: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
    launching_id: u64,
) -> Result<LaunchStateResponse, String> {
    let states = state.lock().await;
    let launching = states.get(&launching_id).ok_or("启动状态未找到")?;
    Ok(LaunchStateResponse::from(launching))
}

#[tauri::command]
pub async fn launch_list_states(
    state: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
) -> Result<Vec<LaunchStateResponse>, String> {
    let states = state.lock().await;
    Ok(states.values().map(LaunchStateResponse::from).collect())
}

#[tauri::command]
pub async fn launch_export_crash(
    state: State<'_, Arc<Mutex<HashMap<u64, LaunchingState>>>>,
    launching_id: u64,
    save_path: String,
) -> Result<String, String> {
    let states = state.lock().await;
    let launching = states.get(&launching_id).ok_or("启动状态未找到")?;

    let game_dir = std::path::PathBuf::from(&launching.game_directory);
    let save = std::path::PathBuf::from(&save_path);

    if let Err(e) = save_crash_report(&launching.instance_id, &game_dir, &save).await {
        return Err(format!("创建崩溃报告失败: {e}"));
    }

    Ok(save_path)
}
