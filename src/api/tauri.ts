import { invoke } from "@tauri-apps/api/core";

export interface IdentityResponse {
  peer_id: string;
  public_key: string;
  club: string | null;
}

export interface ProxyInfo {
  local_port: number;
}

export async function health(): Promise<string> {
  return invoke("health");
}

export async function getIdentity(): Promise<IdentityResponse> {
  return invoke("get_identity");
}

export async function getProxyPort(): Promise<ProxyInfo> {
  return invoke("get_proxy_port");
}

export interface ProxySession {
  instance_id: string;
  local_port: number;
  target_peer_id: string;
  bytes_in: number;
  bytes_out: number;
  state: string;
  started_at: string;
}

export async function listProxySessions(): Promise<ProxySession[]> {
  return invoke("list_proxy_sessions");
}

export interface ResolvedInstance {
  instance_id: string;
  peer_id: string;
  proxy_address: string | null;
  public_ips: string[];
  multiaddrs: string[];
  resolved_at: string;
}

export async function listPeers(): Promise<string[]> {
  return invoke("list_peers");
}

export interface InstanceInfo {
  id: string;
  name: string;
  kind: string;
  status: string;
  host: string;
  mode: string;
  club: string;
  players: number;
  max_players: number;
  version: string;
  peer_id: string;
  discovered_at: string;
  updated_at: string;
}

export async function listInstances(): Promise<InstanceInfo[]> {
  return invoke("list_instances");
}

export async function measureLatency(peerId: string): Promise<number | null> {
  return invoke("measure_latency", { peerId });
}

export interface HttpInstance {
  id: string;
  name: string;
  kind: string;
  owner: string;
  club: string;
  host: string;
  status: string;
  created_at: string;
  updated_at: string;
  host_port: number;
}

export async function listInstancesHttp(): Promise<HttpInstance[]> {
  return invoke("list_instances_http");
}

export async function createInstance(
  name: string,
  kind: string,
  club: string,
  version: string
): Promise<HttpInstance> {
  return invoke("create_instance", { name, kind, club, version });
}

export async function resolveInstance(
  instanceId: string
): Promise<ResolvedInstance | null> {
  return invoke("resolve_instance", { instanceId });
}

export type VcHolderState = "Unverified" | "Member" | "Expired" | "Revoked";

export interface VcStatus {
  state: VcHolderState;
  role: string | null;
  club: string | null;
  issuer: string | null;
  verified: boolean;
}

export interface VcImportResult {
  state: VcHolderState;
  verified: boolean;
  expired: boolean;
}

export async function getVcStatus(): Promise<VcStatus> {
  return invoke("get_vc_status");
}

export async function importVc(vcJson: string): Promise<VcImportResult> {
  return invoke("import_vc", { vcJson });
}

export async function clearVc(): Promise<void> {
  return invoke("clear_vc");
}

export interface ClusterMessage {
  topic: string;
  peer_id: string;
  payload: Record<string, unknown>;
  received_at: string;
}

export async function getClusterMessages(): Promise<ClusterMessage[]> {
  return invoke("get_cluster_messages");
}

export interface PeerLatency {
  peer_id: string;
  latency_ms: number;
  stale: boolean;
}

export interface NetworkDiagnostics {
  connected_peers: number;
  dht_peers: number;
  relay_connected: boolean;
  dcutr_holes_punched: number;
  dcutr_failures: number;
  latencies: PeerLatency[];
}

export async function getNetworkDiagnostics(): Promise<NetworkDiagnostics> {
  return invoke("get_network_diagnostics");
}

export interface Tournament {
  id: string;
  name: string;
  game_type: string;
  mode: string;
  status: string;
  participant_count: number;
  max_participants: number;
  created_at: string;
  created_by: string;
}

export interface Match {
  id: string;
  tournament_id: string;
  round: number;
  participants: string[];
  status: string;
  scheduled_at: string;
}

export async function listTournaments(): Promise<Tournament[]> {
  return invoke("list_tournaments");
}

export async function getTournament(id: string): Promise<Tournament | null> {
  return invoke("get_tournament", { id });
}

export async function listMatches(tournamentId: string): Promise<Match[]> {
  return invoke("list_matches", { tournamentId });
}

export async function registerForTournament(tournamentId: string): Promise<void> {
  return invoke("register_for_tournament", { tournamentId });
}

export interface Team {
  id: string;
  name: string;
  members: string[];
  total_score: number;
  tournament_ids: string[];
}

export async function createTeam(name: string, members: string[]): Promise<void> {
  return invoke("create_team", { name, members });
}

export async function listTeams(): Promise<Team[]> {
  return invoke("list_teams");
}

export interface MuaLoginStatus {
  logged_in: boolean;
  username: string | null;
  uuid: string | null;
  auth_server_url: string;
  peer_bound: boolean;
  is_guest: boolean;
  is_member: boolean;
}

export interface StartAuthResponse {
  user_code: string;
  verification_uri: string;
}

export async function getMuaStatus(): Promise<MuaLoginStatus> {
  return invoke("get_mua_status");
}

export async function startMuaLogin(): Promise<StartAuthResponse> {
  return invoke("start_mua_login");
}

export async function pollMuaLogin(): Promise<MuaLoginStatus> {
  return invoke("poll_mua_login");
}

export async function logoutMua(): Promise<void> {
  return invoke("logout_mua");
}

export interface SkinTextures {
  skin_url: string | null;
  cape_url: string | null;
}

export async function getSkinTextures(): Promise<SkinTextures> {
  return invoke("get_skin_textures");
}

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

export interface SyncStatus {
  manifest_loaded: boolean;
  total_files: number;
  cached_files: number;
  missing_files: number;
  sync_in_progress: boolean;
  last_error: string | null;
}

export interface SyncResult {
  downloaded: number;
  failed: number;
  skipped: number;
}

export async function getResourceSyncStatus(instanceId: string): Promise<SyncStatus> {
  return invoke("get_resource_sync_status", { instanceId });
}

export async function syncResources(instanceId: string): Promise<SyncResult> {
  return invoke("sync_resources", { instanceId });
}

// ==================== Resource Download (CurseForge / Modrinth) ====================

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

// ==================== Discover / News ====================

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

// ==================== Task System ====================

export interface TaskProgress {
  taskId: number;
  name: string;
  current: number;
  total: number;
  status: string;
  message: string;
}

export interface TaskGroup {
  groupId: string;
  tasks: TaskProgress[];
  overallStatus: string;
}

export async function listTaskGroups(): Promise<TaskGroup[]> {
  return invoke("list_task_groups");
}

export async function cancelTaskGroup(groupId: string): Promise<void> {
  return invoke("cancel_task_group", { groupId });
}

export async function removeTaskGroup(groupId: string): Promise<void> {
  return invoke("remove_task_group", { groupId });
}
