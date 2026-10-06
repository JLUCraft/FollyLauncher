import { Textarea } from "@chakra-ui/react";

export function JsonView({ label, value }: { label: string; value: unknown }) {
  return (
    <Textarea
      isReadOnly
      aria-label={label}
      value={JSON.stringify(value, null, 2)}
    />
  );
}
