export type FederationErrorKind =
  "network" | "membership" | "invalid" | "generic";

// Codes come from the native core's stable ClientErrorCode set; messages are diagnostic only.
const KIND_BY_CODE: Record<string, FederationErrorKind> = {
  network: "network",
  timeout: "network",
  identity_unavailable: "membership",
  authority_unavailable: "membership",
  access_denied: "membership",
  invalid_request: "invalid",
  not_found: "invalid",
};

export function classifyFederationError(error: unknown): FederationErrorKind {
  const code =
    typeof error === "object" && error !== null && "code" in error
      ? error.code
      : undefined;
  return (typeof code === "string" && KIND_BY_CODE[code]) || "generic";
}

// Generic failures keep the original combined message.
export function federationErrorKey(kind: FederationErrorKind): string {
  return kind === "generic"
    ? "FederationPage.error"
    : `FederationPage.errors.${kind}`;
}
