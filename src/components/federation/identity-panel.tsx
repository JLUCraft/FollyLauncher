import {
  Button,
  FormControl,
  FormLabel,
  HStack,
  Text,
  Textarea,
  VStack,
} from "@chakra-ui/react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import { IdentityActionForm } from "@/components/federation/identity-action-form";
import { RecordFields } from "@/components/federation/record-fields";
import { SetupGuide } from "@/components/federation/setup-guide";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";
import { useToast } from "@/contexts/toast";
import { IdentityFacts, UnionService } from "@/services/union";
import { copyText } from "@/utils/copy";
import { GENESIS_PATTERN } from "@/utils/federation/connection-link";

export function IdentityPanel() {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const toast = useToast();
  const [publicIdentity, setPublicIdentity] = useState("");
  const primaryColor = config.appearance.theme.primaryColor;
  const {
    identity,
    identityFacts: facts,
    setIdentityFacts: setFacts,
    identityConfigured,
    setIdentityConfigured,
    address,
    peer,
    genesisHash,
    connect,
  } = useFederation();
  const [configuration, setConfiguration] = useState("");
  const [cached, setCached] = useState(false);
  const [saveInvalid, setSaveInvalid] = useState(false);
  const [proposal, setProposal] = useState<unknown>(null);
  const [imported, setImported] = useState("");
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true;
    void UnionService.publicIdentity()
      .then((value) => {
        if (active) setPublicIdentity(JSON.stringify(value, null, 2));
      })
      .catch(() => {
        if (active) setFailed(true);
      });
    void UnionService.identityConfiguration()
      .then((value) => {
        if (active && value) setConfiguration(JSON.stringify(value, null, 2));
      })
      .catch(() => {
        if (active) setFailed(true);
      });
    return () => {
      active = false;
    };
  }, []);
  const configured = identityConfigured === true;
  const needsSetup = identityConfigured === false;
  const run = async (operation: string, draftAction?: unknown) => {
    setBusy(true);
    setFailed(false);
    setSaveInvalid(false);
    try {
      if (operation === "save") {
        let parsed: unknown;
        try {
          parsed = JSON.parse(configuration);
        } catch {
          setSaveInvalid(true);
          return;
        }
        const value = await UnionService.identityConfiguration(parsed);
        setConfiguration(JSON.stringify(value, null, 2));
        setIdentityConfigured(true);
        // Connecting needs this setup, so retry the connection the student already entered.
        if (address && peer && GENESIS_PATTERN.test(genesisHash))
          void connect();
        setCached(false);
        setFacts(null);
        setProposal(null);
      } else if (operation === "import") {
        const config = JSON.parse(configuration);
        setProposal(
          await UnionService.management({
            operation: "review_record",
            genesis: config.genesis,
            genesis_hash: config.genesis_hash,
            record: JSON.parse(imported),
          })
        );
      } else if (operation === "sign_offline") {
        setProposal(await UnionService.signIdentityOffline(proposal));
      } else if (operation === "host") {
        setProposal(await UnionService.bindHostDevice());
      } else if (operation === "automatic") {
        setCached(false);
        setFacts((await UnionService.automatedIdentity()) as IdentityFacts);
      } else {
        const value = await UnionService.identityAction({
          operation,
          ...(operation === "draft" ? { action: draftAction } : {}),
          ...(operation === "sign" || operation === "submit"
            ? { record: proposal }
            : {}),
        });
        if (operation === "draft" || operation === "sign") setProposal(value);
        else {
          setFacts(value as IdentityFacts);
          setCached(operation === "inspect");
          if (operation === "submit") setProposal(null);
        }
      }
    } catch (error) {
      setFailed(true);
      if (operation === "draft") throw error;
    } finally {
      setBusy(false);
    }
  };
  const action =
    proposal &&
    typeof proposal === "object" &&
    "action" in proposal &&
    proposal.action &&
    typeof proposal.action === "object"
      ? (proposal.action as Record<string, unknown>)
      : undefined;
  const binding =
    action?.type === "bind_device" || action?.type === "bind_game_profile";
  const before = facts
    ? (action?.type === "bind_device"
        ? facts.devices[String(action.device)]
        : facts.profiles[String(action?.member)]) || t("IdentityFlow.unbound")
    : t("IdentityFlow.unknown");
  const after =
    action?.type === "bind_device"
      ? String(action.member)
      : String(action?.profile);
  const ownMember = facts?.revoked_devices.includes(identity)
    ? undefined
    : facts?.devices[identity];
  return (
    <Section title={t("FederationNav.identity")}>
      <VStack align="stretch" spacing={4}>
        <SetupGuide />
        <Text>{t("IdentityFlow.intro")}</Text>
        <Section
          title={t("IdentityManagement.publicTitle")}
          description={t("IdentityManagement.publicHint")}
          isAccordion
          initialIsOpen={false}
        >
          <VStack align="stretch" spacing={3}>
            <Textarea
              isReadOnly
              aria-label={t("IdentityManagement.publicTitle")}
              value={publicIdentity}
            />
            <Button
              isDisabled={!publicIdentity}
              onClick={() => void copyText(publicIdentity, { toast })}
            >
              {t("IdentityManagement.copy")}
            </Button>
          </VStack>
        </Section>
        {needsSetup ? (
          <Section title={t("FederationPage.identity.configuration")}>
            <VStack align="stretch" spacing={3}>
              <Text>{t("IdentityFlow.setup")}</Text>
              <Textarea
                aria-label={t("FederationPage.identity.configuration")}
                value={configuration}
                isDisabled={busy}
                onChange={(e) => {
                  setConfiguration(e.target.value);
                  setSaveInvalid(false);
                }}
              />
              {saveInvalid && (
                <Text role="alert">
                  {t("FederationPage.identity.saveInvalid")}
                </Text>
              )}
              <Button
                alignSelf="flex-start"
                colorScheme={primaryColor}
                isLoading={busy}
                isDisabled={!configuration.trim()}
                onClick={() => void run("save")}
              >
                {t("FederationPage.identity.save")}
              </Button>
            </VStack>
          </Section>
        ) : (
          <>
            <VStack align="stretch" spacing={2}>
              <HStack flexWrap="wrap">
                <Button
                  colorScheme={primaryColor}
                  isDisabled={busy || !configured}
                  onClick={() => void run("automatic")}
                >
                  {t("IdentityFlow.automatic")}
                </Button>
                {["sync", "inspect", "host"].map((op) => (
                  <Button
                    key={op}
                    variant="outline"
                    isDisabled={busy || !configured}
                    onClick={() => void run(op)}
                  >
                    {t(`IdentityFlow.${op}`)}
                  </Button>
                ))}
              </HStack>
              <Text fontSize="sm" className="secondary-text">
                {t("IdentityFlow.hint")}
              </Text>
            </VStack>
            {busy && (
              <Text role="status">{t("FederationPage.identity.loading")}</Text>
            )}
            {failed && (
              <Text role="alert">
                {t("FederationPage.identity.unavailable")}
              </Text>
            )}
            {cached && (
              <Text role="status">{t("IdentityManagement.cached")}</Text>
            )}
            {facts ? (
              <AdvancedCard p={4}>
                <VStack align="stretch">
                  <Text>
                    {t("FederationPage.settings.myMember", {
                      id: ownMember || t("IdentityFlow.unbound"),
                    })}
                  </Text>
                  {facts.memberships
                    .filter(([id]) => id === ownMember)
                    .map(([id, c]) => (
                      <Text key={`${id}/${c}`}>
                        {t("FederationPage.settings.myClub", { club: c })}
                      </Text>
                    ))}
                </VStack>
              </AdvancedCard>
            ) : (
              !busy && (
                <Empty
                  size="md"
                  description={t("FederationPage.identity.empty")}
                />
              )
            )}
            {facts && (
              <Section
                title={t("IdentityManagement.facts")}
                isAccordion
                initialIsOpen={false}
              >
                <RecordFields value={facts} />
              </Section>
            )}
            <IdentityActionForm
              disabled={busy || !configured}
              onDraft={(action) => run("draft", action)}
            />
            {proposal !== null && (
              <Section title={t("IdentityFlow.review")}>
                <VStack align="stretch" spacing={3}>
                  <Text>{t("IdentityFlow.reviewHint")}</Text>
                  {typeof action?.type === "string" && (
                    <Text fontWeight="semibold">
                      {t(`IdentityManagement.actions.${action.type}`)}
                    </Text>
                  )}
                  {binding && (
                    <HStack align="start" spacing={4}>
                      <Section title={t("IdentityFlow.before")} flex={1}>
                        <Text overflowWrap="anywhere">{before}</Text>
                      </Section>
                      <Section title={t("IdentityFlow.after")} flex={1}>
                        <Text overflowWrap="anywhere">{after}</Text>
                      </Section>
                    </HStack>
                  )}
                  <RecordFields value={proposal} />
                  <HStack flexWrap="wrap">
                    <Button
                      isDisabled={busy || !configured}
                      onClick={() => void run("sign_offline")}
                    >
                      {t("IdentityManagement.signOffline")}
                    </Button>
                    <Button
                      colorScheme={primaryColor}
                      isDisabled={busy || !configured}
                      onClick={() => void run("sign")}
                    >
                      {t("IdentityFlow.sign")}
                    </Button>
                    <Button
                      colorScheme={primaryColor}
                      isDisabled={busy || !configured}
                      onClick={() => void run("submit")}
                    >
                      {t("IdentityFlow.submit")}
                    </Button>
                  </HStack>
                </VStack>
              </Section>
            )}
            <Section
              title={t("FederationNav.advanced")}
              isAccordion
              initialIsOpen={false}
            >
              <VStack align="stretch" spacing={3}>
                <Text overflowWrap="anywhere">
                  {t("FederationPage.settings.myDevice", { id: identity })}
                </Text>
                {facts && (
                  <Text>
                    {t("IdentityFlow.position", {
                      sequence: facts.checkpoint.sequence,
                    })}
                  </Text>
                )}
                <FormControl>
                  <FormLabel htmlFor="identity-configuration">
                    {t("FederationPage.identity.configuration")}
                  </FormLabel>
                  <Textarea
                    id="identity-configuration"
                    value={configuration}
                    isDisabled={busy}
                    onChange={(e) => setConfiguration(e.target.value)}
                  />
                </FormControl>
                <Button
                  isDisabled={busy || !configuration}
                  onClick={() => void run("save")}
                >
                  {t("FederationPage.identity.save")}
                </Button>
                <FormControl>
                  <FormLabel htmlFor="identity-import">
                    {t("IdentityFlow.import")}
                  </FormLabel>
                  <Textarea
                    id="identity-import"
                    value={imported}
                    onChange={(e) => setImported(e.target.value)}
                  />
                </FormControl>
                <Button
                  isDisabled={busy || !imported}
                  onClick={() => void run("import")}
                >
                  {t("IdentityFlow.review")}
                </Button>
                {proposal !== null && (
                  <>
                    <Section title={t("IdentityFlow.export")}>
                      <RecordFields value={proposal} />
                    </Section>
                    <Button
                      onClick={() =>
                        void copyText(JSON.stringify(proposal, null, 2), {
                          toast,
                        })
                      }
                    >
                      {t("IdentityManagement.copy")}
                    </Button>
                  </>
                )}
              </VStack>
            </Section>
          </>
        )}
      </VStack>
    </Section>
  );
}
