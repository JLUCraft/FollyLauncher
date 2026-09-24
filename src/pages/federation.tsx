import {
  Box,
  Button,
  Flex,
  FormControl,
  FormLabel,
  HStack,
  Heading,
  Input,
  Tag,
  Text,
  VStack,
} from "@chakra-ui/react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { brand } from "@/brand";
import AdvancedCard from "@/components/common/advanced-card";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import { AdminConsole } from "@/components/federation/admin-console";
import { MemberGovernance } from "@/components/federation/member-governance";
import { SeasonPanel } from "@/components/federation/season-panel";
import { useLauncherConfig } from "@/contexts/config";
import { useDeepLink } from "@/hooks/deep-link";
import { UnionService, UnionState } from "@/services/union";

const Scoreboard = ({
  title,
  scores,
  emptyText,
}: {
  title: string;
  scores: Record<string, number>;
  emptyText: string;
}) => {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  return (
    <Box>
      <Text fontWeight="bold" fontSize="sm" mb={2}>
        {title}
      </Text>
      {Object.keys(scores).length === 0 ? (
        <Text fontSize="sm" className="secondary-text">
          {emptyText}
        </Text>
      ) : (
        <VStack spacing={1} align="stretch">
          {Object.entries(scores)
            .sort((a, b) => b[1] - a[1])
            .map(([id, score], index) => (
              <Flex key={id} fontSize="sm" align="center">
                <Text
                  w={8}
                  flexShrink={0}
                  fontWeight={index < 3 ? "bold" : "normal"}
                  color={index < 3 ? `${primaryColor}.500` : "gray.500"}
                >
                  {index + 1}
                </Text>
                <Text flex="1" overflowWrap="anywhere">
                  {id}
                </Text>
                <Text fontWeight="medium" flexShrink={0}>
                  {t("FederationPage.league.points", { score })}
                </Text>
              </Flex>
            ))}
        </VStack>
      )}
    </Box>
  );
};

export default function FederationPage() {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const [gameLink, setGameLink] = useState("");
  const [gameAddress, setGameAddress] = useState<string | null>(null);
  const [gameBusy, setGameBusy] = useState(false);
  useDeepLink({
    trigger: (subpath) => subpath.startsWith("connect"),
    onCall: (url) => setGameLink(url),
  });
  useEffect(() => {
    let active = true;
    const refresh = () => {
      void UnionService.gameConnection()
        .then((value) => {
          if (active) setGameAddress(value);
        })
        .catch(() => {});
    };
    refresh();
    const timer = setInterval(refresh, 3000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, []);
  const handleGameConnection = async () => {
    setGameBusy(true);
    setError("");
    try {
      if (gameAddress) {
        await UnionService.disconnectGame();
        setGameAddress(null);
      } else {
        setGameAddress(await UnionService.connectGame(gameLink));
      }
    } catch {
      setError(t("FederationPage.gameConnection.error"));
    } finally {
      setGameBusy(false);
    }
  };
  const [address, setAddress] = useState(brand.federation.defaultAddress);
  const [peer, setPeer] = useState(brand.federation.defaultPeer);
  const [identity, setIdentity] = useState("");
  const [state, setState] = useState<UnionState>();
  const [teamName, setTeamName] = useState("");
  const [invitee, setInvitee] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const handleRequest = async (action?: Record<string, unknown>) => {
    setBusy(true);
    setError("");
    try {
      setIdentity(await UnionService.identity());
      setState(
        await UnionService.request(address, peer, state?.revision, action)
      );
    } catch {
      setError(t("FederationPage.error"));
    } finally {
      setBusy(false);
    }
  };
  const member = Object.entries(state?.members || {}).find(
    ([, m]) => m.device === identity || m.additional_devices?.includes(identity)
  )?.[0];
  const leagues = Object.entries(state?.leagues || {});
  return (
    <Section
      className="content-full-y"
      title={t("FederationPage.title")}
      description={t("FederationPage.subtitle")}
    >
      <VStack spacing={6} align="stretch">
        {state && (
          <MemberGovernance
            state={state}
            member={member}
            identity={identity}
            busy={busy}
            request={handleRequest}
          />
        )}

        {state && (
          <SeasonPanel
            state={state}
            member={member}
            busy={busy}
            request={handleRequest}
          />
        )}

        {state && identity && state.admins?.includes(identity) && (
          <AdminConsole state={state} busy={busy} request={handleRequest} />
        )}

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
                isDisabled={gameBusy || !!gameAddress}
                onChange={(e) => setGameLink(e.target.value)}
              />
            </FormControl>
            {gameAddress && (
              <Text fontSize="sm">
                {t("FederationPage.gameConnection.connected", {
                  address: gameAddress,
                })}
              </Text>
            )}
            <Button
              colorScheme={primaryColor}
              isLoading={gameBusy}
              isDisabled={!gameAddress && !gameLink}
              onClick={() => void handleGameConnection()}
            >
              {gameAddress
                ? t("FederationPage.gameConnection.disconnect")
                : t("FederationPage.gameConnection.connect")}
            </Button>
          </VStack>
        </Section>

        <Section
          title={t("FederationPage.settings.title")}
          description={t("FederationPage.settings.description")}
          isAccordion
          initialIsOpen={!state}
        >
          <VStack spacing={3} align="stretch">
            <FormControl>
              <FormLabel htmlFor="federation-address">
                {t("FederationPage.settings.addressLabel")}
              </FormLabel>
              <Input
                id="federation-address"
                isDisabled={busy}
                value={address}
                onChange={(e) => {
                  setAddress(e.target.value);
                  setState(undefined);
                }}
              />
            </FormControl>
            <FormControl>
              <FormLabel htmlFor="federation-peer">
                {t("FederationPage.settings.peerLabel")}
              </FormLabel>
              <Input
                id="federation-peer"
                isDisabled={busy}
                value={peer}
                onChange={(e) => {
                  setPeer(e.target.value);
                  setState(undefined);
                }}
              />
            </FormControl>
            <HStack justify="flex-end">
              <Button
                colorScheme={primaryColor}
                isLoading={busy}
                isDisabled={!address || !peer}
                onClick={() => void handleRequest()}
              >
                {t("FederationPage.settings.connect")}
              </Button>
            </HStack>
            {identity && (
              <VStack spacing={1} align="start" fontSize="sm">
                <Text overflowWrap="anywhere">
                  {t("FederationPage.settings.myDevice", { id: identity })}
                </Text>
                {member && (
                  <Text>
                    {t("FederationPage.settings.myMember", { id: member })}
                    {state?.members[member]?.club && (
                      <Text as="span" className="secondary-text">
                        {t("FederationPage.settings.myClub", {
                          club: state.members[member].club,
                        })}
                      </Text>
                    )}
                  </Text>
                )}
              </VStack>
            )}
          </VStack>
        </Section>

        {error && (
          <Text role="alert" color="red.500" fontSize="sm">
            {error}
          </Text>
        )}

        {!state ? (
          <Empty
            size="md"
            description={t("FederationPage.empty.notConnected")}
          />
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
                  <Heading size="md">{league}</Heading>
                  <HStack spacing={2}>
                    <Tag
                      colorScheme={
                        value.phase === "Registration"
                          ? "blue"
                          : value.phase === "Playing"
                            ? "green"
                            : "gray"
                      }
                    >
                      {t(`FederationPage.phase.${value.phase}`, {
                        defaultValue: value.phase,
                      })}
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
                      value={teamName}
                      onChange={(e) => setTeamName(e.target.value)}
                    />
                    <Button
                      colorScheme={primaryColor}
                      flexShrink={0}
                      isDisabled={busy || !teamName}
                      onClick={() =>
                        void handleRequest({
                          type: "create_team",
                          league,
                          team: teamName,
                        })
                      }
                    >
                      {t("FederationPage.league.createTeam")}
                    </Button>
                  </HStack>
                )}

                <Box>
                  <Text fontWeight="bold" fontSize="sm" mb={2}>
                    {t("FederationPage.league.teamsTitle")}
                  </Text>
                  {Object.keys(value.teams).length === 0 ? (
                    <Text fontSize="sm" className="secondary-text">
                      {t("FederationPage.league.teamsEmpty")}
                    </Text>
                  ) : (
                    Object.entries(value.teams).map(([team, roster]) => (
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
                                    isDisabled={busy}
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
                                value={invitee}
                                onChange={(e) => setInvitee(e.target.value)}
                              />
                              <Button
                                flexShrink={0}
                                isDisabled={busy || !invitee}
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
                    ))
                  )}
                </Box>

                <Flex
                  gap={8}
                  direction={{ base: "column", md: "row" }}
                  align="flex-start"
                >
                  <Box flex="1" minW={0}>
                    <Scoreboard
                      title={t("FederationPage.league.teamScoreboard")}
                      scores={value.team_scores}
                      emptyText={t("FederationPage.league.teamScoreboardEmpty")}
                    />
                  </Box>
                  <Box flex="1" minW={0}>
                    <Scoreboard
                      title={t("FederationPage.league.playerScoreboard")}
                      scores={value.player_scores}
                      emptyText={t(
                        "FederationPage.league.playerScoreboardEmpty"
                      )}
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
