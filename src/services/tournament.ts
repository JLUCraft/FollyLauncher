import { invoke } from "@tauri-apps/api/core";

/** Aligned with federated-server league.rs TournamentStatus (kebab-case). */
export type TournamentStatus =
  | "draft"
  | "registration"
  | "ongoing"
  | "paused"
  | "cancelled"
  | "completed";

/** Aligned with federated-server league.rs MatchStatus (kebab-case). */
export type MatchStatus =
  | "scheduled"
  | "live"
  | "finished"
  | "disputed";

export interface TournamentSchedule {
  registration_open: string;
  registration_close: string;
  matches: MatchSchedule[];
}

export interface MatchSchedule {
  round: number;
  datetime: string;
  map: string;
}

export interface ScoringRules {
  win: number;
  kill: number;
  survive_minute: number;
  placement_1: number;
  placement_2: number;
  placement_3: number;
}

export interface Tournament {
  id: string;
  name: string;
  game_type: string;
  mode: string;
  status: TournamentStatus;
  participant_count: number;
  max_participants: number;
  created_at: string;
  created_by: string;
  /** Optional schedule with registration window and match timeline. */
  schedule?: TournamentSchedule;
  /** Optional scoring rules for the tournament. */
  scoring?: ScoringRules;
  /** Minimum score a member must have to register (club gate). */
  min_member_score: number;
}

export interface PlayerResult {
  player_id: string;
  score: number;
  kills: number;
  deaths: number;
  survive_minutes: number;
}

export interface MatchResult {
  rankings: PlayerResult[];
}

export interface Match {
  id: string;
  tournament_id: string;
  round: number;
  participants: string[];
  instance_id?: string | null;
  result?: MatchResult | null;
  status: MatchStatus;
  scheduled_at: string;
}

export interface DisputeMatch {
  dispute_id: string;
  tournament_id: string;
  match_id: string;
  status: string;
  reason: string;
  evidence_urls: string[];
  submitted_by?: string | null;
  resolution?: string | null;
  created_at: string;
  resolved_at?: string | null;
  resolved_by?: string | null;
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

export async function createMatchDispute(
  tournamentId: string,
  matchId: string,
  reason: string,
  evidenceUrls: string[] = [],
): Promise<DisputeMatch> {
  return invoke("create_match_dispute", {
    tournamentId,
    matchId,
    reason,
    evidenceUrls,
  });
}

export async function listDisputes(
  tournamentId?: string | null,
): Promise<DisputeMatch[]> {
  return invoke("list_disputes", { tournamentId: tournamentId ?? null });
}

export async function getDispute(
  disputeId: string,
): Promise<DisputeMatch | null> {
  return invoke("get_dispute", { disputeId });
}

// resolveDispute removed - FollyLauncher is a player-side launcher
// without admin TEE. Dispute resolution must go through
// union-manager's resolveDisputeViaProposal path.

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
