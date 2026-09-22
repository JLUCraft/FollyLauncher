import { invoke } from "@tauri-apps/api/core";
import { brand } from "@/brand";

export interface UnionTeam {
  captain: string;
  members: string[];
  invited: string[];
  registered: boolean;
}
export interface UnionLeague {
  schedule?: { opens_ms: number; closes_ms: number; starts_ms: number };
  manual_pause?: boolean;
  paused_since?: number | null;
  organizer?: string;
  phase: "Registration" | "Playing" | "Finished";
  rules: { cross_club: boolean; seats: number };
  teams: Record<string, UnionTeam>;
  player_scores: Record<string, number>;
  team_scores: Record<string, number>;
}
export interface UnionState {
  revision: number;
  clock_ms: number;
  admins?: string[];
  seasons?: Record<string, { title: string; leagues: string[] }>;
  disputes?: Record<
    string,
    {
      league: string;
      member: string;
      reason: string;
      resolution: { response: string } | null;
    }
  >;
  ballots?: Record<
    string,
    {
      title: string;
      options: string[];
      closes_ms: number;
      electorate: string[];
      votes: Record<string, number>;
    }
  >;
  council?: {
    epoch: number;
    schools: Record<string, string[]>;
    quorum: number;
    change_quorum: number;
    cooldown_ms: number;
  };
  feedback?: Record<
    string,
    { member: string; category: string; text: string; response: string | null }
  >;
  proposals?: Record<
    string,
    {
      proposer: string;
      action: { type: string };
      epoch: number;
      created_ms: number;
      expires_ms: number;
      approvals: Record<string, string>;
      executed: boolean;
      cancelled: boolean;
    }
  >;
  members: Record<
    string,
    { club: string; device: string; additional_devices?: string[] }
  >;
  leagues: Record<string, UnionLeague>;
}
export class UnionService {
  static async connectGame(link: string): Promise<string> {
    const url = new URL(link);
    if (
      url.protocol !== `${brand.deeplinkScheme}:` ||
      url.hostname !== "connect"
    ) {
      throw new Error("连接链接无效");
    }
    const peer = url.searchParams.get("peer");
    const service = url.searchParams.get("service");
    const addresses = url.searchParams.getAll("address");
    if (!peer || !service || !addresses.length)
      throw new Error("连接链接不完整");
    return invoke("union_proxy_start", {
      input: {
        peer,
        service,
        addresses,
        student: null,
        request_student: url.searchParams.get("school") === "1",
      },
    });
  }
  static disconnectGame(): Promise<void> {
    return invoke("union_proxy_stop");
  }
  static gameConnection(): Promise<string | null> {
    return invoke("union_proxy_status");
  }

  static identity(): Promise<string> {
    return invoke("union_identity");
  }
  static async request(
    address: string,
    peer: string,
    revision?: number,
    action?: Record<string, unknown>
  ): Promise<UnionState> {
    const result = await invoke<{
      state: UnionState | null;
      error: string | null;
    }>("union_governance", { input: { address, peer, revision, action } });
    if (result.error || !result.state)
      throw new Error(result.error || "No league state returned");
    return result.state;
  }
}
