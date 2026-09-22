import { invoke } from "@tauri-apps/api/core";

export interface UnionTeam {
  captain: string;
  members: string[];
  invited: string[];
  registered: boolean;
}
export interface UnionLeague {
  phase: "Registration" | "Playing" | "Finished";
  rules: { cross_club: boolean; seats: number };
  teams: Record<string, UnionTeam>;
  player_scores: Record<string, number>;
  team_scores: Record<string, number>;
}
export interface UnionState {
  revision: number;
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
  feedback?: Record<
    string,
    { member: string; category: string; text: string; response: string | null }
  >;
  proposals?: Record<
    string,
    {
      action: { type: string };
      approvals: Record<string, string>;
      executed: boolean;
      cancelled: boolean;
    }
  >;
  members: Record<string, { club: string; device: string }>;
  leagues: Record<string, UnionLeague>;
}
export class UnionService {
  static async connectGame(link: string): Promise<string> {
    const url = new URL(link);
    if (url.protocol !== "jlucraft:" || url.hostname !== "connect") {
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
