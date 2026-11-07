import {
  Button,
  FormControl,
  FormHelperText,
  FormLabel,
  HStack,
  Input,
  Select,
  Text,
  VStack,
} from "@chakra-ui/react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Section } from "@/components/common/section";
import { GAME_PROFILE, UnionService } from "@/services/union";

type Field = {
  name: string;
  kind: "text" | "peer" | "number" | "identifiers" | "chairs";
  optional: boolean;
  generated: boolean;
};
type ActionForm = { action: string; fields: Field[] };

const FIELD_SUGGESTIONS: Record<string, string[]> = {
  service: ["application"],
  permission: ["read", "write", "admin", "use", "manage"],
  kind: [GAME_PROFILE],
  evidence: ["institutional_email", GAME_PROFILE],
};

export function IdentityActionForm({
  disabled,
  onDraft,
}: {
  disabled: boolean;
  onDraft: (action: unknown) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [forms, setForms] = useState<ActionForm[]>([]);
  const [selected, setSelected] = useState("");
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [failed, setFailed] = useState(false);
  const [loading, setLoading] = useState(true);
  const [building, setBuilding] = useState(false);
  useEffect(() => {
    let active = true;
    void UnionService.management({ operation: "forms" })
      .then((result) => {
        if (!active) return;
        const forms = result as ActionForm[];
        setForms(forms);
        setSelected(forms[0]?.action || "");
      })
      .catch(() => {
        if (active) setFailed(true);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);
  const update = (name: string, value: unknown) => {
    setValues((values) => ({ ...values, [name]: value }));
    setFailed(false);
  };
  const build = async () => {
    setBuilding(true);
    setFailed(false);
    try {
      const action = await UnionService.management({
        operation: "build_action",
        action: selected,
        fields: values,
      });
      await onDraft(action);
    } catch {
      setFailed(true);
    } finally {
      setBuilding(false);
    }
  };
  const form = forms.find((form) => form.action === selected);
  const locked = disabled || building;
  const suggestionLabel = (field: string, value: string) => {
    const key =
      field === "service"
        ? `IdentityFlow.services.${value}`
        : field === "permission"
          ? `IdentityFlow.permissions.${value}`
          : field === "kind"
            ? `IdentityFlow.attributeKinds.${value}`
            : field === "evidence"
              ? `IdentityManagement.evidence.${value}`
              : "";
    if (!key) return value;
    const translated = t(key);
    return translated === key ? value : translated;
  };
  return (
    <Section
      title={t("IdentityManagement.title")}
      description={t("IdentityManagement.organizerHint")}
      isAccordion
      initialIsOpen={false}
    >
      <VStack align="stretch" spacing={3}>
        {loading && (
          <Text role="status">{t("FederationPage.identity.loading")}</Text>
        )}
        {failed && <Text role="alert">{t("IdentityManagement.invalid")}</Text>}
        {!!forms.length && (
          <>
            <FormControl>
              <FormLabel htmlFor="identity-action-kind">
                {t("IdentityManagement.action")}
              </FormLabel>
              <Select
                id="identity-action-kind"
                value={selected}
                isDisabled={locked}
                onChange={(e) => {
                  setSelected(e.target.value);
                  setValues({});
                  setFailed(false);
                }}
              >
                {forms.map((form) => (
                  <option key={form.action} value={form.action}>
                    {t(`IdentityManagement.actions.${form.action}`)}
                  </option>
                ))}
              </Select>
            </FormControl>
            {form?.fields.map((field) => (
              <FormControl key={`${selected}/${field.name}`}>
                <FormLabel htmlFor={`identity-action-${field.name}`}>
                  {t(`IdentityManagement.fields.${field.name}`)}
                </FormLabel>
                {field.kind === "chairs" ? (
                  <ChairFields
                    disabled={locked}
                    onChange={(value) => update(field.name, value)}
                  />
                ) : field.kind === "identifiers" ? (
                  <IdentifierFields
                    id={`identity-action-${field.name}`}
                    disabled={locked}
                    value={(values[field.name] as string[]) || []}
                    suggestions={FIELD_SUGGESTIONS[field.name] || []}
                    label={(value) => suggestionLabel(field.name, value)}
                    onChange={(value) => update(field.name, value)}
                  />
                ) : (
                  <>
                    <Input
                      id={`identity-action-${field.name}`}
                      isDisabled={locked}
                      type={field.kind === "number" ? "number" : "text"}
                      min={field.kind === "number" ? 1 : undefined}
                      value={String(values[field.name] ?? "")}
                      onChange={(e) =>
                        update(
                          field.name,
                          field.kind === "number" && e.target.value !== ""
                            ? Number(e.target.value)
                            : e.target.value
                        )
                      }
                    />
                    {FIELD_SUGGESTIONS[field.name] && (
                      <HStack flexWrap="wrap">
                        {FIELD_SUGGESTIONS[field.name].map((suggestion) => (
                          <Button
                            key={suggestion}
                            size="xs"
                            variant={
                              String(values[field.name] ?? "") === suggestion
                                ? "solid"
                                : "outline"
                            }
                            isDisabled={locked}
                            onClick={() => update(field.name, suggestion)}
                          >
                            {suggestionLabel(field.name, suggestion)}
                          </Button>
                        ))}
                      </HStack>
                    )}
                  </>
                )}
                {field.generated && (
                  <Button
                    isDisabled={locked}
                    onClick={() => {
                      setBuilding(true);
                      void UnionService.management({
                        operation: "new_service_id",
                      })
                        .then((value) => update(field.name, value))
                        .catch(() => setFailed(true))
                        .finally(() => setBuilding(false));
                    }}
                  >
                    {t("IdentityManagement.generateId")}
                  </Button>
                )}
                {field.optional && (
                  <FormHelperText>
                    {t("IdentityManagement.optional")}
                  </FormHelperText>
                )}
                {field.name === "value" && values.kind === GAME_PROFILE && (
                  <FormHelperText>
                    {t("IdentityManagement.gameProfileHint")}
                  </FormHelperText>
                )}
              </FormControl>
            ))}
            <Button
              isDisabled={locked || !form}
              isLoading={building}
              onClick={() => void build()}
            >
              {t("IdentityFlow.review")}
            </Button>
          </>
        )}
      </VStack>
    </Section>
  );
}

function IdentifierFields({
  id,
  disabled,
  value,
  suggestions,
  label,
  onChange,
}: {
  id: string;
  disabled: boolean;
  value: string[];
  suggestions: string[];
  label: (value: string) => string;
  onChange: (value: string[]) => void;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState("");
  const options = [...new Set([...suggestions, ...value])];
  const toggle = (item: string) =>
    onChange(
      value.includes(item) ? value.filter((v) => v !== item) : [...value, item]
    );
  const add = () => {
    const item = draft.trim();
    if (!item) return;
    if (!value.includes(item)) onChange([...value, item]);
    setDraft("");
  };
  return (
    <VStack align="stretch" spacing={2}>
      <HStack flexWrap="wrap">
        {options.map((item) => (
          <Button
            key={item}
            size="xs"
            variant={value.includes(item) ? "solid" : "outline"}
            aria-pressed={value.includes(item)}
            isDisabled={disabled}
            onClick={() => toggle(item)}
          >
            {label(item)}
          </Button>
        ))}
      </HStack>
      <HStack>
        <Input
          id={id}
          isDisabled={disabled}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key !== "Enter") return;
            e.preventDefault();
            add();
          }}
        />
        <Button isDisabled={disabled || !draft.trim()} onClick={add}>
          {t("General.add")}
        </Button>
      </HStack>
    </VStack>
  );
}

type Chair = { id: string; club: string; member: string };
function ChairFields({
  disabled,
  onChange,
}: {
  disabled: boolean;
  onChange: (value: unknown) => void;
}) {
  const { t } = useTranslation();
  const [rows, setRows] = useState<Chair[]>([]);
  const change = (rows: Chair[]) => {
    setRows(rows);
    const clubs = rows.map((r) => r.club);
    onChange(
      rows.some((r) => !r.club.trim() || !r.member.trim()) ||
        new Set(clubs).size !== rows.length
        ? null
        : Object.fromEntries(rows.map((r) => [r.club, r.member]))
    );
  };
  const edit = (id: string, patch: Partial<Chair>) =>
    change(rows.map((r) => (r.id === id ? { ...r, ...patch } : r)));
  return (
    <VStack align="stretch" spacing={3}>
      {rows.map((row) => (
        <HStack key={row.id} align="start">
          <Input
            isDisabled={disabled}
            aria-label={t("IdentityManagement.fields.club")}
            placeholder={t("IdentityManagement.fields.club")}
            value={row.club}
            onChange={(e) => edit(row.id, { club: e.target.value })}
          />
          <Input
            isDisabled={disabled}
            aria-label={t("IdentityManagement.fields.member")}
            placeholder={t("IdentityManagement.fields.member")}
            value={row.member}
            onChange={(e) => edit(row.id, { member: e.target.value })}
          />
          <Button
            isDisabled={disabled}
            onClick={() => change(rows.filter((r) => r.id !== row.id))}
          >
            {t("IdentityManagement.removeChair")}
          </Button>
        </HStack>
      ))}
      <Button
        isDisabled={disabled}
        onClick={() =>
          change([...rows, { id: crypto.randomUUID(), club: "", member: "" }])
        }
      >
        {t("IdentityManagement.addChair")}
      </Button>
    </VStack>
  );
}
