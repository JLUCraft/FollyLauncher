use crate::tasks::models::{TaskGroup, TaskProgress, TaskStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

#[tauri::command]
pub async fn schedule_task_group(
    app: AppHandle,
    state: State<'_, Arc<Mutex<HashMap<String, TaskGroup>>>>,
    group_id: String,
    tasks: Vec<TaskProgress>,
) -> Result<TaskGroup, String> {
    let mut groups = state.lock().await;
    let group = TaskGroup {
        group_id: group_id.clone(),
        tasks,
        overall_status: TaskStatus::Pending,
    };
    groups.insert(group_id, group.clone());
    let _ = app.emit("task-group-created", &group);
    Ok(group)
}

#[tauri::command]
pub async fn update_task_progress(
    app: AppHandle,
    group_id: String,
    task_id: u64,
    current: u64,
    total: u64,
    status: TaskStatus,
    message: String,
) -> Result<(), String> {
    let state = app.state::<Arc<Mutex<HashMap<String, TaskGroup>>>>();
    let mut groups = state.lock().await;
    if let Some(group) = groups.get_mut(&group_id) {
        if let Some(task) = group.tasks.iter_mut().find(|t| t.task_id == task_id) {
            task.current = current;
            task.total = total;
            task.status = status.clone();
            task.message = message;
        }
        // Update overall status
        if group
            .tasks
            .iter()
            .all(|t| t.status == TaskStatus::Completed)
        {
            group.overall_status = TaskStatus::Completed;
        } else if group.tasks.iter().any(|t| t.status == TaskStatus::Failed) {
            group.overall_status = TaskStatus::Failed;
        } else if group.tasks.iter().any(|t| t.status == TaskStatus::Running) {
            group.overall_status = TaskStatus::Running;
        }
        let _ = app.emit("task-group-updated", group.clone());
    }
    Ok(())
}

#[tauri::command]
pub async fn get_task_group(
    state: State<'_, Arc<Mutex<HashMap<String, TaskGroup>>>>,
    group_id: String,
) -> Result<Option<TaskGroup>, String> {
    let groups = state.lock().await;
    Ok(groups.get(&group_id).cloned())
}

#[tauri::command]
pub async fn list_task_groups(
    state: State<'_, Arc<Mutex<HashMap<String, TaskGroup>>>>,
) -> Result<Vec<TaskGroup>, String> {
    let groups = state.lock().await;
    Ok(groups.values().cloned().collect())
}

#[tauri::command]
pub async fn cancel_task_group(
    app: AppHandle,
    state: State<'_, Arc<Mutex<HashMap<String, TaskGroup>>>>,
    group_id: String,
) -> Result<(), String> {
    let mut groups = state.lock().await;
    if let Some(group) = groups.get_mut(&group_id) {
        for task in &mut group.tasks {
            if task.status == TaskStatus::Running || task.status == TaskStatus::Pending {
                task.status = TaskStatus::Cancelled;
            }
        }
        group.overall_status = TaskStatus::Cancelled;
        let _ = app.emit("task-group-cancelled", group_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn remove_task_group(
    state: State<'_, Arc<Mutex<HashMap<String, TaskGroup>>>>,
    group_id: String,
) -> Result<(), String> {
    let mut groups = state.lock().await;
    groups.remove(&group_id);
    Ok(())
}
