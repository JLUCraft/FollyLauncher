import {
  AlertDialog,
  AlertDialogBody,
  AlertDialogContent,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogOverlay,
  Button,
  Divider,
  FormControl,
  FormHelperText,
  FormLabel,
  HStack,
  Input,
  Modal,
  ModalBody,
  ModalCloseButton,
  ModalContent,
  ModalFooter,
  ModalHeader,
  ModalOverlay,
  Progress,
  Select,
  Spinner,
  Tag,
  Text,
  Textarea,
  VStack,
  useDisclosure,
} from "@chakra-ui/react";
import { save } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import { useLauncherConfig } from "@/contexts/config";
import { useApplicationRequest, useFederation } from "@/contexts/federation";
import { useToast } from "@/contexts/toast";
import { PointPresentation, UnionService, UnionState } from "@/services/union";
import { UtilsService } from "@/services/utils";
import { sanitizeFileName } from "@/utils/string";

const TOKENS_PER_UNIT = BigInt(1000);
const MAX_U32 = BigInt(4_294_967_295);
const MAX_SAFE = BigInt(Number.MAX_SAFE_INTEGER);
const POINTS_HISTORY_PAGE_SIZE = 8;
const OUTPUT_LENGTH_PRESETS = [
  {
    key: "short",
    labelKey: "FederationPage.points.lengthShort",
    tokens: BigInt(256),
  },
  {
    key: "standard",
    labelKey: "FederationPage.points.lengthStandard",
    tokens: BigInt(1024),
  },
  {
    key: "detailed",
    labelKey: "FederationPage.points.lengthDetailed",
    tokens: BigInt(4096),
  },
];

function pageCount(length: number): number {
  return Math.max(1, Math.ceil(length / POINTS_HISTORY_PAGE_SIZE));
}

function pageItems<T>(items: T[], page: number): T[] {
  const safePage = Math.min(Math.max(page, 0), pageCount(items.length) - 1);
  return items.slice(
    safePage * POINTS_HISTORY_PAGE_SIZE,
    (safePage + 1) * POINTS_HISTORY_PAGE_SIZE
  );
}

function aiStatusKey(status: string): string {
  switch (status) {
    case "queued":
      return "FederationPage.points.aiStatusQueued";
    case "running":
      return "FederationPage.points.aiStatusRunning";
    case "completed":
      return "FederationPage.points.aiStatusCompleted";
    case "failed":
      return "FederationPage.points.aiStatusFailed";
    case "refunded":
      return "FederationPage.points.aiStatusRefunded";
    case "cancelled":
      return "FederationPage.points.aiStatusCancelled";
    default:
      return "FederationPage.points.aiStatusUnknown";
  }
}

function aiPurposeKey(purpose: string): string {
  switch (purpose) {
    case "member.assist":
      return "FederationPage.points.purposeMemberAssist";
    case "events.draft":
      return "FederationPage.points.purposeEventsDraft";
    case "events.review":
      return "FederationPage.points.purposeEventsReview";
    case "events.summary":
      return "FederationPage.points.purposeEventsSummary";
    default:
      return "FederationPage.points.purposeOther";
  }
}

function AiFlowActions({
  state,
  points,
  row,
}: {
  state: UnionState;
  points: PointPresentation;
  row: PointPresentation["ai"][number];
}) {
  const { t } = useTranslation();
  const { busy, error, handleRequest } = useApplicationRequest();
  const [confirming, setConfirming] = useState(false);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const flow = state.flows?.[row.id];
  const isCurrent = flow?.current === row.stage;
  const retryable =
    isCurrent && row.flow_status === "active" && row.status === "failed";
  const cancellable =
    isCurrent &&
    (row.flow_status === "active" || row.flow_status === "approved");
  const quote = parseAmount(row.quote_points);
  const available = parseAmount(points.available_points);
  const retrySupported =
    quote !== null && available !== null && available >= quote;
  const payer = `${points.account.club} · ${points.account.member}`;
  if (!retryable && !cancellable) return null;
  const retry = async () => {
    const accepted = await handleRequest({
      type: "flows.retry_ai",
      instance: row.id,
    });
    if (accepted) setConfirming(false);
  };
  return (
    <VStack align="stretch" spacing={1}>
      <HStack spacing={2}>
        {retryable && (
          <Button
            size="xs"
            variant="outline"
            isDisabled={busy || !retrySupported}
            onClick={() => setConfirming(true)}
          >
            {t("FederationPage.points.aiRetry")}
          </Button>
        )}
        {cancellable && (
          <Button
            size="xs"
            variant="ghost"
            isDisabled={busy}
            onClick={() =>
              void handleRequest({
                type: "flows.cancel",
                instance: row.id,
              })
            }
          >
            {t("FederationPage.points.aiCancel")}
          </Button>
        )}
      </HStack>
      {error && (
        <Text role="alert" color="red.600" fontSize="xs">
          {error}
        </Text>
      )}
      <AlertDialog
        isOpen={confirming}
        leastDestructiveRef={cancelRef}
        onClose={() => setConfirming(false)}
        isCentered
      >
        <AlertDialogOverlay>
          <AlertDialogContent>
            <AlertDialogHeader>
              {t("FederationPage.points.aiRetryConfirmTitle")}
            </AlertDialogHeader>
            <AlertDialogBody>
              <VStack align="stretch" spacing={2}>
                <Text>
                  {t("FederationPage.points.aiRetryConfirmBody", {
                    stage: row.stage + 1,
                    quote: row.quote_points,
                    payer,
                    available: points.available_points,
                  })}
                </Text>
                {!retrySupported && (
                  <Text role="alert" color="red.600" fontSize="sm">
                    {t("FederationPage.points.insufficient")}
                  </Text>
                )}
                {error && (
                  <Text role="alert" color="red.600" fontSize="sm">
                    {error}
                  </Text>
                )}
              </VStack>
            </AlertDialogBody>
            <AlertDialogFooter>
              <Button ref={cancelRef} onClick={() => setConfirming(false)}>
                {t("General.cancel")}
              </Button>
              <Button
                colorScheme="gray"
                ml={3}
                isLoading={busy}
                isDisabled={!retrySupported}
                onClick={() => void retry()}
              >
                {t("FederationPage.points.aiRetryConfirmAction")}
              </Button>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialogOverlay>
      </AlertDialog>
    </VStack>
  );
}

function parseAmount(value: string): bigint | null {
  const text = value.trim();
  if (!/^\d+$/.test(text)) return null;
  return BigInt(text);
}

function exactNumber(value: bigint): number | null {
  if (value < BigInt(0) || value > MAX_SAFE) return null;
  return Number(value);
}

function nativeOutputLimit(points: PointPresentation): bigint | null {
  const value = points.max_output_tokens;
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0)
    return null;
  return BigInt(value);
}

function supportedLengthPresets(maxOutputTokens: bigint | null) {
  if (maxOutputTokens === null) return [];
  return OUTPUT_LENGTH_PRESETS.filter(
    (preset) => preset.tokens <= maxOutputTokens
  );
}

function quotePoints(
  price: PointPresentation["prices"][string],
  maxTokens: bigint
): bigint | null {
  const base = parseAmount(price.base_points);
  const inputRate = parseAmount(price.input_points_per_1k);
  const outputRate = parseAmount(price.output_points_per_1k);
  const maxInput = parseAmount(price.max_input_tokens);
  if (
    base === null ||
    inputRate === null ||
    outputRate === null ||
    maxInput === null ||
    maxTokens <= BigInt(0)
  )
    return null;
  const rate = maxInput * inputRate + maxTokens * outputRate;
  return base + (rate + TOKENS_PER_UNIT - BigInt(1)) / TOKENS_PER_UNIT;
}

function progressValue(points: PointPresentation): number {
  const earned = parseAmount(points.earned_today);
  const cap = parseAmount(points.daily_member_cap || "");
  if (!earned || !cap || cap <= BigInt(0)) return 0;
  return Math.min(100, Number((earned * BigInt(100)) / cap));
}

function formatTime(ms: number): string {
  return new Date(ms).toLocaleString();
}

function workerModels(state: UnionState | undefined): [string, string[]][] {
  return Object.entries(state?.ai_workers || {})
    .filter(([, worker]) => worker.active && worker.models.length > 0)
    .map(([id, worker]): [string, string[]] => [id, [...worker.models].sort()])
    .sort(([a], [b]) => a.localeCompare(b));
}

function budgetScopes(
  state: UnionState | undefined,
  member: string | undefined,
  isAdmin: boolean
): { kind: "event" | "report"; id: string; title: string; club: string }[] {
  if (!state || !member) return [];
  const own = state.members[member];
  const manager =
    isAdmin ||
    !!own?.roles?.some(
      (role) => role === "president" || role === "admin" || role === "manager"
    );
  const scopes: {
    kind: "event" | "report";
    id: string;
    title: string;
    club: string;
  }[] = [];
  if (manager) {
    Object.entries(state.events).forEach(([id, event]) => {
      if (isAdmin || event.organizer === own?.club) {
        scopes.push({
          kind: "event",
          id,
          title: event.title || id,
          club: event.organizer,
        });
      }
    });
    const reports = (
      state.organization as unknown as
        | {
            reports?: Record<string, { title?: string; club: string }>;
          }
        | undefined
    )?.reports;
    Object.entries(reports || {}).forEach(([id, report]) => {
      if (isAdmin || report.club === own?.club) {
        scopes.push({
          kind: "report",
          id,
          title: report.title || id,
          club: report.club,
        });
      }
    });
  }
  return scopes.sort((a, b) =>
    `${a.kind}:${a.id}`.localeCompare(`${b.kind}:${b.id}`)
  );
}

function StatementExport({ points }: { points: PointPresentation }) {
  const { t } = useTranslation();
  const toast = useToast();
  const [busy, setBusy] = useState(false);

  const exportStatement = async () => {
    try {
      const date = new Date(points.as_of_ms).toISOString().slice(0, 10);
      const path = await save({
        defaultPath: sanitizeFileName(
          `points-statement-${date}-${points.account.club}-${points.account.member}.json`
        ),
        filters: [
          {
            name: t("General.dialog.filterName.json"),
            extensions: ["json"],
          },
        ],
      });
      if (!path) return;
      setBusy(true);
      const response = await UtilsService.writeFile(
        path,
        JSON.stringify(points.statement, null, 2),
        "string"
      );
      toast({
        title: t(
          response.status === "success"
            ? "FederationPage.points.exportDone"
            : "FederationPage.points.exportFailed"
        ),
        status: response.status,
      });
    } catch {
      toast({
        title: t("FederationPage.points.exportFailed"),
        status: "error",
      });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Button
      alignSelf="flex-start"
      size="sm"
      aria-label={t("FederationPage.points.exportStatement")}
      isLoading={busy}
      onClick={() => void exportStatement()}
    >
      {t("FederationPage.points.exportStatement")}
    </Button>
  );
}

function RequestAi({ points }: { points: PointPresentation }) {
  const { t } = useTranslation();
  const toast = useToast();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const { state } = useFederation();
  const { busy, error, handleRequest } = useApplicationRequest();
  const [step, setStep] = useState(0);
  const [text, setText] = useState("");
  const [id, setId] = useState("");
  const workers = useMemo(() => workerModels(state), [state]);
  const [worker, setWorker] = useState(workers[0]?.[0] || "");
  const [model, setModel] = useState(workers[0]?.[1][0] || "");
  const [outputLengthKey, setOutputLengthKey] = useState("");
  const [customOutputLength, setCustomOutputLength] = useState(false);
  const [maxTokensText, setMaxTokensText] = useState("");
  const [showAdvancedTokens, setShowAdvancedTokens] = useState(false);
  const confirm = useDisclosure();
  const cancelRef = useRef<HTMLButtonElement>(null);

  const activeModels = useMemo(
    () => workers.find(([name]) => name === worker)?.[1] || [],
    [workers, worker]
  );
  useEffect(() => {
    const ids = workers.map(([name]) => name);
    if (!ids.includes(worker)) setWorker(ids[0] || "");
  }, [worker, workers]);
  useEffect(() => {
    if (!activeModels.includes(model)) setModel(activeModels[0] || "");
  }, [activeModels, model]);
  const nativeMaxOutput = nativeOutputLimit(points);
  const lengthPresets = supportedLengthPresets(nativeMaxOutput);
  const defaultPreset =
    lengthPresets.find((preset) => preset.key === "standard") ||
    lengthPresets[0] ||
    null;
  const selectedPreset =
    lengthPresets.find((preset) => preset.key === outputLengthKey) ||
    defaultPreset;
  const customMaxTokens = parseAmount(maxTokensText);
  const customValidTokens =
    customMaxTokens !== null &&
    customMaxTokens > BigInt(0) &&
    customMaxTokens <= MAX_U32 &&
    nativeMaxOutput !== null &&
    customMaxTokens <= nativeMaxOutput;
  const maxTokens = customOutputLength
    ? customValidTokens
      ? customMaxTokens
      : null
    : selectedPreset?.tokens || null;
  const validTokens = maxTokens !== null;
  const price = points.prices[model];
  const quote = price && maxTokens ? quotePoints(price, maxTokens) : null;
  const maxPoints = quote ? exactNumber(quote) : null;
  const available = parseAmount(points.available_points);
  const insufficient =
    quote !== null && available !== null && available < quote;

  useEffect(() => {
    if (lengthPresets.length === 0) setShowAdvancedTokens(true);
  }, [lengthPresets.length]);

  const submit = async () => {
    if (!id || !quote || maxPoints === null || maxTokens === null) return;
    const accepted = await handleRequest({
      type: "local.points.request_ai",
      id,
      text,
      worker,
      model,
      max_tokens: Number(maxTokens),
      max_points: maxPoints,
    });
    if (!accepted) return;
    setText("");
    setMaxTokensText("");
    setOutputLengthKey("");
    setCustomOutputLength(false);
    setId(`ai-${crypto.randomUUID()}`);
    setStep(0);
    toast({
      title: t("FederationPage.points.requestAccepted"),
      status: "success",
    });
  };

  return (
    <AdvancedCard level="front" p={4}>
      <VStack align="stretch" spacing={3}>
        <Text fontWeight="semibold">
          {t("FederationPage.points.requestTitle")}
        </Text>
        <Text fontSize="sm" className="secondary-text">
          {t("FederationPage.points.requestHelp")}
        </Text>
        {step === 0 && (
          <>
            <FormControl>
              <FormLabel htmlFor="points-request-text">
                {t("FederationPage.points.requestText")}
              </FormLabel>
              <Textarea
                id="points-request-text"
                value={text}
                isDisabled={busy}
                rows={5}
                onChange={(event) => setText(event.target.value)}
              />
              <FormHelperText>
                {t("FederationPage.points.requestSize", {
                  size: new TextEncoder().encode(text).length,
                })}
              </FormHelperText>
            </FormControl>
            <Button
              alignSelf="flex-start"
              colorScheme={primaryColor}
              isDisabled={!text.trim()}
              onClick={() => {
                if (!id) setId(`ai-${crypto.randomUUID()}`);
                setStep(1);
              }}
            >
              {t("FederationPage.points.next")}
            </Button>
          </>
        )}
        {step === 1 && (
          <>
            {workers.length === 0 ? (
              <Empty
                size="sm"
                description={t("FederationPage.points.noWorker")}
              />
            ) : (
              <>
                <FormControl>
                  <FormLabel htmlFor="points-worker">
                    {t("FederationPage.points.worker")}
                  </FormLabel>
                  <Select
                    id="points-worker"
                    value={worker}
                    isDisabled={busy}
                    onChange={(event) => {
                      const next = event.target.value;
                      setWorker(next);
                      setModel(
                        workers.find(([name]) => name === next)?.[1][0] || ""
                      );
                    }}
                  >
                    {workers.map(([name]) => (
                      <option key={name} value={name}>
                        {name}
                      </option>
                    ))}
                  </Select>
                </FormControl>
                <FormControl>
                  <FormLabel htmlFor="points-model">
                    {t("FederationPage.points.model")}
                  </FormLabel>
                  <Select
                    id="points-model"
                    value={model}
                    isDisabled={busy}
                    onChange={(event) => setModel(event.target.value)}
                  >
                    {activeModels.map((name) => (
                      <option key={name} value={name}>
                        {priceLabel(name, points)}
                      </option>
                    ))}
                  </Select>
                </FormControl>
                {lengthPresets.length === 0 ? (
                  <Text fontSize="sm" className="secondary-text">
                    {t("FederationPage.points.lengthUnavailable")}
                  </Text>
                ) : (
                  <FormControl>
                    <FormLabel>{t("FederationPage.points.length")}</FormLabel>
                    <HStack spacing={2} flexWrap="wrap">
                      {lengthPresets.map((preset) => (
                        <Button
                          key={preset.key}
                          size="sm"
                          variant={
                            !customOutputLength &&
                            selectedPreset?.key === preset.key
                              ? "solid"
                              : "outline"
                          }
                          isDisabled={busy}
                          onClick={() => {
                            setCustomOutputLength(false);
                            setMaxTokensText("");
                            setOutputLengthKey(preset.key);
                          }}
                        >
                          {t("FederationPage.points.lengthTokens", {
                            label: t(preset.labelKey),
                            tokens: preset.tokens.toString(),
                          })}
                        </Button>
                      ))}
                    </HStack>
                  </FormControl>
                )}
                <Button
                  alignSelf="flex-start"
                  size="xs"
                  variant="ghost"
                  isDisabled={busy}
                  onClick={() => setShowAdvancedTokens((value) => !value)}
                >
                  {t(
                    showAdvancedTokens
                      ? "FederationPage.points.advancedHide"
                      : "FederationPage.points.advancedShow"
                  )}
                </Button>
                {showAdvancedTokens && (
                  <FormControl
                    isInvalid={
                      customOutputLength &&
                      maxTokensText !== "" &&
                      !customValidTokens
                    }
                  >
                    <FormLabel htmlFor="points-max-tokens">
                      {t("FederationPage.points.maxTokens")}
                    </FormLabel>
                    <Input
                      id="points-max-tokens"
                      inputMode="numeric"
                      value={maxTokensText}
                      isDisabled={busy}
                      onChange={(event) => {
                        const next = event.target.value;
                        setMaxTokensText(next);
                        setCustomOutputLength(next.trim() !== "");
                      }}
                    />
                    <FormHelperText>
                      {nativeMaxOutput === null
                        ? t("FederationPage.points.maxTokensHelp")
                        : t("FederationPage.points.maxTokensNativeHelp", {
                            max: nativeMaxOutput.toString(),
                          })}
                    </FormHelperText>
                  </FormControl>
                )}
              </>
            )}
            <HStack justify="flex-end">
              <Button onClick={() => setStep(0)}>
                {t("FederationPage.points.back")}
              </Button>
              <Button
                colorScheme={primaryColor}
                isDisabled={
                  !id ||
                  !worker ||
                  !model ||
                  !validTokens ||
                  !quote ||
                  !maxPoints
                }
                onClick={() => setStep(2)}
              >
                {t("FederationPage.points.next")}
              </Button>
            </HStack>
          </>
        )}
        {step === 2 && quote && maxPoints !== null && (
          <>
            <AdvancedCard level="back" p={3}>
              <VStack align="stretch" spacing={1}>
                <Text fontWeight="semibold">
                  {t("FederationPage.points.reviewMaxPoints", {
                    maxPoints: maxPoints.toString(),
                  })}
                </Text>
                <Text fontSize="sm">
                  {t("FederationPage.points.reviewPayer", {
                    payer: `${points.account.club} · ${points.account.member}`,
                  })}
                </Text>
                <Text fontSize="sm">
                  {t("FederationPage.points.available", {
                    available: points.available_points,
                  })}
                </Text>
                <Text fontSize="xs" className="secondary-text">
                  {t("FederationPage.points.reviewWorker", { worker, model })}
                </Text>
                <Text fontSize="xs" className="secondary-text">
                  {t("FederationPage.points.quote", {
                    base: price?.base_points,
                    variable: (
                      quote - BigInt(price?.base_points || 0)
                    ).toString(),
                    total: quote.toString(),
                  })}
                </Text>
              </VStack>
            </AdvancedCard>
            {insufficient && (
              <Text role="alert" color="red.600" fontSize="sm">
                {t("FederationPage.points.insufficient")}
              </Text>
            )}
            <Text fontSize="xs" className="secondary-text">
              {t("FederationPage.points.automaticBudget")}
            </Text>
            {error && (
              <Text role="alert" color="red.600" fontSize="sm">
                {error}
              </Text>
            )}
            <HStack justify="flex-end">
              <Button onClick={() => setStep(1)}>
                {t("FederationPage.points.back")}
              </Button>
              <Button
                colorScheme={primaryColor}
                isLoading={busy}
                isDisabled={!id || insufficient || !maxPoints}
                onClick={confirm.onOpen}
              >
                {t("FederationPage.points.confirm")}
              </Button>
            </HStack>
          </>
        )}
      </VStack>
      <AlertDialog
        isOpen={confirm.isOpen}
        leastDestructiveRef={cancelRef}
        onClose={confirm.onClose}
        isCentered
      >
        <AlertDialogOverlay>
          <AlertDialogContent>
            <AlertDialogHeader>
              {t("FederationPage.points.confirmTitle")}
            </AlertDialogHeader>
            <AlertDialogBody>
              {t("FederationPage.points.confirmBody", {
                maxPoints: maxPoints?.toString() || "0",
              })}
            </AlertDialogBody>
            <AlertDialogFooter>
              <Button ref={cancelRef} onClick={confirm.onClose}>
                {t("General.cancel")}
              </Button>
              <Button
                colorScheme={primaryColor}
                ml={3}
                isLoading={busy}
                onClick={() => {
                  confirm.onClose();
                  void submit();
                }}
              >
                {t("FederationPage.points.confirmAction")}
              </Button>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialogOverlay>
      </AlertDialog>
    </AdvancedCard>
  );
}

function priceLabel(model: string, points: PointPresentation): string {
  const price = points.prices[model];
  if (!price) return model;
  return `${model} · ${price.base_points}/${price.input_points_per_1k}/${price.output_points_per_1k}`;
}

function BudgetAuthorization({
  points,
  state,
  member,
  isAdmin,
}: {
  points: PointPresentation;
  state: UnionState | undefined;
  member: string | undefined;
  isAdmin: boolean;
}) {
  const { t } = useTranslation();
  const toast = useToast();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const { busy, error, handleRequest } = useApplicationRequest();
  const scopes = budgetScopes(state, member, isAdmin);
  const [scope, setScope] = useState("");
  const [budgetText, setBudgetText] = useState("");
  const activeScope = scopes.find(
    (item) => `${item.kind}:${item.id}` === scope
  );
  const budget = parseAmount(budgetText);
  const available = parseAmount(points.available_points);
  const withinAvailable =
    budget !== null && available !== null && budget <= available;
  const safeBudget = budget ? exactNumber(budget) : null;

  const submit = async () => {
    if (!activeScope || safeBudget === null || !withinAvailable) return;
    const accepted = await handleRequest({
      type: "organization.authorize_ai",
      scope: `${activeScope.kind}:${activeScope.id}`,
      budget: safeBudget,
    });
    if (!accepted) return;
    setBudgetText("");
    toast({
      title: t("FederationPage.points.budgetAccepted"),
      status: "success",
    });
  };

  return (
    <AdvancedCard level="front" p={4}>
      <VStack align="stretch" spacing={3}>
        <Text fontWeight="semibold">
          {t("FederationPage.points.budgetTitle")}
        </Text>
        <Text fontSize="sm" className="secondary-text">
          {t("FederationPage.points.budgetHelp", {
            club: points.account.club,
            member: points.account.member,
            available: points.available_points,
          })}
        </Text>
        {scopes.length === 0 ? (
          <Empty
            size="sm"
            description={t("FederationPage.points.budgetEmpty")}
          />
        ) : (
          <>
            <FormControl>
              <FormLabel htmlFor="points-budget-scope">
                {t("FederationPage.points.budgetScope")}
              </FormLabel>
              <Select
                id="points-budget-scope"
                value={scope}
                isDisabled={busy}
                onChange={(event) => setScope(event.target.value)}
              >
                <option value="">
                  {t("FederationPage.points.budgetChoose")}
                </option>
                {scopes.map((item) => (
                  <option
                    key={`${item.kind}:${item.id}`}
                    value={`${item.kind}:${item.id}`}
                  >
                    {t(
                      item.kind === "event"
                        ? "FederationPage.points.budgetEvent"
                        : "FederationPage.points.budgetReport",
                      { title: item.title, club: item.club }
                    )}
                  </option>
                ))}
              </Select>
            </FormControl>
            <FormControl isInvalid={budgetText !== "" && !withinAvailable}>
              <FormLabel htmlFor="points-budget-amount">
                {t("FederationPage.points.budgetAmount")}
              </FormLabel>
              <Input
                id="points-budget-amount"
                inputMode="numeric"
                value={budgetText}
                isDisabled={busy}
                onChange={(event) => setBudgetText(event.target.value)}
              />
              <FormHelperText>
                {t("FederationPage.points.budgetLimit", {
                  available: points.available_points,
                })}
              </FormHelperText>
            </FormControl>
            {error && (
              <Text role="alert" color="red.600" fontSize="sm">
                {error}
              </Text>
            )}
            <Button
              alignSelf="flex-start"
              colorScheme={primaryColor}
              isLoading={busy}
              isDisabled={!activeScope || !safeBudget || !withinAvailable}
              onClick={() => void submit()}
            >
              {t("FederationPage.points.budgetSubmit")}
            </Button>
          </>
        )}
      </VStack>
    </AdvancedCard>
  );
}

function AiResultDialog({
  state,
  row,
  onClose,
}: {
  state: UnionState;
  row: PointPresentation["ai"][number] | null;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [failed, setFailed] = useState(false);
  const [output, setOutput] = useState("");
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    if (!row) return;
    let cancelled = false;
    setLoading(true);
    setFailed(false);
    setOutput("");
    UnionService.pointOutput(state, row.id, row.stage)
      .then((text) => {
        if (!cancelled) setOutput(text);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [attempt, row, state]);

  if (!row) return null;
  return (
    <Modal isOpen onClose={onClose} isCentered size="xl">
      <ModalOverlay />
      <ModalContent>
        <ModalHeader>
          {t("FederationPage.points.resultTitle", {
            model: row.model || row.id,
            stage: row.stage + 1,
          })}
        </ModalHeader>
        <ModalCloseButton />
        <ModalBody>
          {loading ? (
            <HStack spacing={3}>
              <Spinner />
              <Text fontSize="sm">
                {t("FederationPage.points.resultLoading")}
              </Text>
            </HStack>
          ) : failed ? (
            <VStack align="stretch" spacing={3}>
              <Text role="alert" color="red.600" fontSize="sm">
                {t("FederationPage.points.resultFailed")}
              </Text>
              <Button
                alignSelf="flex-start"
                size="sm"
                onClick={() => setAttempt((value) => value + 1)}
              >
                {t("FederationPage.points.resultRetry")}
              </Button>
            </VStack>
          ) : (
            <Text
              as="pre"
              fontFamily="var(--font-mono)"
              fontSize="sm"
              whiteSpace="pre-wrap"
              wordBreak="break-word"
            >
              {output}
            </Text>
          )}
        </ModalBody>
        <ModalFooter>
          <Button onClick={onClose}>{t("General.close")}</Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}

export function PointsPanel() {
  const { t } = useTranslation();
  const { state, member, isAdmin } = useFederation();
  const points = state?.points_view;
  const activity = points?.activity || [];
  const ai = points?.ai || [];
  const [tab, setTab] = useState<"activity" | "ai">("activity");
  const [activityPage, setActivityPage] = useState(0);
  const [aiPage, setAiPage] = useState(0);
  const [resultRow, setResultRow] = useState<
    PointPresentation["ai"][number] | null
  >(null);
  const accountKey = points
    ? `${points.account.club}:${points.account.member}`
    : "";

  useEffect(() => {
    setActivityPage((value) => Math.min(value, pageCount(activity.length) - 1));
  }, [activity.length]);
  useEffect(() => {
    setAiPage((value) => Math.min(value, pageCount(ai.length) - 1));
  }, [ai.length]);
  // A different points owner invalidates any retry or output dialog from the old account.
  useEffect(() => {
    setResultRow(null);
  }, [accountKey]);

  if (!state || !points) return null;
  const activityPages = pageCount(activity.length);
  const aiPages = pageCount(ai.length);
  const safeActivityPage = Math.min(activityPage, activityPages - 1);
  const safeAiPage = Math.min(aiPage, aiPages - 1);
  const visibleActivity = pageItems(activity, safeActivityPage);
  const visibleAi = pageItems(ai, safeAiPage);
  const currentPage = tab === "activity" ? safeActivityPage : safeAiPage;
  const pages = tab === "activity" ? activityPages : aiPages;

  return (
    <Section title={t("FederationPage.points.title")} initialIsOpen>
      <VStack align="stretch" spacing={3}>
        <AdvancedCard level="front" p={4}>
          <VStack align="stretch" spacing={2}>
            <HStack justify="space-between" wrap="wrap">
              <VStack align="start" spacing={0}>
                <Text fontWeight="semibold">
                  {t("FederationPage.points.balance")}
                </Text>
                <Text fontSize="xs" className="secondary-text">
                  {t("FederationPage.points.account", {
                    club: points.account.club,
                    member: points.account.member,
                  })}
                </Text>
              </VStack>
              <Tag colorScheme="gray">
                {t("FederationPage.points.revision", {
                  revision: points.revision,
                })}
              </Tag>
            </HStack>
            <HStack spacing={6} wrap="wrap">
              <Text fontSize="sm">
                {t("FederationPage.points.available", {
                  available: points.available_points,
                })}
              </Text>
              <Text fontSize="sm">
                {t("FederationPage.points.frozen", {
                  frozen: points.frozen_points,
                })}
              </Text>
            </HStack>
            {points.daily_member_cap ? (
              <VStack align="stretch" spacing={1}>
                <Text fontSize="sm">
                  {t("FederationPage.points.today", {
                    earned: points.earned_today,
                    cap: points.daily_member_cap,
                  })}
                </Text>
                <Progress
                  value={progressValue(points)}
                  aria-label={t("FederationPage.points.todayProgress")}
                />
                <Text fontSize="xs" className="secondary-text">
                  {t("FederationPage.points.slices", {
                    active: points.active_slices,
                    afk: points.afk_slices,
                  })}
                </Text>
              </VStack>
            ) : (
              <Text fontSize="sm">
                {t("FederationPage.points.todayNoCap", {
                  earned: points.earned_today,
                })}
              </Text>
            )}
            <Text fontSize="xs" className="secondary-text">
              {t("FederationPage.points.resetAt", {
                time: formatTime(points.day_ends_ms),
              })}
            </Text>
            <Text fontSize="xs" className="secondary-text">
              {t("FederationPage.points.asOf", {
                time: formatTime(points.as_of_ms),
              })}
            </Text>
            <StatementExport points={points} />
          </VStack>
        </AdvancedCard>
        <Section
          title={t("FederationPage.points.historyTitle")}
          isAccordion
          initialIsOpen={false}
        >
          <AdvancedCard level="back" p={4}>
            <VStack align="stretch" spacing={2}>
              <HStack spacing={2}>
                <Button
                  size="sm"
                  variant={tab === "activity" ? "solid" : "ghost"}
                  onClick={() => setTab("activity")}
                >
                  {t("FederationPage.points.activityTitle")}
                </Button>
                <Button
                  size="sm"
                  variant={tab === "ai" ? "solid" : "ghost"}
                  onClick={() => setTab("ai")}
                >
                  {t("FederationPage.points.aiTitle")}
                </Button>
              </HStack>
              {tab === "activity" ? (
                activity.length === 0 ? (
                  <Empty
                    size="sm"
                    description={t("FederationPage.points.activityEmpty")}
                  />
                ) : (
                  visibleActivity.map((row, index) => (
                    <VStack key={row.id} align="stretch" spacing={1}>
                      {index > 0 && <Divider />}
                      <HStack justify="space-between">
                        <Text fontSize="sm">{row.source}</Text>
                        <Text fontSize="sm">
                          {t("FederationPage.points.pointsValue", {
                            points: row.points,
                          })}
                        </Text>
                      </HStack>
                      <Text fontSize="xs" className="secondary-text">
                        {t("FederationPage.points.activityRow", {
                          active: row.active_slices,
                          afk: row.afk_slices,
                          time: formatTime(row.credited_ms),
                        })}
                      </Text>
                    </VStack>
                  ))
                )
              ) : ai.length === 0 ? (
                <Empty
                  size="sm"
                  description={t("FederationPage.points.aiEmpty")}
                />
              ) : (
                visibleAi.map((row, index) => (
                  <VStack
                    key={`${accountKey}:${row.id}:${row.stage}`}
                    align="stretch"
                    spacing={1}
                  >
                    {index > 0 && <Divider />}
                    <HStack justify="space-between" wrap="wrap">
                      <Text fontSize="sm">{row.model || row.id}</Text>
                      <Tag colorScheme="gray">{t(aiStatusKey(row.status))}</Tag>
                    </HStack>
                    <Text fontSize="xs" className="secondary-text">
                      {t("FederationPage.points.aiStagePurpose", {
                        stage: row.stage + 1,
                        purpose: t(aiPurposeKey(row.purpose)),
                      })}
                    </Text>
                    <Text fontSize="xs" className="secondary-text">
                      {t("FederationPage.points.aiRow", {
                        frozen: row.frozen_points,
                        charged: row.charged_points,
                        refunded: row.refunded_points,
                        time: formatTime(row.started_ms),
                      })}
                    </Text>
                    {row.output && (
                      <Button
                        alignSelf="flex-start"
                        size="xs"
                        variant="outline"
                        onClick={() => setResultRow(row)}
                      >
                        {t("FederationPage.points.openResult")}
                      </Button>
                    )}
                    <AiFlowActions state={state} points={points} row={row} />
                  </VStack>
                ))
              )}
              {pages > 1 && (
                <HStack justify="space-between" align="center">
                  <Button
                    size="sm"
                    variant="ghost"
                    isDisabled={currentPage <= 0}
                    onClick={() =>
                      tab === "activity"
                        ? setActivityPage(safeActivityPage - 1)
                        : setAiPage(safeAiPage - 1)
                    }
                  >
                    {t("FederationPage.points.previousPage")}
                  </Button>
                  <Text fontSize="xs" className="secondary-text">
                    {t("FederationPage.points.page", {
                      page: currentPage + 1,
                      pages,
                    })}
                  </Text>
                  <Button
                    size="sm"
                    variant="ghost"
                    isDisabled={currentPage + 1 >= pages}
                    onClick={() =>
                      tab === "activity"
                        ? setActivityPage(safeActivityPage + 1)
                        : setAiPage(safeAiPage + 1)
                    }
                  >
                    {t("FederationPage.points.nextPage")}
                  </Button>
                </HStack>
              )}
            </VStack>
          </AdvancedCard>
        </Section>
        <RequestAi points={points} />
        <BudgetAuthorization
          points={points}
          state={state}
          member={member}
          isAdmin={isAdmin}
        />
        {resultRow && state ? (
          <AiResultDialog
            state={state}
            row={resultRow}
            onClose={() => setResultRow(null)}
          />
        ) : null}
      </VStack>
    </Section>
  );
}
