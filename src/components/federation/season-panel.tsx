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
import { Section } from "@/components/common/section";
import { UnionState } from "@/services/union";

export function SeasonPanel({
  state,
  member,
  busy,
  request,
}: {
  state: UnionState;
  member?: string;
  busy: boolean;
  request: (action: Record<string, unknown>) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [league, setLeague] = useState(""),
    [reason, setReason] = useState("");
  // Mirrors union_core::season::season_rankings: only finished leagues count.
  const rankings = (ids: string[], kind: "team_scores" | "player_scores") => {
    const sums = new Map<string, number>();
    ids.forEach((id) => {
      const match = state.leagues[id];
      if (match?.phase === "Finished")
        Object.entries(match[kind]).forEach(([name, score]) =>
          sums.set(name, (sums.get(name) ?? 0) + score)
        );
    });
    return Array.from(sums).sort(
      (a, b) => b[1] - a[1] || a[0].localeCompare(b[0])
    );
  };
  const eligible = Object.entries(state.leagues).filter(
    ([, l]) =>
      l.phase === "Finished" &&
      member &&
      Object.values(l.teams).some(
        (t) => t.registered && t.members.includes(member)
      )
  );
  async function handleSubmit() {
    await request({
      type: "dispute_result",
      id: crypto.randomUUID(),
      league,
      reason,
    });
    setReason("");
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
          <Box key={id} borderWidth="1px" borderRadius="md" p={3}>
            <Text fontWeight="bold">{s.title}</Text>
            <VStack align="stretch" mt={2} spacing={1}>
              {s.leagues.map((id) => {
                const l = state.leagues[id];
                return (
                  <Text key={id} fontSize="sm">
                    {id} ·{" "}
                    {l?.schedule
                      ? new Date(l.schedule.starts_ms).toLocaleString()
                      : t("SeasonPanel.pendingSchedule")}{" "}
                    ·{" "}
                    {l?.phase === "Finished"
                      ? t("FederationPage.phase.Finished")
                      : l?.phase === "Playing"
                        ? l.manual_pause
                          ? t("SeasonPanel.paused")
                          : t("FederationPage.phase.Playing")
                        : t("FederationPage.phase.Registration")}
                  </Text>
                );
              })}
            </VStack>
            {(["team_scores", "player_scores"] as const).map((kind) => (
              <VStack key={kind} align="stretch" mt={3} spacing={1}>
                <Text fontSize="sm" fontWeight="semibold">
                  {kind === "team_scores"
                    ? t("FederationPage.league.teamScoreboard")
                    : t("FederationPage.league.playerScoreboard")}
                </Text>
                {rankings(s.leagues, kind).map(([name, score], index) => (
                  <HStack key={name} fontSize="sm" spacing={2}>
                    {index < 3 ? (
                      <Badge
                        colorScheme={
                          index === 0
                            ? "yellow"
                            : index === 1
                              ? "gray"
                              : "orange"
                        }
                      >
                        {index + 1}
                      </Badge>
                    ) : (
                      <Text w="2em" className="secondary-text">
                        {index + 1}
                      </Text>
                    )}
                    <Text>{name}</Text>
                    <Text ml="auto" className="secondary-text">
                      {score}
                    </Text>
                  </HStack>
                ))}
                {!rankings(s.leagues, kind).length && (
                  <Text fontSize="xs" className="secondary-text">
                    {t("SeasonPanel.noRecords")}
                  </Text>
                )}
              </VStack>
            ))}
          </Box>
        ))}
        {!Object.keys(state.seasons ?? {}).length && (
          <Text>{t("SeasonPanel.empty")}</Text>
        )}
        {!!eligible.length && (
          <>
            <Divider />
            <Box borderWidth="1px" borderRadius="md" p={3}>
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
            </Box>
          </>
        )}
        {Object.entries(state.disputes ?? {})
          .filter(([, d]) => d.member === member)
          .map(([id, d]) => (
            <Box key={id} borderWidth="1px" borderRadius="md" p={3}>
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
            </Box>
          ))}
      </VStack>
    </Section>
  );
}
