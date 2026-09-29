import { Button, Text, VStack } from "@chakra-ui/react";
import Link from "next/link";
import { useTranslation } from "react-i18next";
import Empty from "@/components/common/empty";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";

export function ConnectionRequired() {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const { connecting, connectionError, identityRequired, identityError } =
    useFederation();
  return (
    <VStack align="stretch">
      {identityRequired ? (
        <Text role="alert">{t("FederationPage.identityRequired")}</Text>
      ) : (
        connectionError && <Text role="alert">{t("FederationPage.error")}</Text>
      )}
      <Empty
        size="lg"
        description={t(
          connecting
            ? "ConnectionFlow.connecting"
            : "FederationNav.disconnected"
        )}
      >
        {identityRequired ? (
          <Button
            as={Link}
            href="/federation/identity"
            colorScheme={primaryColor}
          >
            {t("FederationPage.openIdentity")}
          </Button>
        ) : (
          <Button
            as={Link}
            href="/federation/overview"
            colorScheme={primaryColor}
          >
            {t("ConnectionFlow.setup")}
          </Button>
        )}
      </Empty>
      {identityError && (
        <Text role="alert">{t("FederationPage.identity.unavailable")}</Text>
      )}
    </VStack>
  );
}
