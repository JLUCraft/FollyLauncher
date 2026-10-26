import { t } from "i18next";

type SupportedOsType = "linux" | "macos" | "windows";

/**
 * Normalize the OS identifier reported by `tauri-plugin-os` (or the launcher
 * config) to the identifiers used by the `Enums.<modifier>Key.<osType>` locale
 * entries. Returns `null` when the OS is not known yet (the default config has
 * an empty `osType`) or is not supported, so callers can omit the shortcut hint
 * instead of rendering an unresolved i18n key.
 */
export const normalizeOsType = (osType: string): SupportedOsType | null => {
  switch (osType?.toLowerCase()) {
    case "linux":
      return "linux";
    case "windows":
      return "windows";
    case "macos":
    case "darwin":
      return "macos";
    default:
      return null;
  }
};

/**
 * Locale label of the primary modifier key (Ctrl / Command) for the given OS.
 * Returns `null` for unknown operating systems.
 */
export const getModifierKeyLabel = (
  osType: string,
  options?: { short?: boolean }
): string | null => {
  const normalizedOsType = normalizeOsType(osType);
  if (!normalizedOsType) return null;

  const modifier = normalizedOsType === "macos" ? "metaKey" : "ctrlKey";
  const suffix = options?.short ? "Short" : "";
  return t(`Enums.${modifier}${suffix}.${normalizedOsType}`);
};
