const { readFileSync } = require("node:fs");
const { isRecord, parseArray } = require("../json.cjs");

/** @typedef {{status: "passed" | "failed", [key: string]: unknown}} BrowserResult */

/** @param {string} filePath @returns {BrowserResult[]} */
function loadResults(filePath) {
  return parseArray(readFileSync(filePath, "utf8")).map((value) => {
    if (
      !isRecord(value) ||
      (value.status !== "passed" && value.status !== "failed")
    )
      throw new Error("Invalid recorded browser result");
    return { ...value, status: value.status };
  });
}

module.exports = { loadResults };
