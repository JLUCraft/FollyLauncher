import {
  Box,
  Button,
  Flex,
  FormControl,
  FormLabel,
  HStack,
  Input,
  Tag,
  Text,
  VStack,
} from "@chakra-ui/react";
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
import { useFederation, useLeagueRequest } from "@/contexts/federation";
import { useGameConnection } from "@/contexts/game-connection";
import { UnionService } from "@/services/union";

export default function LeaguePage() {
  const { t } = useTranslation();
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
  const { busy, error, handleRequest } = useLeagueRequest();
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
  const leagues = Object.entries(state?.leagues || {});
  return (
    <Section title={t("FederationNav.league")}>
      <VStack spacing={6} align="stretch">
        <Section
          title={t("FederationPage.gameConnection.title")}
          description={t("FederationPage.gameConnection.description")}
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
        {state && (
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

        {!state ? (
          <ConnectionRequired />
        ) : leagues.length === 0 ? (
          <Empty size="md" description={t("FederationPage.empty.noLeagues")} />
        ) : (
          leagues.map(([league, value]) => (
            <AdvancedCard key={league} level="front">
              <VStack spacing={4} align="stretch">
                <Flex
                  wrap="wrap"
                  gap={2}
                  align="center"
                  justify="space-between"
                >
                  <Text fontWeight="bold" fontSize="sm">
                    {league}
                  </Text>
                  <HStack spacing={2}>
                    <Tag colorScheme={LEAGUE_PHASE_COLOR_SCHEME[value.phase]}>
                      {leaguePhaseLabel(t, value.phase)}
                    </Tag>
                    <Tag colorScheme="gray">
                      {t("FederationPage.league.seatsPerTeam", {
                        seats: value.rules.seats,
                      })}
                    </Tag>
                    <Tag
                      colorScheme={value.rules.cross_club ? "green" : "gray"}
                    >
                      {value.rules.cross_club
                        ? t("FederationPage.league.crossClub")
                        : t("FederationPage.league.sameClubOnly")}
                    </Tag>
                    {value.organizer === identity &&
                      value.phase === "Registration" && (
                        <Button
                          size="sm"
                          colorScheme="green"
                          isDisabled={busy}
                          onClick={() => {
                            const seed = new Uint8Array(32);
                            crypto.getRandomValues(seed);
                            void handleRequest({
                              type: "start",
                              league,
                              seed: Array.from(seed),
                            });
                          }}
                        >
                          {t("FederationPage.league.start")}
                        </Button>
                      )}
                    {value.organizer === identity &&
                      value.phase === "Playing" && (
                        <Button
                          size="sm"
                          isDisabled={busy}
                          onClick={() =>
                            void handleRequest({
                              type: "pause_league",
                              league,
                              paused: !value.manual_pause,
                            })
                          }
                        >
                          {value.manual_pause
                            ? t("FederationPage.league.resume")
                            : t("FederationPage.league.pause")}
                        </Button>
                      )}
                  </HStack>
                </Flex>

                {value.phase === "Registration" && member && (
                  <HStack spacing={2}>
                    <Input
                      aria-label={t(
                        "FederationPage.league.teamNamePlaceholder"
                      )}
                      placeholder={t(
                        "FederationPage.league.teamNamePlaceholder"
                      )}
                      value={teamNames[league] || ""}
                      onChange={(e) =>
                        setTeamNames((current) => ({
                          ...current,
                          [league]: e.target.value,
                        }))
                      }
                    />
                    <Button
                      colorScheme={primaryColor}
                      flexShrink={0}
                      isDisabled={busy || !teamNames[league]?.trim()}
                      onClick={() =>
                        void handleRequest({
                          type: "create_team",
                          league,
                          team: teamNames[league].trim(),
                        })
                      }
                    >
                      {t("FederationPage.league.createTeam")}
                    </Button>
                  </HStack>
                )}

                <Section title={t("FederationPage.league.teamsTitle")}>
                  {Object.keys(value.teams).length === 0 ? (
                    <Text fontSize="sm" className="secondary-text">
                      {t("FederationPage.league.teamsEmpty")}
                    </Text>
                  ) : (
                    Object.entries(value.teams).map(([team, roster]) => {
                      const invitationKey = JSON.stringify([league, team]);
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
                              colorScheme={roster.registered ? "green" : "gray"}
                            >
                              {roster.registered
                                ? t("FederationPage.league.registered")
                                : t("FederationPage.league.notRegistered")}
                            </Tag>
                            <HStack spacing={2} ml="auto">
                              {value.phase === "Registration" &&
                                member &&
                                roster.invited.includes(member) && (
                                  <Button
                                    colorScheme={primaryColor}
                                    isDisabled={busy}
                                    onClick={() =>
                                      void handleRequest({
                                        type: "accept",
                                        league,
                                        team,
                                      })
                                    }
                                  >
                                    {t("FederationPage.league.acceptInvite")}
                                  </Button>
                                )}
                              {value.phase === "Registration" &&
                                roster.captain === member && (
                                  <>
                                    <Button
                                      colorScheme={primaryColor}
                                      isDisabled={busy || roster.registered}
                                      onClick={() =>
                                        void handleRequest({
                                          type: "register",
                                          league,
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
                          {value.phase === "Registration" &&
                            roster.captain === member && (
                              <HStack spacing={2} mt={2}>
                                <Input
                                  aria-label={t(
                                    "FederationPage.league.inviteePlaceholder"
                                  )}
                                  placeholder={t(
                                    "FederationPage.league.inviteePlaceholder"
                                  )}
                                  list={`invite-${league}-${team}`}
                                  value={invitee}
                                  onChange={(e) =>
                                    setInvitees((current) => ({
                                      ...current,
                                      [invitationKey]: e.target.value,
                                    }))
                                  }
                                />
                                <datalist id={`invite-${league}-${team}`}>
                                  {Object.keys(state.members).map((id) => (
                                    <option key={id} value={id} />
                                  ))}
                                </datalist>
                                <Button
                                  flexShrink={0}
                                  isDisabled={busy || !state.members[invitee]}
                                  onClick={() =>
                                    void handleRequest({
                                      type: "invite",
                                      league,
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
                    })
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
                      entries={Object.entries(value.team_scores).sort(
                        (a, b) => b[1] - a[1]
                      )}
                      emptyText={t("FederationPage.league.teamScoreboardEmpty")}
                      formatScore={(score) =>
                        t("FederationPage.league.points", { score })
                      }
                    />
                  </Box>
                  <Box flex="1" minW={0}>
                    <Scoreboard
                      title={t("FederationPage.league.playerScoreboard")}
                      entries={Object.entries(value.player_scores).sort(
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
          ))
        )}
      </VStack>
    </Section>
  );
}
