export function encodeHex(bytes: ArrayLike<number>): string {
  return Array.from(bytes, (n) => n.toString(16).padStart(2, "0")).join("");
}
