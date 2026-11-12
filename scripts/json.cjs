/** @param {unknown} value @returns {value is Record<string, unknown>} */
function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** @param {string} text @returns {Record<string, unknown>} */
function parseRecord(text) {
  return objectValue(parseValue(text));
}

/** @param {string} text @returns {unknown} */
function parseValue(text) {
  /** @type {unknown} */
  const value = JSON.parse(text);
  return value;
}

/** @param {unknown} value @returns {Record<string, unknown>} */
function objectValue(value) {
  if (!isRecord(value)) throw new Error("Expected a JSON object");
  return value;
}

module.exports = {
  isRecord,
  parseRecord,
};
