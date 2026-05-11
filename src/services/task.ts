import { invoke } from "@tauri-apps/api/core";

export type TaskStatusType = "Pending" | "Running" | "Paused" | "Completed" | "Failed" | "Cancelled";

export interface TaskProgress {
  taskId: number;
  name: string;
  current: number;
  total: number;
  status: TaskStatusType;
  message: string;
  createdAt: string;
  updatedAt: string;
}

export interface TaskGroup {
  groupId: string;
  tasks: TaskProgress[];
  overallStatus: TaskStatusType;
  createdAt: string;
  updatedAt: string;
  completedTasks: number;
  totalTasks: number;
  progressPercent: number;
}

export async function listTaskGroups(): Promise<TaskGroup[]> {
  return invoke("list_task_groups");
}

export async function getTaskGroup(groupId: string): Promise<TaskGroup | null> {
  return invoke("get_task_group", { groupId });
}

export async function cancelTaskGroup(groupId: string): Promise<void> {
  return invoke("cancel_task_group", { groupId });
}

export async function removeTaskGroup(groupId: string): Promise<void> {
  return invoke("remove_task_group", { groupId });
}

// ── Phase 36: Snapshot import/export ──────────────────────────────────────

export interface TaskSnapshotBundle {
  schemaVersion: number;
  exportedAt: string;
  groups: TaskGroup[];
}

export interface ImportTaskSnapshotRequest {
  bundleJson: string;
  replaceExisting: boolean;
  dropActive: boolean;
}

export interface ImportTaskSnapshotResult {
  imported: number;
  skipped: number;
  failed: number;
  total: number;
}

export async function exportTaskSnapshot(): Promise<string> {
  return invoke("export_task_snapshot");
}

export async function importTaskSnapshot(
  request: ImportTaskSnapshotRequest,
): Promise<ImportTaskSnapshotResult> {
  return invoke("import_task_snapshot", { request });
}
