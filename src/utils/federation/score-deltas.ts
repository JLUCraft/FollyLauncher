// Reject malformed, fractional, unsafe or duplicate deltas as a whole.
export function parseDeltas(input: string): Record<string, number> | null {
  const deltas: Record<string, number> = Object.create(null);
  for (const line of input.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const match = /^(.*?)\s+(-?\d+)$/.exec(trimmed);
    if (
      !match ||
      !match[1] ||
      !Number.isSafeInteger(Number(match[2])) ||
      Object.hasOwn(deltas, match[1])
    )
      return null;
    deltas[match[1]] = Number(match[2]);
  }
  return deltas;
}
