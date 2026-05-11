import { invoke } from "@tauri-apps/api/core";

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

// createInstance removed — players must use createQuickRoom, which
// auto-derives kind/club/owner from the current identity and room config.
// Admin instance creation goes through union-manager's governance path.

export async function resolveInstance(
  instanceId: string
): Promise<ResolvedInstance | null> {
  return invoke("resolve_instance", { instanceId });
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

export interface LaunchInstanceResult {
  bridge_port: number;
  pid: number | null;
  instance_id: string;
  target_peer_id: string;
  username: string;
  uuid: string;
  member_vc: boolean;
}

export interface BootstrapStatus {
  configured: boolean;
  peer_count: number;
  peers: string[];
  message: string;
}

export async function launchInstance(
  instanceId: string,
  version: string,
): Promise<LaunchInstanceResult> {
  return invoke("launch_instance", { instanceId, version });
}

export async function getBootstrapStatus(): Promise<BootstrapStatus> {
  return invoke("get_bootstrap_status");
}

export async function getResourceSyncStatus(instanceId: string): Promise<SyncStatus> {
  return invoke("get_resource_sync_status", { instanceId });
}

export async function syncResources(instanceId: string): Promise<SyncResult> {
  return invoke("sync_resources", { instanceId });
}

// Eligibility check (P1: admission-aware joining)

export interface EligibilityResult {
  instance_id: string;
  eligible: boolean;
  reason: string | null;
  admission_mode: import("../types").AdmissionMode;
  requires_vc: boolean;
  user_club?: string | null;
  user_role?: string | null;
  resources_missing: boolean;
  available_memory_mb: number;
  available_disk_mb: number;
  memory_warning: boolean;
  disk_warning: boolean;
}

export async function checkInstanceEligibility(
  instanceId: string,
): Promise<EligibilityResult> {
  return invoke("check_instance_eligibility", { instanceId });
}

// Quick room creation (P1: one-click room from server page)

export interface QuickRoomResult {
  id: string;
  name: string;
  kind: string;
  club: string;
  status: string;
  peer_id: string | null;
}

export async function createQuickRoom(
  name: string,
  version: string,
  admission?: string | null,
): Promise<QuickRoomResult> {
  return invoke("create_quick_room", { name, version, admission });
}

// PubSub topic subscriptions

export async function subscribeInstanceEvents(instanceId: string): Promise<void> {
  return invoke("subscribe_instance_events", { instanceId });
}

export async function unsubscribeInstanceEvents(instanceId: string): Promise<void> {
  return invoke("unsubscribe_instance_events", { instanceId });
}

export async function subscribeTournamentEvents(
  tournamentId: string,
): Promise<void> {
  return invoke("subscribe_tournament_events", { tournamentId });
}

export async function unsubscribeTournamentEvents(
  tournamentId: string,
): Promise<void> {
  return invoke("unsubscribe_tournament_events", { tournamentId });
}

// Invite players

export interface InvitePlayersResult {
  instance_id: string;
  invited_count: number;
  missing_recipients: string[];
}

export async function invitePlayers(
  instanceId: string,
  players: string[],
): Promise<InvitePlayersResult> {
  return invoke("invite_players", { instanceId, players });
}

// Network diagnostics

export interface NetworkDiagnostics {
  connected_peers: number;
  dht_peers: number;
  relay_connected: boolean;
  dcutr_holes_punched: number;
  dcutr_failures: number;
  latencies: PeerLatency[];
  active_sessions: number;
  total_bytes_rx: number;
  total_bytes_tx: number;
  bootstrap_peer_count: number;
  bootstrap_reachable: boolean;
}

export interface PeerLatency {
  peer_id: string;
  latency_ms: number;
  stale: boolean;
}

export async function getNetworkDiagnostics(): Promise<NetworkDiagnostics> {
  return invoke("get_network_diagnostics");
}

// DESIGN.md 2.6: Migration health probe

export interface MigrationProbeResponse {
  status: string;
  target_peer_id: string;
  supported_protocols: string[];
  available_disk_mb: number;
  cpu_headroom_pct: number;
  memory_headroom_mb: number;
  estimated_rtt_ms: number;
  reason: string | null;
  checked_at: string;
}

export async function probeMigrationHealth(
  instanceId: string,
  sourcePeerId: string,
): Promise<MigrationProbeResponse> {
  return invoke("probe_migration_health", {
    instanceId,
    sourcePeerId,
  });
}
