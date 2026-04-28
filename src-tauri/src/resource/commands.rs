use crate::resource::helpers::curseforge::{
    fetch_remote_resource_by_id_curseforge, fetch_remote_resource_by_local_curseforge,
    fetch_resource_list_by_name_curseforge, fetch_resource_version_packs_curseforge,
};
use crate::resource::helpers::loader_meta::fabric::get_fabric_meta_by_game_version;
use crate::resource::helpers::loader_meta::forge::get_forge_meta_by_game_version;
use crate::resource::helpers::loader_meta::neoforge::get_neoforge_meta_by_game_version;
use crate::resource::helpers::loader_meta::optifine::get_optifine_meta_by_game_version;
use crate::resource::helpers::loader_meta::quilt::get_quilt_meta_by_game_version;
use crate::resource::helpers::misc::get_source_priority_list;
use crate::resource::helpers::modrinth::{
    fetch_remote_resource_by_id_modrinth, fetch_remote_resource_by_local_modrinth,
    fetch_resource_list_by_name_modrinth, fetch_resource_version_packs_modrinth,
};
use crate::resource::helpers::version_manifest::get_game_version_manifest;
use crate::resource::models::{
    GameClientResourceInfo, ModLoaderResourceInfo, ModLoaderType, ModUpdateQuery,
    OptiFineResourceInfo, OtherResourceFileInfo, OtherResourceInfo,
    OtherResourceSearchQuery, OtherResourceSearchRes, OtherResourceSource,
    OtherResourceVersionPack, OtherResourceVersionPackQuery,
};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

#[tauri::command]
pub async fn fetch_game_version_list(app: AppHandle) -> Result<Vec<GameClientResourceInfo>, String> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let use_mirror = app
        .try_state::<Arc<Mutex<bool>>>()
        .map(|s| *s.blocking_lock())
        .unwrap_or(false);
    let priority_list = get_source_priority_list(use_mirror);
    get_game_version_manifest(&app, &client, &priority_list)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_mod_loader_version_list(
    app: AppHandle,
    game_version: String,
    mod_loader_type: ModLoaderType,
) -> Result<Vec<ModLoaderResourceInfo>, String> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let use_mirror = app
        .try_state::<Arc<Mutex<bool>>>()
        .map(|s| *s.blocking_lock())
        .unwrap_or(false);
    let priority_list = get_source_priority_list(use_mirror);

    match mod_loader_type {
        ModLoaderType::Forge | ModLoaderType::LegacyForge => {
            get_forge_meta_by_game_version(&app, &client, &priority_list, &game_version)
                .await
                .map_err(|e| e.to_string())
        }
        ModLoaderType::Fabric => {
            get_fabric_meta_by_game_version(&app, &client, &priority_list, &game_version)
                .await
                .map_err(|e| e.to_string())
        }
        ModLoaderType::NeoForge => {
            get_neoforge_meta_by_game_version(&app, &client, &priority_list, &game_version)
                .await
                .map_err(|e| e.to_string())
        }
        ModLoaderType::Quilt => {
            get_quilt_meta_by_game_version(&app, &client, &priority_list, &game_version)
                .await
                .map_err(|e| e.to_string())
        }
        _ => Err("Mod loader not supported for version listing".to_string()),
    }
}

#[tauri::command]
pub async fn fetch_optifine_version_list(
    app: AppHandle,
    game_version: String,
) -> Result<Vec<OptiFineResourceInfo>, String> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let use_mirror = app
        .try_state::<Arc<Mutex<bool>>>()
        .map(|s| *s.blocking_lock())
        .unwrap_or(false);
    let priority_list = get_source_priority_list(use_mirror);
    get_optifine_meta_by_game_version(&app, &client, &priority_list, &game_version)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_resource_list_by_name(
    app: AppHandle,
    download_source: OtherResourceSource,
    query: OtherResourceSearchQuery,
) -> Result<OtherResourceSearchRes, String> {
    match download_source {
        OtherResourceSource::CurseForge => {
            fetch_resource_list_by_name_curseforge(&app, &query)
                .await
                .map_err(|e| e.to_string())
        }
        OtherResourceSource::Modrinth => {
            fetch_resource_list_by_name_modrinth(&app, &query)
                .await
                .map_err(|e| e.to_string())
        }
        _ => Err("Unsupported download source".to_string()),
    }
}

#[tauri::command]
pub async fn fetch_resource_version_packs(
    app: AppHandle,
    download_source: OtherResourceSource,
    query: OtherResourceVersionPackQuery,
) -> Result<Vec<OtherResourceVersionPack>, String> {
    match download_source {
        OtherResourceSource::CurseForge => {
            fetch_resource_version_packs_curseforge(&app, &query)
                .await
                .map_err(|e| e.to_string())
        }
        OtherResourceSource::Modrinth => {
            fetch_resource_version_packs_modrinth(&app, &query)
                .await
                .map_err(|e| e.to_string())
        }
        _ => Err("Unsupported download source".to_string()),
    }
}

#[tauri::command]
pub async fn fetch_remote_resource_by_local(
    app: AppHandle,
    download_source: OtherResourceSource,
    file_path: String,
) -> Result<OtherResourceFileInfo, String> {
    match download_source {
        OtherResourceSource::CurseForge => {
            fetch_remote_resource_by_local_curseforge(&app, &file_path)
                .await
                .map_err(|e| e.to_string())
        }
        OtherResourceSource::Modrinth => {
            fetch_remote_resource_by_local_modrinth(&app, &file_path)
                .await
                .map_err(|e| e.to_string())
        }
        _ => Err("Unsupported download source".to_string()),
    }
}

#[tauri::command]
pub async fn fetch_remote_resource_by_id(
    app: AppHandle,
    download_source: OtherResourceSource,
    resource_id: String,
) -> Result<OtherResourceInfo, String> {
    match download_source {
        OtherResourceSource::CurseForge => {
            fetch_remote_resource_by_id_curseforge(&app, &resource_id)
                .await
                .map_err(|e| e.to_string())
        }
        OtherResourceSource::Modrinth => {
            fetch_remote_resource_by_id_modrinth(&app, &resource_id)
                .await
                .map_err(|e| e.to_string())
        }
        _ => Err("Unsupported download source".to_string()),
    }
}

#[tauri::command]
pub async fn download_game_server(
    app: AppHandle,
    resource_info: GameClientResourceInfo,
    dest: String,
) -> Result<(), String> {
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
            return Err(format!("SHA1 mismatch: expected {}, got {}", sha1, actual));
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
) -> Result<(), String> {
    if queries.is_empty() {
        return Ok(());
    }

    let client = app.state::<reqwest::Client>().inner().clone();

    for query in &queries {
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
                return Err(format!("SHA1 mismatch for {}: expected {}, got {}", query.file_name, query.sha1, actual));
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
            let _ = std::fs::rename(&query.old_file_path, &old_backup);
        }
    }

    Ok(())
}
