use crate::error::LauncherError;
use crate::AppState;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use super::models::{
    CreateLocalInstanceRequest, LocalInstance, LocalInstanceKind, UpdateLocalInstanceRequest,
};

fn instances_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join("instances").join("instances.json")
}

fn instance_dir_path(data_dir: &Path, id: &str) -> PathBuf {
    data_dir.join("instances").join(id).join("minecraft")
}

fn load_instances(data_dir: &Path) -> Result<Vec<LocalInstance>, LauncherError> {
    let path = instances_file_path(data_dir);
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| LauncherError::from(format!("无法创建实例数据目录: {e}")))?;
        }
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| LauncherError::from(format!("无法读取实例数据文件: {e}")))?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&content)
        .map_err(|e| LauncherError::from(format!("实例数据文件已损坏，无法解析: {e}")))
}

fn save_instances(data_dir: &Path, instances: &[LocalInstance]) -> Result<(), LauncherError> {
    let path = instances_file_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| LauncherError::from(format!("无法创建实例数据目录: {e}")))?;
    }

    let json = serde_json::to_string_pretty(instances)
        .map_err(|e| LauncherError::from(format!("无法序列化实例数据: {e}")))?;

    std::fs::write(&path, json)
        .map_err(|e| LauncherError::from(format!("无法写入实例数据文件: {e}")))
}

pub fn create_instance_in(
    data_dir: &Path,
    request: CreateLocalInstanceRequest,
) -> Result<LocalInstance, LauncherError> {
    let name = request.name.trim().to_string();
    if name.is_empty() {
        return Err(LauncherError::from("实例名称不能为空"));
    }

    let game_version = request.game_version.trim().to_string();
    if game_version.is_empty() {
        return Err(LauncherError::from("游戏版本不能为空"));
    }

    let kind = request.kind.unwrap_or(LocalInstanceKind::Vanilla);
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::utils::now_iso8601();
    let game_dir = instance_dir_path(data_dir, &id)
        .to_string_lossy()
        .to_string();

    std::fs::create_dir_all(instance_dir_path(data_dir, &id))
        .map_err(|e| LauncherError::from(format!("无法创建实例目录: {e}")))?;

    let instance = LocalInstance {
        id,
        name,
        game_version,
        kind,
        game_dir,
        icon: None,
        last_played_at: None,
        created_at: now.clone(),
        updated_at: now,
    };

    let mut instances = load_instances(data_dir)?;
    instances.push(instance.clone());
    save_instances(data_dir, &instances)?;

    Ok(instance)
}

pub fn update_instance_in(
    data_dir: &Path,
    request: UpdateLocalInstanceRequest,
) -> Result<LocalInstance, LauncherError> {
    let mut instances = load_instances(data_dir)?;

    let idx = instances
        .iter()
        .position(|i| i.id == request.id)
        .ok_or_else(|| LauncherError::from(format!("未找到实例: {}", request.id)))?;

    let instance = &mut instances[idx];

    if let Some(name) = &request.name {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return Err(LauncherError::from("实例名称不能为空"));
        }
        instance.name = trimmed;
    }

    if let Some(version) = &request.game_version {
        let trimmed = version.trim().to_string();
        if trimmed.is_empty() {
            return Err(LauncherError::from("游戏版本不能为空"));
        }
        instance.game_version = trimmed;
    }

    if let Some(kind) = &request.kind {
        instance.kind = kind.clone();
    }

    match &request.icon {
        Some(Some(icon)) => instance.icon = Some(icon.clone()),
        Some(None) => instance.icon = None,
        None => {  }
    }

    instance.updated_at = crate::utils::now_iso8601();

    let result = instance.clone();
    save_instances(data_dir, &instances)?;

    Ok(result)
}


pub fn get_instance_in(data_dir: &Path, id: &str) -> Result<LocalInstance, LauncherError> {
    let instances = load_instances(data_dir)?;
    instances
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| LauncherError::from(format!("未找到实例: {id}")))
}



pub fn update_instance_loader_in(
    data_dir: &Path,
    id: &str,
    game_version: String,
    kind: LocalInstanceKind,
) -> Result<LocalInstance, LauncherError> {
    let mut instances = load_instances(data_dir)?;
    let idx = instances
        .iter()
        .position(|i| i.id == id)
        .ok_or_else(|| LauncherError::from(format!("未找到实例: {id}")))?;

    instances[idx].game_version = game_version;
    instances[idx].kind = kind;
    instances[idx].updated_at = crate::utils::now_iso8601();

    let result = instances[idx].clone();
    save_instances(data_dir, &instances)?;
    Ok(result)
}



pub fn mark_instance_played_in(data_dir: &Path, id: &str) -> Result<LocalInstance, LauncherError> {
    let mut instances = load_instances(data_dir)?;
    let idx = instances
        .iter()
        .position(|i| i.id == id)
        .ok_or_else(|| LauncherError::from(format!("未找到实例: {id}")))?;

    let now = crate::utils::now_iso8601();
    instances[idx].last_played_at = Some(now.clone());
    instances[idx].updated_at = now;

    let updated = instances[idx].clone();
    save_instances(data_dir, &instances)?;
    Ok(updated)
}

pub fn delete_instance_from(data_dir: &Path, id: &str) -> Result<(), LauncherError> {
    let mut instances = load_instances(data_dir)?;

    let idx = instances
        .iter()
        .position(|i| i.id == id)
        .ok_or_else(|| LauncherError::from(format!("未找到实例: {id}")))?;

    instances.remove(idx);
    save_instances(data_dir, &instances)?;

    Ok(())
}



#[tauri::command]
pub async fn list_local_instances(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<LocalInstance>, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    load_instances(&data_dir)
}

#[tauri::command]
pub async fn create_local_instance(
    state: State<'_, Arc<Mutex<AppState>>>,
    request: CreateLocalInstanceRequest,
) -> Result<LocalInstance, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    create_instance_in(&data_dir, request)
}

#[tauri::command]
pub async fn update_local_instance(
    state: State<'_, Arc<Mutex<AppState>>>,
    request: UpdateLocalInstanceRequest,
) -> Result<LocalInstance, LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    update_instance_in(&data_dir, request)
}

#[tauri::command]
pub async fn delete_local_instance(
    state: State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<(), LauncherError> {
    let data_dir = {
        let s = state.lock().await;
        s.data_dir.clone()
    };
    delete_instance_from(&data_dir, &id)
}



#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_temp_dir() -> TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    #[test]
    fn empty_repo_returns_empty_list_and_creates_parent_dir() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let result = load_instances(data_dir).expect("should succeed");
        assert!(result.is_empty());


        let instances_dir = data_dir.join("instances");
        assert!(instances_dir.exists(), "instances dir should be created");
        assert!(
            instances_dir.is_dir(),
            "instances path should be a directory"
        );
    }

    #[test]
    fn create_generates_uuid_and_instance_dir() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let request = CreateLocalInstanceRequest {
            name: "Test Instance".to_string(),
            game_version: "1.21.4".to_string(),
            kind: Some(LocalInstanceKind::Fabric),
        };

        let instance = create_instance_in(data_dir, request).expect("should succeed");

        assert!(!instance.id.is_empty(), "id should not be empty");
        assert_eq!(instance.name, "Test Instance");
        assert_eq!(instance.game_version, "1.21.4");
        assert_eq!(instance.kind, LocalInstanceKind::Fabric);


        let expected_dir = instance_dir_path(data_dir, &instance.id);
        assert!(expected_dir.exists(), "instance dir should be created");
        assert!(expected_dir.is_dir(), "instance path should be a directory");


        let json_path = instances_file_path(data_dir);
        assert!(json_path.exists(), "instances.json should exist");

        let loaded = load_instances(data_dir).expect("should load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, instance.id);
        assert_eq!(loaded[0].name, "Test Instance");
    }

    #[test]
    fn create_with_empty_name_returns_error() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let request = CreateLocalInstanceRequest {
            name: "   ".to_string(),
            game_version: "1.21.4".to_string(),
            kind: None,
        };

        let err = create_instance_in(data_dir, request).unwrap_err();
        assert!(err.contains("名称"), "error should mention name");
    }

    #[test]
    fn create_with_empty_game_version_returns_error() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let request = CreateLocalInstanceRequest {
            name: "Test".to_string(),
            game_version: "".to_string(),
            kind: None,
        };

        let err = create_instance_in(data_dir, request).unwrap_err();
        assert!(err.contains("版本"), "error should mention version");
    }

    #[test]
    fn update_modifies_fields_and_updates_time() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let create_req = CreateLocalInstanceRequest {
            name: "Original".to_string(),
            game_version: "1.20".to_string(),
            kind: Some(LocalInstanceKind::Vanilla),
        };
        let instance = create_instance_in(data_dir, create_req).expect("should create");
        let original_updated_at = instance.updated_at.clone();


        std::thread::sleep(std::time::Duration::from_millis(10));

        let update_req = UpdateLocalInstanceRequest {
            id: instance.id.clone(),
            name: Some("Updated Name".to_string()),
            game_version: Some("1.21.4".to_string()),
            kind: Some(LocalInstanceKind::Forge),
            icon: Some(Some("icon_data".to_string())),
        };

        let updated = update_instance_in(data_dir, update_req).expect("should update");

        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.game_version, "1.21.4");
        assert_eq!(updated.kind, LocalInstanceKind::Forge);
        assert_eq!(updated.icon, Some("icon_data".to_string()));
        assert_ne!(updated.updated_at, original_updated_at);


        let loaded = load_instances(data_dir).expect("should load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Updated Name");
    }

    #[test]
    fn update_nonexistent_id_returns_error() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let update_req = UpdateLocalInstanceRequest {
            id: "nonexistent-id".to_string(),
            name: Some("New Name".to_string()),
            game_version: None,
            kind: None,
            icon: None,
        };

        let err = update_instance_in(data_dir, update_req).unwrap_err();
        assert!(err.contains("未找到"), "error should mention not found");
    }

    #[test]
    fn delete_removes_record_but_not_instance_directory() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let create_req = CreateLocalInstanceRequest {
            name: "To Delete".to_string(),
            game_version: "1.20".to_string(),
            kind: None,
        };
        let instance = create_instance_in(data_dir, create_req).expect("should create");

        let instance_dir = instance_dir_path(data_dir, &instance.id);
        assert!(
            instance_dir.exists(),
            "instance dir should exist before delete"
        );


        delete_instance_from(data_dir, &instance.id).expect("should delete");


        let loaded = load_instances(data_dir).expect("should load");
        assert!(loaded.is_empty(), "instances list should be empty");


        assert!(
            instance_dir.exists(),
            "instance dir should still exist after delete"
        );
    }

    #[test]
    fn delete_nonexistent_id_returns_error() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let err = delete_instance_from(data_dir, "nonexistent-id").unwrap_err();
        assert!(err.contains("未找到"), "error should mention not found");
    }

    #[test]
    fn corrupted_json_returns_error() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let json_path = instances_file_path(data_dir);
        std::fs::create_dir_all(json_path.parent().unwrap()).expect("should create dir");
        std::fs::write(&json_path, "this is not valid json{{{").expect("should write");

        let err = load_instances(data_dir).unwrap_err();
        assert!(
            err.contains("损坏") || err.contains("无法解析"),
            "error should indicate corruption: {err}"
        );
    }

    #[test]
    fn multiple_instances_persistence() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();


        let req1 = CreateLocalInstanceRequest {
            name: "Instance A".to_string(),
            game_version: "1.20".to_string(),
            kind: None,
        };
        let req2 = CreateLocalInstanceRequest {
            name: "Instance B".to_string(),
            game_version: "1.21".to_string(),
            kind: Some(LocalInstanceKind::Fabric),
        };

        let i1 = create_instance_in(data_dir, req1).expect("should create");
        let i2 = create_instance_in(data_dir, req2).expect("should create");

        let loaded = load_instances(data_dir).expect("should load");
        assert_eq!(loaded.len(), 2);
        assert_ne!(i1.id, i2.id, "ids should be unique");


        delete_instance_from(data_dir, &i1.id).expect("should delete");

        let loaded = load_instances(data_dir).expect("should load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, i2.id);
    }

    #[test]
    fn update_with_empty_name_after_trim_returns_error() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let create_req = CreateLocalInstanceRequest {
            name: "Original".to_string(),
            game_version: "1.20".to_string(),
            kind: None,
        };
        let instance = create_instance_in(data_dir, create_req).expect("should create");

        let update_req = UpdateLocalInstanceRequest {
            id: instance.id,
            name: Some("   ".to_string()),
            game_version: None,
            kind: None,
            icon: None,
        };

        let err = update_instance_in(data_dir, update_req).unwrap_err();
        assert!(err.contains("名称"), "error should mention name");
    }

    #[test]
    fn update_partial_fields_keeps_others() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let create_req = CreateLocalInstanceRequest {
            name: "Original".to_string(),
            game_version: "1.20".to_string(),
            kind: Some(LocalInstanceKind::Quilt),
        };
        let instance = create_instance_in(data_dir, create_req).expect("should create");


        let update_req = UpdateLocalInstanceRequest {
            id: instance.id.clone(),
            name: Some("Only Name Changed".to_string()),
            game_version: None,
            kind: None,
            icon: None,
        };

        let updated = update_instance_in(data_dir, update_req).expect("should update");
        assert_eq!(updated.name, "Only Name Changed");
        assert_eq!(updated.game_version, "1.20");
        assert_eq!(updated.kind, LocalInstanceKind::Quilt);
    }

    #[test]
    fn update_clear_icon() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let create_req = CreateLocalInstanceRequest {
            name: "With Icon".to_string(),
            game_version: "1.20".to_string(),
            kind: None,
        };
        let instance = create_instance_in(data_dir, create_req).expect("should create");


        let set_req = UpdateLocalInstanceRequest {
            id: instance.id.clone(),
            name: None,
            game_version: None,
            kind: None,
            icon: Some(Some("icon_data".to_string())),
        };
        let updated = update_instance_in(data_dir, set_req).expect("should set icon");
        assert_eq!(updated.icon, Some("icon_data".to_string()));


        let clear_req = UpdateLocalInstanceRequest {
            id: instance.id.clone(),
            name: None,
            game_version: None,
            kind: None,
            icon: Some(None),
        };
        let cleared = update_instance_in(data_dir, clear_req).expect("should clear icon");
        assert_eq!(cleared.icon, None);
    }



    #[test]
    fn get_instance_in_finds_existing() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let created = create_instance_in(
            data_dir,
            CreateLocalInstanceRequest {
                name: "Find Me".to_string(),
                game_version: "1.21".to_string(),
                kind: None,
            },
        )
        .expect("should create");

        let found = get_instance_in(data_dir, &created.id).expect("should find");
        assert_eq!(found.id, created.id);
        assert_eq!(found.name, "Find Me");
    }

    #[test]
    fn get_instance_in_errors_on_nonexistent() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let err = get_instance_in(data_dir, "nonexistent-id").unwrap_err();
        assert!(
            err.contains("未找到"),
            "error should mention not found: {err}"
        );
    }



    #[test]
    fn mark_instance_played_sets_last_played_at() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let created = create_instance_in(
            data_dir,
            CreateLocalInstanceRequest {
                name: "Play Me".to_string(),
                game_version: "1.20".to_string(),
                kind: None,
            },
        )
        .expect("should create");

        assert!(created.last_played_at.is_none(), "initially not played");

        let marked = mark_instance_played_in(data_dir, &created.id).expect("should mark");
        assert!(
            marked.last_played_at.is_some(),
            "should now have last_played_at"
        );
        assert!(!marked.last_played_at.as_ref().unwrap().is_empty());


        let loaded = load_instances(data_dir).expect("should load");
        assert_eq!(loaded[0].last_played_at, marked.last_played_at);
    }

    #[test]
    fn mark_instance_played_errors_on_nonexistent() {
        let dir = setup_temp_dir();
        let data_dir = dir.path();

        let err = mark_instance_played_in(data_dir, "nonexistent-id").unwrap_err();
        assert!(
            err.contains("未找到"),
            "error should mention not found: {err}"
        );
    }
}
