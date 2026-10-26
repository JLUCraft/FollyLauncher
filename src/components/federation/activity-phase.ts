import type { TFunction } from "i18next";
import type { UnionActivity } from "@/services/union";

export const ACTIVITY_PHASE_COLOR_SCHEME: Record<
  UnionActivity["phase"],
  string
> = {
  draft: "gray",
  review: "purple",
  approved: "brand",
  open: "green",
  done: "gray",
  cancelled: "red",
};

export function activityPhaseLabel(
  t: TFunction,
  phase: UnionActivity["phase"]
): string {
  return t(`FederationPage.activityPhase.${phase}`, { defaultValue: phase });
}
