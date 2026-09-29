import {
  Button,
  FormControl,
  FormLabel,
  HStack,
  Input,
  Select,
  Text,
  Textarea,
  VStack,
} from "@chakra-ui/react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";
import { IdentityFacts, UnionService } from "@/services/union";
import { encodeHex } from "@/utils/federation/hex";

// All fields remain visible during signature review, including unfamiliar future actions.
function RecordFields({ value }: { value: unknown }) {
  const { t } = useTranslation();
  if (value === null || value === undefined) return <Text>—</Text>;
  if (typeof value !== "object")
    return <Text overflowWrap="anywhere">{String(value)}</Text>;
  if (Array.isArray(value) && value.every((v) => typeof v === "number"))
    return <Text overflowWrap="anywhere">{encodeHex(value)}</Text>;
  return (
    <VStack align="stretch" pl={3} borderLeftWidth="1px">
      {Object.entries(value).map(([key, entry]) => (
        <Section
          key={key}
          title={t(`IdentityFlow.fields.${key}`, { defaultValue: key })}
        >
          <RecordFields value={entry} />
        </Section>
      ))}
    </VStack>
  );
}
export function IdentityPanel() {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const {
    identity,
    identityFacts: facts,
    setIdentityFacts: setFacts,
  } = useFederation();
  const [configuration, setConfiguration] = useState("");
  const [configured, setConfigured] = useState(false);
  const [configurationLoaded, setConfigurationLoaded] = useState(false);
  const [proposal, setProposal] = useState<unknown>(null);
  const [imported, setImported] = useState("");
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const [kind, setKind] = useState("bind_device");
  const [member, setMember] = useState("");
  const [device, setDevice] = useState("");
  const [club, setClub] = useState("");
  const [profile, setProfile] = useState("");
  useEffect(() => {
    let active = true;
    void UnionService.identityConfiguration()
      .then((value) => {
        if (active && value) {
          setConfiguration(JSON.stringify(value, null, 2));
          setConfigured(true);
        }
      })
      .catch(() => {
        if (active) setFailed(true);
      })
      .finally(() => {
        if (active) setConfigurationLoaded(true);
      });
    return () => {
      active = false;
    };
  }, []);
  const needsSetup = configurationLoaded && !configured;
  const run = async (operation: string) => {
    setBusy(true);
    setFailed(false);
    try {
      if (operation === "save") {
        const value = await UnionService.identityConfiguration(
          JSON.parse(configuration)
        );
        setConfiguration(JSON.stringify(value, null, 2));
        setConfigured(true);
        setFacts(null);
        setProposal(null);
      } else if (operation === "import") {
        setProposal(JSON.parse(imported));
      } else if (operation === "host") {
        setProposal(await UnionService.bindHostDevice());
      } else if (operation === "automatic") {
        setFacts((await UnionService.automatedIdentity()) as IdentityFacts);
      } else {
        const value = await UnionService.identityAction({
          operation,
          ...(operation === "draft"
            ? {
                action:
                  kind === "bind_device"
                    ? { type: kind, member, device }
                    : { type: kind, member, club, profile },
              }
            : {}),
          ...(operation === "sign" || operation === "submit"
            ? { record: proposal }
            : {}),
        });
        if (operation === "draft" || operation === "sign") setProposal(value);
        else {
          setFacts(value as IdentityFacts);
          if (operation === "submit") setProposal(null);
        }
      }
    } catch {
      setFailed(true);
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
        <Text>{t("IdentityFlow.intro")}</Text>
        {needsSetup && <Text role="status">{t("IdentityFlow.setup")}</Text>}
        <HStack flexWrap="wrap">
          {["sync", "automatic", "host"].map((op) => (
            <Button
              key={op}
              isDisabled={busy || !configured}
              onClick={() => void run(op)}
            >
              {t(`IdentityFlow.${op}`)}
            </Button>
          ))}
        </HStack>
        {busy && (
          <Text role="status">{t("FederationPage.identity.loading")}</Text>
        )}
        {failed && (
          <Text role="alert">{t("FederationPage.identity.unavailable")}</Text>
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
            <Empty size="md" description={t("FederationPage.identity.empty")} />
          )
        )}
        <Section
          title={t("IdentityFlow.binding")}
          isAccordion
          initialIsOpen={false}
        >
          <VStack align="stretch" spacing={3}>
            <Select
              aria-label={t("IdentityFlow.binding")}
              value={kind}
              onChange={(e) => setKind(e.target.value)}
            >
              <option value="bind_device">{t("IdentityFlow.device")}</option>
              <option value="bind_game_profile">
                {t("IdentityFlow.profile")}
              </option>
            </Select>
            <FormControl>
              <FormLabel htmlFor="identity-member">
                {t("IdentityFlow.member")}
              </FormLabel>
              <Select
                id="identity-member"
                placeholder={t("IdentityFlow.member")}
                value={member}
                isDisabled={!facts || busy}
                onChange={(e) => {
                  setMember(e.target.value);
                  setClub("");
                  setDevice("");
                  setProfile(facts?.profiles[e.target.value] || "");
                }}
              >
                {Array.from(
                  new Set([
                    ...(facts?.memberships.map(([id]) => id) || []),
                    ...Object.values(facts?.devices || {}),
                  ])
                )
                  .sort()
                  .map((id) => (
                    <option key={id} value={id}>
                      {id}
                    </option>
                  ))}
              </Select>
            </FormControl>
            {kind === "bind_device" ? (
              <FormControl>
                <FormLabel htmlFor="identity-device">
                  {t("IdentityFlow.device")}
                </FormLabel>
                <Input
                  id="identity-device"
                  list="identity-known-devices"
                  value={device}
                  onChange={(e) => setDevice(e.target.value)}
                />
                <datalist id="identity-known-devices">
                  {Object.entries(facts?.devices || {})
                    .filter(
                      ([id, owner]) =>
                        owner === member && !facts?.revoked_devices.includes(id)
                    )
                    .map(([id]) => (
                      <option key={id} value={id} />
                    ))}
                </datalist>
              </FormControl>
            ) : (
              <>
                <FormControl>
                  <FormLabel htmlFor="identity-club">
                    {t("IdentityFlow.club")}
                  </FormLabel>
                  <Select
                    id="identity-club"
                    placeholder={t("IdentityFlow.club")}
                    value={club}
                    isDisabled={!member || busy}
                    onChange={(e) => setClub(e.target.value)}
                  >
                    {facts?.memberships
                      .filter(([id]) => id === member)
                      .map(([, club]) => (
                        <option key={club} value={club}>
                          {club}
                        </option>
                      ))}
                  </Select>
                </FormControl>
                <FormControl>
                  <FormLabel htmlFor="identity-profile">
                    {t("IdentityFlow.profile")}
                  </FormLabel>
                  <Input
                    id="identity-profile"
                    value={profile}
                    onChange={(e) => setProfile(e.target.value)}
                  />
                </FormControl>
              </>
            )}
            <Button
              colorScheme={primaryColor}
              isDisabled={
                busy ||
                !configured ||
                !member ||
                (kind === "bind_device" ? !device : !club || !profile)
              }
              onClick={() => void run("draft")}
            >
              {t("IdentityFlow.review")}
            </Button>
          </VStack>
        </Section>
        {proposal !== null && (
          <Section title={t("IdentityFlow.review")}>
            <VStack align="stretch" spacing={3}>
              <Text>{t("IdentityFlow.reviewHint")}</Text>
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
              <HStack>
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
          key={needsSetup ? "setup" : "ready"}
          title={t("FederationNav.advanced")}
          isAccordion
          initialIsOpen={needsSetup}
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
              <Textarea
                aria-label={t("IdentityFlow.export")}
                isReadOnly
                value={JSON.stringify(proposal, null, 2)}
              />
            )}
          </VStack>
        </Section>
      </VStack>
    </Section>
  );
}
