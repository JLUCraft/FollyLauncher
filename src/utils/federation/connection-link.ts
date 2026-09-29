export interface FederationTarget {
  address: string;
  peer: string;
  genesis: string;
}

export const GENESIS_PATTERN = /^[a-f0-9]{64}$/;
const LINK_HOST = "federation";

// `<scheme>://federation?address=…&peer=…&genesis=…` is what an organizer hands to students.
export function buildConnectionLink(
  target: FederationTarget,
  scheme: string
): string {
  const query = new URLSearchParams(
    target as unknown as Record<string, string>
  );
  return `${scheme}://${LINK_HOST}?${query}`;
}

export function parseConnectionLink(
  text: string,
  scheme: string
): FederationTarget | null {
  let url: URL;
  try {
    url = new URL(text.trim());
  } catch {
    return null;
  }
  if (url.protocol !== `${scheme}:` || url.hostname !== LINK_HOST) return null;
  const address = url.searchParams.get("address");
  const peer = url.searchParams.get("peer");
  const genesis = url.searchParams.get("genesis");
  if (!address || !peer || !genesis || !GENESIS_PATTERN.test(genesis))
    return null;
  return { address, peer, genesis };
}
