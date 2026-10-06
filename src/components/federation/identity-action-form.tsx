import {
  Button,
  Checkbox,
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
import { UnionService } from "@/services/union";

type Field = {
  name: string;
  kind: "text" | "peer" | "profile" | "number" | "choices" | "chairs";
  optional: boolean;
  generated: boolean;
  choices: string[];
};
type ActionForm = { action: string; fields: Field[] };

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
                ) : field.kind === "choices" ? (
                  <HStack flexWrap="wrap">
                    {field.choices.map((choice) => {
                      const current = (values[field.name] as string[]) || [];
                      return (
                        <Checkbox
                          key={choice}
                          isDisabled={locked}
                          isChecked={current.includes(choice)}
                          onChange={(e) =>
                            update(
                              field.name,
                              e.target.checked
                                ? [...current, choice]
                                : current.filter((v) => v !== choice)
                            )
                          }
                        >
                          {t(`IdentityManagement.evidence.${choice}`)}
                        </Checkbox>
                      );
                    })}
                  </HStack>
                ) : (
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
