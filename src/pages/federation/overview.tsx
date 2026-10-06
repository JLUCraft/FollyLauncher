import {
  Button,
  FormControl,
  FormErrorMessage,
  FormHelperText,
  FormLabel,
  HStack,
  Input,
  Text,
  VStack,
} from "@chakra-ui/react";
import Link from "next/link";
import { useRouter } from "next/router";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { brand } from "@/brand";
import { Section } from "@/components/common/section";
import { SetupGuide } from "@/components/federation/setup-guide";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";
import { useToast } from "@/contexts/toast";
import { copyText } from "@/utils/copy";
import {
  GENESIS_PATTERN,
  buildConnectionLink,
} from "@/utils/federation/connection-link";
import { federationErrorKey } from "@/utils/federation/errors";

export default function OverviewPage() {
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
    connect: handleRequest,
    connectWithLink,
  } = useFederation();
  const router = useRouter();
  // Connecting needs the organizer's setup first, so take the student straight to that step.
  useEffect(() => {
    if (identityRequired) void router.push("/federation/identity");
  }, [identityRequired, router]);
  const [link, setLink] = useState("");
  const [linkInvalid, setLinkInvalid] = useState(false);
  const genesisInvalid =
    genesisHash !== "" && !GENESIS_PATTERN.test(genesisHash);
  const shareLink =
    state && activeTarget
      ? buildConnectionLink(activeTarget, brand.deeplinkScheme)
      : "";
  return (
    <Section title={t("FederationNav.overview")}>
      <VStack align="stretch" spacing={4}>
        <SetupGuide />
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
        {state ? (
          <>
            <Button
              as={Link}
              href="/federation/league"
              colorScheme={primaryColor}
              alignSelf="flex-start"
            >
              {t("FederationNav.league")}
            </Button>
            <FormControl>
              <FormLabel htmlFor="federation-share">
                {t("FederationPage.settings.shareLabel")}
              </FormLabel>
              <HStack>
                <Input id="federation-share" value={shareLink} isReadOnly />
                <Button
                  onClick={() => void copyText(shareLink, { toast })}
                  flexShrink={0}
                >
                  {t("FederationPage.settings.shareCopy")}
                </Button>
              </HStack>
            </FormControl>
          </>
        ) : (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              setLinkInvalid(!connectWithLink(link));
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
                  placeholder={t("FederationPage.settings.linkPlaceholder")}
                  value={link}
                  onChange={(e) => {
                    setLink(e.target.value);
                    setLinkInvalid(false);
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
        )}
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
              {t(federationErrorKey(error))}
            </Text>
          )
        )}
      </VStack>
    </Section>
  );
}
