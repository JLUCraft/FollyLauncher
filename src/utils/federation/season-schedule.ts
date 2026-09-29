import type { UnionState } from "@/services/union";

export function schedulableLeagues(
  state: UnionState,
  season: string
): string[] {
  if (!state.seasons?.[season]) return [];
  return Object.entries(state.leagues)
    .filter(
      ([id, league]) =>
        league.phase === "Registration" &&
        !Object.values(league.teams).some((team) => team.registered) &&
        !Object.entries(state.seasons || {}).some(
          ([other, value]) => other !== season && value.leagues.includes(id)
        )
    )
    .map(([id]) => id);
}
