mod api;
mod api_auth;
mod discover;
mod identity;
mod java;
mod launch;
mod mua_auth;
mod network;
mod notification;
mod proxy;
mod resource;
mod resource_sync;
mod settings;
mod tasks;
mod workspace;

use anyhow::Context;
use directories::ProjectDirs;
use identity::{IdentityManager, VcHolderState, VcImportResult, VcStatus};
use java::models::JavaRuntime;
use launch::models::LaunchingState;
use mua_auth::{MuaAuthService, MuaLoginStatus};
use network::Network;
use notification::NotificationService;
use proxy::InstanceProxy;
use resource_sync::{ResourceSyncService, BitSwapStatus, SyncResult, SyncStatus};
use serde::Serialize;
use settings::GameSettings;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tasks::models::TaskGroup;
use tauri::{Manager, State};
use tokio::sync::Mutex;

pub struct AppState {
    identity: IdentityManager,
    proxy_port: u16,
    proxy: proxy::InstanceProxy,
    network: network::NetworkHandle,
    api: api::FederatedApiClient,
    api_auth: Arc<api_auth::ApiAuth>,
    mua_auth: MuaAuthService,
    resource_sync: ResourceSyncService,
    game_settings: GameSettings,
    data_dir: PathBuf,
}

#[derive(Serialize)]
struct IdentityResponse {
    peer_id: String,
    public_key: String,
    club: Option<String>,
}

#[derive(Serialize)]
struct ProxyInfo {
    local_port: u16,
}

#[tauri::command]
fn health() -> &'static str {
    "ready"
}

#[tauri::command]
async fn get_identity(state: State<'_, Arc<Mutex<AppState>>>) -> Result<IdentityResponse, String> {
    let state = state.lock().await;
    let id = state.identity.identity();
    Ok(IdentityResponse {
        peer_id: id.peer_id.clone(),
        public_key: id.public_key.clone(),
        club: id.club.clone(),
    })
}

#[tauri::command]
async fn get_proxy_port(state: State<'_, Arc<Mutex<AppState>>>) -> Result<ProxyInfo, String> {
    let state = state.lock().await;
    Ok(ProxyInfo {
        local_port: state.proxy_port,
    })
}

#[tauri::command]
async fn list_proxy_sessions(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<proxy::ProxySession>, String> {
    let state = state.lock().await;
    Ok(state.proxy.active_sessions().await)
}

#[tauri::command]
async fn get_vc_status(state: State<'_, Arc<Mutex<AppState>>>) -> Result<VcStatus, String> {
    let state = state.lock().await;
    Ok(state.identity.vc_status())
}

#[tauri::command]
async fn import_vc(
    state: State<'_, Arc<Mutex<AppState>>>,
    vc_json: String,
) -> Result<VcImportResult, String> {
    let mut state = state.lock().await;
    state
        .identity
        .import_vc(&vc_json)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_vc(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let mut state = state.lock().await;
    state.identity.clear_vc().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_peers(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Vec<String>, String> {
    let state = state.lock().await;
    Ok(state.network.get_peers().await)
}

#[tauri::command]
async fn list_instances(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<network::InstanceInfo>, String> {
    let state = state.lock().await;
    Ok(state.network.list_instances().await)
}

#[tauri::command]
async fn measure_latency(
    state: State<'_, Arc<Mutex<AppState>>>,
    peer_id: String,
) -> Result<Option<u32>, String> {
    let state = state.lock().await;
    Ok(state.network.measure_latency(peer_id).await)
}

#[tauri::command]
async fn get_cluster_messages(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<network::ClusterMessage>, String> {
    let state = state.lock().await;
    Ok(state.network.get_messages().await)
}

#[tauri::command]
async fn get_network_diagnostics(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<network::NetworkDiagnostics, String> {
    let state = state.lock().await;
    Ok(state.network.get_diagnostics().await)
}

#[tauri::command]
async fn list_tournaments(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<api::Tournament>, String> {
    let state = state.lock().await;
    state.api.list_tournaments().await
}

#[tauri::command]
async fn get_tournament(
    state: State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<Option<api::Tournament>, String> {
    let state = state.lock().await;
    state.api.get_tournament(&id).await
}

#[tauri::command]
async fn list_matches(
    state: State<'_, Arc<Mutex<AppState>>>,
    tournament_id: String,
) -> Result<Vec<api::Match>, String> {
    let state = state.lock().await;
    state.api.list_matches(&tournament_id).await
}

#[tauri::command]
async fn register_for_tournament(
    state: State<'_, Arc<Mutex<AppState>>>,
    tournament_id: String,
) -> Result<(), String> {
    let state = state.lock().await;
    state.api.register_for_tournament(&tournament_id).await
}

#[tauri::command]
async fn create_team(
    state: State<'_, Arc<Mutex<AppState>>>,
    name: String,
    members: Vec<String>,
) -> Result<(), String> {
    let state = state.lock().await;
    state.api.create_team(&name, members).await
}

#[tauri::command]
async fn list_teams(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Vec<api::Team>, String> {
    let state = state.lock().await;
    state.api.list_teams().await
}

#[tauri::command]
async fn get_mua_status(state: State<'_, Arc<Mutex<AppState>>>) -> Result<MuaLoginStatus, String> {
    let state = state.lock().await;
    let account = state.identity.mua_account();
    let peer_bound = state.identity.serverless_token().is_some();
    let vc_state = state.identity.vc_status().state;
    let is_member = vc_state == VcHolderState::Member;
    Ok(MuaLoginStatus {
        logged_in: account.is_some(),
        username: account.map(|a| a.username.clone()),
        uuid: account.map(|a| a.uuid.clone()),
        auth_server_url: state.mua_auth.auth_server_url().to_string(),
        peer_bound,
        is_guest: account.is_some() && !is_member,
        is_member,
    })
}

#[tauri::command]
async fn start_mua_login(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<mua_auth::StartAuthResponse, String> {
    let state = state.lock().await;
    state
        .mua_auth
        .start_device_auth()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn poll_mua_login(state: State<'_, Arc<Mutex<AppState>>>) -> Result<MuaLoginStatus, String> {
    let state_arc = state.inner().clone();
    let mut state = state.lock().await;
    let account = state
        .mua_auth
        .poll_token()
        .await
        .map_err(|e| e.to_string())?;

    state
        .identity
        .set_mua_account(account.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Auto-bind peer_id to Yggdrasil UUID in the background
    let api_auth_bg = state.api_auth.clone();
    let peer_id = state.identity.peer_id().to_string();
    let uuid = account.uuid.clone();

    drop(state);

    tokio::spawn(async move {
        let body = serde_json::json!({
            "mua_identifier": uuid,
            "peer_id": peer_id,
        });
        match api_auth_bg.post_with_auth(
            "/v1/auth/mua-peer-bind",
            &body,
            "mua-peer-bind",
        ).await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let mut state = state_arc.lock().await;
                    if let Err(e) = state
                        .identity
                        .set_serverless_token("bound".to_string())
                        .await
                    {
                        tracing::warn!(error = %e, "failed to mark peer binding");
                    } else {
                        tracing::info!(username = %uuid, %peer_id, "peer_id bound to Yggdrasil account");
                    }
                } else {
                    tracing::warn!(status = %resp.status(), "mua-peer-bind returned non-success");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "auto peer_id bind failed (auth may be required), user can retry manually");
            }
        }
    });

    Ok(MuaLoginStatus {
        logged_in: true,
        username: Some(account.username),
        uuid: Some(account.uuid),
        auth_server_url: account.auth_server_url,
        peer_bound: false, // Will be updated in background task
        is_guest: true,    // New MUA login without VC is guest
        is_member: false,
    })
}

#[tauri::command]
async fn logout_mua(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let mut state = state.lock().await;
    state
        .identity
        .clear_mua_account()
        .await
        .map_err(|e| e.to_string())?;
    state
        .identity
        .clear_serverless_token()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_skin_textures(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<mua_auth::SkinTextures, String> {
    let state = state.lock().await;
    let account = state
        .identity
        .mua_account()
        .ok_or_else(|| "not logged in".to_string())?;
    state
        .mua_auth
        .get_skin_textures(
            &account.auth_server_url,
            &account.access_token,
            &account.uuid,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_game_settings(state: State<'_, Arc<Mutex<AppState>>>) -> Result<GameSettings, String> {
    let state = state.lock().await;
    Ok(state.game_settings.clone())
}

#[tauri::command]
async fn update_game_settings(
    state: State<'_, Arc<Mutex<AppState>>>,
    settings: GameSettings,
) -> Result<(), String> {
    let mut state = state.lock().await;
    if let Err(e) = settings.save(&state.data_dir) {
        return Err(format!("failed to save settings: {}", e));
    }
    state.game_settings = settings;
    Ok(())
}

#[tauri::command]
async fn get_resource_sync_status(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
) -> Result<SyncStatus, String> {
    let state = state.lock().await;
    let base_url = state.api.get_base_url();
    let manifest = state
        .resource_sync
        .fetch_manifest(&base_url, &instance_id)
        .await
        .map_err(|e| e.to_string())?;

    let (cached, missing) = state.resource_sync.check_cache(&manifest);
    Ok(SyncStatus {
        manifest_loaded: true,
        total_files: manifest.files.len(),
        cached_files: cached.len(),
        missing_files: missing.len(),
        sync_in_progress: false,
        last_error: None,
    })
}

#[tauri::command]
async fn sync_resources(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
) -> Result<SyncResult, String> {
    let state = state.lock().await;
    let base_url = state.api.get_base_url();
    let manifest = state
        .resource_sync
        .fetch_manifest(&base_url, &instance_id)
        .await
        .map_err(|e| e.to_string())?;

    let (_cached, missing) = state.resource_sync.check_cache(&manifest);
    let result = state.resource_sync.sync_files(&missing, &base_url).await;
    Ok(result)
}

#[tauri::command]
async fn get_bitswap_status(
    state: State<'_, Arc<Mutex<AppState>>>,
    file_hash: String,
    chunks_json: String,
) -> Result<BitSwapStatus, String> {
    let state = state.lock().await;
    let chunks: Vec<resource_sync::ManifestChunk> =
        serde_json::from_str(&chunks_json).map_err(|e| e.to_string())?;
    let file = resource_sync::ManifestFile {
        path: String::new(),
        hash: file_hash,
        size: 0,
        required: false,
        download_url: None,
        chunks,
    };
    Ok(state.resource_sync.get_bitswap_status(&file).await)
}

#[tauri::command]
async fn select_best_relay(
    state: State<'_, Arc<Mutex<AppState>>>,
    target_peer_id: String,
    club: Option<String>,
) -> Result<Option<String>, String> {
    let state = state.lock().await;
    Ok(state
        .network
        .select_best_relay(target_peer_id, club)
        .await
        .map(|p| p.to_string()))
}

#[tauri::command]
async fn migrate_instance_connection(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
    current_peer_id: String,
) -> Result<String, String> {
    let state = state.lock().await;
    let (_, new_peer_id) = state
        .proxy
        .migrate_instance_connection(&instance_id, &current_peer_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(new_peer_id)
}

#[tauri::command]
async fn list_instances_http(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<api::HttpInstance>, String> {
    let state = state.lock().await;
    state.api.list_instances().await
}

#[tauri::command]
async fn create_instance(
    state: State<'_, Arc<Mutex<AppState>>>,
    name: String,
    kind: String,
    club: String,
    version: String,
) -> Result<api::HttpInstance, String> {
    let state = state.lock().await;
    let owner = state.identity.peer_id().to_string();
    let room = state.mua_auth.room_config();
    let instance_json = serde_json::json!({
        "name": name,
        "kind": kind,
        "club": club,
        "owner": owner,
        "runtime": {
            "image": format!("{}{}", room.image_prefix, version),
            "command": [],
            "env": {},
            "labels": {},
            "working_dir": "/data",
            "data_mount_path": "/data",
            "log_path": "/data/logs/latest.log",
        },
        "resources": {
            "cpu_cores": room.cpu_cores,
            "memory_gb": room.memory_gb,
            "disk_gb": room.disk_gb,
        },
        "auto_restart": room.auto_restart,
        "admission": {
            "mode": room.admission_mode,
            "allowed_clubs": [],
            "allowed_players": [],
            "requires_verified_email": false,
            "allowed_email_domains": [],
        },
    });
    let auth = state.api_auth.clone();
    let resp = auth
        .post_with_auth("/v1/instances", &instance_json, "create-instance")
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("create_instance failed: {}", text));
    }
    resp.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn resolve_instance(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
) -> Result<Option<network::ResolvedInstance>, String> {
    let state = state.lock().await;
    Ok(state.network.resolve_instance(instance_id).await)
}

#[tauri::command]
async fn launch_instance(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
    version: String,
) -> Result<u16, String> {
    tracing::info!(%instance_id, %version, "launching instance via dedicated proxy");

    let state_guard = state.lock().await;

    // Resolve instance first to fail fast if not found
    let resolved = state_guard
        .network
        .resolve_instance(instance_id.clone())
        .await
        .ok_or_else(|| format!("实例 {instance_id} 未在 DHT 中找到"))?;

    tracing::info!(
        target_peer = %resolved.peer_id,
        proxy = ?resolved.proxy_address,
        "instance resolved"
    );

    let identity = state_guard.identity.identity();
    let peer_id = identity.peer_id.clone();
    let vc_state = state_guard.identity.vc_status().state;
    let has_member_vc = vc_state == VcHolderState::Member;

    let mua = identity.mua_account.clone();
    let club = identity.club.clone();

    let proxy = state_guard.proxy.clone();
    let launcher_name = state_guard.api.get_launcher_name();
    drop(state_guard);

    // Use the shared proxy to allocate a dedicated port for this instance
    let bridge_port = proxy
        .bridge_instance(instance_id, peer_id, club, has_member_vc)
        .await
        .map_err(|e| format!("创建实例桥接失败: {e}"))?;

    tracing::info!(%bridge_port, "dedicated proxy port ready for minecraft");

    // Launch Minecraft pointing to the dedicated proxy port
    let account = mua.ok_or_else(|| "缺少 MUA 账号，无法启动实例".to_string())?;
    tracing::info!(username = %account.username, uuid = %account.uuid, "launching with MUA credentials");

    let settings = {
        let state = state.lock().await;
        state.game_settings.clone()
    };
    let game_dir = std::path::PathBuf::from(&settings.game_directory);

    let jvm_args = settings.build_jvm_args();

    let options = mc_launcher_core::types::MinecraftOptions {
        username: Some(account.username),
        uuid: Some(account.uuid),
        token: Some(account.access_token),
        server: Some("127.0.0.1".to_string()),
        port: Some(bridge_port.to_string()),
        launcher_name: Some(launcher_name),
        executable_path: Some(settings.java_path.clone()),
        jvm_arguments: Some(jvm_args),
        game_directory: Some(game_dir.to_string_lossy().to_string()),
        custom_resolution: Some(true),
        resolution_width: Some(settings.resolution_width.to_string()),
        resolution_height: Some(settings.resolution_height.to_string()),
        ..Default::default()
    };

    let command = mc_launcher_core::command::get_minecraft_command(&version, &game_dir, &options)
        .map_err(|e| format!("构建启动命令失败 (版本 {version} 可能未安装): {e}"))?;

    let mut cmd = tokio::process::Command::new(&command[0]);
    cmd.args(&command[1..])
        .current_dir(&game_dir)
        .stdin(std::process::Stdio::null());

    if settings.show_game_log {
        cmd.stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
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

    tracing::info!(pid = %child.id().unwrap_or(0), %bridge_port, "minecraft process started via dedicated proxy");
    Ok(bridge_port)
}

#[tauri::command]
async fn launch_minecraft(
    state: State<'_, Arc<Mutex<AppState>>>,
    server_address: String,
    _instance_id: String,
    version: String,
) -> Result<(), String> {
    tracing::info!(%server_address, %version, "launching minecraft (legacy mode)");

    let settings = {
        let state = state.lock().await;
        state.game_settings.clone()
    };
    let launcher_name = {
        let state = state.lock().await;
        state.api.get_launcher_name()
    };
    let game_dir = std::path::PathBuf::from(&settings.game_directory);

    let (host, port) =
        parse_server_address(&server_address).map_err(|e| format!("解析服务器地址失败: {e}"))?;

    let jvm_args = settings.build_jvm_args();

    let options = mc_launcher_core::types::MinecraftOptions {
        username: None,
        uuid: None,
        token: None,
        server: Some(host),
        port: Some(port),
        launcher_name: Some(launcher_name),
        executable_path: Some(settings.java_path.clone()),
        jvm_arguments: Some(jvm_args),
        game_directory: Some(game_dir.to_string_lossy().to_string()),
        custom_resolution: Some(true),
        resolution_width: Some(settings.resolution_width.to_string()),
        resolution_height: Some(settings.resolution_height.to_string()),
        ..Default::default()
    };

    let command = mc_launcher_core::command::get_minecraft_command(&version, &game_dir, &options)
        .map_err(|e| format!("构建启动命令失败 (版本 {version} 可能未安装): {e}"))?;

    let mut cmd = tokio::process::Command::new(&command[0]);
    cmd.args(&command[1..])
        .current_dir(&game_dir)
        .stdin(std::process::Stdio::null());

    if settings.show_game_log {
        cmd.stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
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

    tracing::info!(pid = %child.id().unwrap_or(0), "minecraft process started");
    Ok(())
}

fn parse_server_address(addr: &str) -> anyhow::Result<(String, String)> {
    let base = addr
        .split(';')
        .next()
        .context("missing server address segment")?;
    let mut parts = base.splitn(2, ':');
    let host = parts.next().context("missing host")?.to_string();
    let port = parts.next().context("missing port")?.to_string();
    Ok((host, port))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            health,
            get_identity,
            get_proxy_port,
            list_proxy_sessions,
            list_peers,
            resolve_instance,
            list_instances,
            list_instances_http,
            create_instance,
            measure_latency,
            launch_instance,
            launch_minecraft,
            get_vc_status,
            import_vc,
            clear_vc,
            get_cluster_messages,
            get_network_diagnostics,
            list_tournaments,
            get_tournament,
            list_matches,
            register_for_tournament,
            create_team,
            list_teams,
            get_mua_status,
            start_mua_login,
            poll_mua_login,
            logout_mua,
            get_skin_textures,
            get_game_settings,
            update_game_settings,
            get_resource_sync_status,
            sync_resources,
            get_bitswap_status,
            select_best_relay,
            migrate_instance_connection,
            // Phase 1: Launch pipeline
            launch::commands::launch_select_jre,
            launch::commands::launch_validate_files,
            launch::commands::launch_game,
            launch::commands::launch_cancel,
            launch::commands::launch_get_state,
            launch::commands::launch_list_states,
            launch::commands::launch_export_crash,
            // Phase 2: Java runtime management
            java::commands::retrieve_java_list,
            java::commands::validate_java,
            // Phase 3: Task system
            tasks::commands::schedule_task_group,
            tasks::commands::update_task_progress,
            tasks::commands::get_task_group,
            tasks::commands::list_task_groups,
            tasks::commands::cancel_task_group,
            tasks::commands::remove_task_group,
            // Phase 4: Workspace
            workspace::commands::retrieve_world_list,
            workspace::commands::retrieve_screenshot_list,
            workspace::commands::retrieve_resource_pack_list,
            workspace::commands::retrieve_shader_pack_list,
            workspace::commands::retrieve_game_server_list,
            // Resource download (CurseForge + Modrinth)
            resource::commands::fetch_game_version_list,
            resource::commands::fetch_mod_loader_version_list,
            resource::commands::fetch_optifine_version_list,
            resource::commands::fetch_resource_list_by_name,
            resource::commands::fetch_resource_version_packs,
            resource::commands::fetch_remote_resource_by_local,
            resource::commands::fetch_remote_resource_by_id,
            resource::commands::download_game_server,
            resource::commands::update_mods,
            // Discover / News
            discover::commands::fetch_news_sources_info,
            discover::commands::fetch_news_post_summaries,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = init_backend(handle).await {
                    tracing::error!(error = %e, "backend initialization failed");
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn init_backend(handle: tauri::AppHandle) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let data_dir = project_data_dir()?;
    let config = api::LauncherConfig::load_from_resource(&handle)?;
    let default_fed = config.default_federated_server()
        .ok_or_else(|| anyhow::anyhow!("no federated server configured in defaults.toml"))?;
    let default_ygg = config.default_yggdrasil_server()
        .ok_or_else(|| anyhow::anyhow!("no yggdrasil server configured in defaults.toml"))?;
    info!(federated_url = %default_fed.url, yggdrasil_url = %default_ygg.url, "launcher config loaded from resources");

    let mut identity = IdentityManager::load_or_create(data_dir.join("identity")).await?;
    info!(peer_id = %identity.peer_id(), "identity loaded");

    let api_client =
        api::FederatedApiClient::new(default_fed.url.clone(), config.launcher_name.clone());

    let bootstrap_peers = config.bootstrap_peers.clone();
    info!(count = bootstrap_peers.len(), "bootstrap peers loaded from defaults");

    let keypair = identity.libp2p_keypair().map_err(|e| {
        tracing::error!(error = %e, "failed to derive libp2p keypair from identity");
        e
    })?;

    let api_auth = Arc::new(api_auth::ApiAuth::new(
        default_fed.url.clone(),
        keypair.clone(),
    ));

    // Configure CRL HTTP client before network start
    let crl_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("failed to build CRL HTTP client")?;
    identity.set_crl_http_client(crl_client);

    // Auto-refresh CRL and check for revoked VC
    let crl_api_base = default_fed.url.clone();
    if let Err(e) = identity.refresh_crl(&crl_api_base).await {
        tracing::warn!(error = %e, "initial CRL refresh failed, will retry in background");
    }
    identity.try_auto_clear_revoked_vc().await;

    let (_network, network_handle) = Network::start(keypair, bootstrap_peers).await?;

    let proxy = InstanceProxy::bind(network_handle.clone()).await?;
    let proxy_port = proxy.local_addr()?.port();
    info!(port = proxy_port, "proxy bound");

    let proxy_for_run = proxy.clone();
    tokio::spawn(async move { proxy_for_run.run().await });

    let mua_auth = MuaAuthService::new(default_ygg, api::RoomConfig::default());
    let resource_sync = ResourceSyncService::new(data_dir.join("resources"));
    let game_settings = GameSettings::load(&data_dir)?;

    let launch_states = Arc::new(Mutex::new(HashMap::<u64, LaunchingState>::new()));
    let java_runtimes = Arc::new(Mutex::new(Vec::<JavaRuntime>::new()));
    let task_groups = Arc::new(Mutex::new(HashMap::<String, TaskGroup>::new()));

    let app_state = Arc::new(Mutex::new(AppState {
        identity,
        proxy_port,
        proxy,
        network: network_handle,
        api: api_client,
        api_auth,
        mua_auth,
        resource_sync,
        game_settings,
        data_dir: data_dir.clone(),
    }));

    handle.manage(app_state.clone());
    handle.manage(launch_states);
    handle.manage(java_runtimes);
    handle.manage(task_groups);

    let notif = Arc::new(NotificationService::new());
    notif.start(handle.clone()).await;

    // Background tasks: periodic CRL refresh and VC expiry cleanup
    let app_state_bg = app_state.clone();
    let api_base_bg = default_fed.url.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(6 * 3600));
        loop {
            interval.tick().await;
            let mut state = app_state_bg.lock().await;

            // Refresh CRL
            if let Err(e) = state.identity.refresh_crl(&api_base_bg).await {
                tracing::warn!(error = %e, "background CRL refresh failed");
            } else {
                // Check if local VC was revoked
                state.identity.try_auto_clear_revoked_vc().await;
            }

            // Cleanup expired VCs (if expired > 1 day ago)
            let vc_state = state.identity.vc_status().state;
            if vc_state == VcHolderState::Expired {
                if let Err(e) = state.identity.clear_vc().await {
                    tracing::warn!(error = %e, "failed to clear expired VC");
                } else {
                    tracing::info!("cleared expired VC");
                }
            }
        }
    });

    Ok(())
}

fn project_data_dir() -> anyhow::Result<PathBuf> {
    let dirs = ProjectDirs::from("craft", "jlu", "FollyLauncher")
        .context("failed to determine project directories")?;
    Ok(dirs.data_dir().to_path_buf())
}

use tracing::info;
