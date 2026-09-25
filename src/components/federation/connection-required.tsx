import { Button, Text, VStack } from "@chakra-ui/react";
import Link from "next/link";
import { useTranslation } from "react-i18next";
import Empty from "@/components/common/empty";
import { useFederation } from "@/contexts/federation";

export function ConnectionRequired() {
  const { t } = useTranslation();
  const { connecting, connectionError, identityRequired, identityError } =
    useFederation();
  return (
    <VStack align="stretch">
      <Empty
        description={t(
          connecting
            ? "ConnectionFlow.connecting"
            : "FederationNav.disconnected"
        )}
      />
      {identityRequired ? (
        <>
          <Text role="alert">{t("FederationPage.identityRequired")}</Text>
          <Button as={Link} href="/federation/identity">
            {t("FederationPage.openIdentity")}
          </Button>
        </>
      ) : (
        <>
          {connectionError && (
            <Text role="alert">{t("FederationPage.error")}</Text>
          )}
          <Button as={Link} href="/federation/overview">
            {t("ConnectionFlow.setup")}
          </Button>
        </>
      )}
      {identityError && (
        <Text role="alert">{t("FederationPage.identity.unavailable")}</Text>
      )}
    </VStack>
  );
}
