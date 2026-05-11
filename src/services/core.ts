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
  /** QUIC substream ID (if available). Used for diagnostics. */
  substream_id: string | null;
  /** OS process ID of the Minecraft client. */
  process_id: number | null;
  bytes_in: number;
  bytes_out: number;
  state: string;
  started_at: string;
}

export async function listProxySessions(): Promise<ProxySession[]> {
  return invoke("list_proxy_sessions");
}

// PeerLatency, NetworkDiagnostics, and getNetworkDiagnostics are now
// the canonical definitions in services/network.ts (extended with
// active_sessions, total_bytes_rx/tx, bootstrap fields per DESIGN.md §3.4).
