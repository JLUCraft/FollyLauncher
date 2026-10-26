import { Flex, HStack, Tag, Text, VStack } from "@chakra-ui/react";
import Link from "next/link";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import {
  ACTIVITY_PHASE_COLOR_SCHEME,
  activityPhaseLabel,
} from "@/components/federation/activity-phase";
import { ConnectionRequired } from "@/components/federation/connection-required";
import {
  LEAGUE_PHASE_COLOR_SCHEME,
  leaguePhaseLabel,
} from "@/components/federation/league-phase";
import { PointsPanel } from "@/components/federation/points-panel";
import { useFederation } from "@/contexts/federation";
import { UnionState } from "@/services/union";

function competitionsByEvent(state: UnionState) {
  const owners: Record<string, string> = {};
  Object.entries(state.organization?.plans ?? {}).forEach(([eventId, plan]) => {
    if (!state.events[eventId]) return;
    Object.values(plan.games ?? {}).forEach((game) => {
      if (game.league && !owners[game.league]) owners[game.league] = eventId;
    });
  });
  const linked: Record<string, string[]> = {};
  const standalone: string[] = [];
  Object.keys(state.leagues).forEach((league) => {
    const eventId = owners[league];
    if (eventId) (linked[eventId] ??= []).push(league);
    else standalone.push(league);
  });
  return { linked, standalone };
}

export default function ActivitiesPage() {
  const { t } = useTranslation();
  const { state } = useFederation();
  const events = Object.entries(state?.events ?? {});
  const {
    linked,
    standalone,
  }: { linked: Record<string, string[]>; standalone: string[] } = state
    ? competitionsByEvent(state)
    : { linked: {}, standalone: [] };
  return (
    <Section
      title={t("FederationNav.activities")}
      description={t("FederationPage.activities.description")}
    >
      <VStack spacing={4} align="stretch">
        <PointsPanel />
        {!state ? (
          <ConnectionRequired />
        ) : events.length === 0 && standalone.length === 0 ? (
          <Empty
            size="md"
            description={t("FederationPage.empty.noActivities")}
          />
        ) : (
          <>
            {events.map(([eventId, event]) => {
              const competitions = linked[eventId] ?? [];
              return (
                <AdvancedCard key={eventId} level="front">
                  <VStack spacing={3} align="stretch">
                    <Flex
                      wrap="wrap"
                      gap={2}
                      align="center"
                      justify="space-between"
                    >
                      <Link
                        href={{
                          pathname: "/federation/activities/event",
                          query: { id: eventId },
                        }}
                      >
                        <Text as="span" fontWeight="bold" fontSize="sm">
                          {event.title}
                        </Text>
                      </Link>
                      <HStack spacing={2}>
                        <Tag
                          colorScheme={ACTIVITY_PHASE_COLOR_SCHEME[event.phase]}
                        >
                          {activityPhaseLabel(t, event.phase)}
                        </Tag>
                        <Tag colorScheme="gray">
                          {t("FederationPage.activities.signupCount", {
                            count: state.signups[eventId]?.length ?? 0,
                          })}
                        </Tag>
                      </HStack>
                    </Flex>
                    <Section
                      title={t("FederationPage.activities.linkedCompetitions")}
                    >
                      {competitions.length === 0 ? (
                        <Text fontSize="sm" className="secondary-text">
                          {t("FederationPage.activities.competitionsEmpty")}
                        </Text>
                      ) : (
                        <VStack align="stretch" spacing={1}>
                          {competitions.map((competitionId) => {
                            const competition = state.leagues[competitionId];
                            return (
                              <Link
                                key={competitionId}
                                href={{
                                  pathname:
                                    "/federation/activities/competition",
                                  query: { id: competitionId },
                                }}
                              >
                                <HStack
                                  justify="space-between"
                                  align="center"
                                  spacing={2}
                                >
                                  <Text fontSize="sm">{competitionId}</Text>
                                  <Tag
                                    colorScheme={
                                      LEAGUE_PHASE_COLOR_SCHEME[
                                        competition.phase
                                      ]
                                    }
                                  >
                                    {leaguePhaseLabel(t, competition.phase)}
                                  </Tag>
                                </HStack>
                              </Link>
                            );
                          })}
                        </VStack>
                      )}
                    </Section>
                  </VStack>
                </AdvancedCard>
              );
            })}
            {standalone.length > 0 && (
              <Section title={t("FederationPage.activities.standaloneTitle")}>
                <VStack align="stretch" spacing={2}>
                  {standalone.map((competitionId) => {
                    const competition = state.leagues[competitionId];
                    return (
                      <AdvancedCard key={competitionId} level="front">
                        <Flex
                          wrap="wrap"
                          gap={2}
                          align="center"
                          justify="space-between"
                        >
                          <Link
                            href={{
                              pathname: "/federation/activities/competition",
                              query: { id: competitionId },
                            }}
                          >
                            <Text as="span" fontWeight="bold" fontSize="sm">
                              {competitionId}
                            </Text>
                          </Link>
                          <Tag
                            colorScheme={
                              LEAGUE_PHASE_COLOR_SCHEME[competition.phase]
                            }
                          >
                            {leaguePhaseLabel(t, competition.phase)}
                          </Tag>
                        </Flex>
                      </AdvancedCard>
                    );
                  })}
                </VStack>
              </Section>
            )}
          </>
        )}
      </VStack>
    </Section>
  );
}
