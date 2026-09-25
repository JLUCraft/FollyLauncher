import { Button, useClipboard } from "@chakra-ui/react";
import { useTranslation } from "react-i18next";

const PREFIX_LENGTH = 8;
const SUFFIX_LENGTH = 6;
export function HashChip({ value }: { value: string }) {
  const { t } = useTranslation();
  const { onCopy, hasCopied } = useClipboard(value);
  if (!value) return null;
  const label =
    value.length > PREFIX_LENGTH + SUFFIX_LENGTH
      ? `${value.slice(0, PREFIX_LENGTH)}…${value.slice(-SUFFIX_LENGTH)}`
      : value;
  return (
    <Button
      size="sm"
      variant="outline"
      alignSelf="start"
      title={value}
      aria-label={`${t("IdentifierDisplay.copy")}: ${value}`}
      onClick={onCopy}
    >
      {hasCopied ? t("IdentifierDisplay.copied") : label}
    </Button>
  );
}
