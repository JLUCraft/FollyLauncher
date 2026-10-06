import { Text, VStack } from "@chakra-ui/react";
import { useTranslation } from "react-i18next";
import { Section } from "@/components/common/section";
import { encodeHex } from "@/utils/federation/hex";

// All fields remain visible during signature review, including unfamiliar future actions.
export function RecordFields({ value }: { value: unknown }) {
  const { t } = useTranslation();
  if (value === null || value === undefined) return <Text>—</Text>;
  if (typeof value !== "object")
    return <Text overflowWrap="anywhere">{String(value)}</Text>;
  if (Array.isArray(value) && value.every((v) => typeof v === "number"))
    return <Text overflowWrap="anywhere">{encodeHex(value)}</Text>;
  return (
    <VStack align="stretch" pl={3} borderLeftWidth="1px">
      {Object.entries(value).map(([key, entry]) => {
        const labelKey = `IdentityFlow.fields.${key}`;
        const label = t(labelKey);
        return (
          <Section
            key={key}
            title={label === labelKey ? t("IdentityFlow.unnamedField") : label}
          >
            <RecordFields value={entry} />
          </Section>
        );
      })}
    </VStack>
  );
}
