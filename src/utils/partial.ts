// Prevent prototype pollution by blocking dangerous keys
// ref: https://github.com/UNIkeEN/SJMCL/security/code-scanning/4
const isUnsafeKey = (key: string): boolean =>
  ["__proto__", "constructor", "prototype"].includes(key);

export const updateByKeyPath = (
  obj: object,
  path: string,
  value: unknown
): void => {
  const keys = path.split(".");
  let current = obj as Record<string, unknown>;

  for (let i = 0; i < keys.length - 1; i++) {
    const key = keys[i];
    if (isUnsafeKey(key)) return;

    if (!(key in current)) {
      current[key] = {};
    }
    const next = current[key];
    if (next === null || typeof next !== "object") return;
    current = next as Record<string, unknown>;
  }

  const lastKey = keys[keys.length - 1];
  if (isUnsafeKey(lastKey)) return;

  current[lastKey] = value;
};
