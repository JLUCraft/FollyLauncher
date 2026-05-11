import { invoke } from "@tauri-apps/api/core";

export interface GameClientResourceInfo {
  id: string;
  gameType: string;
  releaseTime: string;
  url: string;
}

export async function fetchGameVersionList(): Promise<GameClientResourceInfo[]> {
  return invoke("fetch_game_version_list");
}

export type ModLoaderType = "Unknown" | "Forge" | "Fabric" | "NeoForge" | "Quilt" | "LiteLoader" | "LegacyForge" | "OptiFine";

export interface ModLoaderResourceInfo {
  loaderType: ModLoaderType;
  version: string;
  description: string;
  stable: boolean;
  branch: string | null;
}

export async function fetchModLoaderVersionList(
  gameVersion: string,
  modLoaderType: ModLoaderType,
): Promise<ModLoaderResourceInfo[]> {
  return invoke("fetch_mod_loader_version_list", { gameVersion, modLoaderType });
}

export interface OptiFineResourceInfo {
  filename: string;
  patch: string;
  type: string;
}

export async function fetchOptiFineVersionList(gameVersion: string): Promise<OptiFineResourceInfo[]> {
  return invoke("fetch_optifine_version_list", { gameVersion });
}

export type OtherResourceSource = "CurseForge" | "Modrinth";

export interface OtherResourceInfo {
  id: string;
  mcmodId: number;
  _type: string;
  name: string;
  slug: string;
  translatedName: string | null;
  description: string;
  translatedDescription: string | null;
  iconSrc: string;
  tags: string[];
  lastUpdated: string;
  downloads: number;
  source: OtherResourceSource;
  websiteUrl: string;
  author: string | null;
}

export interface OtherResourceSearchRes {
  list: OtherResourceInfo[];
  total: number;
  page: number;
  pageSize: number;
}

export interface OtherResourceSearchQuery {
  resourceType: string;
  searchQuery: string;
  gameVersion: string;
  selectedTag: string;
  sortBy: string;
  page: number;
  pageSize: number;
}

export async function fetchResourceListByName(
  downloadSource: OtherResourceSource,
  query: OtherResourceSearchQuery,
): Promise<OtherResourceSearchRes> {
  return invoke("fetch_resource_list_by_name", { downloadSource, query });
}

export interface OtherResourceFileInfo {
  resourceId: string;
  name: string;
  releaseType: string;
  downloads: number;
  fileDate: string;
  downloadUrl: string;
  sha1: string;
  fileName: string;
  dependencies: OtherResourceDependency[];
  loader: string | null;
}

export interface OtherResourceDependency {
  resourceId: string;
  relation: string;
}

export interface ResourceDependencySummary {
  required: number;
  optional: number;
  embedded: number;
  other: number;
  items: OtherResourceDependency[];
}

export interface InstallResourceResult {
  instanceId: string;
  destPath: string;
  fileName: string;
  bytesWritten: number;
  sha1Verified: boolean;
  replacedExisting: boolean;
  dependencySummary: ResourceDependencySummary;
}

export interface OtherResourceVersionPack {
  name: string;
  items: OtherResourceFileInfo[];
}

export interface OtherResourceVersionPackQuery {
  resourceId: string;
  modLoader: string;
  gameVersions: string[];
}

export async function fetchResourceVersionPacks(
  downloadSource: OtherResourceSource,
  query: OtherResourceVersionPackQuery,
): Promise<OtherResourceVersionPack[]> {
  return invoke("fetch_resource_version_packs", { downloadSource, query });
}

export async function fetchRemoteResourceByLocal(
  downloadSource: OtherResourceSource,
  filePath: string,
): Promise<OtherResourceFileInfo> {
  return invoke("fetch_remote_resource_by_local", { downloadSource, filePath });
}

export async function fetchRemoteResourceById(
  downloadSource: OtherResourceSource,
  resourceId: string,
): Promise<OtherResourceInfo> {
  return invoke("fetch_remote_resource_by_id", { downloadSource, resourceId });
}

export async function downloadGameServer(
  resourceInfo: GameClientResourceInfo,
  dest: string,
): Promise<void> {
  return invoke("download_game_server", { resourceInfo, dest });
}

export interface ModUpdateQuery {
  url: string;
  sha1: string;
  fileName: string;
  oldFilePath: string;
}

export async function updateMods(
  instanceId: string,
  queries: ModUpdateQuery[],
): Promise<void> {
  return invoke("update_mods", { instanceId, queries });
}

export type InstallResourceKind = "Mod" | "ResourcePack" | "ShaderPack";

export interface InstallResourceRequest {
  instanceId: string;
  kind: InstallResourceKind;
  file: OtherResourceFileInfo;
  overwrite: boolean;
}

export async function installResourceToInstance(
  request: InstallResourceRequest,
): Promise<InstallResourceResult> {
  return invoke("install_resource_to_instance", { request });
}

export interface NewsSourceInfo {
  name: string;
  fullName: string;
  endpointUrl: string;
  iconSrc: string;
}

export interface NewsPostSummary {
  title: string;
  abstract: string | null;
  keywords: string | null;
  imageSrc: [string, number, number] | null;
  source: NewsSourceInfo;
  createAt: string;
  link: string;
}

export interface NewsPostRequest {
  url: string;
  cursor: number | null;
}

export interface NewsPostResponse {
  posts: NewsPostSummary[];
  next: number | null;
  cursors: Record<string, number> | null;
}

export async function fetchNewsSourcesInfo(): Promise<NewsSourceInfo[]> {
  return invoke("fetch_news_sources_info");
}

export async function fetchNewsPostSummaries(
  requests: NewsPostRequest[],
): Promise<NewsPostResponse> {
  return invoke("fetch_news_post_summaries", { requests });
}

// ── Client version install ───────────────────────────────────────────────

export interface InstallClientVersionRequest {
  instanceId: string;
  overwrite: boolean;
}

export interface InstallClientVersionResult {
  instanceId: string;
  gameVersion: string;
  versionJsonPath: string;
  clientJarPath: string;
  jsonBytesWritten: number;
  jarBytesWritten: number;
  jarSha1Verified: boolean;
  usedManifestUrl: string;
  replacedExisting: boolean;
}

export async function installClientVersionForInstance(
  request: InstallClientVersionRequest,
): Promise<InstallClientVersionResult> {
  return invoke("install_client_version_for_instance", { request });
}

// ── Phase 31: Async client version install ───────────────────────────

export interface AsyncInstallClientVersionRequest {
  instanceId: string;
  overwrite: boolean;
}

export interface AsyncInstallTaskStarted {
  groupId: string;
  taskId: number;
  instanceId: string;
  gameVersion: string;
}

export async function startInstallClientVersionTask(
  request: AsyncInstallClientVersionRequest,
): Promise<AsyncInstallTaskStarted> {
  return invoke("start_install_client_version_task", { request });
}

// ── Phase 16: Libraries install ────────────────────────────────────────────

export interface InstallLibrariesRequest {
  instanceId: string;
  overwrite: boolean;
}

export interface InstallLibrariesResult {
  instanceId: string;
  gameVersion: string;
  scanned: number;
  downloaded: number;
  skipped: number;
  failed: number;
  bytesWritten: number;
  librariesDir: string;
}

export async function installLibrariesForInstance(
  request: InstallLibrariesRequest,
): Promise<InstallLibrariesResult> {
  return invoke("install_libraries_for_instance", { request });
}

// ── Phase 32: Async libraries install ──────────────────────────────────────

export interface AsyncInstallLibrariesRequest {
  instanceId: string;
  overwrite: boolean;
}

export interface AsyncInstallLibrariesStarted {
  groupId: string;
  taskId: number;
  instanceId: string;
  gameVersion: string;
}

export async function startInstallLibrariesTask(
  request: AsyncInstallLibrariesRequest,
): Promise<AsyncInstallLibrariesStarted> {
  return invoke("start_install_libraries_task", { request });
}

// ── Phase 17: Assets install ─────────────────────────────────────────────────

export interface InstallAssetsRequest {
  instanceId: string;
  overwrite: boolean;
}

export interface InstallAssetsResult {
  instanceId: string;
  gameVersion: string;
  assetIndexId: string;
  indexBytesWritten: number;
  scanned: number;
  downloaded: number;
  skipped: number;
  failed: number;
  bytesWritten: number;
  assetsDir: string;
}

export async function installAssetsForInstance(
  req: InstallAssetsRequest,
): Promise<InstallAssetsResult> {
  return invoke("install_assets_for_instance", { request: req });
}

// ── Phase 33: Async assets install ───────────────────────────────────────

export interface AsyncInstallAssetsRequest {
  instanceId: string;
  overwrite: boolean;
}

export interface AsyncInstallAssetsStarted {
  groupId: string;
  taskId: number;
  instanceId: string;
  gameVersion: string;
}

export async function startInstallAssetsTask(
  request: AsyncInstallAssetsRequest,
): Promise<AsyncInstallAssetsStarted> {
  return invoke("start_install_assets_task", { request });
}

// ── Phase 18: Loader install ─────────────────────────────────────────────────

export type InstallLoaderKind = "Fabric" | "Quilt" | "Forge" | "NeoForge";

export interface InstallLoaderRequest {
  instanceId: string;
  kind: InstallLoaderKind;
  loaderVersion: string | null;
  overwrite: boolean;
}

export interface InstallLoaderResult {
  instanceId: string;
  previousGameVersion: string;
  newGameVersion: string;
  kind: InstallLoaderKind;
  loaderVersion: string;
  versionJsonPath: string;
  bytesWritten: number;
  replacedExisting: boolean;
}

export async function installLoaderForInstance(
  request: InstallLoaderRequest,
): Promise<InstallLoaderResult> {
  return invoke("install_loader_for_instance", { request });
}

// ── Phase 34: Async loader install ─────────────────────────────────────────

export interface AsyncInstallLoaderRequest {
  instanceId: string;
  kind: InstallLoaderKind;
  loaderVersion?: string;
  overwrite: boolean;
}

export interface AsyncInstallLoaderStarted {
  groupId: string;
  taskId: number;
  instanceId: string;
  kind: string;
}

export async function startInstallLoaderTask(
  request: AsyncInstallLoaderRequest,
): Promise<AsyncInstallLoaderStarted> {
  return invoke("start_install_loader_task", { request });
}

// ── Phase 35: Async resource install ────────────────────────────────────────

/** Phase 35: 异步资源安装请求 */
export interface AsyncInstallResourceRequest {
  instanceId: string;
  kind: InstallResourceKind;
  file: OtherResourceFileInfo;
  overwrite: boolean;
}

/** Phase 35: 异步资源安装启动响应 */
export interface AsyncInstallResourceStarted {
  groupId: string;
  taskId: number;
  instanceId: string;
  fileName: string;
}

/** 启动后台异步资源安装任务，立即返回 group_id */
export async function startInstallResourceTask(
  request: AsyncInstallResourceRequest,
): Promise<AsyncInstallResourceStarted> {
  return invoke("start_install_resource_task", { request });
}
