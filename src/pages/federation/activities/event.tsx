import { Button, HStack, Tag, Text, VStack } from "@chakra-ui/react";
import { useRouter } from "next/router";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import {
  ACTIVITY_PHASE_COLOR_SCHEME,
  activityPhaseLabel,
} from "@/components/federation/activity-phase";
import { ConnectionRequired } from "@/components/federation/connection-required";
import { useLauncherConfig } from "@/contexts/config";
import { useApplicationRequest, useFederation } from "@/contexts/federation";
import { ISOToDatetime } from "@/utils/datetime";

function formatWindow(ms: number) {
  return ISOToDatetime(new Date(ms).toISOString());
}

export default function EventPage() {
  const { t } = useTranslation();
  const router = useRouter();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const { state, member } = useFederation();
  const { busy, error, handleRequest } = useApplicationRequest();
  const rawId = router.query.id;
  const eventId = (Array.isArray(rawId) ? rawId[0] : rawId) ?? "";
  const event = eventId && state ? state.events[eventId] : undefined;
  const brief = eventId && state ? state.activity_briefs?.[eventId] : undefined;
  const signedUp =
    !!member && !!eventId && !!state?.signups[eventId]?.includes(member);
  const organizer =
    event && state
      ? (state.members[event.organizer]?.club ?? event.organizer)
      : "";
  let content = null;
  if (!state) content = <ConnectionRequired />;
  else if (!router.isReady)
    content = <Empty size="md" description={t("ConnectionFlow.connecting")} />;
  else if (!eventId || !event)
    content = (
      <Empty
        size="md"
        description={t("FederationPage.activities.eventMissing")}
      />
    );
  return (
    <Section
      title={event?.title ?? t("FederationNav.activities")}
      withBackButton
      backRoutePath="/federation/activities"
    >
      {content
        ? content
        : event && (
            <AdvancedCard level="front">
              <VStack align="stretch" spacing={4}>
                <Text whiteSpace="pre-wrap">
                  {brief?.description || t("FederationPage.activities.noBrief")}
                </Text>
                <HStack spacing={2} wrap="wrap">
                  <Tag colorScheme={ACTIVITY_PHASE_COLOR_SCHEME[event.phase]}>
                    {activityPhaseLabel(t, event.phase)}
                  </Tag>
                  <Tag colorScheme="gray">
                    {t("FederationPage.activities.signupCount", {
                      count: state?.signups[eventId]?.length ?? 0,
                    })}
                  </Tag>
                </HStack>
                <VStack align="stretch" spacing={2}>
                  <HStack
                    justify="space-between"
                    align="flex-start"
                    spacing={4}
                  >
                    <Text fontSize="sm" className="secondary-text">
                      {t("FederationPage.activities.organizer")}
                    </Text>
                    <Text fontSize="sm" textAlign="right">
                      {organizer}
                    </Text>
                  </HStack>
                  <HStack
                    justify="space-between"
                    align="flex-start"
                    spacing={4}
                  >
                    <Text fontSize="sm" className="secondary-text">
                      {t("FederationPage.activities.opens")}
                    </Text>
                    <Text fontSize="sm" textAlign="right">
                      {formatWindow(event.window.opens_ms)}
                    </Text>
                  </HStack>
                  <HStack
                    justify="space-between"
                    align="flex-start"
                    spacing={4}
                  >
                    <Text fontSize="sm" className="secondary-text">
                      {t("FederationPage.activities.closes")}
                    </Text>
                    <Text fontSize="sm" textAlign="right">
                      {formatWindow(event.window.closes_ms)}
                    </Text>
                  </HStack>
                  <HStack
                    justify="space-between"
                    align="flex-start"
                    spacing={4}
                  >
                    <Text fontSize="sm" className="secondary-text">
                      {t("FederationPage.activities.starts")}
                    </Text>
                    <Text fontSize="sm" textAlign="right">
                      {formatWindow(event.window.starts_ms)}
                    </Text>
                  </HStack>
                </VStack>
                {error && (
                  <Text role="alert" color="red.600" fontSize="sm">
                    {error}
                  </Text>
                )}
                {member && event.phase === "open" && (
                  <Button
                    alignSelf="flex-start"
                    colorScheme={signedUp ? "gray" : primaryColor}
                    isLoading={busy}
                    onClick={() =>
                      void handleRequest({
                        type: signedUp ? "events.cancel" : "events.signup",
                        id: eventId,
                      })
                    }
                  >
                    {t(
                      signedUp
                        ? "FederationPage.activities.cancelSignup"
                        : "FederationPage.activities.signup"
                    )}
                  </Button>
                )}
              </VStack>
            </AdvancedCard>
          )}
    </Section>
  );
}
