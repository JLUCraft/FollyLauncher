import {
  Box,
  Button,
  Divider,
  FormControl,
  FormLabel,
  HStack,
  Input,
  Select,
  Text,
  Textarea,
  VStack,
} from "@chakra-ui/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Section } from "@/components/common/section";
import { useLauncherConfig } from "@/contexts/config";
import { UnionState } from "@/services/union";
import { parseDeltas } from "@/utils/federation/score-deltas";

const MAX_IDENTIFIER_LENGTH = 128;
const MAX_TITLE_LENGTH = 256;
const MAX_REVIEW_LENGTH = 4096;

export function AdminConsole({
  state,
  busy,
  request,
}: {
  state: UnionState;
  busy: boolean;
  request: (action: Record<string, unknown>) => Promise<void>;
}) {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const [seasonId, setSeasonId] = useState(""),
    [seasonTitle, setSeasonTitle] = useState(""),
    [scheduleSeason, setScheduleSeason] = useState(""),
    [scheduleLeague, setScheduleLeague] = useState(""),
    [opensAt, setOpensAt] = useState(""),
    [closesAt, setClosesAt] = useState(""),
    [startsAt, setStartsAt] = useState(""),
    [reviewId, setReviewId] = useState(""),
    [reviewResponse, setReviewResponse] = useState(""),
    [playerDeltas, setPlayerDeltas] = useState(""),
    [teamDeltas, setTeamDeltas] = useState("");

  const invalidDeltas =
    parseDeltas(playerDeltas) === null || parseDeltas(teamDeltas) === null;
  const seasonLeagues = scheduleSeason
    ? (state.seasons?.[scheduleSeason]?.leagues ?? [])
    : [];
  const pendingDisputes = Object.entries(state.disputes ?? {}).filter(
    ([, d]) => !d.resolution
  );

  const toMs = (value: string) =>
    value ? new Date(value).getTime() : Number.NaN;

  return (
    <Section
      title={t("AdminConsole.title")}
      description={t("AdminConsole.description")}
      isAccordion
      initialIsOpen={false}
    >
      <VStack align="stretch" spacing={4}>
        <Box borderWidth="1px" borderRadius="md" p={3}>
          <VStack align="stretch" spacing={3}>
            <Text fontWeight="semibold" fontSize="sm">
              {t("AdminConsole.createSeason")}
            </Text>
            <HStack spacing={2} align="flex-end">
              <FormControl>
                <FormLabel htmlFor="season-id">
                  {t("AdminConsole.seasonId")}
                </FormLabel>
                <Input
                  id="season-id"
                  value={seasonId}
                  maxLength={MAX_IDENTIFIER_LENGTH}
                  onChange={(e) => setSeasonId(e.target.value)}
                />
              </FormControl>
              <FormControl>
                <FormLabel htmlFor="season-title">
                  {t("AdminConsole.seasonTitle")}
                </FormLabel>
                <Input
                  id="season-title"
                  value={seasonTitle}
                  maxLength={MAX_TITLE_LENGTH}
                  onChange={(e) => setSeasonTitle(e.target.value)}
                />
              </FormControl>
              <Button
                flexShrink={0}
                colorScheme={primaryColor}
                isDisabled={busy || !seasonId.trim() || !seasonTitle.trim()}
                onClick={() =>
                  void request({
                    type: "create_season",
                    id: seasonId.trim(),
                    title: seasonTitle.trim(),
                  })
                }
              >
                {t("AdminConsole.create")}
              </Button>
            </HStack>
          </VStack>
        </Box>

        <Box borderWidth="1px" borderRadius="md" p={3}>
          <VStack align="stretch" spacing={3}>
            <Text fontWeight="semibold" fontSize="sm">
              {t("AdminConsole.schedule")}
            </Text>
            <FormControl>
              <FormLabel htmlFor="schedule-season">
                {t("AdminConsole.scheduleSeason")}
              </FormLabel>
              <Select
                id="schedule-season"
                value={scheduleSeason}
                onChange={(e) => {
                  setScheduleSeason(e.target.value);
                  setScheduleLeague("");
                }}
                placeholder={t("AdminConsole.selectSeason")}
              >
                {Object.keys(state.seasons ?? {}).map((id) => (
                  <option key={id} value={id}>
                    {state.seasons?.[id]?.title || id}
                  </option>
                ))}
              </Select>
            </FormControl>
            <FormControl>
              <FormLabel htmlFor="schedule-league">
                {t("AdminConsole.scheduleLeague")}
              </FormLabel>
              <Select
                id="schedule-league"
                value={scheduleLeague}
                onChange={(e) => setScheduleLeague(e.target.value)}
                placeholder={t("AdminConsole.selectLeague")}
              >
                {seasonLeagues.map((id) => (
                  <option key={id} value={id}>
                    {id}
                  </option>
                ))}
              </Select>
            </FormControl>
            <HStack spacing={2} align="flex-end">
              {(
                [
                  ["opens", opensAt, setOpensAt],
                  ["closes", closesAt, setClosesAt],
                  ["starts", startsAt, setStartsAt],
                ] as const
              ).map(([key, value, setValue]) => (
                <FormControl key={key}>
                  <FormLabel htmlFor={`schedule-${key}`}>
                    {t(`AdminConsole.${key}At`)}
                  </FormLabel>
                  <Input
                    id={`schedule-${key}`}
                    type="datetime-local"
                    value={value}
                    onChange={(e) => setValue(e.target.value)}
                  />
                </FormControl>
              ))}
              <Button
                flexShrink={0}
                colorScheme={primaryColor}
                isDisabled={
                  busy ||
                  !scheduleSeason ||
                  !scheduleLeague ||
                  !opensAt ||
                  !closesAt ||
                  !startsAt
                }
                onClick={() =>
                  void request({
                    type: "schedule_league",
                    season: scheduleSeason,
                    league: scheduleLeague,
                    schedule: {
                      opens_ms: toMs(opensAt),
                      closes_ms: toMs(closesAt),
                      starts_ms: toMs(startsAt),
                    },
                  })
                }
              >
                {t("AdminConsole.submit")}
              </Button>
            </HStack>
          </VStack>
        </Box>

        {!!pendingDisputes.length && (
          <Box borderWidth="1px" borderRadius="md" p={3}>
            <VStack align="stretch" spacing={3}>
              <Text fontWeight="semibold" fontSize="sm">
                {t("AdminConsole.review")}
              </Text>
              <FormControl>
                <FormLabel htmlFor="review-select">
                  {t("AdminConsole.reviewSelect")}
                </FormLabel>
                <Select
                  id="review-select"
                  value={reviewId}
                  onChange={(e) => setReviewId(e.target.value)}
                  placeholder={t("AdminConsole.reviewPlaceholder")}
                >
                  {pendingDisputes.map(([id, d]) => (
                    <option key={id} value={id}>
                      {d.league} · {d.member}
                    </option>
                  ))}
                </Select>
              </FormControl>
              {reviewId && (
                <>
                  <Text fontSize="sm" className="secondary-text">
                    {
                      pendingDisputes.find(([id]) => id === reviewId)?.[1]
                        .reason
                    }
                  </Text>
                  <FormControl>
                    <FormLabel htmlFor="review-response">
                      {t("AdminConsole.reviewResponse")}
                    </FormLabel>
                    <Textarea
                      id="review-response"
                      value={reviewResponse}
                      maxLength={MAX_REVIEW_LENGTH}
                      onChange={(e) => setReviewResponse(e.target.value)}
                    />
                  </FormControl>
                  <FormControl>
                    <FormLabel htmlFor="review-players">
                      {t("AdminConsole.playerDeltas")}
                    </FormLabel>
                    <Textarea
                      id="review-players"
                      value={playerDeltas}
                      placeholder={t("AdminConsole.deltasPlaceholder")}
                      onChange={(e) => setPlayerDeltas(e.target.value)}
                    />
                  </FormControl>
                  <FormControl>
                    <FormLabel htmlFor="review-teams">
                      {t("AdminConsole.teamDeltas")}
                    </FormLabel>
                    <Textarea
                      id="review-teams"
                      value={teamDeltas}
                      placeholder={t("AdminConsole.deltasPlaceholder")}
                      onChange={(e) => setTeamDeltas(e.target.value)}
                    />
                  </FormControl>
                  {invalidDeltas && (
                    <Text role="alert">{t("AdminConsole.invalidDeltas")}</Text>
                  )}
                  <Button
                    alignSelf="flex-start"
                    colorScheme={primaryColor}
                    isDisabled={busy || !reviewResponse.trim() || invalidDeltas}
                    onClick={() => {
                      const players = parseDeltas(playerDeltas);
                      const teams = parseDeltas(teamDeltas);
                      if (!players || !teams) return;
                      void request({
                        type: "resolve_dispute",
                        id: reviewId,
                        response: reviewResponse.trim(),
                        ...(Object.keys(players).length
                          ? { players }
                          : undefined),
                        ...(Object.keys(teams).length ? { teams } : undefined),
                      });
                      setReviewId("");
                      setReviewResponse("");
                      setPlayerDeltas("");
                      setTeamDeltas("");
                    }}
                  >
                    {t("AdminConsole.resolve")}
                  </Button>
                </>
              )}
            </VStack>
          </Box>
        )}

        {!Object.keys(state.seasons ?? {}).length &&
          !pendingDisputes.length && (
            <>
              <Divider />
              <Text fontSize="sm" className="secondary-text">
                {t("AdminConsole.empty")}
              </Text>
            </>
          )}
      </VStack>
    </Section>
  );
}
