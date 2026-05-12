mod account;
mod api;
mod control_client;
mod discover;
mod error;
mod identity;
mod instance;
mod java;
mod launch;
mod launcher_config;
mod modpack;
mod mua_auth;
mod network;
mod notification;
pub mod protos;
mod proxy;
mod resource;
mod resource_sync;
mod settings;
mod tasks;
mod utils;
mod workspace;

use anyhow::Context;
use directories::ProjectDirs;
use error::LauncherError;
use identity::{IdentityManager, OnboardingStatus, VcHolderState};
use java::models::JavaRuntime;
use launch::models::LaunchingState;
use mua_auth::{MuaAuthService, MuaLoginStatus};
use notification::NotificationService;
use proxy::InstanceProxy;
use resource::validator::{validate_version, ValidationSummary};
use resource_sync::{ResourceSyncService, SyncResult, SyncStatus};
use serde::Serialize;
use settings::GameSettings;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tasks::models::TaskGroup;
use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex;

pub struct AppState {
    identity: IdentityManager,
    proxy_port: u16,
    proxy: proxy::InstanceProxy,
    network: network::NetworkHandle,
    control: Arc<control_client::ControlClient>,
    launcher_name: String,
    mua_auth: MuaAuthService,
    resource_sync: ResourceSyncService,
    game_settings: GameSettings,
    data_dir: PathBuf,
    bootstrap_peers: Vec<String>,
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
async fn get_proxy_port(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ProxyInfo, LauncherError> {
    let state = state.lock().await;
    Ok(ProxyInfo {
        local_port: state.proxy_port,
    })
}

#[tauri::command]
async fn list_proxy_sessions(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<proxy::ProxySession>, LauncherError> {
    let state = state.lock().await;
    Ok(state.proxy.active_sessions().await)
}

#[tauri::command]
async fn list_tournaments(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<api::Tournament>, LauncherError> {
    let state = state.lock().await;
    state
        .control
        .list_tournaments()
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
async fn get_tournament(
    state: State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<Option<api::Tournament>, LauncherError> {
    let state = state.lock().await;
    state
        .control
        .get_tournament(&id)
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
async fn list_matches(
    state: State<'_, Arc<Mutex<AppState>>>,
    tournament_id: String,
) -> Result<Vec<api::Match>, LauncherError> {
    let state = state.lock().await;
    state
        .control
        .list_matches(&tournament_id)
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
async fn register_for_tournament(
    state: State<'_, Arc<Mutex<AppState>>>,
    tournament_id: String,
) -> Result<(), LauncherError> {
    let state = state.lock().await;
    let player_id = state.identity.peer_id().to_string();
    if player_id.is_empty() {
        return Err(LauncherError::new(
            "ERROR",
            "未找到玩家身份，请先完成身份初始化",
        ));
    }
    state
        .control
        .register_for_tournament(&tournament_id, &player_id)
        .await
        .map_err(LauncherError::from)
}



#[tauri::command]
async fn create_match_dispute(
    state: State<'_, Arc<Mutex<AppState>>>,
    tournament_id: String,
    match_id: String,
    reason: String,
    evidence_urls: Vec<String>,
) -> Result<api::DisputeMatch, LauncherError> {
    let state = state.lock().await;
    state
        .control
        .create_match_dispute(&tournament_id, &match_id, &reason, evidence_urls)
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
async fn list_disputes(
    state: State<'_, Arc<Mutex<AppState>>>,
    tournament_id: Option<String>,
) -> Result<Vec<api::DisputeMatch>, LauncherError> {
    let state = state.lock().await;
    state
        .control
        .list_disputes(tournament_id.as_deref())
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
async fn get_dispute(
    state: State<'_, Arc<Mutex<AppState>>>,
    dispute_id: String,
) -> Result<Option<api::DisputeMatch>, LauncherError> {
    let state = state.lock().await;
    state
        .control
        .get_dispute(&dispute_id)
        .await
        .map_err(LauncherError::from)
}







#[tauri::command]
async fn create_team(
    state: State<'_, Arc<Mutex<AppState>>>,
    name: String,
    members: Vec<String>,
) -> Result<(), LauncherError> {
    let state = state.lock().await;
    state
        .control
        .create_team(&name, members)
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
async fn list_teams(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<api::Team>, LauncherError> {
    let state = state.lock().await;
    state
        .control
        .list_teams()
        .await
        .map_err(LauncherError::from)
}

#[tauri::command]
async fn get_mua_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<MuaLoginStatus, LauncherError> {
    let state = state.lock().await;
    let account = state.identity.mua_account();
    let peer_bound = state.identity.serverless_token().is_some();
    let vc_state = state.identity.vc_status().await.state;
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
async fn get_onboarding_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<OnboardingStatus, LauncherError> {
    let (identity, vc_status, account, peer_bound, auth_server_url) = {
        let s = state.lock().await;
        let identity = s.identity.identity().clone();
        let vc_status = s.identity.vc_status().await;
        let account = s.identity.mua_account().cloned();
        let peer_bound = s.identity.serverless_token().is_some();
        let auth_server_url = s.mua_auth.auth_server_url().to_string();
        (identity, vc_status, account, peer_bound, auth_server_url)
    };
    let is_member = vc_status.state == VcHolderState::Member;
    let mua_status = MuaLoginStatus {
        logged_in: account.is_some(),
        username: account.as_ref().map(|a| a.username.clone()),
        uuid: account.as_ref().map(|a| a.uuid.clone()),
        auth_server_url,
        peer_bound,
        is_guest: account.is_some() && !is_member,
        is_member,
    };
    Ok(identity::build_onboarding_status(
        &identity,
        &vc_status,
        &mua_status,
    ))
}

#[tauri::command]
async fn start_mua_login(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<mua_auth::StartAuthResponse, LauncherError> {
    let state = state.lock().await;
    state
        .mua_auth
        .start_device_auth()
        .await
        .map_err(|e| LauncherError::from(e.to_string()))
}

#[tauri::command]
async fn poll_mua_login(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<MuaLoginStatus, LauncherError> {
    let state_arc = state.inner().clone();
    let mut state = state.lock().await;
    let account = state
        .mua_auth
        .poll_token()
        .await
        .map_err(|e| LauncherError::from(e.to_string()))?;

    state
        .identity
        .set_mua_account(account.clone())
        .await
        .map_err(|e| LauncherError::from(e.to_string()))?;


    let peer_id = state.identity.peer_id().to_string();
    let uuid = account.uuid.clone();
    let access_token = account.access_token.clone();
    let auth_server_url = account.auth_server_url.clone();

    drop(state);

    tokio::spawn(async move {
        let mut state = state_arc.lock().await;
        match state
            .control
            .bind_mua_peer(&access_token, &auth_server_url)
            .await
        {
            Ok(()) => {
                if let Err(e) = state
                    .identity
                    .set_serverless_token("bound".to_string())
                    .await
                {
                    tracing::warn!(error = %e, "failed to mark peer binding");
                } else {
                    tracing::info!(username = %uuid, %peer_id, "peer_id bound to Yggdrasil account");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "auto peer_id bind failed, user can retry manually");
            }
        }
    });

    Ok(MuaLoginStatus {
        logged_in: true,
        username: Some(account.username),
        uuid: Some(account.uuid),
        auth_server_url: account.auth_server_url,
        peer_bound: false,
        is_guest: true,
        is_member: false,
    })
}

#[tauri::command]
async fn logout_mua(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), LauncherError> {
    let mut state = state.lock().await;
    state
        .identity
        .clear_mua_account()
        .await
        .map_err(|e| LauncherError::from(e.to_string()))?;
    state
        .identity
        .clear_serverless_token()
        .await
        .map_err(|e| LauncherError::from(e.to_string()))
}

#[tauri::command]
async fn get_skin_textures(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<mua_auth::SkinTextures, LauncherError> {
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
        .map_err(|e| LauncherError::from(e.to_string()))
}

#[tauri::command]
async fn get_game_settings(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<GameSettings, LauncherError> {
    let state = state.lock().await;
    Ok(state.game_settings.clone())
}

#[tauri::command]
async fn update_game_settings(
    state: State<'_, Arc<Mutex<AppState>>>,
    mut settings: GameSettings,
) -> Result<(), LauncherError> {
    let mut state = state.lock().await;


    settings.last_instance_id = state.game_settings.last_instance_id.clone();
    settings.last_peer_id = state.game_settings.last_peer_id.clone();
    settings.last_connected_at = state.game_settings.last_connected_at.clone();
    settings.last_version = state.game_settings.last_version.clone();
    if let Err(e) = settings.save(&state.data_dir) {
        return Err(LauncherError::new(
            "SAVE_ERROR",
            format!("failed to save settings: {}", e),
        ));
    }
    state.game_settings = settings;
    Ok(())
}



async fn show_blocking_dialog<F>(
    app: tauri::AppHandle,
    pick: F,
) -> Result<Option<String>, LauncherError>
where
    F: FnOnce(tauri::AppHandle) -> Option<tauri_plugin_dialog::FilePath> + Send + 'static,
{
    tokio::task::spawn_blocking(move || Ok(pick(app).map(|p| p.to_string())))
        .await
        .map_err(|e| format!("dialog task panicked: {e}"))?
}

#[tauri::command]
async fn select_game_dir(app: tauri::AppHandle) -> Result<Option<String>, LauncherError> {
    show_blocking_dialog(app, |h| {
        use tauri_plugin_dialog::DialogExt;
        h.dialog().file().blocking_pick_folder()
    })
    .await
}

#[tauri::command]
async fn select_java_path(app: tauri::AppHandle) -> Result<Option<String>, LauncherError> {
    show_blocking_dialog(app, |h| {
        use tauri_plugin_dialog::DialogExt;
        h.dialog().file().blocking_pick_file()
    })
    .await
}

#[tauri::command]
async fn get_resource_sync_status(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
) -> Result<SyncStatus, LauncherError> {
    let state = state.lock().await;
    let manifest = state
        .control
        .get_instance_manifest(&instance_id)
        .await
        .map_err(LauncherError::from)?;

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
) -> Result<SyncResult, LauncherError> {
    let state = state.lock().await;
    let manifest = state
        .control
        .get_instance_manifest(&instance_id)
        .await
        .map_err(LauncherError::from)?;

    let (_cached, missing) = state.resource_sync.check_cache(&manifest);


    let result = state.resource_sync.sync_files(&missing, "").await;
    Ok(result)
}


#[tauri::command]
async fn probe_migration_health(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
    source_peer_id: String,
) -> Result<api::MigrationProbeResponse, LauncherError> {
    let state = state.lock().await;
    state
        .control
        .probe_migration(&instance_id, &source_peer_id)
        .await
        .map_err(LauncherError::from)
}





#[tauri::command]
async fn check_instance_eligibility(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
) -> Result<EligibilityResult, LauncherError> {
    let (vc_status, account, game_dir, control) = {
        let s = state.lock().await;
        let vc_status = s.identity.vc_status().await;
        let account = s.identity.mua_account().cloned();
        let game_dir = s.data_dir.join("minecraft");
        let control = s.control.clone();
        (vc_status, account, game_dir, control)
    };

    let logged_in = account.is_some();
    let is_member = vc_status.state == VcHolderState::Member;
    let user_club = vc_status.club.clone();
    let user_role = vc_status.role.clone();


    let (instance_resolved, instance_info) = tokio::join!(
        async {
            let s = state.lock().await;
            s.network.resolve_instance(instance_id.clone()).await
        },
        async {
            let instances = control
                .list_instances("", "", "")
                .await
                .map_err(LauncherError::from)?;
            Ok::<_, LauncherError>(instances.into_iter().find(|i| i.id == instance_id))
        },
    );
    let instance_resolved = instance_resolved?;
    let instance_info = instance_info?;
    let found = instance_resolved.is_some();


    let instance_club = instance_info.as_ref().and_then(|i| {
        if i.club.is_empty() {
            None
        } else {
            Some(i.club.clone())
        }
    });

    let admission_mode = api::AdmissionMode::normalize(
        &instance_info
            .as_ref()
            .ok_or_else(|| format!("实例 {instance_id} 缺少权威控制面元数据"))?
            .mode,
    );


    let (admission_blocked, admission_reason) = compute_admission_block(
        admission_mode.as_str(),
        is_member,
        user_club.as_deref(),
        instance_club.as_deref(),
    );


    let resources_missing: bool = {

        let versions_dir = game_dir.join("versions");
        if !versions_dir.exists() {
            true
        } else {
            let mut has_version = false;
            if let Ok(entries) = std::fs::read_dir(&versions_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        has_version = true;
                        break;
                    }
                }
            }
            !has_version
        }
    };


    let (available_memory_mb, memory_warning) = {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        let avail = sys.available_memory() / (1024 * 1024);
        (avail, avail < 2048)
    };

    let (available_disk_mb, disk_warning) = {
        let path = game_dir.clone();

        let disks = sysinfo::Disks::new_with_refreshed_list();
        let mut avail_disk = 0u64;
        for disk in disks.list() {
            let mount = disk.mount_point();
            if path.starts_with(mount) {
                avail_disk = disk.available_space() / (1024 * 1024);
                break;
            }
        }
        (avail_disk, avail_disk < 1024)
    };


    let (eligible, reason) = if !logged_in {
        (false, Some("请先在「我的」页面完成 MUA 登录".to_string()))
    } else if !found {
        (false, Some("实例未在 DHT 中找到，可能已下线".to_string()))
    } else if admission_blocked {
        (false, admission_reason)
    } else if resources_missing {
        (
            false,
            Some("游戏资源尚未安装，请先安装客户端版本".to_string()),
        )
    } else {
        (true, None)
    };

    let requires_vc = !is_member;

    Ok(EligibilityResult {
        instance_id,
        eligible,
        reason,
        admission_mode,
        requires_vc,
        user_club,
        user_role,
        resources_missing,
        available_memory_mb,
        available_disk_mb,
        memory_warning,
        disk_warning,
    })
}


#[tauri::command]
async fn validate_and_update_game(
    app_state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
    version: Option<String>,
) -> Result<ValidationSummary, LauncherError> {
    let (game_dir, game_version) = {
        let s = app_state.lock().await;
        let settings = s.game_settings.clone();
        let dir = std::path::PathBuf::from(&settings.game_directory);
        let ver = version.ok_or_else(|| "validate_and_update_game requires version".to_string())?;
        (dir, ver)
    };

    validate_version(&instance_id, &game_dir, &game_version)
        .map_err(|e| LauncherError::from(e.to_string()))
}


#[tauri::command]
async fn generate_launch_plan(
    app_state: State<'_, Arc<Mutex<AppState>>>,
    _instance_id: String,
    version: String,
    server_address: Option<String>,
    server_port: Option<u16>,
) -> Result<launch::command_gen::LaunchPlan, LauncherError> {
    let (game_dir, settings) = {
        let s = app_state.lock().await;
        (
            std::path::PathBuf::from(&s.game_settings.game_directory),
            s.game_settings.clone(),
        )
    };

    let options = launch::command_gen::LaunchOptions {
        username: Some("Player".to_string()),
        uuid: Some(uuid::Uuid::new_v4().to_string()),
        token: Some("0".to_string()),
        java_executable: settings.java_path.clone(),
        jvm_args: settings.build_jvm_args(),
        custom_jvm_flags: vec![],
        resolution_width: Some(settings.resolution_width),
        resolution_height: Some(settings.resolution_height),
        fullscreen: settings.fullscreen,
        server_address,
        server_port,
        quick_play: None,
        launcher_name: Some("FollyLauncher".to_string()),
    };

    launch::command_gen::generate_launch_plan(&version, &game_dir, &options)
        .map_err(|e| LauncherError::from(e.to_string()))
}










fn compute_admission_block(
    admission_mode: &str,
    is_member: bool,
    user_club: Option<&str>,
    instance_club: Option<&str>,
) -> (bool, Option<String>) {
    match admission_mode {
        "public" => (false, None),
        "vc_only" | "vc-only" => {
            if !is_member {
                (
                    true,
                    Some("该实例需要平台身份 (VC)，请导入有效的成员 VC 后再尝试".to_string()),
                )
            } else {
                (false, None)
            }
        }
        "club_only" | "club-only" => {
            if !is_member {
                (
                    true,
                    Some("该实例仅限社团成员加入，请先导入社团 VC".to_string()),
                )
            } else if let (Some(uc), Some(ic)) = (user_club, instance_club) {
                if !uc.eq_ignore_ascii_case(ic) {
                    (
                        true,
                        Some(format!(
                            "该实例属于社团「{}」，你当前的 VC 社团为「{}」，请使用匹配的社团 VC",
                            ic, uc
                        )),
                    )
                } else {
                    (false, None)
                }
            } else {
                (false, None)
            }
        }
        "mua_member" | "mua-member" => (false, None),
        _ => (
            true,
            Some(format!(
                "实例使用了未知的准入模式「{}」，请联系实例负责人确认",
                admission_mode
            )),
        ),
    }
}

#[derive(Serialize)]
struct EligibilityResult {
    instance_id: String,
    eligible: bool,
    reason: Option<String>,
    admission_mode: api::AdmissionMode,
    requires_vc: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    user_club: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    user_role: Option<String>,

    #[serde(default)]
    resources_missing: bool,

    #[serde(default)]
    available_memory_mb: u64,

    #[serde(default)]
    available_disk_mb: u64,

    #[serde(default)]
    memory_warning: bool,

    #[serde(default)]
    disk_warning: bool,
}

#[derive(Serialize)]
struct QuickRoomResult {
    id: String,
    name: String,
    kind: String,
    club: String,
    status: String,
    peer_id: Option<String>,
}

#[tauri::command]
async fn create_quick_room(
    state: State<'_, Arc<Mutex<AppState>>>,
    name: String,
    version: String,
    admission: Option<String>,
) -> Result<QuickRoomResult, LauncherError> {
    let state = state.lock().await;
    let owner = state.identity.peer_id().to_string();
    let room_config = state.mua_auth.room_config();
    let admission_mode = match admission.as_deref() {
        Some(raw) => api::AdmissionMode::normalize(raw),
        None => room_config.admission_mode,
    };
    let club = state.identity.identity().club.clone().unwrap_or_default();
    let image = format!("{}{}", room_config.image_prefix, version);

    let instance = state
        .control
        .create_instance(control_client::CreateInstanceArgs {
            name: &name,
            kind: "room",
            owner: &owner,
            club: &club,
            image: &image,
            cpu_cores: room_config.cpu_cores as u32,
            memory_gb: room_config.memory_gb,
            disk_gb: room_config.disk_gb,
            auto_restart: room_config.auto_restart,
            admission_mode: &admission_mode,
        })
        .await
        .map_err(LauncherError::from)?;

    Ok(QuickRoomResult {
        id: instance.id.clone(),
        name: instance.name,
        kind: instance.kind,
        club: instance.club,
        status: instance.status,
        peer_id: None,
    })
}


#[tauri::command]
async fn invite_players(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
    players: Vec<String>,
) -> Result<control_client::InvitePlayersResult, LauncherError> {
    let state = state.lock().await;

    let topic = format!(
        "{}{}",
        crate::network::MC_INSTANCE_TOPIC_PREFIX,
        instance_id
    );
    let result = state
        .control
        .invite_players(&instance_id, &topic, players)
        .await
        .map_err(LauncherError::from)?;

    tracing::info!(
        instance_id = %instance_id,
        player_count = result.invited_count,
        "players invited"
    );

    Ok(result)
}

#[derive(Serialize)]
struct BootstrapStatus {
    configured: bool,
    peer_count: usize,
    peers: Vec<String>,
    message: String,
}

#[tauri::command]
async fn get_bootstrap_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<BootstrapStatus, LauncherError> {
    let state = state.lock().await;
    let peers = state.bootstrap_peers.clone();
    let peer_count = peers.len();
    let configured = peer_count > 0;
    let message = if configured {
        format!("已配置 {} 个引导节点", peer_count)
    } else {
        "未配置引导节点：DHT 发现和 P2P 服务器列表可能不可用；将依赖本地缓存和直接连接".to_string()
    };
    Ok(BootstrapStatus {
        configured,
        peer_count,
        peers,
        message,
    })
}

#[derive(Serialize)]
struct LaunchInstanceResult {
    bridge_port: u16,
    pid: Option<u32>,
    instance_id: String,
    target_peer_id: String,
    username: String,
    uuid: String,
    member_vc: bool,
}





#[tauri::command]
async fn resume_last_instance(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<LaunchInstanceResult>, LauncherError> {
    let (last_id, last_ver) = {
        let s = state.lock().await;
        (
            s.game_settings.last_instance_id.clone(),
            s.game_settings.last_version.clone(),
        )
    };

    let (instance_id, version) = match (last_id, last_ver) {
        (Some(id), Some(ver)) => (id, ver),
        _ => return Ok(None),
    };

    tracing::info!(%instance_id, %version, "resuming last instance from persisted record");
    let result = do_launch_instance(&state, instance_id, version).await?;
    Ok(Some(result))
}


async fn do_launch_instance(
    state: &State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
    version: String,
) -> Result<LaunchInstanceResult, LauncherError> {
    tracing::info!(%instance_id, %version, "launching instance via dedicated proxy");


    let (account, resolved, peer_id, club, has_member_vc, settings, proxy, launcher_name, data_dir) = {
        let state_guard = state.lock().await;


        let identity = state_guard.identity.identity();
        let account = identity
            .mua_account
            .clone()
            .ok_or_else(|| "请先在「我的」页面完成 MUA 登录后再启动游戏".to_string())?;


        state_guard
            .game_settings
            .validate()
            .map_err(|e| LauncherError::from(e.to_string()))?;


        let resolved = state_guard
            .network
            .resolve_instance(instance_id.clone())
            .await?
            .ok_or_else(|| format!("实例 {instance_id} 未在 DHT 中找到"))?;

        let peer_id = identity.peer_id.clone();
        let club = identity.club.clone();
        let has_member_vc = state_guard.identity.vc_status().await.state == VcHolderState::Member;

        let settings = state_guard.game_settings.clone();
        let proxy = state_guard.proxy.clone();
        let launcher_name = state_guard.launcher_name.clone();
        let data_dir = state_guard.data_dir.clone();

        (
            account,
            resolved,
            peer_id,
            club,
            has_member_vc,
            settings,
            proxy,
            launcher_name,
            data_dir,
        )
    };

    tracing::info!(
        target_peer = %resolved.peer_id,
        proxy = ?resolved.proxy_address,
        username = %account.username,
        uuid = %account.uuid,
        "instance resolved"
    );


    let bridge_port = proxy
        .bridge_instance(proxy::BridgeConfig {
            instance_id: instance_id.clone(),
            peer_id,
            club,
            has_member_vc,
        })
        .await
        .map_err(|e| format!("创建实例桥接失败: {e}"))?;

    tracing::info!(%bridge_port, "dedicated proxy port ready for minecraft");

    let game_dir = std::path::PathBuf::from(&settings.game_directory);
    let jvm_args = settings.build_jvm_args();

    let options = crate::launch::minecraft_command::MinecraftOptions {
        username: Some(account.username.clone()),
        uuid: Some(account.uuid.clone()),
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
    };

    let command =
        crate::launch::minecraft_command::get_minecraft_command(&version, &game_dir, &options)
            .map_err(|e| format!("构建启动命令失败 (版本 {version} 可能未安装): {e}"))?;

    let log_output = if settings.show_game_log {
        crate::launch::process::LogOutput::Inherit
    } else {
        crate::launch::process::LogOutput::Null
    };
    let child = crate::launch::process::spawn_minecraft_process(&command, &game_dir, log_output)?;

    let pid = child.id();
    tracing::info!(pid = %pid.unwrap_or(0), %bridge_port, "minecraft process started via dedicated proxy");


    let now_iso = chrono::Utc::now().to_rfc3339();
    let mut persisted_settings = settings.clone();
    persisted_settings.last_instance_id = Some(instance_id.clone());
    persisted_settings.last_peer_id = Some(resolved.peer_id.clone());
    persisted_settings.last_connected_at = Some(now_iso);
    persisted_settings.last_version = Some(version.clone());
    if let Err(e) = persisted_settings.save(&data_dir) {
        tracing::warn!(error = %e, "failed to persist last-launch tracking");
    } else {

        let mut state_guard = state.lock().await;
        state_guard.game_settings = persisted_settings;
    }

    Ok(LaunchInstanceResult {
        bridge_port,
        pid,
        instance_id,
        target_peer_id: resolved.peer_id,
        username: account.username,
        uuid: account.uuid,
        member_vc: has_member_vc,
    })
}

#[tauri::command]
async fn launch_instance(
    state: State<'_, Arc<Mutex<AppState>>>,
    instance_id: String,
    version: String,
) -> Result<LaunchInstanceResult, LauncherError> {
    do_launch_instance(&state, instance_id, version).await
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
            identity::commands::get_identity,
            get_proxy_port,
            list_proxy_sessions,
            network::commands::list_peers,
            network::commands::resolve_instance,
            network::commands::list_instances,
            network::commands::measure_latency,
            launch_instance,
            resume_last_instance,
            identity::commands::get_vc_status,
            identity::commands::import_vc,
            identity::commands::clear_vc,
            network::commands::get_network_diagnostics,
            list_tournaments,
            get_tournament,
            list_matches,
            register_for_tournament,
            create_team,
            list_teams,
            get_mua_status,
            get_onboarding_status,
            start_mua_login,
            poll_mua_login,
            logout_mua,
            get_skin_textures,
            get_game_settings,
            update_game_settings,
            select_game_dir,
            select_java_path,
            get_resource_sync_status,
            sync_resources,
            get_bootstrap_status,

            check_instance_eligibility,
            create_quick_room,

            invite_players,

            probe_migration_health,

            create_match_dispute,
            list_disputes,
            get_dispute,

            launch::commands::launch_local_instance,
            launch::commands::launch_cancel,
            launch::commands::launch_get_state,
            launch::commands::launch_list_states,
            launch::commands::launch_export_crash,

            java::commands::retrieve_java_list,
            java::commands::validate_java,

            tasks::commands::get_task_group,
            tasks::commands::list_task_groups,
            tasks::commands::cancel_task_group,
            tasks::commands::remove_task_group,
            tasks::commands::export_task_snapshot,
            tasks::commands::import_task_snapshot,

            workspace::commands::retrieve_world_list,
            workspace::commands::retrieve_screenshot_list,
            workspace::commands::retrieve_resource_pack_list,
            workspace::commands::retrieve_shader_pack_list,
            workspace::commands::retrieve_instance_workspace,
            workspace::commands::set_mod_enabled,
            workspace::commands::delete_mod_file,

            resource::commands::fetch_game_version_list,
            resource::commands::fetch_mod_loader_version_list,
            resource::commands::fetch_optifine_version_list,
            resource::commands::fetch_resource_list_by_name,
            resource::commands::fetch_resource_version_packs,
            resource::commands::fetch_remote_resource_by_local,
            resource::commands::fetch_remote_resource_by_id,
            resource::commands::download_game_server,
            resource::commands::update_mods,
            resource::commands::install_resource_to_instance,
            resource::commands::install_client_version_for_instance,
            resource::commands::start_install_client_version_task,
            resource::commands::install_libraries_for_instance,
            resource::commands::start_install_libraries_task,
            resource::commands::install_assets_for_instance,
            resource::commands::start_install_assets_task,
            resource::commands::install_loader_for_instance,
            resource::commands::start_install_loader_task,
            resource::commands::start_install_resource_task,

            discover::commands::fetch_news_sources_info,
            discover::commands::fetch_news_post_summaries,

            instance::commands::list_local_instances,
            instance::commands::create_local_instance,
            instance::commands::update_local_instance,
            instance::commands::delete_local_instance,

            account::commands::list_launcher_accounts,
            account::commands::add_offline_account,
            account::commands::select_launcher_account,
            account::commands::delete_launcher_account,
            account::commands::add_third_party_account,

            account::commands::start_microsoft_login,
            account::commands::poll_microsoft_login,

            account::commands::refresh_microsoft_account,

            account::commands::update_account_avatar,
            account::commands::refresh_account_avatar,

            account::commands::export_launcher_accounts,
            account::commands::import_launcher_accounts,

            account::commands::import_external_accounts,

            launcher_config::commands::retrieve_launcher_config,
            launcher_config::commands::update_launcher_config,

            modpack::commands::export_modpack_manifest,
            modpack::commands::import_modpack_manifest,

            modpack::commands::export_modpack_zip,
            modpack::commands::import_modpack_zip,

            validate_and_update_game,
            generate_launch_plan,
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
    let default_fed = config
        .federated_servers
        .iter()
        .find(|s| s.default)
        .ok_or_else(|| anyhow::anyhow!("no federated server configured in defaults.toml"))?;
    let default_ygg = config
        .yggdrasil_servers
        .iter()
        .find(|s| s.default)
        .ok_or_else(|| anyhow::anyhow!("no yggdrasil server configured in defaults.toml"))?;
    info!(federated_url = %default_fed.url, yggdrasil_url = %default_ygg.url, "launcher config loaded from resources");

    let identity = IdentityManager::load_or_create(data_dir.join("identity")).await?;
    let peer_id = identity.peer_id().to_string();
    let launcher_name = config.launcher_name.clone();
    info!(%peer_id, "identity loaded");

    let bootstrap_peers = config.bootstrap_peers.clone();
    info!(
        count = bootstrap_peers.len(),
        "bootstrap peers loaded from defaults"
    );

    let keypair = identity.libp2p_keypair().map_err(|e| {
        tracing::error!(error = %e, "failed to derive libp2p keypair from identity");
        e
    })?;

    let network_handle = network::start(keypair.clone(), bootstrap_peers.clone()).await?;

    let control_client = Arc::new(control_client::ControlClient::new(
        network_handle.clone(),
        keypair.clone(),
        peer_id,
    ));

    let proxy = InstanceProxy::bind(network_handle.clone(), control_client.clone()).await?;
    let proxy_port = proxy.local_addr()?.port();
    info!(port = proxy_port, "proxy bound");

    let proxy_for_run = proxy.clone();
    tokio::spawn(async move { proxy_for_run.run().await });

    let mua_auth = MuaAuthService::new(default_ygg, api::RoomConfig::default());
    let mut resource_sync = ResourceSyncService::new(data_dir.join("resources"));
    resource_sync.set_network(network_handle.clone());
    let game_settings = GameSettings::load_or_default(&data_dir)?;

    let launch_states = Arc::new(Mutex::new(HashMap::<u64, LaunchingState>::new()));
    let java_runtimes = Arc::new(Mutex::new(Vec::<JavaRuntime>::new()));
    let task_groups = Arc::new(Mutex::new(HashMap::<String, TaskGroup>::new()));

    let app_state = Arc::new(Mutex::new(AppState {
        identity,
        proxy_port,
        proxy,
        network: network_handle,
        control: control_client,
        launcher_name,
        mua_auth,
        resource_sync,
        game_settings,
        data_dir: data_dir.clone(),
        bootstrap_peers,
    }));

    handle.manage(app_state.clone());
    handle.manage(launch_states);
    handle.manage(java_runtimes);
    handle.manage(task_groups);

    let notif = Arc::new(NotificationService::new());
    {
        let control = {
            let state = app_state.lock().await;
            state.control.clone()
        };
        let handle_for_events = handle.clone();
        let notif_for_events = notif.clone();
        tokio::spawn(async move {
            match control.subscribe_events_stream(Vec::new()).await {
                Ok(mut events) => {
                    while let Some(event) = events.recv().await {
                        if event.event_type.starts_with("instance-")
                            || event.topic.starts_with(network::MC_INSTANCE_TOPIC_PREFIX)
                        {
                            if let Err(e) = handle_for_events.emit("instances-changed", ()) {
                                tracing::warn!(error = %e, "failed to emit instances-changed event");
                            }
                        }
                        notif_for_events
                            .handle_event(&handle_for_events, event)
                            .await;
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "subscribe_events background task failed to start");
                }
            }
        });
    }


    let app_state_bg = app_state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(6 * 3600));
        loop {
            interval.tick().await;
            let mut state = app_state_bg.lock().await;


            match state.control.list_revoked_credentials().await {
                Ok(ids) => {
                    if let Err(e) = state.identity.update_crl_entries(ids).await {
                        tracing::warn!(error = %e, "failed to update CRL entries");
                    } else {
                        state.identity.try_auto_clear_revoked_vc().await;
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "background CRL refresh failed");
                }
            }


            let vc_state = state.identity.vc_status().await.state;
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





#[cfg(test)]
mod type_contract_tests {
    use super::*;
    use serde_json::json;



    #[test]
    fn test_eligibility_result_contract() {
        let result = EligibilityResult {
            instance_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            eligible: true,
            reason: None,
            admission_mode: api::AdmissionMode::ClubOnly,
            requires_vc: false,
            user_club: Some("builders".to_string()),
            user_role: Some("member".to_string()),
            resources_missing: false,
            available_memory_mb: 8192,
            available_disk_mb: 51200,
            memory_warning: false,
            disk_warning: false,
        };
        let json = serde_json::to_value(&result).unwrap();


        assert_eq!(json["instance_id"], "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(json["eligible"], true);
        assert_eq!(json["reason"], json!(null));
        assert_eq!(json["admission_mode"], "club-only");
        assert_eq!(json["requires_vc"], false);
        assert_eq!(json["user_club"], "builders");
        assert_eq!(json["user_role"], "member");
        assert_eq!(json["resources_missing"], false);
        assert_eq!(json["available_memory_mb"], 8192);
        assert_eq!(json["available_disk_mb"], 51200);
        assert_eq!(json["memory_warning"], false);
        assert_eq!(json["disk_warning"], false);


        let result_no_club = EligibilityResult {
            instance_id: "test".to_string(),
            eligible: false,
            reason: Some("blocked".to_string()),
            admission_mode: api::AdmissionMode::Public,
            requires_vc: true,
            user_club: None,
            user_role: None,
            resources_missing: false,
            available_memory_mb: 0,
            available_disk_mb: 0,
            memory_warning: false,
            disk_warning: false,
        };
        let json_no_club = serde_json::to_value(&result_no_club).unwrap();
        assert!(
            !json_no_club.as_object().unwrap().contains_key("user_club"),
            "user_club should be absent when None (skip_serializing_if)"
        );
    }


    #[test]
    fn test_invite_result_contract() {
        let result = control_client::InvitePlayersResult {
            instance_id: "inst-001".to_string(),
            invited_count: 3,
            missing_recipients: vec!["offline-a".to_string()],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["instance_id"], "inst-001");
        assert_eq!(json["invited_count"], 3);
        assert_eq!(json["missing_recipients"], json!(["offline-a"]));
    }


    #[test]
    fn test_bootstrap_status_contract() {
        let status = BootstrapStatus {
            configured: true,
            peer_count: 3,
            peers: vec![
                "/dns4/peer1.example.com/tcp/443/quic-v1/p2p/12D3KooWA".to_string(),
                "/dns4/peer2.example.com/tcp/443/quic-v1/p2p/12D3KooWB".to_string(),
                "/dns4/peer3.example.com/tcp/443/quic-v1/p2p/12D3KooWC".to_string(),
            ],
            message: "已配置 3 个引导节点".to_string(),
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["configured"], true);
        assert_eq!(json["peer_count"], 3);
        assert!(json["peers"].is_array());
        assert_eq!(json["peers"].as_array().unwrap().len(), 3);


        let empty = BootstrapStatus {
            configured: false,
            peer_count: 0,
            peers: vec![],
            message: "未配置引导节点".to_string(),
        };
        let empty_json = serde_json::to_value(&empty).unwrap();
        assert_eq!(empty_json["configured"], false);
        assert_eq!(empty_json["peer_count"], 0);
        assert!(empty_json["peers"].as_array().unwrap().is_empty());
    }


    #[test]
    fn test_launch_result_contract() {
        let result = LaunchInstanceResult {
            bridge_port: 25566,
            pid: Some(12345),
            instance_id: "inst-abc".to_string(),
            target_peer_id: "12D3KooWTarget".to_string(),
            username: "Steve".to_string(),
            uuid: "abcdef1234567890abcdef1234567890".to_string(),
            member_vc: true,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["bridge_port"], 25566);
        assert_eq!(json["pid"], 12345);
        assert_eq!(json["instance_id"], "inst-abc");
        assert_eq!(json["username"], "Steve");
        assert_eq!(json["uuid"], "abcdef1234567890abcdef1234567890");
        assert_eq!(json["member_vc"], true);


        let result_no_pid = LaunchInstanceResult {
            bridge_port: 25567,
            pid: None,
            instance_id: "inst-def".to_string(),
            target_peer_id: "12D3KooWTarget2".to_string(),
            username: "Alex".to_string(),
            uuid: "fedcba0987654321fedcba0987654321".to_string(),
            member_vc: false,
        };
        let json_no_pid = serde_json::to_value(&result_no_pid).unwrap();
        assert_eq!(json_no_pid["pid"], json!(null));
    }


    #[test]
    fn test_quick_room_contract() {
        let result = QuickRoomResult {
            id: "room-uuid".to_string(),
            name: "My Room".to_string(),
            kind: "room".to_string(),
            club: "builders".to_string(),
            status: "created".to_string(),
            peer_id: Some("12D3KooWHost".to_string()),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["id"], "room-uuid");
        assert_eq!(json["name"], "My Room");
        assert_eq!(json["kind"], "room");
        assert_eq!(json["club"], "builders");
        assert_eq!(json["status"], "created");
        assert_eq!(json["peer_id"], "12D3KooWHost");
    }




    #[test]
    fn admission_public_always_allows() {
        let (blocked, reason) = compute_admission_block("public", false, None, None);
        assert!(!blocked);
        assert!(reason.is_none());


        let (blocked2, reason2) =
            compute_admission_block("public", true, Some("builders"), Some("builders"));
        assert!(!blocked2);
        assert!(reason2.is_none());
    }


    #[test]
    fn admission_vc_only_blocks_guest() {
        let (blocked, reason) = compute_admission_block("vc_only", false, None, None);
        assert!(blocked);
        assert!(reason.unwrap().contains("VC"));


        let (blocked2, _) = compute_admission_block("vc-only", false, None, None);
        assert!(blocked2);
    }


    #[test]
    fn admission_vc_only_allows_member() {
        let (blocked, reason) = compute_admission_block("vc_only", true, None, None);
        assert!(!blocked);
        assert!(reason.is_none());
    }


    #[test]
    fn admission_club_only_blocks_guest() {
        let (blocked, reason) = compute_admission_block("club_only", false, None, None);
        assert!(blocked);
        assert!(reason.unwrap().contains("社团"));
    }


    #[test]
    fn admission_club_only_allows_matching_club() {
        let (blocked, reason) =
            compute_admission_block("club_only", true, Some("builders"), Some("builders"));
        assert!(!blocked);
        assert!(reason.is_none());
    }


    #[test]
    fn admission_club_only_blocks_mismatched_club() {
        let (blocked, reason) =
            compute_admission_block("club_only", true, Some("redstone"), Some("builders"));
        assert!(blocked);
        let msg = reason.unwrap();
        assert!(msg.contains("builders"));
        assert!(msg.contains("redstone"));
    }


    #[test]
    fn admission_club_only_case_insensitive() {
        let (blocked, _) =
            compute_admission_block("club-only", true, Some("Builders"), Some("BUILDERS"));
        assert!(!blocked);
    }


    #[test]
    fn admission_mua_member_no_block() {
        let (blocked, reason) = compute_admission_block("mua_member", false, None, None);
        assert!(!blocked);
        assert!(reason.is_none());
    }


    #[test]
    fn admission_mua_member_alias() {
        let (blocked, reason) = compute_admission_block("mua-member", false, None, None);
        assert!(!blocked);
        assert!(reason.is_none());
    }


    #[test]
    fn admission_unknown_mode_blocks() {
        let (blocked, reason) = compute_admission_block("invite_only", true, None, None);
        assert!(blocked);
        assert!(reason.unwrap().contains("invite_only"));

        let (blocked2, reason2) = compute_admission_block("", false, None, None);
        assert!(blocked2);
        assert!(reason2.unwrap().contains("未知"));
    }




    #[test]
    fn admission_unknown_explicit_blocks() {
        let (blocked, reason) = compute_admission_block("unknown", true, None, None);
        assert!(blocked);
        assert!(reason.unwrap().contains("unknown"));
    }




    #[test]
    fn admission_club_only_empty_instance_club_not_mismatched() {


        let (blocked, reason) = compute_admission_block("club_only", true, Some("builders"), None);

        assert!(!blocked);
        assert!(reason.is_none());
    }



    #[test]
    fn admission_club_only_non_member_empty_instance_club() {
        let (blocked, reason) = compute_admission_block("club_only", false, None, None);
        assert!(blocked);
        let msg = reason.unwrap();
        assert!(
            msg.contains("社团"),
            "should block for club-only membership requirement"
        );
    }




    #[test]
    fn test_download_dedup_applied_in_place() {
        use crate::resource::downloader::DownloadManager;
        use crate::resource::downloader::DownloadTask;

        let mut tasks = vec![
            DownloadTask {
                id: "t1".into(),
                name: "a.jar".into(),
                urls: vec!["http://a/a.jar".into()],
                dest_path: std::path::PathBuf::from("/tmp/x/a.jar"),
                expected_sha1: None,
                expected_sha256: None,
                expected_size: None,
                hash_algorithm: Default::default(),
            },
            DownloadTask {
                id: "t2".into(),
                name: "a.jar".into(),
                urls: vec!["http://b/a.jar".into()],
                dest_path: std::path::PathBuf::from("/tmp/x/a.jar"),
                expected_sha1: None,
                expected_sha256: None,
                expected_size: None,
                hash_algorithm: Default::default(),
            },
            DownloadTask {
                id: "t3".into(),
                name: "b.jar".into(),
                urls: vec!["http://a/b.jar".into()],
                dest_path: std::path::PathBuf::from("/tmp/x/b.jar"),
                expected_sha1: None,
                expected_sha256: None,
                expected_size: None,
                hash_algorithm: Default::default(),
            },
        ];

        DownloadManager::dedup(&mut tasks);
        assert_eq!(tasks.len(), 2, "duplicate a.jar should be removed");
        assert_eq!(tasks[0].dest_path, std::path::PathBuf::from("/tmp/x/a.jar"));
        assert_eq!(tasks[1].dest_path, std::path::PathBuf::from("/tmp/x/b.jar"));
    }
}
