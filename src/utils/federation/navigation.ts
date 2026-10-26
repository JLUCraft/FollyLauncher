export const FEDERATION_ROOMS_PATH = "/instances/rooms";
export const FEDERATION_ACTIVITIES_PATH = "/federation/activities";
export const FEDERATION_SETTINGS_PATH = "/settings/federation";
export const FEDERATION_MANAGEMENT_SETTINGS_PATH =
  "/settings/federation-management";

const PLAY_PATHS = new Set([
  FEDERATION_ROOMS_PATH,
  FEDERATION_ACTIVITIES_PATH,
  `${FEDERATION_ACTIVITIES_PATH}/event`,
  `${FEDERATION_ACTIVITIES_PATH}/competition`,
]);

export function federationReturnPath(value: unknown): string {
  if (typeof value !== "string") return FEDERATION_ROOMS_PATH;
  const pathname = value.split(/[?#]/)[0];
  return PLAY_PATHS.has(pathname) ? value : FEDERATION_ROOMS_PATH;
}

export function federationSettingsLink(
  path: string,
  returnTo: unknown
): string {
  return `${path}?${new URLSearchParams({ returnTo: federationReturnPath(returnTo) })}`;
}
