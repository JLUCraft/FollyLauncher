import {
  Button,
  FormControl,
  FormErrorMessage,
  FormLabel,
  HStack,
  Input,
  SimpleGrid,
  Text,
  VStack,
} from "@chakra-ui/react";
import Link from "next/link";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import { Section } from "@/components/common/section";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";

const GENESIS_PATTERN = /^[a-f0-9]{64}$/;

export default function OverviewPage() {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const {
    address,
    setAddress,
    peer,
    setPeer,
    genesisHash,
    setGenesisHash,
    state,
    connecting: busy,
    connectionError: error,
    identityRequired,
    connect: handleRequest,
  } = useFederation();
  const genesisInvalid =
    genesisHash !== "" && !GENESIS_PATTERN.test(genesisHash);
  return (
    <Section title={t("FederationNav.overview")}>
      <VStack align="stretch" spacing={4}>
        <Text
          role="status"
          fontWeight="semibold"
          color={busy ? "orange.500" : state ? "green.500" : "gray.500"}
        >
          {t(
            busy
              ? "ConnectionFlow.connecting"
              : state
                ? "FederationNav.connected"
                : "FederationNav.disconnected"
          )}
        </Text>
        <SimpleGrid columns={{ base: 1, md: 2 }} spacing={4}>
          {["rooms", "league"].map((destination) => (
            <AdvancedCard key={destination} p={4}>
              <Button
                as={Link}
                href={`/federation/${destination}`}
                colorScheme={primaryColor}
                variant="ghost"
                w="100%"
              >
                {t(`FederationNav.${destination}`)}
              </Button>
            </AdvancedCard>
          ))}
        </SimpleGrid>
        <Section
          title={t("FederationPage.settings.title")}
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
                }}
              />
            </FormControl>
            <FormControl isInvalid={genesisInvalid}>
              <FormLabel htmlFor="federation-genesis">
                {t("FederationPage.settings.genesisLabel")}
              </FormLabel>
              <Input
                id="federation-genesis"
                isDisabled={busy}
                value={genesisHash}
                onChange={(e) => {
                  setGenesisHash(e.target.value.trim());
                }}
              />
              <FormErrorMessage>
                {t("FederationPage.settings.genesisInvalid")}
              </FormErrorMessage>
            </FormControl>
            <HStack justify="flex-end">
              <Button
                colorScheme={primaryColor}
                isLoading={busy}
                isDisabled={
                  !address || !peer || !GENESIS_PATTERN.test(genesisHash)
                }
                onClick={() => void handleRequest()}
              >
                {t("FederationPage.settings.connect")}
              </Button>
            </HStack>
          </VStack>
        </Section>

        {identityRequired ? (
          <>
            <Text role="alert" color="red.600">
              {t("FederationPage.identityRequired")}
            </Text>
            <Button
              colorScheme={primaryColor}
              alignSelf="flex-start"
              as={Link}
              href="/federation/identity"
            >
              {t("FederationPage.openIdentity")}
            </Button>
          </>
        ) : (
          error && (
            <Text role="alert" color="red.600">
              {t("FederationPage.error")}
            </Text>
          )
        )}
      </VStack>
    </Section>
  );
}
