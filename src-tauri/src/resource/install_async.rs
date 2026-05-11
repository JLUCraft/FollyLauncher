use std::future::Future;
use tauri::AppHandle;

/// Run an install inner function in the background. On completion (success or
/// failure), update the corresponding task group entry (task_id=1) with either
/// Completed or Failed status.
///
/// The `on_success` closure receives a reference to the result and returns
/// `(current, total, message)` to update the task group entry with.
pub async fn run_install_with_task_update<R, F, S>(
    app: &AppHandle,
    background_group_id: &str,
    inner: F,
    on_success: S,
) where
    F: Future<Output = Result<R, crate::error::LauncherError>>,
    S: FnOnce(&R) -> (u64, u64, String),
{
    match inner.await {
        Ok(r) => {
            let (current, total, message) = on_success(&r);
            let _ = crate::tasks::commands::update_single_task_group(
                app,
                background_group_id,
                |group| {
                    if let Some(task) = group.tasks.iter_mut().find(|t| t.task_id == 1) {
                        task.current = current;
                        task.total = total;
                        task.status = crate::tasks::models::TaskStatus::Completed;
                        task.message = message;
                    }
                },
            )
            .await;
        }
        Err(e) => {
            let truncated: String = e.to_string().chars().take(256).collect();
            let _ = crate::tasks::commands::update_single_task_group(
                app,
                background_group_id,
                |group| {
                    if let Some(task) = group.tasks.iter_mut().find(|t| t.task_id == 1) {
                        task.current = 0;
                        task.total = 1;
                        task.status = crate::tasks::models::TaskStatus::Failed;
                        task.message = truncated;
                    }
                },
            )
            .await;
        }
    }
}
