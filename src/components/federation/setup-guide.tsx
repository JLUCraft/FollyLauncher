import { Button, HStack, Icon, Text, VStack } from "@chakra-ui/react";
import Link from "next/link";
import { useRouter } from "next/router";
import { useTranslation } from "react-i18next";
import { LuCircle, LuCircleCheck } from "react-icons/lu";
import { Section } from "@/components/common/section";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";

// First-run path. Every mark comes from real state, and the guide disappears once all steps are done.
export function SetupGuide() {
  const { t } = useTranslation();
  const router = useRouter();
  const { config } = useLauncherConfig();
  const { state, identityConfigured, member } = useFederation();
  const steps = [
    {
      key: "setup",
      done: identityConfigured === true,
      href: "/federation/identity",
    },
    { key: "connect", done: !!state, href: "/federation/overview" },
    { key: "verify", done: !!member, href: "/federation/identity" },
  ];
  if (identityConfigured === null || steps.every((step) => step.done))
    return null;
  const current = steps.findIndex((step) => !step.done);
  return (
    <Section title={t("FederationPage.guide.title")}>
      <VStack align="stretch" spacing={3}>
        {steps.map((step, index) => (
          <HStack key={step.key} spacing={3} align="center">
            <Icon
              as={step.done ? LuCircleCheck : LuCircle}
              boxSize={5}
              color={step.done ? "green.500" : undefined}
              aria-label={
                step.done ? t("FederationPage.guide.done") : undefined
              }
            />
            <Text
              flex={1}
              fontWeight={index === current ? "semibold" : "normal"}
            >
              {t(`FederationPage.guide.${step.key}`)}
            </Text>
            {index === current && router.pathname !== step.href && (
              <Button
                as={Link}
                href={step.href}
                size="sm"
                colorScheme={config.appearance.theme.primaryColor}
              >
                {t("FederationPage.guide.next")}
              </Button>
            )}
          </HStack>
        ))}
      </VStack>
    </Section>
  );
}
