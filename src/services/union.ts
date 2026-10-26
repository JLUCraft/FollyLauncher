import { invoke } from "@tauri-apps/api/core";
import { brand } from "@/brand";

export function isIdentityNotConfigured(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    error.code === "identity_not_configured"
  );
}

export type GameConnectionOrigin = "room" | "match";
export interface GameConnection {
  address: string;
  peer: string;
  service: string;
  origin?: GameConnectionOrigin;
}
export interface IdentityFacts {
  federation: string;
  checkpoint: { sequence: number; head: string };
  devices: Record<string, string>;
  revoked_devices: string[];
  memberships: [string, string][];
  profiles: Record<string, string>;
}
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
export interface UnionActivity {
  title: string;
  organizer: string;
  creator: string;
  phase: "draft" | "review" | "approved" | "open" | "done" | "cancelled";
  window: { opens_ms: number; closes_ms: number; starts_ms: number };
  brief_hash: string;
  epoch: number;
}
export interface PointStatement {
  kind: string;
  payload: string;
  public_key: number[];
  signature: number[];
}
export interface PointPresentation {
  max_output_tokens: number;
  genesis: string;
  revision: number;
  as_of_ms: number;
  day_ends_ms: number;
  account: { club: string; member: string };
  available_points: string;
  frozen_points: string;
  earned_today: string;
  active_slices: number;
  afk_slices: number;
  daily_member_cap: string | null;
  activity: {
    id: string;
    source: string;
    credited_ms: number;
    points: string;
    active_slices: number;
    afk_slices: number;
  }[];
  ai: {
    quote_points: string;
    id: string;
    stage: number;
    flow_status: string;
    status: string;
    started_ms: number;
    model: string;
    purpose: string;
    frozen_points: string;
    charged_points: string;
    refunded_points: string;
    input_tokens: string;
    output_tokens: string;
    output: string | null;
  }[];
  prices: Record<
    string,
    {
      base_points: string;
      input_points_per_1k: string;
      output_points_per_1k: string;
      max_input_tokens: string;
      settlement: string;
    }
  >;
  statement: PointStatement;
}
export interface UnionState {
  points_view?: PointPresentation;
  ai_workers?: Record<string, { models: string[]; active: boolean }>;
  consensus_members?: Record<string, { peer: string; addresses: string[] }>;
  digest_anchors?: Record<
    string,
    { digest: string; actor: string; time_ms: number }
  >;
  season_rankings?: Record<
    string,
    { teams: [string, string][]; players: [string, string][] }
  >;
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
  members: Record<
    string,
    {
      club: string;
      device: string;
      additional_devices?: string[];
      roles?: string[];
    }
  >;
  activity_briefs?: Record<
    string,
    { description: string; attachment?: string | null }
  >;
  leagues: Record<string, UnionLeague>;
  events: Record<string, UnionActivity>;
  signups: Record<string, string[]>;
  checkins: Record<string, string[]>;
  tasks: Record<
    string,
    {
      event: string;
      title: string;
      assignee: string | null;
      done: boolean;
      due_ms: number | null;
    }
  >;
  flows?: Record<
    string,
    {
      status: string;
      current: number;
      payload: Record<string, unknown>;
      proposer: string;
    }
  >;
  organization?: {
    plans?: Record<string, { games?: Record<string, { league: string }> }>;
  };
}
export interface DiscoveredServer {
  peer: string;
  address: string;
  federation: string;
  genesis: unknown;
  genesis_hash: string;
  application: string | null;
  services: string[];
}
export class UnionService {
  static async discover(
    expectedGenesis?: string,
    waitMs?: number
  ): Promise<DiscoveredServer[]> {
    const result = await invoke<{ servers: DiscoveredServer[] }>(
      "union_discover",
      {
        input: {
          expected_genesis: expectedGenesis ?? null,
          wait_ms: waitMs ?? null,
        },
      }
    );
    return result.servers;
  }
  static pointOutput(
    snapshot: UnionState,
    instance: string,
    stage: number
  ): Promise<string> {
    return invoke("union_points_output", { snapshot, instance, stage });
  }
  static management(request: Record<string, unknown>): Promise<unknown> {
    return invoke("union_management", { request });
  }
  static publicIdentity(): Promise<{ peer: string; public_key: number[] }> {
    return invoke("union_public_identity");
  }
  static signIdentityOffline(record: unknown): Promise<unknown> {
    return invoke("union_sign_identity_offline", { record });
  }
  static identityConfiguration(config?: unknown): Promise<unknown> {
    return invoke("union_identity_configuration", { config: config ?? null });
  }
  static identityAction(operation: Record<string, unknown>): Promise<unknown> {
    return invoke("union_identity_action", { operation });
  }
  static bindHostDevice(): Promise<unknown> {
    return invoke("union_bind_host_device");
  }
  static automatedIdentity(): Promise<unknown> {
    return invoke("union_automated_identity");
  }

  static async connectGame(link: string): Promise<GameConnection> {
    const url = new URL(link);
    if (
      url.protocol !== `${brand.deeplinkScheme}:` ||
      url.hostname !== "connect"
    ) {
      throw new Error("INVALID_CONNECT_LINK");
    }
    const peer = url.searchParams.get("peer");
    const service = url.searchParams.get("service");
    const addresses = url.searchParams.getAll("address");
    if (!peer || !service || !addresses.length)
      throw new Error("INCOMPLETE_CONNECT_LINK");
    return invoke("union_proxy_start", {
      input: {
        peer,
        service,
        addresses,
        origin: "match",
      },
    });
  }
  static disconnectGame(): Promise<void> {
    return invoke("union_proxy_stop");
  }
  static gameConnection(): Promise<GameConnection | null> {
    return invoke("union_proxy_status");
  }

  static identity(): Promise<string> {
    return invoke("union_identity");
  }
  static async request(
    address: string,
    peer: string,
    genesisHash: string,
    revision?: number,
    action?: Record<string, unknown>
  ): Promise<UnionState> {
    const result = await invoke<{
      state: UnionState | null;
      season_rankings: UnionState["season_rankings"];
      activity_briefs: UnionState["activity_briefs"];
      error: string | null;
      error_code: string | null;
      identity_view?: { members: UnionState["members"]; admins: string[] };
    }>("union_governance", {
      input: { address, peer, genesis_hash: genesisHash, revision, action },
    });
    if (result.error || !result.state || !result.identity_view)
      throw {
        code: result.error_code ?? "invalid_response",
        message: result.error || "NO_APPLICATION_STATE",
      };
    return {
      ...result.state,
      ...result.identity_view,
      season_rankings: result.season_rankings,
      activity_briefs: result.activity_briefs,
    };
  }
}
