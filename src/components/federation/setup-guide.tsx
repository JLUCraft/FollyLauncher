import { Badge, Button, HStack, Icon, Text, VStack } from "@chakra-ui/react";
import { useTranslation } from "react-i18next";
import { LuCheck } from "react-icons/lu";
import { Section } from "@/components/common/section";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";

// First-run path. Every mark comes from real state, and the guide disappears once
// all steps are done. Connection is automatic, so the last step reports progress
// instead of asking for input.
export function SetupGuide({
  onAction,
}: {
  onAction: (step: "setup" | "verify" | "connect") => void;
}) {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const { state, identityConfigured, member } = useFederation();
  const steps = [
    { key: "setup", done: identityConfigured === true },
    { key: "verify", done: !!member },
    { key: "connect", done: !!state },
  ];
  if (identityConfigured === null || steps.every((step) => step.done))
    return null;
  const current = steps.findIndex((step) => !step.done);
  return (
    <Section title={t("FederationPage.guide.title")}>
      <VStack align="stretch" spacing={3}>
        {steps.map((step, index) => (
          <HStack key={step.key} spacing={3} align="center">
            {/* Numbered steps, so the list no longer looks like radio buttons. */}
            <Badge
              flexShrink={0}
              boxSize={6}
              borderRadius="full"
              display="flex"
              alignItems="center"
              justifyContent="center"
              colorScheme={step.done ? "green" : undefined}
            >
              {step.done ? <Icon as={LuCheck} boxSize={3} /> : index + 1}
            </Badge>
            <Text
              flex={1}
              fontWeight={index === current ? "semibold" : "normal"}
            >
              {t(`FederationPage.guide.${step.key}`)}
            </Text>
            {/* The setup and verification editors are rendered inline below, so
                only the connection step needs its own action here. */}
            {index === current && step.key === "connect" && (
              <Button
                size="sm"
                colorScheme={config.appearance.theme.primaryColor}
                onClick={() => onAction("connect")}
              >
                {t("ConnectionFlow.searchRetry")}
              </Button>
            )}
          </HStack>
        ))}
      </VStack>
    </Section>
  );
}
