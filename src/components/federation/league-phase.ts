import type { TFunction } from "i18next";
import type { UnionLeague } from "@/services/union";

export const LEAGUE_PHASE_COLOR_SCHEME: Record<UnionLeague["phase"], string> = {
  Registration: "brand",
  Playing: "green",
  Finished: "gray",
};

export function leaguePhaseLabel(
  t: TFunction,
  phase: UnionLeague["phase"],
  manualPause = false
): string {
  if (phase === "Playing" && manualPause) return t("SeasonPanel.paused");
  return t(`FederationPage.phase.${phase}`, { defaultValue: phase });
}
