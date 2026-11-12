import type { CardProps } from "@chakra-ui/react";

export function insetCardPadding(
  padding: NonNullable<CardProps["padding"]>,
  borderWidth: string,
  spacing: Record<string, string>
): CardProps["padding"] {
  const inset = (value: string | number) =>
    `calc(${spacing[value] ?? value} - ${borderWidth})`;
  if (Array.isArray(padding))
    return padding.map((value: string | number | null) =>
      value === null ? null : inset(value)
    );
  if (typeof padding === "object")
    return Object.fromEntries(
      Object.entries(padding).map(([key, value]) => [
        key,
        value === undefined ? undefined : inset(value),
      ])
    );
  return inset(padding);
}
