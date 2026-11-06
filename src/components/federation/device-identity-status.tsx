import { Button, HStack, Spinner, Text, VStack } from "@chakra-ui/react";
import { useTranslation } from "react-i18next";
import { useFederation } from "@/contexts/federation";

export function DeviceIdentityStatus() {
  const { t } = useTranslation();
  const { identity, identityError, retryIdentity } = useFederation();
  if (identity) return null;
  return (
    <VStack align="stretch" spacing={3}>
      <HStack role={identityError ? "alert" : "status"}>
        {!identityError && <Spinner size="sm" />}
        <Text>
          {t(
            identityError
              ? "FederationPage.identity.storageError"
              : "FederationPage.identity.storagePending"
          )}
        </Text>
      </HStack>
      {identityError && (
        <Button onClick={retryIdentity}>{t("General.retry")}</Button>
      )}
    </VStack>
  );
}
