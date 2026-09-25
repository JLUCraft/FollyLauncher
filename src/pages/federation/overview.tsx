import {
  Button,
  FormControl,
  FormLabel,
  HStack,
  Input,
  Text,
  VStack,
} from "@chakra-ui/react";
import Link from "next/link";
import { useTranslation } from "react-i18next";
import { Section } from "@/components/common/section";
import { HashChip } from "@/components/federation/hash-chip";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";
import { useOffice } from "@/contexts/office";

export default function OverviewPage() {
  const { t } = useTranslation();
  const office = useOffice();
  const { identityFacts } = useFederation();
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
    identity,
    member,
    connecting: busy,
    connectionError: error,
    identityRequired,
    connect: handleRequest,
    activeTarget,
  } = useFederation();
  const displayedMember = identityFacts
    ? identityFacts.revoked_devices.includes(identity)
      ? undefined
      : identityFacts.devices[identity]
    : member;
  const displayedClubs = displayedMember
    ? identityFacts
      ? identityFacts.memberships
          .filter(([id]) => id === displayedMember)
          .map(([, club]) => club)
      : [state?.members[displayedMember]?.club].filter(Boolean)
    : [];
  return (
    <Section title={t("FederationNav.overview")}>
      <VStack align="stretch" spacing={4}>
        <Text role="status">
          {t(
            busy
              ? "ConnectionFlow.connecting"
              : state
                ? "FederationNav.connected"
                : "FederationNav.disconnected"
          )}
        </Text>
        {state && activeTarget && <HashChip value={activeTarget.peer} />}
        <Section
          title={t("FederationPage.settings.title")}
          description={t("FederationPage.settings.description")}
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
            <FormControl>
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
            </FormControl>
            <HStack justify="flex-end">
              <Button
                colorScheme={primaryColor}
                isLoading={busy}
                isDisabled={
                  !address || !peer || !/^[a-f0-9]{64}$/.test(genesisHash)
                }
                onClick={() => void handleRequest()}
              >
                {t("FederationPage.settings.connect")}
              </Button>
            </HStack>
            {identity && (
              <VStack spacing={1} align="start" fontSize="sm">
                {identityFacts && (
                  <Text>
                    {t("IdentityFlow.position", {
                      sequence: identityFacts.checkpoint.sequence,
                    })}
                  </Text>
                )}
                <Text overflowWrap="anywhere">
                  {t("FederationPage.settings.myDevice", { id: identity })}
                </Text>
                {displayedMember && (
                  <Text>
                    {t("FederationPage.settings.myMember", {
                      id: displayedMember,
                    })}
                  </Text>
                )}
                {displayedClubs.map((club) => (
                  <Text key={club}>
                    {t("FederationPage.settings.myClub", { club })}
                  </Text>
                ))}
              </VStack>
            )}
          </VStack>
        </Section>

        <Section title={t("Office.connection")}>
          <FormControl>
            <FormLabel htmlFor="office-genesis">
              {t("Office.genesis")}
            </FormLabel>
            <Input
              id="office-genesis"
              value={office.genesis}
              onChange={(e) => office.setGenesis(e.target.value.trim())}
              isDisabled={office.connecting}
            />
          </FormControl>
          <Text fontSize="sm">{t("ConnectionFlow.domains")}</Text>
          <Button
            isLoading={office.connecting}
            isDisabled={!office.canConnect}
            onClick={() => void office.connect()}
          >
            {t("ConnectionFlow.connectOffice")}
          </Button>
          <Text role="status">
            {t(
              office.connecting
                ? "ConnectionFlow.connecting"
                : office.data
                  ? "ConnectionFlow.officeConnected"
                  : "FederationNav.disconnected"
            )}
          </Text>
          {office.connectionError && (
            <Text role="alert">{t("Office.error")}</Text>
          )}
          {office.target && office.data && (
            <HashChip value={office.target.peer} />
          )}
        </Section>
        {identityRequired ? (
          <>
            <Text role="alert" color="red.500">
              {t("FederationPage.identityRequired")}
            </Text>
            <Button
              colorScheme={primaryColor}
              as={Link}
              href="/federation/identity"
            >
              {t("FederationPage.openIdentity")}
            </Button>
          </>
        ) : (
          error && (
            <Text role="alert" color="red.500">
              {t("FederationPage.error")}
            </Text>
          )
        )}
      </VStack>
    </Section>
  );
}
