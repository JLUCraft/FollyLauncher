use crate::error::LauncherError;
use crate::instance::commands::{get_instance_in, update_instance_loader_in};
use crate::instance::models::LocalInstanceKind;
use crate::resource::curseforge::{
    fetch_remote_resource_by_id_curseforge, fetch_remote_resource_by_local_curseforge,
    fetch_resource_list_by_name_curseforge, fetch_resource_version_packs_curseforge,
};
use crate::resource::loader_meta::{
    get_fabric_meta_by_game_version, get_forge_meta_by_game_version,
    get_neoforge_meta_by_game_version, get_optifine_meta_by_game_version,
    get_quilt_meta_by_game_version,
};
use crate::resource::misc::{get_source_priority_list, get_use_mirror, get_download_api};
use crate::resource::modrinth::{
    fetch_remote_resource_by_id_modrinth, fetch_remote_resource_by_local_modrinth,
    fetch_resource_list_by_name_modrinth, fetch_resource_version_packs_modrinth,
};
use crate::resource::models::{
    AsyncInstallAssetsRequest, AsyncInstallAssetsStarted, AsyncInstallClientVersionRequest,
    AsyncInstallLibrariesRequest, AsyncInstallLibrariesStarted, AsyncInstallLoaderRequest,
    AsyncInstallLoaderStarted, AsyncInstallResourceRequest, AsyncInstallResourceStarted,
    AsyncInstallTaskStarted, GameClientResourceInfo, InstallAssetsRequest,
    InstallAssetsResult, InstallClientVersionRequest, InstallClientVersionResult,
    InstallLibrariesRequest, InstallLibrariesResult, InstallLoaderKind, InstallLoaderRequest,
    InstallLoaderResult, InstallResourceKind, InstallResourceRequest, InstallResourceResult,
    ModLoaderResourceInfo, ModLoaderType, ModUpdateQuery,
    OptiFineResourceInfo, OtherResourceFileInfo, OtherResourceInfo,
    OtherResourceSearchQuery, OtherResourceSearchRes, OtherResourceSource,
    OtherResourceVersionPack, OtherResourceVersionPackQuery, ResourceDependencySummary,
    ResourceType, SourceType,
};
use crate::resource::validator::{library_artifact_path, parse_version_json};
use crate::resource::version_manifest::get_game_version_manifest;
use crate::tasks::commands::{build_running_task_group, record_running_task_group, update_single_task_group};
use crate::tasks::models::{TaskStatus};

use crate::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;
use tracing::warn;

#[tauri::command]
pub async fn fetch_game_version_list(app: AppHandle) -> Result<Vec<GameClientResourceInfo>, LauncherError> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let priority_list = get_source_priority_list(get_use_mirror(&app));
    get_game_version_manifest(&app, &client, &priority_list)
        .await
        .map_err(|e| LauncherError::from(e.to_string()))
}

#[tauri::command]
pub async fn fetch_mod_loader_version_list(
    app: AppHandle,
    game_version: String,
    mod_loader_type: ModLoaderType,
) -> Result<Vec<ModLoaderResourceInfo>, LauncherError> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let priority_list = get_source_priority_list(get_use_mirror(&app));

    match mod_loader_type {
        ModLoaderType::Forge | ModLoaderType::LegacyForge => {
            get_forge_meta_by_game_version(&app, &client, &priority_list, &game_version)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        ModLoaderType::Fabric => {
            get_fabric_meta_by_game_version(&app, &client, &priority_list, &game_version)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        ModLoaderType::NeoForge => {
            get_neoforge_meta_by_game_version(&app, &client, &priority_list, &game_version)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        ModLoaderType::Quilt => {
            get_quilt_meta_by_game_version(&app, &client, &priority_list, &game_version)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        _ => Err(LauncherError::from("Mod loader not supported for version listing")),
    }
}

#[tauri::command]
pub async fn fetch_optifine_version_list(
    app: AppHandle,
    game_version: String,
) -> Result<Vec<OptiFineResourceInfo>, LauncherError> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let priority_list = get_source_priority_list(get_use_mirror(&app));
    get_optifine_meta_by_game_version(&app, &client, &priority_list, &game_version)
        .await
        .map_err(|e| LauncherError::from(e.to_string()))
}

#[tauri::command]
pub async fn fetch_resource_list_by_name(
    app: AppHandle,
    download_source: OtherResourceSource,
    query: OtherResourceSearchQuery,
) -> Result<OtherResourceSearchRes, LauncherError> {
    match download_source {
        OtherResourceSource::CurseForge => {
            fetch_resource_list_by_name_curseforge(&app, &query)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        OtherResourceSource::Modrinth => {
            fetch_resource_list_by_name_modrinth(&app, &query)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        _ => Err(LauncherError::from("Unsupported download source")),
    }
}

#[tauri::command]
pub async fn fetch_resource_version_packs(
    app: AppHandle,
    download_source: OtherResourceSource,
    query: OtherResourceVersionPackQuery,
) -> Result<Vec<OtherResourceVersionPack>, LauncherError> {
    match download_source {
        OtherResourceSource::CurseForge => {
            fetch_resource_version_packs_curseforge(&app, &query)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        OtherResourceSource::Modrinth => {
            fetch_resource_version_packs_modrinth(&app, &query)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        _ => Err(LauncherError::from("Unsupported download source")),
    }
}

#[tauri::command]
pub async fn fetch_remote_resource_by_local(
    app: AppHandle,
    download_source: OtherResourceSource,
    file_path: String,
) -> Result<OtherResourceFileInfo, LauncherError> {
    match download_source {
        OtherResourceSource::CurseForge => {
            fetch_remote_resource_by_local_curseforge(&app, &file_path)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        OtherResourceSource::Modrinth => {
            fetch_remote_resource_by_local_modrinth(&app, &file_path)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        _ => Err(LauncherError::from("Unsupported download source")),
    }
}

#[tauri::command]
pub async fn fetch_remote_resource_by_id(
    app: AppHandle,
    download_source: OtherResourceSource,
    resource_id: String,
) -> Result<OtherResourceInfo, LauncherError> {
    match download_source {
        OtherResourceSource::CurseForge => {
            fetch_remote_resource_by_id_curseforge(&app, &resource_id)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        OtherResourceSource::Modrinth => {
            fetch_remote_resource_by_id_modrinth(&app, &resource_id)
                .await
                .map_err(|e| LauncherError::from(e.to_string()))
        }
        _ => Err(LauncherError::from("Unsupported download source")),
    }
}

#[tauri::command]
pub async fn download_game_server(
    app: AppHandle,
    resource_info: GameClientResourceInfo,
    dest: String,
) -> Result<(), LauncherError> {
    let client = app.state::<reqwest::Client>().inner().clone();

    let version_details: serde_json::Value = client
        .get(&resource_info.url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let download_info = version_details
        .get("downloads")
        .and_then(|d| d.get("server"))
        .ok_or("Server download info not found")?;

    let download_url = download_info
        .get("url")
        .and_then(|u| u.as_str())
        .ok_or("Download URL not found")?;

    let sha1 = download_info
        .get("sha1")
        .and_then(|s| s.as_str())
        .unwrap_or("");

    let response = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read failed: {}", e))?;

    if !sha1.is_empty() {
        let actual = hex::encode(sha1_smol::Sha1::from(&bytes).digest().bytes());
        if actual != sha1 {
            return Err(LauncherError::from(format!("SHA1 mismatch: expected {}, got {}", sha1, actual)));
        }
    }

    std::fs::write(&dest, &bytes).map_err(|e| format!("Write failed: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn update_mods(
    app: AppHandle,
    _instance_id: String,
    queries: Vec<ModUpdateQuery>,
) -> Result<(), LauncherError> {
    let client = app.state::<reqwest::Client>().inner().clone();

    let tasks = queries.iter().map(|query| {
        let client = client.clone();
        async move {
            let response = client
                .get(&query.url)
                .send()
                .await
                .map_err(|e| format!("Download failed: {}", e))?;

            let bytes = response
                .bytes()
                .await
                .map_err(|e| format!("Read failed: {}", e))?;

            if !query.sha1.is_empty() {
                let actual = hex::encode(sha1_smol::Sha1::from(&bytes).digest().bytes());
                if actual != query.sha1 {
                    return Err(LauncherError::from(format!("SHA1 mismatch for {}: expected {}, got {}", query.file_name, query.sha1, actual)));
                }
            }

            let dest_path = std::path::Path::new(&query.old_file_path)
                .parent()
                .map(|p| p.join(&query.file_name))
                .unwrap_or_else(|| std::path::PathBuf::from(&query.file_name));

            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("Mkdir failed: {}", e))?;
            }

            std::fs::write(&dest_path, &bytes).map_err(|e| format!("Write failed: {}", e))?;

            if query.old_file_path != dest_path.to_string_lossy() {
                let old_backup = format!("{}.old", query.old_file_path);
                if let Err(e) = std::fs::rename(&query.old_file_path, &old_backup) {
                    warn!("Failed to rename old mod file {}: {}", query.old_file_path, e);
                }
            }

            Ok(())
        }
    });

    futures::future::try_join_all(tasks).await?;
    Ok(())
}

// ── Helpers for deriving install paths from an instance ─────────────────

fn versions_dir(game_dir: &str) -> PathBuf {
    PathBuf::from(game_dir).join("versions")
}

fn libraries_dir(game_dir: &str) -> PathBuf {
    PathBuf::from(game_dir).join("libraries")
}

fn assets_dir(game_dir: &str) -> PathBuf {
    PathBuf::from(game_dir).join("assets")
}

fn resource_subdir(kind: InstallResourceKind) -> &'static str {
    match kind {
        InstallResourceKind::Mod => "mods",
        InstallResourceKind::ResourcePack => "resourcepacks",
        InstallResourceKind::ShaderPack => "shaderpacks",
    }
}

fn data_dir(app: &AppHandle) -> PathBuf {
    app.state::<Arc<Mutex<AppState>>>()
        .blocking_lock()
        .data_dir
        .clone()
}

// ── Phase 15: Resource (mod / resource pack / shader pack) install ──────

#[tauri::command]
pub async fn install_resource_to_instance(
    app: AppHandle,
    request: InstallResourceRequest,
) -> Result<InstallResourceResult, LauncherError> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let dd = data_dir(&app);
    let instance = get_instance_in(&dd, &request.instance_id)?;
    let game_dir = PathBuf::from(&instance.game_dir);

    let dest = game_dir
        .join(resource_subdir(request.kind))
        .join(&request.file.file_name);
    let replaced_existing = dest.exists();

    if replaced_existing && !request.overwrite {
        return Err(LauncherError::from(format!(
            "文件 {} 已存在，且未启用覆盖",
            dest.display()
        )));
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| LauncherError::from(format!("Mkdir failed: {}", e)))?;
    }

    let response = client
        .get(&request.file.download_url)
        .send()
        .await
        .map_err(|e| LauncherError::from(format!("Download failed: {}", e)))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| LauncherError::from(format!("Read failed: {}", e)))?;

    let mut sha1_verified = false;
    if !request.file.sha1.is_empty() {
        let actual = hex::encode(sha1_smol::Sha1::from(&bytes).digest().bytes());
        if actual != request.file.sha1 {
            return Err(LauncherError::from(format!(
                "SHA1 mismatch: expected {}, got {}",
                request.file.sha1, actual
            )));
        }
        sha1_verified = true;
    }

    let bytes_written = bytes.len() as u64;
    std::fs::write(&dest, &bytes)
        .map_err(|e| LauncherError::from(format!("Write failed: {}", e)))?;

    Ok(InstallResourceResult {
        instance_id: request.instance_id.clone(),
        dest_path: dest.to_string_lossy().to_string(),
        file_name: request.file.file_name.clone(),
        bytes_written,
        sha1_verified,
        replaced_existing,
        dependency_summary: ResourceDependencySummary {
            required: 0,
            optional: 0,
            embedded: 0,
            other: 0,
            items: Vec::new(),
        },
    })
}

// ── Phase 31: Client version install ────────────────────────────────────

#[tauri::command]
pub async fn install_client_version_for_instance(
    app: AppHandle,
    request: InstallClientVersionRequest,
) -> Result<InstallClientVersionResult, LauncherError> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let dd = data_dir(&app);
    let instance = get_instance_in(&dd, &request.instance_id)?;
    let game_version = &instance.game_version;

    let priority_list = get_source_priority_list(get_use_mirror(&app));
    let versions = get_game_version_manifest(&app, &client, &priority_list)
        .await
        .map_err(|e| LauncherError::from(e.to_string()))?;

    let version_info = versions
        .iter()
        .find(|v| &v.id == game_version)
        .ok_or_else(|| {
            LauncherError::from(format!("Game version {} not found", game_version))
        })?;

    let version_json: serde_json::Value = client
        .get(&version_info.url)
        .send()
        .await
        .map_err(|e| LauncherError::from(format!("Failed to fetch version JSON: {}", e)))?
        .json()
        .await
        .map_err(|e| LauncherError::from(format!("Failed to parse version JSON: {}", e)))?;

    let version_dir = versions_dir(&instance.game_dir).join(game_version);
    std::fs::create_dir_all(&version_dir)
        .map_err(|e| LauncherError::from(format!("Mkdir failed: {}", e)))?;

    let json_path = version_dir.join(format!("{}.json", game_version));
    let json_str = serde_json::to_string_pretty(&version_json)
        .map_err(|e| LauncherError::from(format!("Serialize failed: {}", e)))?;
    let json_bytes_written = json_str.len() as u64;
    std::fs::write(&json_path, &json_str)
        .map_err(|e| LauncherError::from(format!("Write failed: {}", e)))?;

    let client_jar_url = version_json
        .get("downloads")
        .and_then(|d| d.get("client"))
        .and_then(|c| c.get("url"))
        .and_then(|u| u.as_str())
        .ok_or_else(|| LauncherError::from("Client download URL not found"))?;

    let client_jar_sha1 = version_json
        .get("downloads")
        .and_then(|d| d.get("client"))
        .and_then(|c| c.get("sha1"))
        .and_then(|s| s.as_str())
        .unwrap_or("");

    let client_jar_path = version_dir.join(format!("{}.jar", game_version));
    let replaced_existing = client_jar_path.exists();
    if replaced_existing && !request.overwrite {
        return Err(LauncherError::from("Client JAR already exists and overwrite is disabled"));
    }

    let response = client
        .get(client_jar_url)
        .send()
        .await
        .map_err(|e| LauncherError::from(format!("JAR download failed: {}", e)))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| LauncherError::from(format!("Read failed: {}", e)))?;

    let mut jar_sha1_verified = false;
    if !client_jar_sha1.is_empty() {
        let actual = hex::encode(sha1_smol::Sha1::from(&bytes).digest().bytes());
        if actual != client_jar_sha1 {
            return Err(LauncherError::from(format!(
                "Client JAR SHA1 mismatch: expected {}, got {}",
                client_jar_sha1, actual
            )));
        }
        jar_sha1_verified = true;
    }

    let jar_bytes_written = bytes.len() as u64;
    std::fs::write(&client_jar_path, &bytes)
        .map_err(|e| LauncherError::from(format!("Write failed: {}", e)))?;

    Ok(InstallClientVersionResult {
        instance_id: request.instance_id.clone(),
        game_version: game_version.clone(),
        version_json_path: json_path.to_string_lossy().to_string(),
        client_jar_path: client_jar_path.to_string_lossy().to_string(),
        json_bytes_written,
        jar_bytes_written,
        jar_sha1_verified,
        used_manifest_url: version_info.url.clone(),
        replaced_existing,
    })
}

#[tauri::command]
pub async fn start_install_client_version_task(
    app: AppHandle,
    request: AsyncInstallClientVersionRequest,
) -> Result<AsyncInstallTaskStarted, LauncherError> {
    let instance_id = request.instance_id.clone();
    let dd = data_dir(&app);
    let instance = get_instance_in(&dd, &instance_id)?;
    let game_version = instance.game_version.clone();
    let group_id = format!("{}-install-client", instance_id);

    let group = build_running_task_group(
        &group_id,
        "安装游戏版本",
        &format!("正在安装 Minecraft {}", game_version),
    );
    record_running_task_group(&app, group).await?;

    let app_clone = app.clone();
    let gid = group_id.clone();
    let iid = instance_id.clone();
    tokio::spawn(async move {
        let install_req = InstallClientVersionRequest {
            instance_id,
            overwrite: request.overwrite,
        };
        let result = install_client_version_for_instance(app_clone.clone(), install_req).await;

        let (status, message) = match &result {
            Ok(r) => (
                TaskStatus::Completed,
                format!("游戏版本 {} 安装完成", r.game_version),
            ),
            Err(e) => (TaskStatus::Failed, format!("安装失败: {}", e)),
        };
        let _ = update_single_task_group(&app_clone, &gid, |group| {
            group.overall_status = status.clone();
            if let Some(task) = group.tasks.first_mut() {
                task.status = status;
                task.message = message;
            }
        }).await;
    });

    Ok(AsyncInstallTaskStarted {
        group_id,
        task_id: 1,
        instance_id: iid,
        game_version,
    })
}

// ── Phase 16: Libraries install ─────────────────────────────────────────

#[tauri::command]
pub async fn install_libraries_for_instance(
    app: AppHandle,
    request: InstallLibrariesRequest,
) -> Result<InstallLibrariesResult, LauncherError> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let dd = data_dir(&app);
    let instance = get_instance_in(&dd, &request.instance_id)?;
    let game_version = instance.game_version.clone();

    let version_json_path = versions_dir(&instance.game_dir)
        .join(&game_version)
        .join(format!("{}.json", game_version));

    let version_json = parse_version_json(&version_json_path)?;
    let libs_dir = libraries_dir(&instance.game_dir);
    std::fs::create_dir_all(&libs_dir)
        .map_err(|e| LauncherError::from(format!("Mkdir failed: {}", e)))?;

    let mut downloaded: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;
    let mut bytes_written: u64 = 0;
    let scanned = version_json.libraries.len() as u32;

    for entry in &version_json.libraries {
        let lib_path = match library_artifact_path(&entry.name, &libs_dir) {
            Some(p) => p,
            None => {
                failed += 1;
                continue;
            }
        };

        if lib_path.exists() {
            if !request.overwrite {
                skipped += 1;
                continue;
            }
            let _ = std::fs::remove_file(&lib_path);
        }

        // Get download info from the library entry
        let artifact = entry
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref());

        let url = match artifact {
            Some(a) => &a.url,
            None => {
                // Build fallback URL from Maven coordinates
                failed += 1;
                continue;
            }
        };

        match client.get(url).send().await {
            Ok(resp) => match resp.bytes().await {
                Ok(bytes) => {
                    if let Some(a) = artifact {
                        if !a.sha1.is_empty() {
                            let actual =
                                hex::encode(sha1_smol::Sha1::from(&bytes).digest().bytes());
                            if actual != a.sha1 {
                                failed += 1;
                                continue;
                            }
                        }
                    }
                    if let Some(parent) = lib_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    match std::fs::write(&lib_path, &bytes) {
                        Ok(_) => {
                            downloaded += 1;
                            bytes_written += bytes.len() as u64;
                        }
                        Err(_) => {
                            failed += 1;
                        }
                    }
                }
                Err(_) => {
                    failed += 1;
                }
            },
            Err(_) => {
                failed += 1;
            }
        }
    }

    Ok(InstallLibrariesResult {
        instance_id: request.instance_id.clone(),
        game_version,
        scanned,
        downloaded,
        skipped,
        failed,
        bytes_written,
        libraries_dir: libs_dir.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn start_install_libraries_task(
    app: AppHandle,
    request: AsyncInstallLibrariesRequest,
) -> Result<AsyncInstallLibrariesStarted, LauncherError> {
    let instance_id = request.instance_id.clone();
    let dd = data_dir(&app);
    let instance = get_instance_in(&dd, &instance_id)?;
    let game_version = instance.game_version.clone();
    let group_id = format!("{}-install-libraries", instance_id);

    let group = build_running_task_group(
        &group_id,
        "安装游戏库文件",
        "正在下载 Minecraft 运行库...",
    );
    record_running_task_group(&app, group).await?;

    let app_clone = app.clone();
    let gid = group_id.clone();
    let iid = instance_id.clone();
    tokio::spawn(async move {
        let install_req = InstallLibrariesRequest {
            instance_id,
            overwrite: request.overwrite,
        };
        let result = install_libraries_for_instance(app_clone.clone(), install_req).await;

        let (status, message) = match &result {
            Ok(r) => {
                if r.failed > 0 {
                    (
                        TaskStatus::Failed,
                        format!("库文件安装完成，{} 成功，{} 失败", r.downloaded, r.failed),
                    )
                } else {
                    (
                        TaskStatus::Completed,
                        format!("库文件安装完成，共 {} 个", r.downloaded),
                    )
                }
            }
            Err(e) => (TaskStatus::Failed, format!("安装失败: {}", e)),
        };
        let _ = update_single_task_group(&app_clone, &gid, |group| {
            group.overall_status = status.clone();
            if let Some(task) = group.tasks.first_mut() {
                task.status = status;
                task.message = message;
            }
        }).await;
    });

    Ok(AsyncInstallLibrariesStarted {
        group_id,
        task_id: 1,
        instance_id: iid,
        game_version,
    })
}

// ── Phase 17: Assets install ────────────────────────────────────────────

#[tauri::command]
pub async fn install_assets_for_instance(
    app: AppHandle,
    request: InstallAssetsRequest,
) -> Result<InstallAssetsResult, LauncherError> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let dd = data_dir(&app);
    let instance = get_instance_in(&dd, &request.instance_id)?;
    let game_version = instance.game_version.clone();

    let version_json_path = versions_dir(&instance.game_dir)
        .join(&game_version)
        .join(format!("{}.json", game_version));

    let version_json = parse_version_json(&version_json_path)?;

    let asset_index_info = version_json
        .asset_index
        .as_ref()
        .ok_or_else(|| LauncherError::from("Version JSON missing asset index info"))?;

    // Download asset index
    let asset_index: serde_json::Value = client
        .get(&asset_index_info.url)
        .send()
        .await
        .map_err(|e| LauncherError::from(format!("Asset index download failed: {}", e)))?
        .json()
        .await
        .map_err(|e| LauncherError::from(format!("Asset index parse failed: {}", e)))?;

    let assets = assets_dir(&instance.game_dir);
    let indexes_dir = assets.join("indexes");
    std::fs::create_dir_all(&indexes_dir)
        .map_err(|e| LauncherError::from(format!("Mkdir failed: {}", e)))?;

    // Save asset index
    let index_path = indexes_dir.join(format!("{}.json", asset_index_info.id));
    let index_bytes = serde_json::to_vec_pretty(&asset_index)
        .map_err(|e| LauncherError::from(format!("Serialize failed: {}", e)))?;
    let index_bytes_written = index_bytes.len() as u64;
    std::fs::write(&index_path, &index_bytes)
        .map_err(|e| LauncherError::from(format!("Write failed: {}", e)))?;

    let objects = asset_index
        .get("objects")
        .and_then(|o| o.as_object())
        .ok_or_else(|| LauncherError::from("No objects in asset index"))?;

    let resource_base = "https://resources.download.minecraft.net";

    let mut scanned: u64 = 0;
    let mut downloaded: u64 = 0;
    let mut skipped: u64 = 0;
    let mut failed: u64 = 0;
    let mut total_bytes: u64 = 0;

    for (_name, obj) in objects {
        scanned += 1;
        let hash = obj
            .get("hash")
            .and_then(|h| h.as_str())
            .unwrap_or("");
        let size = obj.get("size").and_then(|s| s.as_u64()).unwrap_or(0);

        if hash.is_empty() {
            failed += 1;
            continue;
        }

        let sub_dir = &hash[..2];
        let obj_path = assets.join("objects").join(sub_dir).join(hash);

        if obj_path.exists() {
            if let Ok(existing) = std::fs::metadata(&obj_path) {
                if existing.len() == size {
                    skipped += 1;
                    continue;
                }
            }
        }

        if let Some(parent) = obj_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let url = format!("{}/{}", resource_base, hash);
        match client.get(&url).send().await {
            Ok(resp) => match resp.bytes().await {
                Ok(bytes) => {
                    match std::fs::write(&obj_path, &bytes) {
                        Ok(_) => {
                            downloaded += 1;
                            total_bytes += bytes.len() as u64;
                        }
                        Err(_) => failed += 1,
                    }
                }
                Err(_) => failed += 1,
            },
            Err(_) => failed += 1,
        }
    }

    Ok(InstallAssetsResult {
        instance_id: request.instance_id.clone(),
        game_version,
        asset_index_id: asset_index_info.id.clone(),
        index_bytes_written,
        scanned,
        downloaded,
        skipped,
        failed,
        bytes_written: total_bytes,
        assets_dir: assets.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn start_install_assets_task(
    app: AppHandle,
    request: AsyncInstallAssetsRequest,
) -> Result<AsyncInstallAssetsStarted, LauncherError> {
    let instance_id = request.instance_id.clone();
    let dd = data_dir(&app);
    let instance = get_instance_in(&dd, &instance_id)?;
    let game_version = instance.game_version.clone();
    let group_id = format!("{}-install-assets", instance_id);

    let group = build_running_task_group(
        &group_id,
        "安装游戏资源",
        "正在下载 Minecraft 资源文件...",
    );
    record_running_task_group(&app, group).await?;

    let app_clone = app.clone();
    let gid = group_id.clone();
    let iid = instance_id.clone();
    tokio::spawn(async move {
        let install_req = InstallAssetsRequest {
            instance_id,
            overwrite: request.overwrite,
        };
        let result = install_assets_for_instance(app_clone.clone(), install_req).await;

        let (status, message) = match &result {
            Ok(r) => {
                if r.failed > 0 {
                    (
                        TaskStatus::Failed,
                        format!("资源文件安装完成，{} 成功，{} 失败", r.downloaded, r.failed),
                    )
                } else {
                    (
                        TaskStatus::Completed,
                        format!("资源文件安装完成，共 {} 个", r.downloaded),
                    )
                }
            }
            Err(e) => (TaskStatus::Failed, format!("安装失败: {}", e)),
        };
        let _ = update_single_task_group(&app_clone, &gid, |group| {
            group.overall_status = status.clone();
            if let Some(task) = group.tasks.first_mut() {
                task.status = status;
                task.message = message;
            }
        }).await;
    });

    Ok(AsyncInstallAssetsStarted {
        group_id,
        task_id: 1,
        instance_id: iid,
        game_version,
    })
}

// ── Phase 18: Loader install ────────────────────────────────────────────

/// Build the profile JSON URL for a specific loader.
fn loader_profile_url(
    source: SourceType,
    kind: InstallLoaderKind,
    game_version: &str,
    loader_version: &str,
) -> Result<String, LauncherError> {
    match kind {
        InstallLoaderKind::Fabric | InstallLoaderKind::Quilt => {
            let res_type = match kind {
                InstallLoaderKind::Fabric => ResourceType::FabricMeta,
                InstallLoaderKind::Quilt => ResourceType::QuiltMeta,
                _ => unreachable!(),
            };
            let base = get_download_api(source, res_type)
                .map_err(|e| LauncherError::from(format!("Loader API: {e}")))?;
            Ok(format!(
                "{}v2/versions/loader/{}/{}/profile/json",
                base, game_version, loader_version
            ))
        }
        InstallLoaderKind::Forge => {
            // BMCLAPI: /forge/download/{version}
            let base = get_download_api(source, ResourceType::ForgeMeta)
                .map_err(|e| LauncherError::from(format!("Forge API: {e}")))?;
            Ok(format!("{}forge/download/{}", base, loader_version))
        }
        InstallLoaderKind::NeoForge => {
            let base = get_download_api(source, ResourceType::NeoforgeMetaNeoforge)
                .map_err(|e| LauncherError::from(format!("NeoForge API: {e}")))?;
            Ok(format!(
                "{}neoforge/versions/{}/download/json",
                base, loader_version
            ))
        }
    }
}

fn loader_kind_to_local(kind: InstallLoaderKind) -> LocalInstanceKind {
    match kind {
        InstallLoaderKind::Fabric => LocalInstanceKind::Fabric,
        InstallLoaderKind::Quilt => LocalInstanceKind::Quilt,
        InstallLoaderKind::Forge => LocalInstanceKind::Forge,
        InstallLoaderKind::NeoForge => LocalInstanceKind::NeoForge,
    }
}

#[tauri::command]
pub async fn install_loader_for_instance(
    app: AppHandle,
    request: InstallLoaderRequest,
) -> Result<InstallLoaderResult, LauncherError> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let priority_list = get_source_priority_list(get_use_mirror(&app));
    let dd = data_dir(&app);
    let instance = get_instance_in(&dd, &request.instance_id)?;
    let game_version = instance.game_version.clone();

    // Determine loader version (explicit or latest)
    let loader_version = match &request.loader_version {
        Some(v) => v.clone(),
        None => {
            // Fetch latest loader version for this game version
            let versions = match request.kind {
                InstallLoaderKind::Fabric => {
                    get_fabric_meta_by_game_version(&app, &client, &priority_list, &game_version)
                        .await
                        .map_err(|e| LauncherError::from(e.to_string()))?
                }
                InstallLoaderKind::Forge => {
                    get_forge_meta_by_game_version(&app, &client, &priority_list, &game_version)
                        .await
                        .map_err(|e| LauncherError::from(e.to_string()))?
                }
                InstallLoaderKind::NeoForge => {
                    get_neoforge_meta_by_game_version(&app, &client, &priority_list, &game_version)
                        .await
                        .map_err(|e| LauncherError::from(e.to_string()))?
                }
                InstallLoaderKind::Quilt => {
                    get_quilt_meta_by_game_version(&app, &client, &priority_list, &game_version)
                        .await
                        .map_err(|e| LauncherError::from(e.to_string()))?
                }
            };
            versions
                .first()
                .map(|v| v.version.clone())
                .ok_or_else(|| LauncherError::from("没有找到可用的加载器版本"))?
        }
    };

    // Build profile JSON URL
    let profile_url = loader_profile_url(
        *priority_list.first().unwrap_or(&SourceType::Official),
        request.kind,
        &game_version,
        &loader_version,
    )?;

    let profile_json: serde_json::Value = client
        .get(&profile_url)
        .send()
        .await
        .map_err(|e| LauncherError::from(format!("Profile download failed: {}", e)))?
        .json()
        .await
        .map_err(|e| LauncherError::from(format!("Profile parse failed: {}", e)))?;

    let version_dir = versions_dir(&instance.game_dir);
    std::fs::create_dir_all(&version_dir)
        .map_err(|e| LauncherError::from(format!("Mkdir failed: {}", e)))?;

    let version_name = match request.kind {
        InstallLoaderKind::Forge => format!("{}-forge-{}", game_version, loader_version),
        InstallLoaderKind::Fabric => format!("fabric-loader-{}-{}", loader_version, game_version),
        InstallLoaderKind::NeoForge => format!("neoforge-{}-{}", game_version, loader_version),
        InstallLoaderKind::Quilt => format!("quilt-loader-{}-{}", loader_version, game_version),
    };

    let json_path = version_dir.join(format!("{}.json", version_name.clone()));
    let replaced_existing = json_path.exists();
    if replaced_existing && !request.overwrite {
        return Err(LauncherError::from("Loader version JSON already exists and overwrite is disabled"));
    }

    // Build a minimal inheriting version JSON
    let loader_version_json = serde_json::json!({
        "id": version_name,
        "inheritsFrom": game_version,
        "mainClass": profile_json.get("mainClass").and_then(|v| v.as_str()).or_else(|| profile_json.get("mainClass").and_then(|v| v.get("client")).and_then(|v| v.as_str())).unwrap_or(""),
        "libraries": profile_json.get("libraries").or_else(|| profile_json.get("libraries").and_then(|v| v.get("client"))).unwrap_or(&serde_json::Value::Null),
        "arguments": profile_json.get("arguments").unwrap_or(&serde_json::json!({"game": [], "jvm": []})),
    });

    let json_bytes = serde_json::to_vec_pretty(&loader_version_json)
        .map_err(|e| LauncherError::from(format!("Serialize failed: {}", e)))?;
    let bytes_written = json_bytes.len() as u64;
    std::fs::write(&json_path, &json_bytes)
        .map_err(|e| LauncherError::from(format!("Write failed: {}", e)))?;

    // Update instance kind and game version
    update_instance_loader_in(
        &dd,
        &instance.id,
        version_name.clone(),
        loader_kind_to_local(request.kind),
    )?;

    Ok(InstallLoaderResult {
        instance_id: request.instance_id.clone(),
        previous_game_version: game_version,
        new_game_version: version_name,
        kind: request.kind,
        loader_version,
        version_json_path: json_path.to_string_lossy().to_string(),
        bytes_written,
        replaced_existing,
    })
}

#[tauri::command]
pub async fn start_install_loader_task(
    app: AppHandle,
    request: AsyncInstallLoaderRequest,
) -> Result<AsyncInstallLoaderStarted, LauncherError> {
    let instance_id = request.instance_id.clone();
    let kind = request.kind;
    let loader_version = request.loader_version.clone();
    let group_id = format!("{}-install-loader-{:?}", instance_id, kind);

    let loader_label = format!("{:?}", kind);
    let version_label = loader_version.as_deref().unwrap_or("latest");
    let group = build_running_task_group(
        &group_id,
        "安装模组加载器",
        &format!("正在安装 {} {}", loader_label, version_label),
    );
    record_running_task_group(&app, group).await?;

    let app_clone = app.clone();
    let gid = group_id.clone();
    let iid = instance_id.clone();
    let k = kind;
    tokio::spawn(async move {
        let install_req = InstallLoaderRequest {
            instance_id,
            kind: k,
            loader_version,
            overwrite: request.overwrite,
        };
        let result = install_loader_for_instance(app_clone.clone(), install_req).await;

        let (status, message) = match &result {
            Ok(r) => (
                TaskStatus::Completed,
                format!("{:?} {} 安装完成", r.kind, r.loader_version),
            ),
            Err(e) => (TaskStatus::Failed, format!("安装失败: {}", e)),
        };
        let _ = update_single_task_group(&app_clone, &gid, |group| {
            group.overall_status = status.clone();
            if let Some(task) = group.tasks.first_mut() {
                task.status = status;
                task.message = message;
            }
        }).await;
    });

    Ok(AsyncInstallLoaderStarted {
        group_id,
        task_id: 1,
        instance_id: iid,
        kind,
    })
}

// ── Phase 35: Async resource install ────────────────────────────────────

#[tauri::command]
pub async fn start_install_resource_task(
    app: AppHandle,
    request: AsyncInstallResourceRequest,
) -> Result<AsyncInstallResourceStarted, LauncherError> {
    let instance_id = request.instance_id.clone();
    let file_name = request.file.file_name.clone();
    let group_id = format!("{}-install-resource", instance_id);

    let group = build_running_task_group(
        &group_id,
        "安装资源文件",
        &format!("正在下载 {}", file_name),
    );
    record_running_task_group(&app, group).await?;

    let app_clone = app.clone();
    let gid = group_id.clone();
    let iid = instance_id.clone();
    tokio::spawn(async move {
        let install_req = InstallResourceRequest {
            instance_id,
            kind: request.kind,
            file: request.file,
            overwrite: request.overwrite,
        };
        let result = install_resource_to_instance(app_clone.clone(), install_req).await;

        let (status, message) = match &result {
            Ok(r) => (
                TaskStatus::Completed,
                format!("资源文件 {} 下载完成", r.file_name),
            ),
            Err(e) => (TaskStatus::Failed, format!("下载失败: {}", e)),
        };
        let _ = update_single_task_group(&app_clone, &gid, |group| {
            group.overall_status = status.clone();
            if let Some(task) = group.tasks.first_mut() {
                task.status = status;
                task.message = message;
            }
        }).await;
    });

    Ok(AsyncInstallResourceStarted {
        group_id,
        task_id: 1,
        instance_id: iid,
        file_name,
    })
}
