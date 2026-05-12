import { invoke } from "@tauri-apps/api/core";



export type ModpackFileKind = "Mod" | "ResourcePack" | "ShaderPack";

export interface ModpackFileEntry {
  kind: ModpackFileKind;
  file_name: string;
  relative_path: string;
  source_path: string;
  size: number;
  sha1: string;
  enabled: boolean;
}

export interface ModpackManifest {
  schema_version: number;
  name: string;
  source_instance_id: string;
  game_version: string;
  instance_kind: string;
  exported_at: string;
  files: ModpackFileEntry[];
}

export interface ExportModpackManifestResult {
  manifest: ModpackManifest;
  file_count: number;
  total_bytes: number;
}

export interface ImportModpackManifestRequest {
  target_instance_id: string;
  manifest: ModpackManifest;
  overwrite: boolean;
}

export interface ImportModpackManifestResult {
  target_instance_id: string;
  imported: number;
  skipped: number;
  failed: number;
  bytes_written: number;
}



export async function exportModpackManifest(
  instanceId: string,
): Promise<ExportModpackManifestResult> {
  return invoke("export_modpack_manifest", { instanceId });
}

export async function importModpackManifest(
  request: ImportModpackManifestRequest,
): Promise<ImportModpackManifestResult> {
  return invoke("import_modpack_manifest", { request });
}



export interface ExportModpackZipRequest {
  instanceId: string;
  outputPath: string;
}

export interface ExportModpackZipResult {
  outputPath: string;
  fileCount: number;
  totalBytes: number;
  manifestBytes: number;
}

export interface ImportModpackZipRequest {
  targetInstanceId: string;
  zipPath: string;
  overwrite: boolean;
}

export interface ImportModpackZipResult {
  targetInstanceId: string;
  imported: number;
  skipped: number;
  failed: number;
  bytesWritten: number;
}



export async function exportModpackZip(
  request: ExportModpackZipRequest,
): Promise<ExportModpackZipResult> {
  return invoke("export_modpack_zip", { request });
}

export async function importModpackZip(
  request: ImportModpackZipRequest,
): Promise<ImportModpackZipResult> {
  return invoke("import_modpack_zip", { request });
}
