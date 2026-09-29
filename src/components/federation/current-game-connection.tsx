import { Button, Tag, Text, VStack } from "@chakra-ui/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Section } from "@/components/common/section";
import { useLauncherConfig } from "@/contexts/config";
import { useGameConnection } from "@/contexts/game-connection";

export function CurrentGameConnection() {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const { connection, label, busy, disconnect } = useGameConnection();
  const [error, setError] = useState(false);
  if (!connection) return null;
  const handleDisconnect = async () => {
    setError(false);
    try {
      await disconnect();
    } catch {
      setError(true);
    }
  };
  return (
    <Section title={t("ConnectionFlow.currentGame")}>
      <VStack align="stretch">
        {label && <Text>{label}</Text>}
        {connection.origin && (
          <Tag colorScheme={primaryColor} alignSelf="flex-start">
            {t(`ConnectionFlow.origin.${connection.origin}`)}
          </Tag>
        )}
        <Text fontSize="xs" overflowWrap="anywhere">
          {t("ConnectionFlow.service", {
            peer: connection.peer,
            service: connection.service,
          })}
        </Text>
        <Text>{t("Rooms.connected", { address: connection.address })}</Text>
        <Text fontSize="sm">{t("ConnectionFlow.disconnectFirst")}</Text>
        <Button
          alignSelf="flex-start"
          isLoading={busy}
          onClick={() => void handleDisconnect()}
        >
          {t("Rooms.disconnect")}
        </Button>
        {error && (
          <Text role="alert">{t("FederationPage.gameConnection.error")}</Text>
        )}
      </VStack>
    </Section>
  );
}
