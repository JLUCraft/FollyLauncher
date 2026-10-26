import {
  Box,
  Button,
  Flex,
  FormControl,
  FormLabel,
  HStack,
  Input,
  Select,
  Tag,
  Text,
  VStack,
} from "@chakra-ui/react";
import { useRouter } from "next/router";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import { ConnectionRequired } from "@/components/federation/connection-required";
import { CurrentGameConnection } from "@/components/federation/current-game-connection";
import {
  LEAGUE_PHASE_COLOR_SCHEME,
  leaguePhaseLabel,
} from "@/components/federation/league-phase";
import { Scoreboard } from "@/components/federation/scoreboard";
import { SeasonPanel } from "@/components/federation/season-panel";
import { useLauncherConfig } from "@/contexts/config";
import { useApplicationRequest, useFederation } from "@/contexts/federation";
import { useGameConnection } from "@/contexts/game-connection";
import { useSharedModals } from "@/contexts/shared-modal";
import { UnionService } from "@/services/union";

export default function CompetitionPage() {
  const { t } = useTranslation();
  const router = useRouter();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const {
    gameLink,
    setGameLink,
    connection,
    busy: gameBusy,
    connect,
  } = useGameConnection();
  const [gameError, setGameError] = useState(false);
  const { state, identity, member } = useFederation();
  const { busy, error, handleRequest } = useApplicationRequest();
  const { openGenericConfirmDialog } = useSharedModals();
  const rawId = router.query.id;
  const competitionId = (Array.isArray(rawId) ? rawId[0] : rawId) ?? "";
  const competition =
    competitionId && state ? state.leagues[competitionId] : undefined;
  const handleGameConnection = async () => {
    setGameError(false);
    try {
      await connect(
        () => UnionService.connectGame(gameLink),
        t("FederationPage.gameConnection.title")
      );
    } catch {
      setGameError(true);
    }
  };
  const [teamNames, setTeamNames] = useState<Record<string, string>>({});
  const [invitees, setInvitees] = useState<Record<string, string>>({});
  let content = null;
  if (!state) content = <ConnectionRequired />;
  else if (!router.isReady)
    content = <Empty size="md" description={t("ConnectionFlow.connecting")} />;
  else if (!competitionId || !competition)
    content = (
      <Empty
        size="md"
        description={t("FederationPage.activities.competitionMissing")}
      />
    );
  return (
    <Section
      title={competitionId || t("FederationPage.activities.competitionTitle")}
      withBackButton
      backRoutePath="/federation/activities"
    >
      <VStack spacing={6} align="stretch">
        <Section
          title={t("FederationPage.gameConnection.title")}
          description={t("FederationPage.gameConnection.description")}
          isAccordion
          initialIsOpen={!!gameLink}
        >
          <VStack align="stretch" spacing={3}>
            <FormControl>
              <FormLabel htmlFor="game-link">
                {t("FederationPage.gameConnection.linkLabel")}
              </FormLabel>
              <Input
                id="game-link"
                value={gameLink}
                isDisabled={gameBusy || !!connection}
                onChange={(e) => setGameLink(e.target.value)}
              />
            </FormControl>
            <Button
              colorScheme={primaryColor}
              alignSelf="flex-start"
              isLoading={gameBusy}
              isDisabled={!!connection || !gameLink}
              onClick={() => void handleGameConnection()}
            >
              {t("FederationPage.gameConnection.connect")}
            </Button>
          </VStack>
        </Section>

        <CurrentGameConnection />
        {gameError && (
          <Text role="alert">{t("FederationPage.gameConnection.error")}</Text>
        )}
        {state && competition && (
          <SeasonPanel
            state={state}
            member={member}
            busy={busy}
            request={handleRequest}
          />
        )}

        {error && (
          <Text role="alert" color="red.600" fontSize="sm">
            {error}
          </Text>
        )}

        {content
          ? content
          : state &&
            competition && (
              <AdvancedCard level="front">
                <VStack spacing={4} align="stretch">
                  <Flex
                    wrap="wrap"
                    gap={2}
                    align="center"
                    justify="space-between"
                  >
                    <Text fontWeight="bold" fontSize="sm">
                      {competitionId}
                    </Text>
                    <HStack spacing={2}>
                      <Tag
                        colorScheme={
                          LEAGUE_PHASE_COLOR_SCHEME[competition.phase]
                        }
                      >
                        {leaguePhaseLabel(t, competition.phase)}
                      </Tag>
                      <Tag colorScheme="gray">
                        {t("FederationPage.league.seatsPerTeam", {
                          seats: competition.rules.seats,
                        })}
                      </Tag>
                      <Tag
                        colorScheme={
                          competition.rules.cross_club ? "green" : "gray"
                        }
                      >
                        {competition.rules.cross_club
                          ? t("FederationPage.league.crossClub")
                          : t("FederationPage.league.sameClubOnly")}
                      </Tag>
                    </HStack>
                  </Flex>

                  {competition.phase === "Registration" && member && (
                    <HStack spacing={2}>
                      <Input
                        aria-label={t(
                          "FederationPage.league.teamNamePlaceholder"
                        )}
                        placeholder={t(
                          "FederationPage.league.teamNamePlaceholder"
                        )}
                        value={teamNames[competitionId] || ""}
                        onChange={(e) =>
                          setTeamNames((current) => ({
                            ...current,
                            [competitionId]: e.target.value,
                          }))
                        }
                      />
                      <Button
                        colorScheme={primaryColor}
                        flexShrink={0}
                        isDisabled={busy || !teamNames[competitionId]?.trim()}
                        onClick={() =>
                          void handleRequest({
                            type: "create_team",
                            league: competitionId,
                            team: teamNames[competitionId].trim(),
                          })
                        }
                      >
                        {t("FederationPage.league.createTeam")}
                      </Button>
                    </HStack>
                  )}

                  <Section title={t("FederationPage.league.teamsTitle")}>
                    {Object.keys(competition.teams).length === 0 ? (
                      <Text fontSize="sm" className="secondary-text">
                        {t("FederationPage.league.teamsEmpty")}
                      </Text>
                    ) : (
                      Object.entries(competition.teams).map(
                        ([team, roster]) => {
                          const invitationKey = JSON.stringify([
                            competitionId,
                            team,
                          ]);
                          const invitee = invitees[invitationKey] || "";
                          return (
                            <Box
                              key={team}
                              py={3}
                              borderBottomWidth="1px"
                              borderColor="var(--chakra-colors-chakra-border-color)"
                              _last={{ borderBottomWidth: 0 }}
                            >
                              <Flex wrap="wrap" gap={2} align="center">
                                <Text fontWeight="medium">{team}</Text>
                                <Tag colorScheme="gray">
                                  {t("FederationPage.league.memberCount", {
                                    count: roster.members.length,
                                  })}
                                </Tag>
                                <Tag
                                  colorScheme={
                                    roster.registered ? "green" : "gray"
                                  }
                                >
                                  {roster.registered
                                    ? t("FederationPage.league.registered")
                                    : t("FederationPage.league.notRegistered")}
                                </Tag>
                                <HStack spacing={2} ml="auto">
                                  {competition.phase === "Registration" &&
                                    member &&
                                    roster.invited.includes(member) && (
                                      <Button
                                        colorScheme={primaryColor}
                                        isDisabled={busy}
                                        onClick={() =>
                                          void handleRequest({
                                            type: "accept",
                                            league: competitionId,
                                            team,
                                          })
                                        }
                                      >
                                        {t(
                                          "FederationPage.league.acceptInvite"
                                        )}
                                      </Button>
                                    )}
                                  {competition.phase === "Registration" &&
                                    roster.captain === member && (
                                      <>
                                        <Button
                                          colorScheme={primaryColor}
                                          isDisabled={busy || roster.registered}
                                          onClick={() =>
                                            void handleRequest({
                                              type: "register",
                                              league: competitionId,
                                              team,
                                            })
                                          }
                                        >
                                          {t("FederationPage.league.register")}
                                        </Button>
                                      </>
                                    )}
                                </HStack>
                              </Flex>
                              {competition.phase === "Registration" &&
                                roster.captain === member && (
                                  <HStack spacing={2} mt={2}>
                                    <Select
                                      aria-label={t(
                                        "FederationPage.league.inviteePlaceholder"
                                      )}
                                      placeholder={t(
                                        "FederationPage.league.inviteePlaceholder"
                                      )}
                                      value={invitee}
                                      onChange={(e) =>
                                        setInvitees((current) => ({
                                          ...current,
                                          [invitationKey]: e.target.value,
                                        }))
                                      }
                                    >
                                      {Object.entries(state.members)
                                        .filter(
                                          ([id]) =>
                                            id !== member &&
                                            !roster.members.includes(id) &&
                                            !roster.invited.includes(id)
                                        )
                                        .map(([id, candidate]) => (
                                          <option key={id} value={id}>
                                            {`${id} · ${candidate.club}`}
                                          </option>
                                        ))}
                                    </Select>
                                    <Button
                                      flexShrink={0}
                                      isDisabled={busy || !invitee}
                                      onClick={() =>
                                        void handleRequest({
                                          type: "invite",
                                          league: competitionId,
                                          team,
                                          member: invitee,
                                        })
                                      }
                                    >
                                      {t("FederationPage.league.invite")}
                                    </Button>
                                  </HStack>
                                )}
                            </Box>
                          );
                        }
                      )
                    )}
                  </Section>

                  <Flex
                    gap={8}
                    direction={{ base: "column", md: "row" }}
                    align="flex-start"
                  >
                    <Box flex="1" minW={0}>
                      <Scoreboard
                        title={t("FederationPage.league.teamScoreboard")}
                        entries={Object.entries(competition.team_scores).sort(
                          (a, b) => b[1] - a[1]
                        )}
                        emptyText={t(
                          "FederationPage.league.teamScoreboardEmpty"
                        )}
                        formatScore={(score) =>
                          t("FederationPage.league.points", { score })
                        }
                      />
                    </Box>
                    <Box flex="1" minW={0}>
                      <Scoreboard
                        title={t("FederationPage.league.playerScoreboard")}
                        entries={Object.entries(competition.player_scores).sort(
                          (a, b) => b[1] - a[1]
                        )}
                        emptyText={t(
                          "FederationPage.league.playerScoreboardEmpty"
                        )}
                        formatScore={(score) =>
                          t("FederationPage.league.points", { score })
                        }
                      />
                    </Box>
                  </Flex>
                </VStack>
              </AdvancedCard>
            )}

        {/* Organizer-only, irreversible actions live apart from participant
            actions and ask for confirmation first. */}
        {state && competition && competition.organizer === identity && (
          <AdvancedCard level="front" p={4}>
            <VStack align="stretch" spacing={3}>
              <Text fontWeight="semibold" fontSize="sm">
                {t("FederationPage.organizer.areaTitle")}
              </Text>
              <Text fontSize="xs" className="secondary-text">
                {t("FederationPage.organizer.areaHint")}
              </Text>
              <HStack flexWrap="wrap" spacing={2}>
                {competition.phase === "Registration" && (
                  <Button
                    size="sm"
                    colorScheme="green"
                    isDisabled={busy}
                    onClick={() => {
                      openGenericConfirmDialog({
                        title: t("FederationPage.organizer.startConfirmTitle"),
                        body: t("FederationPage.organizer.startConfirmBody"),
                        isAlert: true,
                        btnOK: t("FederationPage.league.start"),
                        onOKCallback: () => {
                          const seed = new Uint8Array(32);
                          crypto.getRandomValues(seed);
                          void handleRequest({
                            type: "start",
                            league: competitionId,
                            seed: Array.from(seed),
                          });
                        },
                      });
                    }}
                  >
                    {t("FederationPage.league.start")}
                  </Button>
                )}
                {competition.phase === "Playing" && (
                  <Button
                    size="sm"
                    isDisabled={busy}
                    onClick={() =>
                      void handleRequest({
                        type: "pause_league",
                        league: competitionId,
                        paused: !competition.manual_pause,
                      })
                    }
                  >
                    {competition.manual_pause
                      ? t("FederationPage.league.resume")
                      : t("FederationPage.league.pause")}
                  </Button>
                )}
              </HStack>
            </VStack>
          </AdvancedCard>
        )}
      </VStack>
    </Section>
  );
}
