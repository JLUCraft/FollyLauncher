import {
  Badge,
  Box,
  Button,
  Divider,
  FormControl,
  FormLabel,
  HStack,
  Select,
  Text,
  Textarea,
  VStack,
} from "@chakra-ui/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import { Section } from "@/components/common/section";
import { leaguePhaseLabel } from "@/components/federation/league-phase";
import { Scoreboard } from "@/components/federation/scoreboard";
import { UnionState } from "@/services/union";
import { ISOToDatetime } from "@/utils/datetime";

export function SeasonPanel({
  state,
  member,
  busy,
  request,
}: {
  state: UnionState;
  member?: string;
  busy: boolean;
  request: (action: Record<string, unknown>) => Promise<boolean>;
}) {
  const { t } = useTranslation();
  const [league, setLeague] = useState(""),
    [reason, setReason] = useState("");
  const eligible = Object.entries(state.leagues).filter(
    ([, l]) =>
      l.phase === "Finished" &&
      member &&
      Object.values(l.teams).some(
        (t) => t.registered && t.members.includes(member)
      )
  );
  async function handleSubmit() {
    const accepted = await request({
      type: "dispute_result",
      id: crypto.randomUUID(),
      league,
      reason,
    });
    if (accepted) setReason("");
  }
  return (
    <Section
      title={t("SeasonPanel.title")}
      description={t("SeasonPanel.description")}
      isAccordion
      initialIsOpen={false}
    >
      <VStack align="stretch" spacing={4}>
        {Object.entries(state.seasons ?? {}).map(([id, s]) => (
          <AdvancedCard key={id} level="front" p={3}>
            <Text fontWeight="bold">{s.title}</Text>
            <VStack align="stretch" mt={2} spacing={1}>
              {s.leagues.map((id) => {
                const l = state.leagues[id];
                return (
                  <Text key={id} fontSize="sm">
                    {id} ·{" "}
                    {l?.schedule
                      ? ISOToDatetime(
                          new Date(l.schedule.starts_ms).toISOString()
                        )
                      : t("SeasonPanel.pendingSchedule")}{" "}
                    ·{" "}
                    {leaguePhaseLabel(
                      t,
                      l?.phase ?? "Registration",
                      l?.manual_pause
                    )}
                  </Text>
                );
              })}
            </VStack>
            {(["team_scores", "player_scores"] as const).map((kind) => (
              <Box key={kind} mt={3}>
                <Scoreboard
                  title={t(
                    kind === "team_scores"
                      ? "FederationPage.league.teamScoreboard"
                      : "FederationPage.league.playerScoreboard"
                  )}
                  entries={
                    state.season_rankings?.[id]?.[
                      kind === "team_scores" ? "teams" : "players"
                    ] ?? []
                  }
                  emptyText={t("SeasonPanel.noRecords")}
                  rankVariant="badge"
                  compact
                />
              </Box>
            ))}
          </AdvancedCard>
        ))}
        {!Object.keys(state.seasons ?? {}).length && (
          <Text>{t("SeasonPanel.empty")}</Text>
        )}
        {!!eligible.length && (
          <>
            <Divider />
            <AdvancedCard level="front" p={3}>
              <VStack align="stretch" spacing={3}>
                <FormControl>
                  <FormLabel htmlFor="review-league">
                    {t("SeasonPanel.review.leagueLabel")}
                  </FormLabel>
                  <Select
                    id="review-league"
                    value={league}
                    onChange={(e) => setLeague(e.target.value)}
                    placeholder={t("SeasonPanel.review.leaguePlaceholder")}
                  >
                    {eligible.map(([id]) => (
                      <option key={id} value={id}>
                        {id}
                      </option>
                    ))}
                  </Select>
                </FormControl>
                <FormControl>
                  <FormLabel htmlFor="review-reason">
                    {t("SeasonPanel.review.reasonLabel")}
                  </FormLabel>
                  <Textarea
                    id="review-reason"
                    value={reason}
                    maxLength={4096}
                    onChange={(e) => setReason(e.target.value)}
                  />
                </FormControl>
                <Button
                  alignSelf="flex-start"
                  isDisabled={busy || !league || !reason.trim()}
                  onClick={() => void handleSubmit()}
                >
                  {t("SeasonPanel.review.submit")}
                </Button>
              </VStack>
            </AdvancedCard>
          </>
        )}
        {Object.entries(state.disputes ?? {})
          .filter(([, d]) => d.member === member)
          .map(([id, d]) => (
            <AdvancedCard key={id} level="front" p={3}>
              <HStack justify="space-between" align="center">
                <Text fontSize="sm" fontWeight="semibold">
                  {d.league}
                </Text>
                <Badge colorScheme={d.resolution ? "green" : "yellow"}>
                  {d.resolution
                    ? t("SeasonPanel.dispute.resolved")
                    : t("SeasonPanel.dispute.pending")}
                </Badge>
              </HStack>
              <Text fontSize="sm" mt={1}>
                {d.reason}
              </Text>
              {d.resolution && (
                <Text fontSize="xs" className="secondary-text" mt={1}>
                  {t("SeasonPanel.dispute.resolution", {
                    response: d.resolution.response,
                  })}
                </Text>
              )}
            </AdvancedCard>
          ))}
      </VStack>
    </Section>
  );
}
