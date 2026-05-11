use crate::error::LauncherError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub task_id: u64,
    pub name: String,
    pub current: u64,
    pub total: u64,
    pub status: TaskStatus,
    pub message: String,
    #[serde(default = "default_timestamp")]
    pub created_at: String,
    #[serde(default = "default_timestamp")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGroup {
    pub group_id: String,
    pub tasks: Vec<TaskProgress>,
    pub overall_status: TaskStatus,
    #[serde(default = "default_timestamp")]
    pub created_at: String,
    #[serde(default = "default_timestamp")]
    pub updated_at: String,
    #[serde(default)]
    pub completed_tasks: usize,
    #[serde(default)]
    pub total_tasks: usize,
    #[serde(default)]
    pub progress_percent: u8,
}

fn default_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ── Phase 36: Snapshot import/export models ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshotBundle {
    pub schema_version: u32,
    pub exported_at: String,
    pub groups: Vec<TaskGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTaskSnapshotRequest {
    pub bundle_json: String,
    pub replace_existing: bool,
    pub drop_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportTaskSnapshotResult {
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub total: usize,
}

// ── Phase 37: Cancel semantics helper ────────────────────────────────────

/// 任务中心取消标记的语义说明，与前端 confirm 及提示文案保持一致。
pub const TASK_CENTER_CANCEL_NOTE: &str =
    "已在任务中心标记为取消；若后台下载已开始，底层操作可能仍会继续至自然结束。";

/// 在任务中心记录中将任务标记为"取消"。
///
/// 仅对 `Running`、`Pending`、`Paused` 状态执行标记：
/// - `status` 设为 `Cancelled`
/// - `updated_at` 置为 `now`
/// - `message` 若为空则写入 `TASK_CENTER_CANCEL_NOTE`
/// - `message` 若已含 `"已在任务中心标记为取消"` 则不重复追加
/// - 其余非空 `message` 会以 `；` 分隔追加说明
///
/// `Completed`、`Failed`、已 `Cancelled` 的任务不做任何修改。
pub fn mark_task_cancelled_in_task_center(task: &mut TaskProgress, now: &str) {
    if matches!(
        task.status,
        TaskStatus::Running | TaskStatus::Pending | TaskStatus::Paused
    ) {
        task.status = TaskStatus::Cancelled;
        task.updated_at = now.to_string();

        if task.message.is_empty() {
            task.message = TASK_CENTER_CANCEL_NOTE.to_string();
        } else if !task.message.contains("已在任务中心标记为取消") {
            task.message = format!("{}；{}", task.message, TASK_CENTER_CANCEL_NOTE);
        }
    }
}

// ── Phase 36: Pure helper functions ──────────────────────────────────────

/// Check whether a task group is "active" — its `overall_status` or any
/// individual task is Running, Pending, or Paused.
pub fn is_active_task_group(group: &TaskGroup) -> bool {
    if matches!(
        group.overall_status,
        TaskStatus::Running | TaskStatus::Pending | TaskStatus::Paused
    ) {
        return true;
    }
    group.tasks.iter().any(|t| {
        matches!(
            t.status,
            TaskStatus::Running | TaskStatus::Pending | TaskStatus::Paused
        )
    })
}

/// Validate a snapshot group before import:
/// - `group_id` trimmed must be non-empty.
/// - `tasks` must be non-empty.
pub fn validate_snapshot_group(group: &TaskGroup) -> Result<(), LauncherError> {
    if group.group_id.trim().is_empty() {
        return Err(LauncherError::from("任务组ID为空，跳过导入"));
    }
    if group.tasks.is_empty() {
        return Err(LauncherError::from(format!("任务组 {} 没有任务，跳过导入", group.group_id)));
    }
    Ok(())
}

/// Fill empty timestamps, handle active tasks based on `drop_active`, and
/// refresh the group summary.
///
/// Returns `true` if the group should be **skipped** (because it is active
/// and `drop_active` is true).
pub fn normalize_imported_task_group(group: &mut TaskGroup, drop_active: bool, now: &str) -> bool {
    // Fill empty timestamps on tasks
    for task in &mut group.tasks {
        if task.created_at.is_empty() {
            task.created_at = now.to_string();
        }
        if task.updated_at.is_empty() {
            task.updated_at = now.to_string();
        }
    }

    let active = is_active_task_group(group);

    if active {
        if drop_active {
            return true; // skip
        } else {
            // Cancel all Running / Pending / Paused tasks
            for task in &mut group.tasks {
                if matches!(
                    task.status,
                    TaskStatus::Running | TaskStatus::Pending | TaskStatus::Paused
                ) {
                    task.status = TaskStatus::Cancelled;
                    task.updated_at = now.to_string();
                    let cancel_msg = "从快照导入，原活跃任务已标记为取消";
                    if task.message.is_empty() {
                        task.message = cancel_msg.to_string();
                    } else if !task.message.contains("从快照导入") {
                        task.message = format!("{}；{}", task.message, cancel_msg);
                    }
                }
            }
        }
    }

    refresh_group_summary(group, now.to_string());
    false // not skipped
}

/// Calculate overall status from task list.
/// Rules:
/// - Empty → Pending
/// - Any Failed → Failed
/// - Any Cancelled and no Running/Pending → Cancelled
/// - Any Running → Running
/// - Any Pending → Pending
/// - All Completed → Completed
pub fn calculate_overall_status(tasks: &[TaskProgress]) -> TaskStatus {
    if tasks.is_empty() {
        return TaskStatus::Pending;
    }
    let has_failed = tasks.iter().any(|t| t.status == TaskStatus::Failed);
    if has_failed {
        return TaskStatus::Failed;
    }
    let has_running = tasks.iter().any(|t| t.status == TaskStatus::Running);
    let has_pending = tasks.iter().any(|t| t.status == TaskStatus::Pending);
    let has_cancelled = tasks.iter().any(|t| t.status == TaskStatus::Cancelled);

    if has_running {
        TaskStatus::Running
    } else if has_pending {
        TaskStatus::Pending
    } else if has_cancelled {
        TaskStatus::Cancelled
    } else {
        // All must be Completed (or Paused which should not happen alone)
        TaskStatus::Completed
    }
}

/// Calculate progress percentage (0..100) from task list.
/// For each task: if total > 0 use current/total; Completed tasks count as 100%.
/// Average across tasks, clamped to 0..100.
pub fn calculate_progress_percent(tasks: &[TaskProgress]) -> u8 {
    if tasks.is_empty() {
        return 0;
    }

    let sum: f64 = tasks
        .iter()
        .map(|t| {
            if t.status == TaskStatus::Completed {
                1.0
            } else if t.total > 0 {
                (t.current as f64 / t.total as f64).min(1.0)
            } else {
                0.0
            }
        })
        .sum();

    let avg = sum / tasks.len() as f64;
    (avg * 100.0).round() as u8
}

/// Refresh group summary fields: overall_status, completed_tasks,
/// total_tasks, progress_percent, updated_at.
pub fn refresh_group_summary(group: &mut TaskGroup, now: String) {
    group.overall_status = calculate_overall_status(&group.tasks);
    group.total_tasks = group.tasks.len();
    group.completed_tasks = group
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .count();
    group.progress_percent = calculate_progress_percent(&group.tasks);
    group.updated_at = now;
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    fn task(status: TaskStatus, current: u64, total: u64) -> TaskProgress {
        TaskProgress {
            task_id: 1,
            name: "test".into(),
            current,
            total,
            status,
            message: String::new(),
            created_at: "".into(),
            updated_at: "".into(),
        }
    }

    // ── calculate_overall_status ─────────────────────────────────────────

    #[test]
    fn empty_tasks_overall_pending() {
        assert_eq!(calculate_overall_status(&[]), TaskStatus::Pending);
    }

    #[test]
    fn any_failed_makes_overall_failed() {
        let tasks = vec![
            task(TaskStatus::Completed, 100, 100),
            task(TaskStatus::Failed, 50, 100),
            task(TaskStatus::Running, 30, 100),
        ];
        assert_eq!(calculate_overall_status(&tasks), TaskStatus::Failed);
    }

    #[test]
    fn running_overrides_pending_and_completed() {
        let tasks = vec![
            task(TaskStatus::Pending, 0, 100),
            task(TaskStatus::Completed, 100, 100),
            task(TaskStatus::Running, 30, 100),
        ];
        assert_eq!(calculate_overall_status(&tasks), TaskStatus::Running);
    }

    #[test]
    fn pending_overrides_completed() {
        let tasks = vec![
            task(TaskStatus::Completed, 100, 100),
            task(TaskStatus::Pending, 0, 100),
        ];
        assert_eq!(calculate_overall_status(&tasks), TaskStatus::Pending);
    }

    #[test]
    fn all_cancelled_is_cancelled() {
        let tasks = vec![
            task(TaskStatus::Cancelled, 0, 100),
            task(TaskStatus::Cancelled, 50, 100),
        ];
        assert_eq!(calculate_overall_status(&tasks), TaskStatus::Cancelled);
    }

    #[test]
    fn cancelled_with_running_is_running() {
        let tasks = vec![
            task(TaskStatus::Cancelled, 0, 100),
            task(TaskStatus::Running, 30, 100),
        ];
        assert_eq!(calculate_overall_status(&tasks), TaskStatus::Running);
    }

    #[test]
    fn cancelled_with_pending_is_pending() {
        let tasks = vec![
            task(TaskStatus::Cancelled, 0, 100),
            task(TaskStatus::Pending, 0, 100),
        ];
        assert_eq!(calculate_overall_status(&tasks), TaskStatus::Pending);
    }

    #[test]
    fn all_completed_is_completed() {
        let tasks = vec![
            task(TaskStatus::Completed, 100, 100),
            task(TaskStatus::Completed, 50, 50),
        ];
        assert_eq!(calculate_overall_status(&tasks), TaskStatus::Completed);
    }

    #[test]
    fn single_paused_is_not_overridden_by_cancelled() {
        // Paused alone with Cancelled → since all Cancelled + Paused (no Pending/Running)
        // → Cancelled per spec
        let tasks = vec![
            task(TaskStatus::Paused, 20, 100),
            task(TaskStatus::Cancelled, 0, 100),
        ];
        assert_eq!(calculate_overall_status(&tasks), TaskStatus::Cancelled);
    }

    // ── calculate_progress_percent ───────────────────────────────────────

    #[test]
    fn empty_tasks_progress_zero() {
        assert_eq!(calculate_progress_percent(&[]), 0);
    }

    #[test]
    fn single_completed_task_100() {
        let tasks = vec![task(TaskStatus::Completed, 100, 100)];
        assert_eq!(calculate_progress_percent(&tasks), 100);
    }

    #[test]
    fn single_half_task_50() {
        let tasks = vec![task(TaskStatus::Running, 50, 100)];
        assert_eq!(calculate_progress_percent(&tasks), 50);
    }

    #[test]
    fn single_zero_total_task_0() {
        let tasks = vec![task(TaskStatus::Running, 0, 0)];
        assert_eq!(calculate_progress_percent(&tasks), 0);
    }

    #[test]
    fn average_across_tasks() {
        let tasks = vec![
            task(TaskStatus::Completed, 100, 100), // 100%
            task(TaskStatus::Running, 0, 100),     // 0%
        ];
        assert_eq!(calculate_progress_percent(&tasks), 50);
    }

    #[test]
    fn progress_clamped_to_100() {
        let tasks = vec![task(TaskStatus::Running, 200, 100)];
        assert_eq!(calculate_progress_percent(&tasks), 100);
    }

    #[test]
    fn mixed_tasks_average() {
        let tasks = vec![
            task(TaskStatus::Completed, 100, 100), // 100%
            task(TaskStatus::Running, 25, 100),    // 25%
            task(TaskStatus::Pending, 0, 100),     // 0%
        ];
        // average = (1.0 + 0.25 + 0.0) / 3 ≈ 41.67 → 42
        assert_eq!(calculate_progress_percent(&tasks), 42);
    }

    // ── refresh_group_summary ────────────────────────────────────────────

    #[test]
    fn refresh_updates_all_summary_fields() {
        let mut group = TaskGroup {
            group_id: "g1".into(),
            tasks: vec![
                task(TaskStatus::Completed, 100, 100),
                task(TaskStatus::Running, 30, 100),
            ],
            overall_status: TaskStatus::Pending,
            created_at: "old".into(),
            updated_at: "old".into(),
            completed_tasks: 0,
            total_tasks: 0,
            progress_percent: 0,
        };
        let now = "2026-05-04T12:00:00Z".to_string();
        refresh_group_summary(&mut group, now.clone());

        assert_eq!(group.overall_status, TaskStatus::Running);
        assert_eq!(group.total_tasks, 2);
        assert_eq!(group.completed_tasks, 1);
        assert_eq!(group.progress_percent, 65); // (100 + 30) / 2 = 65
        assert_eq!(group.updated_at, now);
    }

    #[test]
    fn refresh_empty_group() {
        let mut group = TaskGroup {
            group_id: "g2".into(),
            tasks: vec![],
            overall_status: TaskStatus::Running,
            created_at: "old".into(),
            updated_at: "old".into(),
            completed_tasks: 5,
            total_tasks: 5,
            progress_percent: 100,
        };
        let now = "2026-05-04T12:00:00Z".to_string();
        refresh_group_summary(&mut group, now.clone());

        assert_eq!(group.overall_status, TaskStatus::Pending);
        assert_eq!(group.total_tasks, 0);
        assert_eq!(group.completed_tasks, 0);
        assert_eq!(group.progress_percent, 0);
        assert_eq!(group.updated_at, now);
    }

    // ── Serialization roundtrip (camelCase) ──────────────────────────────

    #[test]
    fn task_progress_serializes_camel_case() {
        let task = TaskProgress {
            task_id: 1,
            name: "Download".into(),
            current: 50,
            total: 100,
            status: TaskStatus::Running,
            message: "working".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:01:00Z".into(),
        };
        let json = serde_json::to_string(&task).expect("should serialize");
        assert!(json.contains("\"taskId\""));
        assert!(json.contains("\"createdAt\""));
        assert!(json.contains("\"updatedAt\""));
        // Roundtrip
        let back: TaskProgress = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(back.task_id, 1);
        assert_eq!(back.created_at, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn task_group_serializes_camel_case() {
        let group = TaskGroup {
            group_id: "g1".into(),
            tasks: vec![],
            overall_status: TaskStatus::Pending,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            completed_tasks: 0,
            total_tasks: 0,
            progress_percent: 0,
        };
        let json = serde_json::to_string(&group).expect("should serialize");
        assert!(json.contains("\"groupId\""));
        assert!(json.contains("\"overallStatus\""));
        assert!(json.contains("\"completedTasks\""));
        assert!(json.contains("\"totalTasks\""));
        assert!(json.contains("\"progressPercent\""));
        let back: TaskGroup = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(back.group_id, "g1");
    }

    #[test]
    fn task_progress_deserializes_with_default_timestamps() {
        // Old-style JSON without createdAt/updatedAt
        let json =
            r#"{"taskId":1,"name":"t","current":0,"total":0,"status":"pending","message":""}"#;
        let task: TaskProgress = serde_json::from_str(json).expect("should deserialize");
        assert!(
            !task.created_at.is_empty(),
            "created_at should have default"
        );
        assert!(
            !task.updated_at.is_empty(),
            "updated_at should have default"
        );
    }

    // ── Phase 36: Snapshot helpers tests ──────────────────────────────────

    fn make_task_snapshot(
        id: u64,
        status: TaskStatus,
        message: &str,
        created: &str,
        updated: &str,
    ) -> TaskProgress {
        TaskProgress {
            task_id: id,
            name: format!("task-{id}"),
            current: 0,
            total: 100,
            status,
            message: message.to_string(),
            created_at: created.to_string(),
            updated_at: updated.to_string(),
        }
    }

    fn make_group_snapshot(
        group_id: &str,
        tasks: Vec<TaskProgress>,
        overall: TaskStatus,
        updated: &str,
    ) -> TaskGroup {
        let mut g = TaskGroup {
            group_id: group_id.to_string(),
            tasks,
            overall_status: overall,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: updated.to_string(),
            completed_tasks: 0,
            total_tasks: 0,
            progress_percent: 0,
        };
        refresh_group_summary(&mut g, updated.to_string());
        g
    }

    // ── is_active_task_group ────────────────────────────────────────────

    #[test]
    fn active_when_overall_is_running() {
        let g = TaskGroup {
            group_id: "g".into(),
            tasks: vec![make_task_snapshot(1, TaskStatus::Completed, "", "", "")],
            overall_status: TaskStatus::Running,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            completed_tasks: 1,
            total_tasks: 1,
            progress_percent: 100,
        };
        assert!(is_active_task_group(&g));
    }

    #[test]
    fn active_when_task_is_running() {
        let g = make_group_snapshot(
            "g",
            vec![
                make_task_snapshot(1, TaskStatus::Completed, "", "", ""),
                make_task_snapshot(2, TaskStatus::Running, "", "", ""),
            ],
            TaskStatus::Completed,
            "2026-01-01T00:00:00Z",
        );
        // overall_status is Completed but a task is Running
        assert!(is_active_task_group(&g));
    }

    #[test]
    fn not_active_when_all_completed_or_failed() {
        let g = make_group_snapshot(
            "g",
            vec![
                make_task_snapshot(1, TaskStatus::Completed, "", "", ""),
                make_task_snapshot(2, TaskStatus::Failed, "", "", ""),
                make_task_snapshot(3, TaskStatus::Cancelled, "", "", ""),
            ],
            TaskStatus::Failed,
            "2026-01-01T00:00:00Z",
        );
        assert!(!is_active_task_group(&g));
    }

    // ── validate_snapshot_group ─────────────────────────────────────────

    #[test]
    fn validate_rejects_empty_group_id() {
        let g = make_group_snapshot(
            "   ",
            vec![make_task_snapshot(1, TaskStatus::Completed, "", "", "")],
            TaskStatus::Completed,
            "2026-01-01T00:00:00Z",
        );
        assert!(validate_snapshot_group(&g).is_err());
    }

    #[test]
    fn validate_rejects_empty_tasks() {
        let g = make_group_snapshot("g", vec![], TaskStatus::Pending, "2026-01-01T00:00:00Z");
        assert!(validate_snapshot_group(&g).is_err());
    }

    #[test]
    fn validate_accepts_valid_group() {
        let g = make_group_snapshot(
            "g",
            vec![make_task_snapshot(1, TaskStatus::Completed, "", "", "")],
            TaskStatus::Completed,
            "2026-01-01T00:00:00Z",
        );
        assert!(validate_snapshot_group(&g).is_ok());
    }

    // ── normalize_imported_task_group ────────────────────────────────────

    #[test]
    fn normalize_fills_empty_timestamps() {
        let tasks = vec![TaskProgress {
            task_id: 1,
            name: "t".into(),
            current: 0,
            total: 100,
            status: TaskStatus::Completed,
            message: "".into(),
            created_at: "".into(),
            updated_at: "".into(),
        }];
        let mut g = make_group_snapshot("g", tasks, TaskStatus::Pending, "2026-01-01T00:00:00Z");
        let now = "2026-05-04T12:00:00Z";
        let skipped = normalize_imported_task_group(&mut g, false, now);
        assert!(!skipped);
        assert_eq!(g.tasks[0].created_at, now);
        assert_eq!(g.tasks[0].updated_at, now);
    }

    #[test]
    fn normalize_drop_active_true_skips_running_group() {
        let mut g = make_group_snapshot(
            "g",
            vec![make_task_snapshot(
                1,
                TaskStatus::Running,
                "downloading",
                "",
                "",
            )],
            TaskStatus::Running,
            "2026-01-01T00:00:00Z",
        );
        let now = "2026-05-04T12:00:00Z";
        let skipped = normalize_imported_task_group(&mut g, true, now);
        assert!(
            skipped,
            "active group should be skipped when drop_active=true"
        );
    }

    #[test]
    fn normalize_drop_active_false_cancels_running_tasks() {
        let mut g = make_group_snapshot(
            "g",
            vec![
                make_task_snapshot(1, TaskStatus::Running, "downloading", "", ""),
                make_task_snapshot(2, TaskStatus::Pending, "queued", "", ""),
                make_task_snapshot(3, TaskStatus::Completed, "done", "", ""),
            ],
            TaskStatus::Running,
            "2026-01-01T00:00:00Z",
        );
        let now = "2026-05-04T12:00:00Z";
        let skipped = normalize_imported_task_group(&mut g, false, now);
        assert!(!skipped);
        // Running + Pending → Cancelled
        assert_eq!(g.tasks[0].status, TaskStatus::Cancelled);
        assert_eq!(g.tasks[1].status, TaskStatus::Cancelled);
        // Completed unchanged
        assert_eq!(g.tasks[2].status, TaskStatus::Completed);
        // Cancelled tasks' messages include the cancel note
        assert!(g.tasks[0].message.contains("从快照导入"));
        assert!(g.tasks[1].message.contains("从快照导入"));
        // Summary refreshed
        assert_eq!(g.updated_at, now);
        // overall_status: Completed + Cancelled + Cancelled → Cancelled
        assert_eq!(g.overall_status, TaskStatus::Cancelled);
    }

    #[test]
    fn normalize_cancel_appends_message_to_existing() {
        let mut g = make_group_snapshot(
            "g",
            vec![make_task_snapshot(
                1,
                TaskStatus::Running,
                "download started",
                "",
                "",
            )],
            TaskStatus::Running,
            "2026-01-01T00:00:00Z",
        );
        let now = "2026-05-04T12:00:00Z";
        normalize_imported_task_group(&mut g, false, now);
        assert!(g.tasks[0].message.starts_with("download started"));
        assert!(g.tasks[0].message.contains("从快照导入"));
    }

    #[test]
    fn normalize_drop_active_false_skips_non_active_group() {
        let mut g = make_group_snapshot(
            "g",
            vec![make_task_snapshot(1, TaskStatus::Completed, "done", "", "")],
            TaskStatus::Completed,
            "2026-01-01T00:00:00Z",
        );
        let now = "2026-05-04T12:00:00Z";
        // drop_active=true but group is not active → should still import
        let skipped = normalize_imported_task_group(&mut g, true, now);
        assert!(!skipped);
    }

    // ── serde camelCase roundtrip for snapshot types ─────────────────────

    #[test]
    fn task_snapshot_bundle_serializes_camel_case() {
        let bundle = TaskSnapshotBundle {
            schema_version: 1,
            exported_at: "2026-05-04T12:00:00Z".into(),
            groups: vec![],
        };
        let json = serde_json::to_string(&bundle).expect("serialize");
        assert!(json.contains("\"schemaVersion\""));
        assert!(json.contains("\"exportedAt\""));
        assert!(json.contains("\"groups\""));
        let back: TaskSnapshotBundle = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.schema_version, 1);
        assert_eq!(back.exported_at, "2026-05-04T12:00:00Z");
    }

    #[test]
    fn import_result_serializes_camel_case() {
        let result = ImportTaskSnapshotResult {
            imported: 3,
            skipped: 1,
            failed: 2,
            total: 6,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: ImportTaskSnapshotResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, result);
    }

    #[test]
    fn import_request_deserializes_camel_case() {
        let json = r#"{"bundleJson":"{}","replaceExisting":true,"dropActive":false}"#;
        let req: ImportTaskSnapshotRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.bundle_json, "{}");
        assert!(req.replace_existing);
        assert!(!req.drop_active);
    }

    // ── snapshot bundle roundtrip with groups ────────────────────────────

    #[test]
    fn bundle_roundtrip_with_groups_preserves_data() {
        let groups = vec![make_group_snapshot(
            "g1",
            vec![make_task_snapshot(
                1,
                TaskStatus::Completed,
                "ok",
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:01Z",
            )],
            TaskStatus::Completed,
            "2026-01-01T00:00:01Z",
        )];
        let bundle = TaskSnapshotBundle {
            schema_version: 1,
            exported_at: "2026-05-04T12:00:00Z".into(),
            groups,
        };
        let json = serde_json::to_string_pretty(&bundle).expect("serialize");
        let back: TaskSnapshotBundle = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.schema_version, 1);
        assert_eq!(back.groups.len(), 1);
        assert_eq!(back.groups[0].group_id, "g1");
        assert_eq!(back.groups[0].tasks[0].status, TaskStatus::Completed);
    }
}
