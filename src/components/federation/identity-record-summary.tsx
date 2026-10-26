import { Text, VStack } from "@chakra-ui/react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { RecordFields } from "@/components/federation/record-fields";
import { encodeHex } from "@/utils/federation/hex";

function translatedValue(
  t: TFunction,
  field: string,
  value: string
): string | null {
  let key = "";
  if (field === "evidence") key = `IdentityManagement.evidence.${value}`;
  else if (field === "permission") key = `IdentityFlow.permissions.${value}`;
  else if (field === "service") key = `IdentityFlow.services.${value}`;
  if (!key) return null;
  const translated = t(key);
  return translated === key ? null : translated;
}

function SummaryValue({ field, value }: { field: string; value: unknown }) {
  const { t } = useTranslation();
  if (value === null || value === undefined) return <Text>—</Text>;
  if (typeof value === "number" || typeof value === "boolean")
    return <Text>{String(value)}</Text>;
  if (typeof value === "string") {
    return (
      <Text overflowWrap="anywhere">
        {translatedValue(t, field, value) ?? value}
      </Text>
    );
  }
  if (Array.isArray(value)) {
    if (value.every((entry) => typeof entry === "number"))
      return (
        <Text overflowWrap="anywhere">{encodeHex(value as number[])}</Text>
      );
    return (
      <VStack align="stretch" spacing={1}>
        {value.map((entry) => (
          <SummaryValue
            key={`${field}-${JSON.stringify(entry)}`}
            field={field}
            value={entry}
          />
        ))}
      </VStack>
    );
  }
  if (typeof value === "object") {
    const record = value as Record<string, unknown>;
    if (field === "chairs")
      return (
        <VStack align="stretch" spacing={1}>
          {Object.entries(record).map(([club, member]) => (
            <Text key={club} overflowWrap="anywhere">
              {club} / {String(member)}
            </Text>
          ))}
        </VStack>
      );
    return <RecordFields value={record} />;
  }
  return <Text overflowWrap="anywhere">{String(value)}</Text>;
}

/** Readable signing review: the action, every affected field and the current signature count. */
export function IdentityRecordSummary({ value }: { value: unknown }) {
  const { t } = useTranslation();
  if (!value || typeof value !== "object" || Array.isArray(value))
    return <RecordFields value={value} />;
  const record = value as Record<string, unknown>;
  const action =
    record.action &&
    typeof record.action === "object" &&
    !Array.isArray(record.action)
      ? (record.action as Record<string, unknown>)
      : undefined;
  if (!action) return <RecordFields value={record} />;
  const type = typeof action.type === "string" ? action.type : "";
  const actionKey = `IdentityManagement.actions.${type}`;
  const actionLabel = type ? t(actionKey) : t("IdentityFlow.unknown");
  const signatures = Array.isArray(record.signatures)
    ? record.signatures.length
    : 0;
  return (
    <VStack align="stretch" spacing={3}>
      <Text fontWeight="semibold">
        {!type || actionLabel === actionKey
          ? t("IdentityFlow.unknownAction", {
              type: type || t("IdentityFlow.unknown"),
            })
          : actionLabel}
      </Text>
      {Object.entries(action)
        .filter(([name]) => name !== "type")
        .map(([name, entry]) => {
          const fieldKey = `IdentityFlow.fields.${name}`;
          const fieldLabel = t(fieldKey);
          return (
            <VStack key={name} align="stretch" spacing={1}>
              <Text fontSize="sm" className="secondary-text">
                {fieldLabel === fieldKey ? name : fieldLabel}
              </Text>
              <SummaryValue field={name} value={entry} />
            </VStack>
          );
        })}
      <Text fontSize="sm" className="secondary-text">
        {t("IdentityFlow.signatureCount", { count: signatures })}
      </Text>
    </VStack>
  );
}
