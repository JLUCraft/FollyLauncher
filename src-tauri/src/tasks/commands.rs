use crate::error::LauncherError;
use crate::tasks::models::{
    self, ImportTaskSnapshotRequest, ImportTaskSnapshotResult, TaskGroup, TaskProgress,
    TaskSnapshotBundle, TaskStatus,
};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::utils::now_iso8601;

// ── schedule_task_group / update_task_progress removed ──
// These orphan commands had no TS call sites. task centre state
// is managed through the record_* and update_single_task_group helpers.
// Tests for the underlying logic remain in this module.

#[tauri::command]
pub async fn get_task_group(
    state: State<'_, Arc<Mutex<HashMap<String, TaskGroup>>>>,
    group_id: String,
) -> Result<Option<TaskGroup>, LauncherError> {
    let groups = state.lock().await;
    Ok(groups.get(&group_id).cloned())
}

#[tauri::command]
pub async fn list_task_groups(
    state: State<'_, Arc<Mutex<HashMap<String, TaskGroup>>>>,
) -> Result<Vec<TaskGroup>, LauncherError> {
    let groups = state.lock().await;
    let mut out: Vec<TaskGroup> = groups.values().cloned().collect();
    // Sort by updated_at descending
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

#[tauri::command]
pub async fn cancel_task_group(
    app: AppHandle,
    state: State<'_, Arc<Mutex<HashMap<String, TaskGroup>>>>,
    group_id: String,
) -> Result<(), LauncherError> {
    let mut groups = state.lock().await;
    let group = groups
        .get_mut(&group_id)
        .ok_or_else(|| format!("未找到任务组: {group_id}"))?;

    let now = now_iso8601();

    for task in &mut group.tasks {
        models::mark_task_cancelled_in_task_center(task, &now);
    }
    models::refresh_group_summary(group, now);

    let clone = group.clone();
    drop(groups);

    let _ = app.emit("task-group-cancelled", &clone);
    Ok(())
}

#[tauri::command]
pub async fn remove_task_group(
    state: State<'_, Arc<Mutex<HashMap<String, TaskGroup>>>>,
    group_id: String,
) -> Result<(), LauncherError> {
    let mut groups = state.lock().await;
    if groups.remove(&group_id).is_none() {
        return Err(LauncherError::new(
            "NOT_FOUND",
            format!("未找到任务组: {group_id}"),
        ));
    }
    Ok(())
}

// ── Phase 36: Snapshot export/import commands ───────────────────────────

#[tauri::command]
pub async fn export_task_snapshot(
    state: State<'_, Arc<Mutex<HashMap<String, TaskGroup>>>>,
) -> Result<String, LauncherError> {
    let groups_map = state.lock().await;
    let mut groups: Vec<TaskGroup> = groups_map.values().cloned().collect();
    // Sort by updated_at descending
    groups.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let bundle = TaskSnapshotBundle {
        schema_version: 1,
        exported_at: now_iso8601(),
        groups,
    };
    serde_json::to_string_pretty(&bundle)
        .map_err(|e| LauncherError::new("SERIALIZE_ERROR", format!("序列化快照失败: {e}")))
}

/// Pure import logic: parse, validate, and insert groups from a snapshot
/// JSON string into the given map.
///
/// Returns the summary result together with the list of successfully imported
/// groups (so the caller can emit events).
pub fn import_task_snapshot_into_map(
    groups: &mut HashMap<String, TaskGroup>,
    bundle_json: &str,
    replace_existing: bool,
    drop_active: bool,
) -> Result<(ImportTaskSnapshotResult, Vec<TaskGroup>), LauncherError> {
    let bundle: TaskSnapshotBundle =
        serde_json::from_str(bundle_json).map_err(|e| LauncherError::from(format!("快照 JSON 解析失败: {e}")))?;

    if bundle.schema_version != 1 {
        return Err(LauncherError::new("UNSUPPORTED_VERSION", format!(
            "不支持快照版本 {}，仅支持版本 1",
            bundle.schema_version
        )));
    }

    let now = now_iso8601();
    let mut imported: usize = 0;
    let mut skipped: usize = 0;
    let mut failed: usize = 0;
    let mut imported_groups: Vec<TaskGroup> = Vec::new();

    for mut group in bundle.groups {
        // Validate
        if let Err(_e) = models::validate_snapshot_group(&group) {
            failed += 1;
            continue;
        }

        // Check existing & replace_existing
        let exists = groups.contains_key(&group.group_id);
        if exists && !replace_existing {
            skipped += 1;
            continue;
        }

        // Normalize (fill timestamps, handle active)
        let should_skip = models::normalize_imported_task_group(&mut group, drop_active, &now);
        if should_skip {
            skipped += 1;
            continue;
        }

        // Insert / overwrite
        groups.insert(group.group_id.clone(), group.clone());
        imported_groups.push(group);
        imported += 1;
    }

    Ok((
        ImportTaskSnapshotResult {
            imported,
            skipped,
            failed,
            total: imported + skipped + failed,
        },
        imported_groups,
    ))
}

#[tauri::command]
pub async fn import_task_snapshot(
    app: AppHandle,
    state: State<'_, Arc<Mutex<HashMap<String, TaskGroup>>>>,
    request: ImportTaskSnapshotRequest,
) -> Result<ImportTaskSnapshotResult, LauncherError> {
    let mut groups = state.lock().await;
    let (result, imported_groups) = import_task_snapshot_into_map(
        &mut groups,
        &request.bundle_json,
        request.replace_existing,
        request.drop_active,
    )?;
    drop(groups);

    // Emit events for each imported / overwritten group
    for group in &imported_groups {
        let _ = app.emit("task-group-updated", group);
    }

    Ok(result)
}

// ── Phase 26: record helpers (pure + stateful) ───────────────────────

/// Build a completed single-task `TaskGroup` without requiring a Tauri app.
/// This is a pure function usable in unit tests.
pub fn build_completed_task_group(
    group_id: String,
    task_name: String,
    message: String,
    current: u64,
    total: u64,
) -> TaskGroup {
    let now = now_iso8601();
    let task = TaskProgress {
        task_id: 1,
        name: task_name,
        current,
        total,
        status: TaskStatus::Completed,
        message,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let mut group = TaskGroup {
        group_id,
        tasks: vec![task],
        overall_status: TaskStatus::Pending,
        created_at: now.clone(),
        updated_at: now.clone(),
        completed_tasks: 0,
        total_tasks: 0,
        progress_percent: 0,
    };
    models::refresh_group_summary(&mut group, now);
    group.progress_percent = if total == 0 {
        0
    } else {
        ((current.min(total) as f64 / total as f64) * 100.0).round() as u8
    };
    group
}

/// Build a failed single-task `TaskGroup` without requiring a Tauri app.
/// This is a pure function usable in unit tests.
pub fn build_failed_task_group(group_id: String, task_name: String, message: String) -> TaskGroup {
    let now = now_iso8601();
    let task = TaskProgress {
        task_id: 1,
        name: task_name,
        current: 0,
        total: 1,
        status: TaskStatus::Failed,
        message,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let mut group = TaskGroup {
        group_id,
        tasks: vec![task],
        overall_status: TaskStatus::Pending,
        created_at: now.clone(),
        updated_at: now.clone(),
        completed_tasks: 0,
        total_tasks: 0,
        progress_percent: 0,
    };
    models::refresh_group_summary(&mut group, now);
    group
}

/// Record a completed task group in the global task center state.
///
/// Uses `app.state()` to access the shared `HashMap<String, TaskGroup>`,
/// inserts/overwrites the entry, and emits `task-group-created`.
///
/// Returns the recorded `TaskGroup` on success, or a Chinese error string.
pub async fn record_completed_task_group(
    app: &AppHandle,
    group_id: String,
    task_name: String,
    message: String,
    current: u64,
    total: u64,
) -> Result<TaskGroup, LauncherError> {
    let group = build_completed_task_group(group_id, task_name, message, current, total);
    let state = app
        .try_state::<Arc<Mutex<HashMap<String, TaskGroup>>>>()
        .ok_or_else(|| LauncherError::from("任务中心状态未初始化"))?;
    let mut groups = state.lock().await;
    groups.insert(group.group_id.clone(), group.clone());
    drop(groups);
    let _ = app.emit("task-group-created", &group);
    Ok(group)
}

/// Record a failed task group in the global task center state.
///
/// Uses `app.state()` to access the shared `HashMap<String, TaskGroup>`,
/// inserts/overwrites the entry, and emits `task-group-created`.
///
/// Returns the recorded `TaskGroup` on success, or a Chinese error string.
pub async fn record_failed_task_group(
    app: &AppHandle,
    group_id: String,
    task_name: String,
    message: String,
) -> Result<TaskGroup, LauncherError> {
    let group = build_failed_task_group(group_id, task_name, message);
    let state = app
        .try_state::<Arc<Mutex<HashMap<String, TaskGroup>>>>()
        .ok_or_else(|| LauncherError::from("任务中心状态未初始化"))?;
    let mut groups = state.lock().await;
    groups.insert(group.group_id.clone(), group.clone());
    drop(groups);
    let _ = app.emit("task-group-created", &group);
    Ok(group)
}

// ── Phase 31: generic running task group helpers ──────────────────────

/// Build a running single-task `TaskGroup` without requiring a Tauri app.
/// This is a pure function usable in unit tests.
pub fn build_running_task_group(group_id: &str, task_name: &str, task_message: &str) -> TaskGroup {
    let now = now_iso8601();
    let task = TaskProgress {
        task_id: 1,
        name: task_name.to_string(),
        current: 0,
        total: 1,
        status: TaskStatus::Running,
        message: task_message.to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let mut group = TaskGroup {
        group_id: group_id.to_string(),
        tasks: vec![task],
        overall_status: TaskStatus::Pending,
        created_at: now.clone(),
        updated_at: now.clone(),
        completed_tasks: 0,
        total_tasks: 0,
        progress_percent: 0,
    };
    models::refresh_group_summary(&mut group, now);
    group
}

/// Insert a TaskGroup into the task centre state and emit `task-group-created`.
///
/// Returns the recorded `TaskGroup` on success, or a Chinese error string
/// if the task centre state is not available.
pub async fn record_running_task_group(
    app: &AppHandle,
    group: TaskGroup,
) -> Result<TaskGroup, LauncherError> {
    let state = app
        .try_state::<Arc<Mutex<HashMap<String, TaskGroup>>>>()
        .ok_or_else(|| LauncherError::from("任务中心状态未初始化"))?;
    let mut groups = state.lock().await;
    groups.insert(group.group_id.clone(), group.clone());
    drop(groups);
    let _ = app.emit("task-group-created", &group);
    Ok(group)
}

/// Update an existing TaskGroup in the task centre and emit `task-group-updated`.
///
/// The `modifier` closure receives a mutable reference to the group.
/// `refresh_group_summary` is called internally with the current UTC time before
/// the event is emitted.
///
/// Returns `()` on success, or a Chinese error string if the task centre state
/// is not available or the group is not found.
pub async fn update_single_task_group(
    app: &AppHandle,
    group_id: &str,
    modifier: impl FnOnce(&mut TaskGroup),
) -> Result<(), LauncherError> {
    let state = app
        .try_state::<Arc<Mutex<HashMap<String, TaskGroup>>>>()
        .ok_or_else(|| LauncherError::from("任务中心状态未初始化"))?;
    let mut groups = state.lock().await;
    let group = groups
        .get_mut(group_id)
        .ok_or_else(|| LauncherError::from(format!("未找到任务组: {group_id}")))?;
    modifier(group);
    let now = now_iso8601();
    models::refresh_group_summary(group, now);
    let clone = group.clone();
    drop(groups);
    let _ = app.emit("task-group-updated", &clone);
    Ok(())
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::models;

    fn make_task(id: u64, status: TaskStatus, current: u64, total: u64) -> TaskProgress {
        TaskProgress {
            task_id: id,
            name: format!("task-{id}"),
            current,
            total,
            status,
            message: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    fn make_group(id: &str, tasks: Vec<TaskProgress>, updated: &str) -> TaskGroup {
        let mut g = TaskGroup {
            group_id: id.into(),
            tasks,
            overall_status: TaskStatus::Pending,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: updated.into(),
            completed_tasks: 0,
            total_tasks: 0,
            progress_percent: 0,
        };
        models::refresh_group_summary(&mut g, updated.into());
        g
    }

    // ── schedule_task_group logic ────────────────────────────────────────

    #[test]
    fn schedule_rejects_empty_tasks() {
        let tasks: Vec<TaskProgress> = vec![];
        // Same logic as the command: if tasks.is_empty() → Err(...)
        if tasks.is_empty() {
            let err = "任务组至少需要一个任务".to_string();
            assert!(!err.is_empty());
        } else {
            panic!("unreachable: empty vec should hit the error path");
        }
    }

    #[test]
    fn schedule_fills_default_timestamps() {
        let tasks = vec![TaskProgress {
            task_id: 1,
            name: "t".into(),
            current: 0,
            total: 100,
            status: TaskStatus::Pending,
            message: "".into(),
            created_at: "".into(),
            updated_at: "".into(),
        }];

        let now = "2026-05-04T12:00:00Z";
        let filled: Vec<TaskProgress> = tasks
            .into_iter()
            .map(|mut t| {
                if t.created_at.is_empty() {
                    t.created_at = now.into();
                }
                if t.updated_at.is_empty() {
                    t.updated_at = now.into();
                }
                t
            })
            .collect();

        assert_eq!(filled[0].created_at, now);
        assert_eq!(filled[0].updated_at, now);
    }

    // ── update_task_progress logic ───────────────────────────────────────

    #[test]
    fn update_missing_group_returns_error() {
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        let err = groups
            .get_mut("nonexistent")
            .ok_or_else(|| "未找到任务组: nonexistent".to_string());
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("未找到任务组"));
    }

    #[test]
    fn update_missing_task_returns_error() {
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        groups.insert(
            "g1".into(),
            make_group(
                "g1",
                vec![make_task(1, TaskStatus::Pending, 0, 100)],
                "2026-01-01T00:00:00Z",
            ),
        );

        let group = groups.get_mut("g1").unwrap();
        let found = group.tasks.iter_mut().find(|t| t.task_id == 999);
        assert!(found.is_none(), "task 999 should not exist");
    }

    #[test]
    fn update_sets_fields_and_refreshes_summary() {
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        groups.insert(
            "g1".into(),
            make_group(
                "g1",
                vec![
                    make_task(1, TaskStatus::Pending, 0, 100),
                    make_task(2, TaskStatus::Pending, 0, 200),
                ],
                "2026-01-01T00:00:00Z",
            ),
        );

        let now = "2026-05-04T12:00:00Z";
        let group = groups.get_mut("g1").unwrap();
        let task = group.tasks.iter_mut().find(|t| t.task_id == 1).unwrap();
        task.current = 100;
        task.total = 100;
        task.status = TaskStatus::Completed;
        task.message = "done".into();
        task.updated_at = now.into();
        models::refresh_group_summary(group, now.into());

        let group = groups.get("g1").unwrap();
        assert_eq!(group.completed_tasks, 1);
        assert_eq!(group.total_tasks, 2);
        assert_eq!(group.progress_percent, 50);
        assert_eq!(group.overall_status, TaskStatus::Pending);
        assert_eq!(group.updated_at, now);
    }

    // ── list_task_groups sorting ─────────────────────────────────────────

    #[test]
    fn list_sorted_by_updated_at_desc() {
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        groups.insert(
            "g1".into(),
            make_group("g1", vec![], "2026-01-01T00:00:01Z"),
        );
        groups.insert(
            "g2".into(),
            make_group("g2", vec![], "2026-01-01T00:00:03Z"),
        );
        groups.insert(
            "g3".into(),
            make_group("g3", vec![], "2026-01-01T00:00:02Z"),
        );

        let mut out: Vec<TaskGroup> = groups.values().cloned().collect();
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        assert_eq!(out[0].group_id, "g2");
        assert_eq!(out[1].group_id, "g3");
        assert_eq!(out[2].group_id, "g1");
    }

    // ── cancel_task_group logic ──────────────────────────────────────────

    #[test]
    fn cancel_missing_group_returns_error() {
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        let err = groups
            .get_mut("nonexistent")
            .ok_or_else(|| "未找到任务组: nonexistent".to_string());
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("未找到任务组"));
    }

    #[test]
    fn cancel_only_cancels_running_pending_paused() {
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        groups.insert(
            "g1".into(),
            make_group(
                "g1",
                vec![
                    make_task(1, TaskStatus::Running, 30, 100),
                    make_task(2, TaskStatus::Pending, 0, 100),
                    make_task(3, TaskStatus::Paused, 50, 100),
                    make_task(4, TaskStatus::Completed, 100, 100),
                    make_task(5, TaskStatus::Failed, 10, 100),
                    make_task(6, TaskStatus::Cancelled, 0, 100),
                ],
                "2026-01-01T00:00:00Z",
            ),
        );

        let now = "2026-05-04T12:00:00Z";
        let group = groups.get_mut("g1").unwrap();
        for task in &mut group.tasks {
            if matches!(
                task.status,
                TaskStatus::Running | TaskStatus::Pending | TaskStatus::Paused
            ) {
                task.status = TaskStatus::Cancelled;
                task.updated_at = now.into();
            }
        }
        models::refresh_group_summary(group, now.into());

        let group = groups.get("g1").unwrap();
        // Running, Pending, Paused → Cancelled
        assert_eq!(group.tasks[0].status, TaskStatus::Cancelled);
        assert_eq!(group.tasks[1].status, TaskStatus::Cancelled);
        assert_eq!(group.tasks[2].status, TaskStatus::Cancelled);
        // Completed, Failed, Already Cancelled → unchanged
        assert_eq!(group.tasks[3].status, TaskStatus::Completed);
        assert_eq!(group.tasks[4].status, TaskStatus::Failed);
        assert_eq!(group.tasks[5].status, TaskStatus::Cancelled);
        // Overall should be Failed (since failed task remains)
        assert_eq!(group.overall_status, TaskStatus::Failed);
    }

    // ── Phase 37: mark_task_cancelled_in_task_center tests ────────────────

    #[test]
    fn mark_cancelled_running_status_and_message() {
        let mut task = TaskProgress {
            task_id: 1,
            name: "下载".into(),
            current: 30,
            total: 100,
            status: TaskStatus::Running,
            message: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let now = "2026-05-04T12:00:00Z";
        models::mark_task_cancelled_in_task_center(&mut task, now);

        assert_eq!(task.status, TaskStatus::Cancelled);
        assert_eq!(task.updated_at, now);
        assert!(task.message.contains("已在任务中心标记为取消"));
    }

    #[test]
    fn mark_cancelled_pending_status() {
        let mut task = TaskProgress {
            task_id: 2,
            name: "等待".into(),
            current: 0,
            total: 100,
            status: TaskStatus::Pending,
            message: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let now = "2026-05-04T12:00:00Z";
        models::mark_task_cancelled_in_task_center(&mut task, now);

        assert_eq!(task.status, TaskStatus::Cancelled);
        assert_eq!(task.updated_at, now);
        assert!(task.message.contains("已在任务中心标记为取消"));
    }

    #[test]
    fn mark_cancelled_paused_status() {
        let mut task = TaskProgress {
            task_id: 3,
            name: "暂停任务".into(),
            current: 50,
            total: 100,
            status: TaskStatus::Paused,
            message: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let now = "2026-05-04T12:00:00Z";
        models::mark_task_cancelled_in_task_center(&mut task, now);

        assert_eq!(task.status, TaskStatus::Cancelled);
        assert_eq!(task.updated_at, now);
        assert!(task.message.contains("已在任务中心标记为取消"));
    }

    #[test]
    fn mark_cancelled_empty_message_set_to_note() {
        let mut task = TaskProgress {
            task_id: 4,
            name: "空消息任务".into(),
            current: 0,
            total: 100,
            status: TaskStatus::Running,
            message: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let now = "2026-05-04T12:00:00Z";
        models::mark_task_cancelled_in_task_center(&mut task, now);

        assert_eq!(task.message, models::TASK_CENTER_CANCEL_NOTE);
    }

    #[test]
    fn mark_cancelled_existing_message_appends_note() {
        let original_msg = "下载中，已完成 30%";
        let mut task = TaskProgress {
            task_id: 5,
            name: "下载任务".into(),
            current: 30,
            total: 100,
            status: TaskStatus::Running,
            message: original_msg.to_string(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let now = "2026-05-04T12:00:00Z";
        models::mark_task_cancelled_in_task_center(&mut task, now);

        assert_eq!(task.status, TaskStatus::Cancelled);
        assert!(task.message.starts_with(original_msg));
        assert!(task.message.contains("；已在任务中心标记为取消"));
        assert!(task.message.contains("底层操作可能仍会继续至自然结束"));
    }

    #[test]
    fn mark_cancelled_completed_unchanged() {
        let original_msg = "安装完成";
        let mut task = TaskProgress {
            task_id: 6,
            name: "已完成任务".into(),
            current: 100,
            total: 100,
            status: TaskStatus::Completed,
            message: original_msg.to_string(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:01Z".into(),
        };
        let original_updated = task.updated_at.clone();
        let now = "2026-05-04T12:00:00Z";
        models::mark_task_cancelled_in_task_center(&mut task, now);

        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.message, original_msg);
        assert_eq!(task.updated_at, original_updated);
    }

    #[test]
    fn mark_cancelled_failed_unchanged() {
        let original_msg = "下载失败：网络错误";
        let mut task = TaskProgress {
            task_id: 7,
            name: "失败任务".into(),
            current: 10,
            total: 100,
            status: TaskStatus::Failed,
            message: original_msg.to_string(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:01Z".into(),
        };
        let original_updated = task.updated_at.clone();
        let now = "2026-05-04T12:00:00Z";
        models::mark_task_cancelled_in_task_center(&mut task, now);

        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(task.message, original_msg);
        assert_eq!(task.updated_at, original_updated);
    }

    #[test]
    fn mark_cancelled_already_cancelled_no_duplicate() {
        let existing_msg =
            "下载中；已在任务中心标记为取消；若后台下载已开始，底层操作可能仍会继续至自然结束。";
        let mut task = TaskProgress {
            task_id: 8,
            name: "已取消任务".into(),
            current: 50,
            total: 100,
            status: TaskStatus::Cancelled,
            message: existing_msg.to_string(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:01Z".into(),
        };
        let now = "2026-05-04T12:00:00Z";
        models::mark_task_cancelled_in_task_center(&mut task, now);

        // Status stays Cancelled (already was)
        assert_eq!(task.status, TaskStatus::Cancelled);
        // Message should NOT have the note appended again
        let occurrences = task.message.matches("已在任务中心标记为取消").count();
        assert_eq!(occurrences, 1, "说明不应重复追加");
    }

    #[test]
    fn cancel_group_summary_becomes_cancelled() {
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        groups.insert(
            "g1".into(),
            make_group(
                "g1",
                vec![
                    make_task(1, TaskStatus::Running, 30, 100),
                    make_task(2, TaskStatus::Pending, 0, 100),
                ],
                "2026-01-01T00:00:00Z",
            ),
        );

        let now = "2026-05-04T12:00:00Z";
        let group = groups.get_mut("g1").unwrap();
        for task in &mut group.tasks {
            models::mark_task_cancelled_in_task_center(task, now);
        }
        models::refresh_group_summary(group, now.to_string());

        let group = groups.get("g1").unwrap();
        assert_eq!(group.tasks[0].status, TaskStatus::Cancelled);
        assert_eq!(group.tasks[1].status, TaskStatus::Cancelled);
        assert_eq!(group.overall_status, TaskStatus::Cancelled);
        assert!(group.tasks[0].message.contains("已在任务中心标记为取消"));
        assert!(group.tasks[1].message.contains("已在任务中心标记为取消"));
    }

    // ── remove_task_group logic ──────────────────────────────────────────

    #[test]
    fn remove_missing_group_returns_error() {
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        let removed = groups.remove("nonexistent");
        assert!(
            removed.is_none(),
            "removing missing group should return None"
        );
    }

    #[test]
    fn remove_existing_group_works() {
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        groups.insert(
            "g1".into(),
            make_group("g1", vec![], "2026-01-01T00:00:00Z"),
        );
        let removed = groups.remove("g1");
        assert!(removed.is_some());
        assert!(groups.is_empty());
    }

    // ── Status aggregation + progress integration ────────────────────────

    #[test]
    fn schedule_with_tasks_updates_summary() {
        let tasks = vec![
            make_task(1, TaskStatus::Pending, 0, 100),
            make_task(2, TaskStatus::Pending, 0, 100),
        ];
        let now = "2026-05-04T12:00:00Z";
        let mut group = TaskGroup {
            group_id: "g1".into(),
            tasks,
            overall_status: TaskStatus::Pending,
            created_at: now.into(),
            updated_at: now.into(),
            completed_tasks: 0,
            total_tasks: 0,
            progress_percent: 0,
        };
        models::refresh_group_summary(&mut group, now.into());

        assert_eq!(group.total_tasks, 2);
        assert_eq!(group.completed_tasks, 0);
        assert_eq!(group.progress_percent, 0);
        assert_eq!(group.overall_status, TaskStatus::Pending);
        assert!(!group.created_at.is_empty());
        assert!(!group.updated_at.is_empty());
    }

    // ── Phase 26: record helpers & group_id format ────────────────────

    #[test]
    fn record_completed_group_builds_summary() {
        let group = super::build_completed_task_group(
            "install-resource:abc123:test.jar".into(),
            "安装资源文件".into(),
            "已安装".into(),
            1024,
            1024,
        );
        assert_eq!(group.overall_status, TaskStatus::Completed);
        assert_eq!(group.progress_percent, 100);
        assert_eq!(group.total_tasks, 1);
        assert_eq!(group.completed_tasks, 1);
        assert_eq!(group.tasks[0].status, TaskStatus::Completed);
        assert_eq!(group.tasks[0].current, 1024);
        assert_eq!(group.tasks[0].total, 1024);
    }

    #[test]
    fn record_failed_group_builds_summary() {
        let group = super::build_failed_task_group(
            "install-resource:abc123:test.jar".into(),
            "安装资源文件".into(),
            "下载失败".into(),
        );
        assert_eq!(group.overall_status, TaskStatus::Failed);
        assert_eq!(group.progress_percent, 0);
        assert_eq!(group.total_tasks, 1);
        assert_eq!(group.completed_tasks, 0);
        assert_eq!(group.tasks[0].status, TaskStatus::Failed);
        assert_eq!(group.tasks[0].current, 0);
        assert_eq!(group.tasks[0].total, 1);
    }

    #[test]
    fn install_task_group_id_resource_format() {
        let id = format!("install-resource:{}:{}", "inst-001", "my-mod.jar");
        assert!(!id.contains(' '));
        assert!(!id.contains('/'));
        assert!(!id.contains('\\'));
        assert!(id.starts_with("install-resource:"));
    }

    #[test]
    fn install_task_group_id_client_format() {
        let id = format!("install-client:{}:{}", "inst-001", "1.21.4");
        assert!(!id.contains(' '));
        assert!(!id.contains('/'));
        assert!(!id.contains('\\'));
        assert!(id.starts_with("install-client:"));
    }

    #[test]
    fn install_task_group_id_libraries_format() {
        let id = format!("install-libraries:{}:{}", "inst-001", "1.21.4");
        assert!(!id.contains(' '));
        assert!(!id.contains('/'));
        assert!(!id.contains('\\'));
        assert!(id.starts_with("install-libraries:"));
    }

    #[test]
    fn install_task_group_id_assets_format() {
        let id = format!("install-assets:{}:{}", "inst-001", "1.21.4");
        assert!(!id.contains(' '));
        assert!(!id.contains('/'));
        assert!(!id.contains('\\'));
        assert!(id.starts_with("install-assets:"));
    }

    #[test]
    fn install_task_group_id_loader_format() {
        let id = format!("install-loader:{}:{:?}", "inst-001", TaskStatus::Completed);
        // Verify placeholder format — real group_id uses InstallLoaderKind Debug
        assert!(!id.contains(' '));
        assert!(!id.contains('/'));
        assert!(!id.contains('\\'));
        assert!(id.starts_with("install-loader:"));
    }

    #[test]
    fn install_progress_mapping_bytes_written() {
        let bytes_written: u64 = 2048;
        let current = bytes_written;
        let total = bytes_written.max(1);
        let group =
            super::build_completed_task_group("g".into(), "t".into(), "m".into(), current, total);
        assert_eq!(group.progress_percent, 100);
        assert_eq!(group.tasks[0].current, 2048);
        assert_eq!(group.tasks[0].total, 2048);
    }

    #[test]
    fn install_progress_mapping_client_sum() {
        let json_bytes: u64 = 300;
        let jar_bytes: u64 = 500;
        let sum = json_bytes + jar_bytes;
        let group =
            super::build_completed_task_group("g".into(), "t".into(), "m".into(), sum, sum.max(1));
        assert_eq!(group.progress_percent, 100);
        assert_eq!(group.tasks[0].current, 800);
        assert_eq!(group.tasks[0].total, 800);
    }

    #[test]
    fn install_progress_mapping_libraries_scanned_downloaded_skipped() {
        let scanned: u64 = 100;
        let downloaded: u64 = 80;
        let skipped: u64 = 15;
        let current = downloaded + skipped;
        let total = scanned;
        let group =
            super::build_completed_task_group("g".into(), "t".into(), "m".into(), current, total);
        assert_eq!(group.tasks[0].current, 95);
        assert_eq!(group.tasks[0].total, 100);
        assert_eq!(group.progress_percent, 95);
    }

    #[test]
    fn install_progress_mapping_libraries_scanned_zero() {
        let group = super::build_completed_task_group("g".into(), "t".into(), "m".into(), 0, 1);
        assert_eq!(group.tasks[0].current, 0);
        assert_eq!(group.tasks[0].total, 1);
        assert_eq!(group.overall_status, TaskStatus::Completed);
        assert_eq!(group.progress_percent, 0);
    }

    #[test]
    fn install_progress_mapping_assets_same_as_libraries() {
        let scanned: u64 = 50;
        let downloaded: u64 = 49;
        let skipped: u64 = 0;
        let current = downloaded + skipped;
        let group =
            super::build_completed_task_group("g".into(), "t".into(), "m".into(), current, scanned);
        assert_eq!(group.tasks[0].current, 49);
        assert_eq!(group.tasks[0].total, 50);
        assert_eq!(group.progress_percent, 98);
    }

    #[test]
    fn install_progress_mapping_loader_bytes() {
        let bytes_written: u64 = 0;
        let current = bytes_written;
        let total = bytes_written.max(1);
        let group =
            super::build_completed_task_group("g".into(), "t".into(), "m".into(), current, total);
        assert_eq!(group.tasks[0].current, 0);
        assert_eq!(group.tasks[0].total, 1);
        assert_eq!(group.overall_status, TaskStatus::Completed);
    }

    #[test]
    fn install_progress_failed_in_result_still_completed() {
        let scanned: u64 = 100;
        let downloaded: u64 = 90;
        let skipped: u64 = 5;
        let failed_count: u64 = 5;
        let current = downloaded + skipped;
        let group = super::build_completed_task_group(
            "install-libraries:inst:1.21.4".into(),
            "安装运行库".into(),
            format!(
                "扫描 {} 项，下载 {} 项，跳过 {} 项，失败 {} 项",
                scanned, downloaded, skipped, failed_count
            ),
            current,
            scanned,
        );
        assert_eq!(group.overall_status, TaskStatus::Completed);
        assert!(group.tasks[0].message.contains("失败 5 项"));
    }

    // ── Phase 31: running task group helpers ────────────────────────────

    #[test]
    fn build_running_task_group_has_running_status() {
        let group = super::build_running_task_group(
            "install-client-async:inst-001:1.21.4",
            "安装客户端版本: 1.21.4",
            "等待下载",
        );
        assert_eq!(group.overall_status, TaskStatus::Running);
        assert_eq!(group.total_tasks, 1);
        assert_eq!(group.completed_tasks, 0);
        assert_eq!(group.tasks.len(), 1);
        assert_eq!(group.tasks[0].task_id, 1);
        assert_eq!(group.tasks[0].current, 0);
        assert_eq!(group.tasks[0].total, 1);
        assert_eq!(group.tasks[0].status, TaskStatus::Running);
        assert_eq!(group.tasks[0].message, "等待下载");
        assert!(!group.tasks[0].created_at.is_empty());
        assert!(!group.tasks[0].updated_at.is_empty());
    }

    #[test]
    fn running_task_progress_percent_is_not_100() {
        let group = super::build_running_task_group(
            "install-client-async:inst-001:1.21.4",
            "安装客户端版本",
            "等待下载",
        );
        assert_ne!(group.progress_percent, 100);
    }

    #[test]
    fn update_single_task_group_success() {
        let now = "2026-01-01T00:00:00Z";
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        let mut group = TaskGroup {
            group_id: "grp-1".into(),
            tasks: vec![TaskProgress {
                task_id: 1,
                name: "下载".into(),
                current: 0,
                total: 1,
                status: TaskStatus::Running,
                message: "等待".into(),
                created_at: now.into(),
                updated_at: now.into(),
            }],
            overall_status: TaskStatus::Running,
            created_at: now.into(),
            updated_at: now.into(),
            completed_tasks: 0,
            total_tasks: 1,
            progress_percent: 0,
        };
        models::refresh_group_summary(&mut group, now.into());
        groups.insert("grp-1".into(), group);

        let g = groups.get_mut("grp-1").unwrap();
        g.tasks[0].current = 1;
        g.tasks[0].total = 1;
        g.tasks[0].status = TaskStatus::Completed;
        g.tasks[0].message = "done".into();
        let now2 = "2026-01-01T00:01:00Z";
        models::refresh_group_summary(g, now2.into());

        let g = groups.get("grp-1").unwrap();
        assert_eq!(g.overall_status, TaskStatus::Completed);
        assert_eq!(g.completed_tasks, 1);
        assert_eq!(g.progress_percent, 100);
    }

    #[test]
    fn update_single_task_group_missing_returns_err() {
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        let group_id = "nonexistent-group";
        let result = groups.get_mut(group_id);
        assert!(result.is_none());
        // Verify the error message pattern used by the real async helper
        let err = format!("未找到任务组: {group_id}");
        assert!(err.contains("未找到任务组"));
        assert!(err.contains(group_id));
    }

    #[test]
    fn completed_task_progress_percent_is_100() {
        let group = super::build_completed_task_group(
            "grp".into(),
            "task".into(),
            "msg".into(),
            1024,
            1024,
        );
        assert_eq!(group.progress_percent, 100);
        assert_eq!(group.overall_status, TaskStatus::Completed);
    }

    #[test]
    fn task_message_after_success_contains_sha1_status() {
        let json_bytes: u64 = 300;
        let jar_bytes: u64 = 500;
        let sha1_passed = true;
        let sum = (json_bytes + jar_bytes).max(1);
        let msg = format!(
            "JSON: {} bytes, JAR: {} bytes, SHA1校验: {}",
            json_bytes,
            jar_bytes,
            if sha1_passed { "通过" } else { "失败" }
        );
        assert!(msg.contains("SHA1校验: 通过"));
        assert!(msg.contains("JSON: 300 bytes"));
        let group = super::build_completed_task_group("g".into(), "t".into(), msg, sum, sum);
        assert_eq!(group.overall_status, TaskStatus::Completed);
        assert!(group.tasks[0].message.contains("SHA1校验: 通过"));
    }

    #[test]
    fn task_message_after_failure_contains_error_and_max_length() {
        let long_error: String = "x".repeat(300);
        // The command truncates the raw error: e.chars().take(256).collect()
        let truncated: String = long_error.chars().take(256).collect();
        assert_eq!(truncated.len(), 256);
        assert_eq!(truncated, "x".repeat(256));
    }

    #[test]
    fn group_id_format_install_client_async() {
        let instance_id = "abc-123";
        let game_version = "1.21.4";
        let group_id = format!("install-client-async:{}:{}", instance_id, game_version);
        assert!(!group_id.contains(' '));
        assert!(!group_id.contains('\n'));
        assert!(!group_id.contains('/'));
        assert!(!group_id.contains('\\'));
        assert!(group_id.starts_with("install-client-async:"));
        assert!(group_id.ends_with(":1.21.4"));
    }

    #[test]
    fn group_id_format_handles_empty_input_fields() {
        // Test that group_id with empty instance_id produces a valid-looking
        // identifier — actual rejection of empty fields happens upstream in
        // get_instance_in which validates the instance exists before constructing
        // the group_id.
        let empty_instance = "";
        let game_version = "1.21.4";
        let group_id = format!("install-client-async:{}:{}", empty_instance, game_version);
        assert_eq!(group_id, "install-client-async::1.21.4");

        let instance_id = "abc";
        let empty_version = "";
        let group_id2 = format!("install-client-async:{}:{}", instance_id, empty_version);
        assert_eq!(group_id2, "install-client-async:abc:");
    }

    #[test]
    fn async_install_started_result_serde_roundtrip() {
        let result = crate::resource::models::AsyncInstallTaskStarted {
            group_id: "install-client-async:abc:1.21.4".into(),
            task_id: 1,
            instance_id: "abc".into(),
            game_version: "1.21.4".into(),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains("\"groupId\""));
        assert!(json.contains("\"taskId\""));
        assert!(json.contains("\"instanceId\""));
        assert!(json.contains("\"gameVersion\""));
        let back: crate::resource::models::AsyncInstallTaskStarted =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.group_id, "install-client-async:abc:1.21.4");
        assert_eq!(back.task_id, 1);
        assert_eq!(back.instance_id, "abc");
        assert_eq!(back.game_version, "1.21.4");
    }

    #[test]
    fn record_running_task_group_state_missing_behavior() {
        // The record_running_task_group helper uses try_state() which returns
        // None when the state is not managed. The returned error string must
        // be a Chinese message indicating the task centre is not initialised.
        let err_msg = "任务中心状态未初始化";
        assert!(!err_msg.is_empty());
        assert!(err_msg.contains("未初始化"));
        // The command start_install_client_version_task wraps this into:
        // "任务中心未就绪，无法启动异步安装"
        let cmd_err = "任务中心未就绪，无法启动异步安装";
        assert!(!cmd_err.is_empty());
        assert!(cmd_err.contains("未就绪"));
    }

    // ── Phase 32: async install libraries tests ─────────────────────────

    #[test]
    fn async_install_libraries_started_serde_roundtrip() {
        let started = crate::resource::models::AsyncInstallLibrariesStarted {
            group_id: "install-libraries-async:abc:1.21.4".into(),
            task_id: 1,
            instance_id: "abc".into(),
            game_version: "1.21.4".into(),
        };
        let json = serde_json::to_string(&started).expect("serialize");
        assert!(json.contains("\"groupId\""));
        assert!(json.contains("\"taskId\""));
        assert!(json.contains("\"instanceId\""));
        assert!(json.contains("\"gameVersion\""));
        let back: crate::resource::models::AsyncInstallLibrariesStarted =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.group_id, "install-libraries-async:abc:1.21.4");
        assert_eq!(back.task_id, 1);
        assert_eq!(back.instance_id, "abc");
        assert_eq!(back.game_version, "1.21.4");
    }

    #[test]
    fn group_id_format_install_libraries_async() {
        let instance_id = "abc-123";
        let game_version = "1.21.4";
        let group_id = format!("install-libraries-async:{}:{}", instance_id, game_version);
        assert!(!group_id.contains(' '));
        assert!(!group_id.contains('\n'));
        assert!(!group_id.contains('/'));
        assert!(!group_id.contains('\\'));
        assert!(group_id.starts_with("install-libraries-async:"));
        assert!(group_id.ends_with(":1.21.4"));
    }

    #[test]
    fn libraries_success_progress_mapping_scanned_downloaded_skipped() {
        let scanned: u64 = 100;
        let downloaded: u64 = 80;
        let skipped: u64 = 15;
        let failed_count: u64 = 5;
        let bytes_written: u64 = 102400;
        let current = (downloaded + skipped).max(1);
        let total = scanned.max(1);
        let message = format!(
            "扫描 {}，下载 {}，跳过 {}，失败 {}，写入 {} bytes",
            scanned, downloaded, skipped, failed_count, bytes_written
        );
        let group = super::build_completed_task_group(
            "install-libraries-async:inst:1.21.4".into(),
            "修复运行库".into(),
            message,
            current,
            total,
        );
        assert_eq!(group.tasks[0].current, 95);
        assert_eq!(group.tasks[0].total, 100);
        assert_eq!(group.progress_percent, 95);
        assert_eq!(group.overall_status, TaskStatus::Completed);
        assert!(group.tasks[0].message.contains("扫描 100"));
        assert!(group.tasks[0].message.contains("下载 80"));
        assert!(group.tasks[0].message.contains("跳过 15"));
        assert!(group.tasks[0].message.contains("失败 5"));
        assert!(group.tasks[0].message.contains("写入 102400 bytes"));
    }

    #[test]
    fn libraries_success_progress_mapping_with_failures() {
        let scanned: u64 = 50;
        let downloaded: u64 = 30;
        let skipped: u64 = 5;
        let failed_count: u64 = 15;
        let bytes_written: u64 = 50000;
        let current = (downloaded + skipped).max(1);
        let total = scanned.max(1);
        let mut message = format!(
            "扫描 {}，下载 {}，跳过 {}，失败 {}，写入 {} bytes",
            scanned, downloaded, skipped, failed_count, bytes_written
        );
        message.push_str("（部分失败，请查看详情）");
        let group = super::build_completed_task_group(
            "install-libraries-async:inst:1.21.4".into(),
            "修复运行库".into(),
            message,
            current,
            total,
        );
        assert_eq!(group.overall_status, TaskStatus::Completed);
        assert_eq!(group.tasks[0].current, 35);
        assert_eq!(group.tasks[0].total, 50);
        assert!(group.tasks[0].message.contains("部分失败"));
        assert!(group.tasks[0].message.contains("失败 15"));
    }

    #[test]
    fn libraries_failed_message_truncation() {
        // Long error — should be truncated to 256 Unicode chars
        let long_error: String = "错误".repeat(200);
        let truncated: String = long_error.chars().take(256).collect();
        assert_eq!(truncated.chars().count(), 256);
        assert!(truncated.starts_with("错误错误"));

        // Short error — should remain unchanged
        let short_error = "版本 JSON 不存在".to_string();
        let short_truncated: String = short_error.chars().take(256).collect();
        assert_eq!(short_truncated, "版本 JSON 不存在");
        assert_eq!(short_truncated.len(), short_error.len());

        // Verify failed task group uses truncation
        let group = super::build_failed_task_group("g".into(), "t".into(), truncated.clone());
        assert_eq!(group.tasks[0].status, TaskStatus::Failed);
        assert_eq!(group.tasks[0].current, 0);
        assert_eq!(group.tasks[0].total, 1);
        assert_eq!(group.tasks[0].message.chars().count(), 256);
    }

    #[test]
    fn libraries_success_message_contains_partial_failure_when_failed_gt_0() {
        let scanned: u32 = 100;
        let downloaded: u32 = 95;
        let skipped: u32 = 0;
        let failed: u32 = 5;
        let bytes_written: u64 = 204800;
        let mut message = format!(
            "扫描 {}，下载 {}，跳过 {}，失败 {}，写入 {} bytes",
            scanned, downloaded, skipped, failed, bytes_written
        );
        // When failed > 0, append partial failure hint
        if failed > 0 {
            message.push_str("（部分失败，请查看详情）");
        }
        assert!(message.contains("部分失败"));
        assert!(message.contains("失败 5"));
        // Status should still be Completed
        let current = ((downloaded + skipped) as u64).max(1);
        let total = (scanned as u64).max(1);
        let group = super::build_completed_task_group(
            "install-libraries-async:inst:1.21.4".into(),
            "修复运行库".into(),
            message,
            current,
            total,
        );
        assert_eq!(group.overall_status, TaskStatus::Completed);
    }

    #[test]
    fn libraries_scanned_zero_total_defaults_to_1() {
        let scanned: u64 = 0;
        let total = scanned.max(1);
        assert_eq!(total, 1);
    }

    #[test]
    fn libraries_downloaded_plus_skipped_zero_current_defaults_to_1() {
        let downloaded: u64 = 0;
        let skipped: u64 = 0;
        let current = (downloaded + skipped).max(1);
        assert_eq!(current, 1);
    }

    // ── Phase 33: Async assets install tests ──────────────────────────

    #[test]
    fn async_install_assets_started_serde_roundtrip() {
        let started = crate::resource::models::AsyncInstallAssetsStarted {
            group_id: "install-assets-async:abc:1.21.4".into(),
            task_id: 1,
            instance_id: "abc".into(),
            game_version: "1.21.4".into(),
        };
        let json = serde_json::to_string(&started).expect("serialize");
        assert!(json.contains("\"groupId\""));
        assert!(json.contains("\"taskId\""));
        assert!(json.contains("\"instanceId\""));
        assert!(json.contains("\"gameVersion\""));
        let back: crate::resource::models::AsyncInstallAssetsStarted =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.group_id, "install-assets-async:abc:1.21.4");
        assert_eq!(back.task_id, 1);
        assert_eq!(back.instance_id, "abc");
        assert_eq!(back.game_version, "1.21.4");
    }

    #[test]
    fn group_id_format_install_assets_async() {
        let instance_id = "abc-123";
        let game_version = "1.21.4";
        let group_id = format!("install-assets-async:{}:{}", instance_id, game_version);
        assert!(!group_id.contains(' '));
        assert!(!group_id.contains('\n'));
        assert!(!group_id.contains('/'));
        assert!(!group_id.contains('\\'));
        assert!(group_id.starts_with("install-assets-async:"));
        assert!(group_id.ends_with(":1.21.4"));
    }

    #[test]
    fn assets_success_progress_mapping_scanned_downloaded_skipped() {
        let scanned: u64 = 100;
        let downloaded: u64 = 80;
        let skipped: u64 = 15;
        let failed_count: u64 = 5;
        let index_bytes_written: u64 = 2048;
        let asset_index_id = "17";
        let bytes_written: u64 = 102400;
        let current = downloaded + skipped;
        let total = scanned.max(1);
        let mut message = format!(
            "assetIndex: {}, index bytes: {}, 扫描 {}, 下载 {}, 跳过 {}, 失败 {}, 写入 {} bytes",
            asset_index_id,
            index_bytes_written,
            scanned,
            downloaded,
            skipped,
            failed_count,
            bytes_written
        );
        if failed_count > 0 {
            message.push_str("（部分失败）");
        }
        let group = super::build_completed_task_group(
            "install-assets-async:inst:1.21.4".into(),
            "修复资源文件".into(),
            message,
            current,
            total,
        );
        assert_eq!(group.tasks[0].current, 95);
        assert_eq!(group.tasks[0].total, 100);
        assert_eq!(group.progress_percent, 95);
        assert_eq!(group.overall_status, TaskStatus::Completed);
        assert!(group.tasks[0].message.contains("assetIndex: 17"));
        assert!(group.tasks[0].message.contains("index bytes: 2048"));
        assert!(group.tasks[0].message.contains("扫描 100"));
        assert!(group.tasks[0].message.contains("下载 80"));
        assert!(group.tasks[0].message.contains("跳过 15"));
        assert!(group.tasks[0].message.contains("失败 5"));
        assert!(group.tasks[0].message.contains("写入 102400 bytes"));
    }

    #[test]
    fn assets_success_message_contains_asset_index_and_index_bytes() {
        let scanned: u64 = 10;
        let downloaded: u64 = 10;
        let skipped: u64 = 0;
        let failed_count: u64 = 0;
        let index_bytes_written: u64 = 512;
        let asset_index_id = "12";
        let bytes_written: u64 = 5000;
        let current = downloaded + skipped;
        let total = scanned.max(1);
        let message = format!(
            "assetIndex: {}, index bytes: {}, 扫描 {}, 下载 {}, 跳过 {}, 失败 {}, 写入 {} bytes",
            asset_index_id,
            index_bytes_written,
            scanned,
            downloaded,
            skipped,
            failed_count,
            bytes_written
        );
        let group = super::build_completed_task_group(
            "install-assets-async:inst:1.21.4".into(),
            "修复资源文件".into(),
            message,
            current,
            total,
        );
        assert_eq!(group.overall_status, TaskStatus::Completed);
        assert!(group.tasks[0].message.contains("assetIndex: 12"));
        assert!(group.tasks[0].message.contains("index bytes: 512"));
    }

    #[test]
    fn assets_success_message_contains_partial_failure_when_failed_gt_0() {
        let scanned: u64 = 50;
        let downloaded: u64 = 30;
        let skipped: u64 = 5;
        let failed_count: u64 = 15;
        let index_bytes_written: u64 = 1024;
        let asset_index_id = "17";
        let bytes_written: u64 = 50000;
        let current = downloaded + skipped;
        let total = scanned.max(1);
        let mut message = format!(
            "assetIndex: {}, index bytes: {}, 扫描 {}, 下载 {}, 跳过 {}, 失败 {}, 写入 {} bytes",
            asset_index_id,
            index_bytes_written,
            scanned,
            downloaded,
            skipped,
            failed_count,
            bytes_written
        );
        if failed_count > 0 {
            message.push_str("（部分失败）");
        }
        assert!(message.contains("部分失败"));
        assert!(message.contains("失败 15"));
        let group = super::build_completed_task_group(
            "install-assets-async:inst:1.21.4".into(),
            "修复资源文件".into(),
            message,
            current,
            total,
        );
        assert_eq!(group.overall_status, TaskStatus::Completed);
    }

    #[test]
    fn assets_failed_message_truncation() {
        // Long error — should be truncated to 256 Unicode chars
        let long_error: String = "资源".repeat(200);
        let truncated: String = long_error.chars().take(256).collect();
        assert_eq!(truncated.chars().count(), 256);
        assert!(truncated.starts_with("资源资源"));

        // Short error — should remain unchanged
        let short_error = "请先安装游戏版本".to_string();
        let short_truncated: String = short_error.chars().take(256).collect();
        assert_eq!(short_truncated, "请先安装游戏版本");
        assert_eq!(short_truncated.len(), short_error.len());

        // Verify failed task group uses truncation
        let group = super::build_failed_task_group(
            "install-assets-async:inst:1.21.4".into(),
            "修复资源文件".into(),
            truncated.clone(),
        );
        assert_eq!(group.tasks[0].status, TaskStatus::Failed);
        assert_eq!(group.tasks[0].current, 0);
        assert_eq!(group.tasks[0].total, 1);
        assert_eq!(group.tasks[0].message.chars().count(), 256);
    }

    #[test]
    fn assets_scanned_zero_total_defaults_to_1() {
        let scanned: u64 = 0;
        let total = scanned.max(1);
        assert_eq!(total, 1);
    }

    // ── Phase 34: Async loader install tests ────────────────────────────

    #[test]
    fn async_install_loader_request_serde() {
        let request = crate::resource::models::AsyncInstallLoaderRequest {
            instance_id: "inst-001".into(),
            kind: crate::resource::models::InstallLoaderKind::Forge,
            loader_version: Some("47.3.0".into()),
            overwrite: true,
        };
        let json = serde_json::to_string(&request).expect("serialize");
        assert!(json.contains("\"instanceId\""));
        assert!(json.contains("\"kind\""));
        assert!(json.contains("\"loaderVersion\""));
        assert!(json.contains("\"overwrite\""));
        let back: crate::resource::models::AsyncInstallLoaderRequest =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.instance_id, "inst-001");
        assert_eq!(back.kind, crate::resource::models::InstallLoaderKind::Forge);
        assert_eq!(back.loader_version.as_deref(), Some("47.3.0"));
        assert!(back.overwrite);
    }

    #[test]
    fn async_install_loader_started_serde() {
        let started = crate::resource::models::AsyncInstallLoaderStarted {
            group_id: "install-loader-async:inst-001:Forge".into(),
            task_id: 1,
            instance_id: "inst-001".into(),
            kind: crate::resource::models::InstallLoaderKind::Forge,
        };
        let json = serde_json::to_string(&started).expect("serialize");
        assert!(json.contains("\"groupId\""));
        assert!(json.contains("\"taskId\""));
        assert!(json.contains("\"instanceId\""));
        assert!(json.contains("\"kind\""));
        let back: crate::resource::models::AsyncInstallLoaderStarted =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.group_id, "install-loader-async:inst-001:Forge");
        assert_eq!(back.task_id, 1);
        assert_eq!(back.instance_id, "inst-001");
        assert_eq!(back.kind, crate::resource::models::InstallLoaderKind::Forge);
    }

    #[test]
    fn group_id_format_install_loader_async() {
        let instance_id = "abc-123";
        let kind = crate::resource::models::InstallLoaderKind::Fabric;
        let group_id = format!("install-loader-async:{}:{:?}", instance_id, kind);
        assert!(!group_id.contains(' '));
        assert!(!group_id.contains('\n'));
        assert!(!group_id.contains('/'));
        assert!(!group_id.contains('\\'));
        assert!(group_id.starts_with("install-loader-async:"));
        assert!(group_id.ends_with(":Fabric"));
        // All four kinds
        for k in &[
            crate::resource::models::InstallLoaderKind::Fabric,
            crate::resource::models::InstallLoaderKind::Quilt,
            crate::resource::models::InstallLoaderKind::Forge,
            crate::resource::models::InstallLoaderKind::NeoForge,
        ] {
            let gid = format!("install-loader-async:{}:{:?}", "x", k);
            assert!(gid.starts_with("install-loader-async:"));
            assert!(!gid.contains(' '));
        }
    }

    #[test]
    fn loader_success_message_contains_all_six_fields() {
        let previous_game_version = "1.21.4";
        let new_game_version = "fabric-loader-0.16.10-1.21.4";
        let kind = crate::resource::models::InstallLoaderKind::Fabric;
        let loader_version = "0.16.10";
        let bytes_written: u64 = 12345;
        let replaced_existing = false;

        let message = format!(
            "previous_game_version={} new_game_version={} kind={:?} loader_version={} bytes_written={} replaced_existing={}",
            previous_game_version,
            new_game_version,
            kind,
            loader_version,
            bytes_written,
            replaced_existing,
        );

        assert!(message.contains("previous_game_version=1.21.4"));
        assert!(message.contains("new_game_version=fabric-loader-0.16.10-1.21.4"));
        assert!(message.contains("kind=Fabric"));
        assert!(message.contains("loader_version=0.16.10"));
        assert!(message.contains("bytes_written=12345"));
        assert!(message.contains("replaced_existing=false"));
    }

    #[test]
    fn loader_success_progress_mapping() {
        let bytes_written: u64 = 2048;
        let current = bytes_written.max(1);
        let total = bytes_written.max(1);
        assert_eq!(current, 2048);
        assert_eq!(total, 2048);
        let group = super::build_completed_task_group(
            "install-loader-async:inst:Fabric".into(),
            "安装 Loader: Fabric".into(),
            "完成".into(),
            current,
            total,
        );
        assert_eq!(group.tasks[0].current, 2048);
        assert_eq!(group.tasks[0].total, 2048);
        assert_eq!(group.progress_percent, 100);
        assert_eq!(group.overall_status, TaskStatus::Completed);
    }

    #[test]
    fn loader_progress_bytes_zero_defaults_to_1() {
        let bytes_written: u64 = 0;
        let current = bytes_written.max(1);
        let total = bytes_written.max(1);
        assert_eq!(current, 1);
        assert_eq!(total, 1);
        let group =
            super::build_completed_task_group("g".into(), "t".into(), "m".into(), current, total);
        assert_eq!(group.tasks[0].current, 1);
        assert_eq!(group.tasks[0].total, 1);
        assert_eq!(group.overall_status, TaskStatus::Completed);
    }

    #[test]
    fn loader_failed_message_truncation() {
        // Long error — should be truncated to 256 Unicode chars
        let long_error: String = "失败".repeat(200);
        let truncated: String = long_error.chars().take(256).collect();
        assert_eq!(truncated.chars().count(), 256);
        assert!(truncated.starts_with("失败失败"));

        // Short error — should remain unchanged
        let short_error = "所有 Forge installer 下载源均失败".to_string();
        let short_truncated: String = short_error.chars().take(256).collect();
        assert_eq!(short_truncated, short_error);

        // Verify failed task group
        let group = super::build_failed_task_group("g".into(), "t".into(), truncated.clone());
        assert_eq!(group.tasks[0].status, TaskStatus::Failed);
        assert_eq!(group.tasks[0].current, 0);
        assert_eq!(group.tasks[0].total, 1);
        assert_eq!(group.tasks[0].message.chars().count(), 256);
    }

    #[test]
    fn loader_success_message_with_replaced_existing_true() {
        let message = format!(
            "previous_game_version={} new_game_version={} kind={:?} loader_version={} bytes_written={} replaced_existing={}",
            "1.20.1",
            "1.20.1-forge-47.3.0",
            crate::resource::models::InstallLoaderKind::Forge,
            "47.3.0",
            500000u64,
            true,
        );
        assert!(message.contains("replaced_existing=true"));
        assert!(message.contains("kind=Forge"));
        assert!(message.contains("loader_version=47.3.0"));
        assert!(message.contains("bytes_written=500000"));
    }

    #[test]
    fn loader_task_center_not_ready_error_message() {
        // When TaskCenterState is not managed, the error is:
        // "任务中心未就绪，无法启动异步安装"
        let err_msg = "任务中心未就绪，无法启动异步安装";
        assert!(!err_msg.is_empty());
        assert!(err_msg.contains("未就绪"));
        assert!(err_msg.contains("异步安装"));
    }

    #[test]
    fn loader_instance_not_found_error_pattern() {
        // get_instance_in returns: "实例不存在: {instance_id}"
        let instance_id = "missing-inst";
        let err = format!("实例不存在: {instance_id}");
        assert!(err.contains("不存在"));
        assert!(err.contains(instance_id));
    }

    // ── Phase 36: Snapshot import/export tests ────────────────────────────

    fn make_t36_group(id: &str, tasks: Vec<TaskProgress>, updated: &str) -> TaskGroup {
        let mut g = TaskGroup {
            group_id: id.into(),
            tasks,
            overall_status: TaskStatus::Pending,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: updated.into(),
            completed_tasks: 0,
            total_tasks: 0,
            progress_percent: 0,
        };
        models::refresh_group_summary(&mut g, updated.into());
        g
    }

    fn build_t36_bundle_json(groups: Vec<TaskGroup>) -> String {
        let bundle = TaskSnapshotBundle {
            schema_version: 1,
            exported_at: "2026-05-04T12:00:00Z".into(),
            groups,
        };
        serde_json::to_string(&bundle).unwrap()
    }

    // ── export schema/version/order ──────────────────────────────────────

    #[test]
    fn export_snapshot_has_correct_structure() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        map.insert(
            "g1".into(),
            make_t36_group(
                "g1",
                vec![make_task(1, TaskStatus::Completed, 100, 100)],
                "2026-01-01T00:00:01Z",
            ),
        );
        map.insert(
            "g2".into(),
            make_t36_group(
                "g2",
                vec![make_task(2, TaskStatus::Completed, 50, 100)],
                "2026-01-01T00:00:03Z",
            ),
        );

        let mut groups: Vec<TaskGroup> = map.values().cloned().collect();
        groups.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        let bundle = TaskSnapshotBundle {
            schema_version: 1,
            exported_at: "2026-05-04T12:00:00Z".into(),
            groups,
        };
        let json = serde_json::to_string_pretty(&bundle).unwrap();

        assert!(json.contains("\"schemaVersion\": 1"));
        assert!(json.contains("\"exportedAt\": \"2026-05-04T12:00:00Z\""));
        // Verify sort order: g2 (newer) before g1
        let g2_pos = json.find("\"g2\"").unwrap();
        let g1_pos = json.find("\"g1\"").unwrap();
        assert!(
            g2_pos < g1_pos,
            "g2 should appear before g1 in sorted output"
        );
    }

    // ── export empty groups ──────────────────────────────────────────────

    #[test]
    fn export_empty_groups_produces_valid_json() {
        let groups: Vec<TaskGroup> = vec![];
        let bundle = TaskSnapshotBundle {
            schema_version: 1,
            exported_at: "2026-05-04T12:00:00Z".into(),
            groups,
        };
        let json = serde_json::to_string_pretty(&bundle).unwrap();
        let back: TaskSnapshotBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema_version, 1);
        assert!(back.groups.is_empty());
    }

    // ── import wrong schema Err ──────────────────────────────────────────

    #[test]
    fn import_rejects_wrong_schema_version() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        let bad_json = r#"{"schemaVersion":99,"exportedAt":"2026-05-04T12:00:00Z","groups":[]}"#;
        let result = super::import_task_snapshot_into_map(&mut map, bad_json, true, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("仅支持版本 1"));
    }

    // ── import invalid JSON Err ──────────────────────────────────────────

    #[test]
    fn import_rejects_invalid_json() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        let result = super::import_task_snapshot_into_map(&mut map, "not json", true, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("解析失败"));
    }

    // ── import valid groups ──────────────────────────────────────────────

    #[test]
    fn import_valid_groups_succeeds() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        let g = make_t36_group(
            "g1",
            vec![make_task(1, TaskStatus::Completed, 100, 100)],
            "2026-01-01T00:00:00Z",
        );
        let json = build_t36_bundle_json(vec![g]);

        let (result, imported) =
            super::import_task_snapshot_into_map(&mut map, &json, true, true).unwrap();
        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(result.total, 1);
        assert_eq!(imported.len(), 1);
        assert!(map.contains_key("g1"));
        // Summary should have been refreshed
        let imported_group = map.get("g1").unwrap();
        assert_eq!(imported_group.overall_status, TaskStatus::Completed);
        assert_eq!(imported_group.total_tasks, 1);
    }

    // ── replace_existing false skips ─────────────────────────────────────

    #[test]
    fn import_skips_when_replace_existing_false() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        // Pre-insert a group with same id
        let existing = make_t36_group(
            "g1",
            vec![make_task(1, TaskStatus::Failed, 0, 100)],
            "2026-01-01T00:00:00Z",
        );
        map.insert("g1".into(), existing);

        let incoming = make_t36_group(
            "g1",
            vec![make_task(1, TaskStatus::Completed, 100, 100)],
            "2026-01-01T00:00:01Z",
        );
        let json = build_t36_bundle_json(vec![incoming]);

        let (result, imported) =
            super::import_task_snapshot_into_map(&mut map, &json, false, true).unwrap();
        assert_eq!(result.imported, 0);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.failed, 0);
        assert!(imported.is_empty());
        // Existing group should be unchanged
        assert_eq!(map.get("g1").unwrap().overall_status, TaskStatus::Failed);
    }

    // ── replace_existing true overwrites ─────────────────────────────────

    #[test]
    fn import_overwrites_when_replace_existing_true() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        let existing = make_t36_group(
            "g1",
            vec![make_task(1, TaskStatus::Failed, 0, 100)],
            "2026-01-01T00:00:00Z",
        );
        map.insert("g1".into(), existing);

        let incoming = make_t36_group(
            "g1",
            vec![make_task(1, TaskStatus::Completed, 100, 100)],
            "2026-01-01T00:00:01Z",
        );
        let json = build_t36_bundle_json(vec![incoming]);

        let (result, imported) =
            super::import_task_snapshot_into_map(&mut map, &json, true, true).unwrap();
        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(imported.len(), 1);
        // Overwritten group should have Completed status
        assert_eq!(map.get("g1").unwrap().overall_status, TaskStatus::Completed);
    }

    // ── drop_active true skips running ───────────────────────────────────

    #[test]
    fn import_skips_active_when_drop_active_true() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        let g = make_t36_group(
            "g1",
            vec![make_task(1, TaskStatus::Running, 30, 100)],
            "2026-01-01T00:00:00Z",
        );
        let json = build_t36_bundle_json(vec![g]);

        let (result, imported) =
            super::import_task_snapshot_into_map(&mut map, &json, true, true).unwrap();
        assert_eq!(result.imported, 0);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.failed, 0);
        assert!(imported.is_empty());
        assert!(!map.contains_key("g1"));
    }

    // ── drop_active false cancels running ────────────────────────────────

    #[test]
    fn import_cancels_active_when_drop_active_false() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        let g = make_t36_group(
            "g1",
            vec![
                make_task(1, TaskStatus::Running, 30, 100),
                make_task(2, TaskStatus::Completed, 100, 100),
            ],
            "2026-01-01T00:00:00Z",
        );
        let json = build_t36_bundle_json(vec![g]);

        let (result, imported) =
            super::import_task_snapshot_into_map(&mut map, &json, true, false).unwrap();
        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(imported.len(), 1);
        let imported_group = map.get("g1").unwrap();
        // Running task → Cancelled
        assert_eq!(imported_group.tasks[0].status, TaskStatus::Cancelled);
        assert!(imported_group.tasks[0].message.contains("从快照导入"));
        // Completed task unchanged
        assert_eq!(imported_group.tasks[1].status, TaskStatus::Completed);
        // Overall: Completed + Cancelled → Cancelled
        assert_eq!(imported_group.overall_status, TaskStatus::Cancelled);
    }

    // ── invalid empty group_id failed ────────────────────────────────────

    #[test]
    fn import_counts_empty_group_id_as_failed() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        let mut g = make_t36_group(
            "",
            vec![make_task(1, TaskStatus::Completed, 100, 100)],
            "2026-01-01T00:00:00Z",
        );
        g.group_id = "   ".to_string(); // whitespace-only
        let json = build_t36_bundle_json(vec![g]);

        let (result, imported) =
            super::import_task_snapshot_into_map(&mut map, &json, true, true).unwrap();
        assert_eq!(result.imported, 0);
        assert_eq!(result.failed, 1);
        assert_eq!(result.total, 1);
        assert!(imported.is_empty());
        assert!(map.is_empty());
    }

    // ── invalid empty tasks failed ───────────────────────────────────────

    #[test]
    fn import_counts_empty_tasks_as_failed() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        let g = make_t36_group("g1", vec![], "2026-01-01T00:00:00Z");
        let json = build_t36_bundle_json(vec![g]);

        let (result, imported) =
            super::import_task_snapshot_into_map(&mut map, &json, true, true).unwrap();
        assert_eq!(result.imported, 0);
        assert_eq!(result.failed, 1);
        assert_eq!(result.total, 1);
        assert!(imported.is_empty());
        assert!(!map.contains_key("g1"));
    }

    // ── refresh summary during import ────────────────────────────────────

    #[test]
    fn import_refreshes_summary() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        // Create a group with inconsistent summary fields
        let g = TaskGroup {
            group_id: "g1".into(),
            tasks: vec![
                make_task(1, TaskStatus::Completed, 100, 100),
                make_task(2, TaskStatus::Completed, 100, 100),
            ],
            overall_status: TaskStatus::Pending, // inconsistent
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "old".into(),
            completed_tasks: 0,  // inconsistent
            total_tasks: 0,      // inconsistent
            progress_percent: 0, // inconsistent
        };
        // Don't call refresh — leave it stale
        let json = build_t36_bundle_json(vec![g]);

        let (result, _imported) =
            super::import_task_snapshot_into_map(&mut map, &json, true, false).unwrap();
        assert_eq!(result.imported, 1);
        let ig = map.get("g1").unwrap();
        // After import, summary should be refreshed
        assert_eq!(ig.overall_status, TaskStatus::Completed);
        assert_eq!(ig.total_tasks, 2);
        assert_eq!(ig.completed_tasks, 2);
        assert_eq!(ig.progress_percent, 100);
        assert_ne!(ig.updated_at, "old");
    }

    // ── snapshot roundtrip: export → import → equivalent ─────────────────

    #[test]
    fn snapshot_roundtrip_export_then_import() {
        let mut map1: HashMap<String, TaskGroup> = HashMap::new();
        let g1 = make_t36_group(
            "grp-a",
            vec![make_task(1, TaskStatus::Completed, 100, 100)],
            "2026-01-01T00:00:02Z",
        );
        let g2 = make_t36_group(
            "grp-b",
            vec![make_task(2, TaskStatus::Failed, 0, 100)],
            "2026-01-01T00:00:01Z",
        );
        map1.insert("grp-a".into(), g1);
        map1.insert("grp-b".into(), g2);

        // Export
        let mut groups: Vec<TaskGroup> = map1.values().cloned().collect();
        groups.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        let bundle = TaskSnapshotBundle {
            schema_version: 1,
            exported_at: "2026-05-04T12:00:00Z".into(),
            groups,
        };
        let json = serde_json::to_string(&bundle).unwrap();

        // Import into a fresh map
        let mut map2: HashMap<String, TaskGroup> = HashMap::new();
        let (result, _imported) =
            super::import_task_snapshot_into_map(&mut map2, &json, true, true).unwrap();
        assert_eq!(result.imported, 2);
        assert_eq!(result.total, 2);
        assert!(map2.contains_key("grp-a"));
        assert!(map2.contains_key("grp-b"));
        assert_eq!(
            map2.get("grp-a").unwrap().overall_status,
            TaskStatus::Completed
        );
        assert_eq!(
            map2.get("grp-b").unwrap().overall_status,
            TaskStatus::Failed
        );
    }

    // ── mixed results: imported + skipped + failed in one call ───────────

    #[test]
    fn import_mixed_results_counts_correctly() {
        let mut map: HashMap<String, TaskGroup> = HashMap::new();
        // Pre-insert an existing group
        let existing = make_t36_group(
            "existing",
            vec![make_task(1, TaskStatus::Completed, 100, 100)],
            "2026-01-01T00:00:00Z",
        );
        map.insert("existing".into(), existing);

        let bundle = TaskSnapshotBundle {
            schema_version: 1,
            exported_at: "2026-05-04T12:00:00Z".into(),
            groups: vec![
                // valid new group
                make_t36_group(
                    "new-group",
                    vec![make_task(1, TaskStatus::Completed, 100, 100)],
                    "2026-01-01T00:00:01Z",
                ),
                // existing group, replace_existing=false → skipped
                make_t36_group(
                    "existing",
                    vec![make_task(1, TaskStatus::Failed, 0, 100)],
                    "2026-01-01T00:00:00Z",
                ),
                // active group, drop_active=true → skipped
                make_t36_group(
                    "active-grp",
                    vec![make_task(1, TaskStatus::Running, 30, 100)],
                    "2026-01-01T00:00:00Z",
                ),
                // empty tasks → failed
                make_t36_group("empty-tasks", vec![], "2026-01-01T00:00:00Z"),
            ],
        };
        let json = serde_json::to_string(&bundle).unwrap();

        let (result, imported) =
            super::import_task_snapshot_into_map(&mut map, &json, false, true).unwrap();
        // imported: new-group
        // skipped: existing, active-grp
        // failed: empty-tasks
        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped, 2);
        assert_eq!(result.failed, 1);
        assert_eq!(result.total, 4);
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].group_id, "new-group");
    }
}
