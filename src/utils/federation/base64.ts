const ENCODE_CHUNK_BYTES = 16 * 1024;
export function encodeFile(bytes: Uint8Array): string {
  const chunks: string[] = [];
  for (let offset = 0; offset < bytes.length; offset += ENCODE_CHUNK_BYTES)
    chunks.push(
      String.fromCharCode(
        ...bytes.subarray(offset, offset + ENCODE_CHUNK_BYTES)
      )
    );
  return btoa(chunks.join(""));
}
export function decodeFile(value: string): Uint8Array<ArrayBuffer> {
  const text = atob(value);
  return Uint8Array.from(text, (char) => char.charCodeAt(0));
}
