export function omitFields<Value extends object, Key extends keyof Value>(
  value: Value,
  keys: readonly Key[]
): Omit<Value, Key> {
  const result = { ...value };
  for (const key of keys) Reflect.deleteProperty(result, key);
  return result;
}
