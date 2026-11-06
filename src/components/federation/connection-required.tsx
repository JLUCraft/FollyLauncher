import { Button, Text, VStack } from "@chakra-ui/react";
import Link from "next/link";
import { useTranslation } from "react-i18next";
import Empty from "@/components/common/empty";
import { DeviceIdentityStatus } from "@/components/federation/device-identity-status";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";
import { federationErrorKey } from "@/utils/federation/errors";
import { FEDERATION_SETTINGS_PATH } from "@/utils/federation/navigation";

export function ConnectionRequired() {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const {
    identity,
    connecting,
    connectionError,
    identityRequired,
    identityConfigured,
  } = useFederation();
  if (!identity) return <DeviceIdentityStatus />;
  const needsSetup = identityRequired || identityConfigured === false;
  return (
    <VStack align="stretch">
      {!needsSetup && connectionError && (
        <Text role="alert">{t(federationErrorKey(connectionError))}</Text>
      )}
      <Empty
        size="lg"
        description={t(
          needsSetup
            ? "FederationPage.identityRequired"
            : connecting
              ? "ConnectionFlow.connecting"
              : "FederationNav.disconnected"
        )}
      >
        {needsSetup ? (
          <Button
            as={Link}
            href={FEDERATION_SETTINGS_PATH}
            colorScheme={primaryColor}
          >
            {t("FederationPage.openIdentity")}
          </Button>
        ) : (
          <Button
            as={Link}
            href={FEDERATION_SETTINGS_PATH}
            colorScheme={primaryColor}
          >
            {t("ConnectionFlow.setup")}
          </Button>
        )}
      </Empty>
    </VStack>
  );
}
