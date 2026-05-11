import { invoke } from "@tauri-apps/api/core";

export interface GameSettings {
  java_path: string;
  max_memory_mb: number;
  jvm_args: string;
  game_directory: string;
  resolution_width: number;
  resolution_height: number;
  fullscreen: boolean;
  show_game_log: boolean;
}

export async function getGameSettings(): Promise<GameSettings> {
  return invoke("get_game_settings");
}

export async function updateGameSettings(settings: GameSettings): Promise<void> {
  return invoke("update_game_settings", { settings });
}

export async function selectGameDir(): Promise<string | null> {
  return invoke("select_game_dir");
}

export async function selectJavaPath(): Promise<string | null> {
  return invoke("select_java_path");
}

export type CloseBehavior = "Ask" | "MinimizeToTray" | "Exit";

export interface LauncherBasicConfig {
  language: string;
  theme: string;
  close_behavior: CloseBehavior;
  download_threads: number;
}

export interface JavaConfig {
  auto_scan: boolean;
  auto_select: boolean;
  preferred_java_path: string | null;
}

export interface AdvancedConfig {
  enable_process_monitor: boolean;
  enable_crash_report: boolean;
  keep_launcher_open: boolean;
}

export interface LauncherConfig {
  basic: LauncherBasicConfig;
  game: GameSettings;
  java: JavaConfig;
  advanced: AdvancedConfig;
}

export async function retrieveLauncherConfig(): Promise<LauncherConfig> {
  return invoke("retrieve_launcher_config");
}

export async function updateLauncherConfig(config: LauncherConfig): Promise<void> {
  return invoke("update_launcher_config", { config });
}

// ── Java runtime scanning (Phase 8) ────────────────────────────────────

export interface JavaRuntime {
  exec_path: string;
  version: string;
  major_version: number;
  is_64bit: boolean;
  vendor: string;
  home_path: string;
}

export async function retrieveJavaList(): Promise<JavaRuntime[]> {
  return invoke("retrieve_java_list");
}

export async function validateJava(javaPath: string): Promise<boolean> {
  return invoke("validate_java", { javaPath });
}
