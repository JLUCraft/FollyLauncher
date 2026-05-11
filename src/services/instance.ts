import { invoke } from "@tauri-apps/api/core";

export type LocalInstanceKind = "Vanilla" | "Fabric" | "Forge" | "NeoForge" | "Quilt" | "Custom";

export interface LocalInstance {
  id: string;
  name: string;
  game_version: string;
  kind: LocalInstanceKind;
  game_dir: string;
  icon: string | null;
  last_played_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateLocalInstanceRequest {
  name: string;
  game_version: string;
  kind?: LocalInstanceKind | null;
}

export interface UpdateLocalInstanceRequest {
  id: string;
  name?: string | null;
  game_version?: string | null;
  kind?: LocalInstanceKind | null;
  icon?: string | null;
}

export interface ModOperationResult {
  instance_id: string;
  old_path: string;
  new_path: string | null;
  file_name: string;
  enabled: boolean | null;
  deleted: boolean;
}

export async function setModEnabled(
  instanceId: string,
  fileName: string,
  enabled: boolean,
): Promise<ModOperationResult> {
  return invoke("set_mod_enabled", { instanceId, fileName, enabled });
}

export async function deleteModFile(
  instanceId: string,
  fileName: string,
): Promise<ModOperationResult> {
  return invoke("delete_mod_file", { instanceId, fileName });
}

export async function listLocalInstances(): Promise<LocalInstance[]> {
  return invoke("list_local_instances");
}

export async function createLocalInstance(
  request: CreateLocalInstanceRequest,
): Promise<LocalInstance> {
  return invoke("create_local_instance", { request });
}

export async function updateLocalInstance(
  request: UpdateLocalInstanceRequest,
): Promise<LocalInstance> {
  return invoke("update_local_instance", { request });
}

export async function deleteLocalInstance(id: string): Promise<void> {
  return invoke("delete_local_instance", { id });
}

export interface ModFileInfo {
  name: string;
  path: string;
  file_name: string;
  enabled: boolean;
  size: number;
  modified_at: number;
}

export interface WorldInfo {
  name: string;
  icon_path: string | null;
  last_played: number | null;
  game_mode: string;
  cheats: boolean;
}

export interface ResourcePackInfo {
  name: string;
  path: string;
  enabled: boolean;
}

export interface ScreenshotInfo {
  name: string;
  path: string;
  timestamp: number;
}

export interface ServerInfo {
  name: string;
  address: string;
}

export interface InstanceWorkspaceInfo {
  instance_id: string;
  game_dir: string;
  mods: ModFileInfo[];
  resource_packs: ResourcePackInfo[];
  shader_packs: ResourcePackInfo[];
  worlds: WorldInfo[];
  screenshots: ScreenshotInfo[];
  servers: ServerInfo[];
}

export async function retrieveInstanceWorkspace(
  instanceId: string,
): Promise<InstanceWorkspaceInfo> {
  return invoke("retrieve_instance_workspace", { instanceId });
}

export interface LaunchLocalInstanceResult {
  launching_id: number;
  pid: number;
  instance_id: string;
  instance_name: string;
  game_version: string;
  username: string;
  uuid: string;
  java_path: string;
  game_dir: string;
}

export async function launchLocalInstance(
  instanceId: string,
): Promise<LaunchLocalInstanceResult> {
  return invoke("launch_local_instance", { instanceId });
}

export interface LaunchStateResponse {
  id: number;
  step: string;
  instance_id: string;
  version: string;
  pid: number;
  exit_code: number | null;
  exit_ok: boolean | null;
  game_ready: boolean;
  start_time: number | null;
  end_time: number | null;
  recent_logs: string[];
}

export async function launchGetState(
  launchingId: number,
): Promise<LaunchStateResponse> {
  return invoke("launch_get_state", { launchingId });
}

export async function launchListStates(): Promise<LaunchStateResponse[]> {
  return invoke("launch_list_states");
}

// ── Phase 8: Launch lifecycle actions ──────────────────────────────────

export async function launchCancel(launchingId: number): Promise<void> {
  return invoke("launch_cancel", { launchingId });
}

export async function launchExportCrash(
  launchingId: number,
  savePath: string,
): Promise<string> {
  return invoke("launch_export_crash", { launchingId, savePath });
}

// ── Phase 4: Enhanced game validation ──────────────────────────────────

export interface ValidationSummary {
  instance_id: string;
  missing_libraries: number;
  missing_assets: number;
  invalid_hashes: number;
  native_actions: number;
  download_group_id: string | null;
  ready_to_launch: boolean;
  download_tasks: DownloadTask[];
}

export interface DownloadTask {
  kind: "Library" | "Asset" | "ClientJar" | "AssetIndex" | "Native";
  name: string;
  url: string;
  dest_path: string;
  sha1: string | null;
  size: number | null;
  required: boolean;
}

export async function validateAndUpdateGame(
  instanceId: string,
  version?: string | null,
): Promise<ValidationSummary> {
  return invoke("validate_and_update_game", { instanceId, version });
}

// ── Phase 4: Launch plan generation ────────────────────────────────────

export interface LaunchPlan {
  java_executable: string;
  jvm_args: string[];
  game_args: string[];
  classpath: string[];
  natives_dir: string;
  game_dir: string;
  main_class: string;
  version: string;
  quick_play: QuickPlayTarget | null;
  custom_jvm_flags: string[];
}

export interface QuickPlayTarget {
  kind: "Multiplayer" | "Realms" | "Singleplayer";
  server_address: string | null;
  server_port: number | null;
}

export async function generateLaunchPlan(
  instanceId: string,
  version: string,
  serverAddress?: string | null,
  serverPort?: number | null,
): Promise<LaunchPlan> {
  return invoke("generate_launch_plan", {
    instanceId,
    version,
    serverAddress,
    serverPort,
  });
}

// ── Phase 4: Download progress event type ──────────────────────────────

export interface DownloadProgress {
  task_id: string;
  group_id: string | null;
  downloaded: number;
  total: number;
  speed_bytes_per_sec: number;
  state: "Pending" | "Downloading" | "Verifying" | "Completed" | "Failed";
}

// ── Phase 4: Game lifecycle events ─────────────────────────────────────

export interface GameLogLine {
  session_id: string;
  stream: "stdout" | "stderr";
  line: string;
}

export interface GameExitedEvent {
  session_id: string;
  code: number | null;
  duration_secs: number;
  exit_ok: boolean;
  game_ready: boolean;
  crash_summary: string;
}
