import {
  Box,
  Button,
  FormControl,
  FormErrorMessage,
  FormHelperText,
  FormLabel,
  HStack,
  Input,
  Spinner,
  Tag,
  Text,
  VStack,
} from "@chakra-ui/react";
import Link from "next/link";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { brand } from "@/brand";
import AdvancedCard from "@/components/common/advanced-card";
import { Section } from "@/components/common/section";
import { IdentityPanel } from "@/components/federation/identity-panel";
import { SetupGuide } from "@/components/federation/setup-guide";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";
import { useToast } from "@/contexts/toast";
import { UnionService } from "@/services/union";
import { copyText } from "@/utils/copy";
import {
  GENESIS_PATTERN,
  buildConnectionLink,
} from "@/utils/federation/connection-link";
import { federationErrorKey } from "@/utils/federation/errors";
import { FEDERATION_ACTIVITIES_PATH } from "@/utils/federation/navigation";
import { shortenIdentifier } from "@/utils/string";

const STATUS_COLOR = {
  connecting: "orange",
  connected: "green",
  disconnected: "gray",
} as const;

export default function FederationSettingsPage() {
  const { t } = useTranslation();
  const toast = useToast();
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
    activeTarget,
    connecting: busy,
    connectionError: error,
    identityRequired,
    identityConfigured,
    connectionLink: link,
    setConnectionLink: setLink,
    connect: handleRequest,
    connectWithLink,
    disconnect,
    discovering,
    discoveryRan,
    discoverAndConnect,
    member,
  } = useFederation();
  const [rejectedLink, setRejectedLink] = useState<string | null>(null);
  const [joining, setJoining] = useState(false);
  const [joinFailed, setJoinFailed] = useState(false);
  // The server answered but refused the read: this device is not a member yet.
  const needsMembership = !state && error === "membership";
  // One tap from "found the union" to "joined and connected": the skin station
  // proves the member, then discovery runs again with the new identity facts.
  const joinUnion = async () => {
    setJoining(true);
    setJoinFailed(false);
    try {
      await UnionService.automatedIdentity();
      await discoverAndConnect();
    } catch {
      setJoinFailed(true);
    } finally {
      setJoining(false);
    }
  };
  const linkInvalid = rejectedLink !== null && rejectedLink === link;
  const genesisInvalid =
    genesisHash !== "" && !GENESIS_PATTERN.test(genesisHash);
  const shareLink =
    state && activeTarget
      ? buildConnectionLink(activeTarget, brand.deeplinkScheme)
      : "";
  const needsIdentity = identityRequired || identityConfigured !== true;
  const status = busy ? "connecting" : state ? "connected" : "disconnected";
  const statusColor = STATUS_COLOR[status];
  // The identity section opens itself while identity steps are pending; once the
  // user toggles it, their choice wins until the underlying state changes.
  const identityPending =
    identityConfigured === false || (identityConfigured === true && !member);
  const [identityToggled, setIdentityToggled] = useState<boolean | null>(null);
  const identityOpen = identityToggled ?? identityPending;
  const focusTarget = (target: string) => {
    if (target.startsWith("identity-")) setIdentityToggled(true);
    // Collapse animation and mount must settle before the element can take focus.
    setTimeout(() => {
      const element = document.getElementById(target);
      element?.scrollIntoView({ block: "center" });
      element?.focus();
    }, 350);
  };
  return (
    <Section title={t("FederationNav.title")}>
      <VStack align="stretch" spacing={4}>
        <SetupGuide
          onAction={(step) => {
            if (step === "connect") void discoverAndConnect();
            else
              focusTarget(
                step === "setup" ? "identity-setup" : "identity-verify"
              );
          }}
        />
        <AdvancedCard
          level="front"
          borderLeftWidth="4px"
          borderLeftColor={`${statusColor}.400`}
        >
          <HStack align="flex-start" spacing={3}>
            <Box pt={1.5} aria-hidden>
              {busy ? (
                <Spinner size="xs" color={`${statusColor}.400`} />
              ) : (
                <Box
                  boxSize={2.5}
                  borderRadius="full"
                  bg={`${statusColor}.400`}
                />
              )}
            </Box>
            <VStack align="stretch" spacing={3} flex={1}>
              <HStack spacing={2}>
                <Text role="status" fontWeight="semibold">
                  {t(
                    busy
                      ? "ConnectionFlow.connecting"
                      : state
                        ? "FederationNav.connected"
                        : needsMembership
                          ? "ConnectionFlow.membershipTitle"
                          : "FederationNav.disconnected"
                  )}
                </Text>
                {state && (
                  <Tag ml="auto" colorScheme="gray" flexShrink={0}>
                    {t("FederationPage.points.revision", {
                      revision: state.revision,
                    })}
                  </Tag>
                )}
              </HStack>
              {state && activeTarget && (
                <Text fontSize="sm" className="secondary-text">
                  {t("FederationPage.settings.serverPeer", {
                    peer: shortenIdentifier(activeTarget.peer),
                  })}
                </Text>
              )}
              {!!link.trim() && (
                <Text role="status" fontSize="sm">
                  {t("ConnectionFlow.linkStaged")}
                </Text>
              )}

              {needsIdentity ? (
                discovering ? (
                  <HStack spacing={2}>
                    <Spinner size="xs" />
                    <Text fontSize="sm" className="secondary-text">
                      {t("ConnectionFlow.searching")}
                    </Text>
                  </HStack>
                ) : (
                  <>
                    <Text fontSize="sm">
                      {t(
                        identityConfigured === null && !identityRequired
                          ? "FederationPage.identity.loading"
                          : "FederationPage.identityRequired"
                      )}
                    </Text>
                    {identityRequired && (
                      <Button
                        colorScheme={primaryColor}
                        alignSelf="flex-start"
                        onClick={() => focusTarget("identity-verify")}
                      >
                        {t("FederationPage.openIdentity")}
                      </Button>
                    )}
                  </>
                )
              ) : needsMembership ? (
                <>
                  <Text fontSize="sm">
                    {t("ConnectionFlow.membershipBody")}
                  </Text>
                  <HStack flexWrap="wrap" spacing={2}>
                    <Button
                      colorScheme={primaryColor}
                      isLoading={joining}
                      onClick={() => void joinUnion()}
                    >
                      {t("ConnectionFlow.membershipAction")}
                    </Button>
                    <Button
                      variant="ghost"
                      onClick={() => void discoverAndConnect()}
                    >
                      {t("ConnectionFlow.searchRetry")}
                    </Button>
                  </HStack>
                  {joinFailed && (
                    <Text role="alert" color="red.600" fontSize="sm">
                      {t("ConnectionFlow.joinFailed")}
                    </Text>
                  )}
                </>
              ) : state && !link.trim() ? (
                <HStack flexWrap="wrap" spacing={2}>
                  <Button
                    as={Link}
                    href={FEDERATION_ACTIVITIES_PATH}
                    colorScheme={primaryColor}
                  >
                    {t("FederationNav.activities")}
                  </Button>
                  <Button
                    variant="outline"
                    onClick={() => void copyText(shareLink, { toast })}
                  >
                    {t("FederationPage.settings.shareAction")}
                  </Button>
                  <Button
                    variant="ghost"
                    colorScheme="red"
                    onClick={disconnect}
                  >
                    {t("FederationPage.settings.disconnect")}
                  </Button>
                </HStack>
              ) : (
                <>
                  {discovering ? (
                    <HStack spacing={2}>
                      <Spinner size="xs" />
                      <Text fontSize="sm" className="secondary-text">
                        {t("ConnectionFlow.searching")}
                      </Text>
                    </HStack>
                  ) : (
                    discoveryRan && (
                      <HStack spacing={2} flexWrap="wrap">
                        <Text fontSize="sm" className="secondary-text">
                          {t("ConnectionFlow.searchEmpty")}
                        </Text>
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => void discoverAndConnect()}
                        >
                          {t("ConnectionFlow.searchRetry")}
                        </Button>
                      </HStack>
                    )
                  )}
                  <form
                    onSubmit={(e) => {
                      e.preventDefault();
                      setRejectedLink(connectWithLink(link) ? null : link);
                    }}
                  >
                    <FormControl isInvalid={linkInvalid}>
                      <FormLabel htmlFor="federation-link">
                        {t("FederationPage.settings.linkLabel")}
                      </FormLabel>
                      <HStack>
                        <Input
                          id="federation-link"
                          isDisabled={busy}
                          placeholder={t(
                            "FederationPage.settings.linkPlaceholder"
                          )}
                          value={link}
                          onChange={(e) => {
                            setLink(e.target.value);
                          }}
                        />
                        <Button
                          type="submit"
                          colorScheme={primaryColor}
                          isLoading={busy}
                          isDisabled={!link.trim()}
                          flexShrink={0}
                        >
                          {t("FederationPage.settings.linkConnect")}
                        </Button>
                      </HStack>
                      <FormHelperText>
                        {t("FederationPage.settings.linkHelp")}
                      </FormHelperText>
                      <FormErrorMessage>
                        {t("FederationPage.settings.linkInvalid")}
                      </FormErrorMessage>
                    </FormControl>
                  </form>
                  {error && (
                    <Text role="alert" color="red.600" fontSize="sm">
                      {t(federationErrorKey(error))}
                    </Text>
                  )}
                </>
              )}
            </VStack>
          </HStack>
        </AdvancedCard>

        {identityConfigured !== null && (
          <Section
            key={String(identityOpen)}
            title={t("FederationNav.identity")}
            isAccordion
            initialIsOpen={identityOpen}
            onAccordionToggle={setIdentityToggled}
          >
            <IdentityPanel />
          </Section>
        )}
        {identityConfigured === true && (
          <Section
            title={t("FederationPage.settings.manualTitle")}
            isAccordion
            initialIsOpen={false}
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
        )}
      </VStack>
    </Section>
  );
}
