#!/usr/bin/env node
/**
 * This script compares locale JSON files, checking for missing or extra keys
 * compared to a reference locale file.
 *
 * Usage:
 *   bun run locale diff <base_locale> <target_locales>
 *   e.g. bun run locale diff en zh-Hans
 *
 * Parameters:
 *   base_locale (optional) : str, defaults to zh-Hans, the base language
 *   which determines the reference locale file against which all other locale files will be compared.
 *   target_locales (optional) : str divided by spaces(can be multiple args)
 *   every target locale in target_locales(if not empty) will be compared with base_locale
 *   if target locale is empty, then all other locales will be checked.
 */

const fs = require("fs");
const path = require("path");
const chalk = require("chalk");
const { isRecord, parseRecord } = require("../json.cjs");

let base = "zh-Hans";
try {
  const arg = process.argv[3] || base;
  if (arg.includes("/") || arg.match(/\.json$/)) {
    base = (arg.split("/").pop() || base).replace(".json", "");
  } else {
    base = arg || "zh-Hans";
  }
} catch {
  base = "zh-Hans";
}

let targets = process.argv.slice(4);
if (targets !== null && targets.length !== 0) {
  for (let i = 0; i < targets.length; i++) {
    let target = targets[i];
    if (target.includes("/") || target.match(/\.json$/)) {
      target = (target.split("/").pop() || "").replace(".json", "");
    }
    targets[i] = target;
  }

  targets = targets.filter((target) => target !== base); // delete item same as base
  targets = [...new Set(targets)]; // delete duplicate item
  if (targets.length === 0) {
    console.log(
      "The target language is provided, but it is identical to the base language. Exiting..."
    );
    process.exit(0);
  }
}

/** @param {Record<string, unknown>} obj @param {string} [parentKey] @returns {string[]} */
function flattenDict(obj, parentKey = "") {
  const items = [];
  for (const key in obj) {
    if (Object.prototype.hasOwnProperty.call(obj, key)) {
      const value = obj[key];
      const newKey = parentKey ? `${parentKey}.${key}` : key;
      if (isRecord(value)) {
        items.push(...flattenDict(value, newKey));
      } else {
        items.push(newKey);
      }
    }
  }
  return items;
}

/** @param {string} filePath */
function loadLocaleFile(filePath) {
  const content = fs.readFileSync(filePath, "utf-8");
  return parseRecord(content);
}
/** @param {string[]} baseKeys @param {string[]} targetKeys */
function compareKeys(baseKeys, targetKeys) {
  const baseSet = new Set(baseKeys);
  const targetSet = new Set(targetKeys);
  const missing = [...baseSet]
    .filter((k) => !targetSet.has(k))
    .sort((left, right) => left.localeCompare(right));
  const extra = [...targetSet]
    .filter((k) => !baseSet.has(k))
    .sort((left, right) => left.localeCompare(right));
  return [missing, extra];
}
/** @param {Record<string, unknown>} locale @param {string} localeName */
function checkTodoValues(locale, localeName) {
  /** @type {string[]} */
  const todoValues = [];

  /** @param {Record<string, unknown>} obj @param {string} [parentKey] */
  function recursiveCheck(obj, parentKey = "") {
    for (const key in obj) {
      if (Object.prototype.hasOwnProperty.call(obj, key)) {
        const value = obj[key];
        const fullKey = parentKey ? `${parentKey}.${key}` : key;

        if (typeof value === "string" && value.startsWith("%TODO")) {
          todoValues.push(fullKey);
        } else if (isRecord(value)) {
          recursiveCheck(value, fullKey);
        }
      }
    }
  }

  recursiveCheck(locale);

  if (todoValues.length > 0) {
    console.log(
      chalk.hex("#FFA500")(
        `⚠️ Warning: '${localeName}.json' contains %TODO values:`
      )
    );
    todoValues.forEach((key) => {
      console.log(chalk.yellow(`  %TODO found: ${key}`));
    });
    throw new Error("  %TODO values detected");
  }
}

/** @param {string} base @param {string} target */
function compareLocales(base, target) {
  const localesPath = path.join(path.resolve(__dirname), `../../src/locales/`);

  const baseLocalePath = path.join(localesPath, `${base}.json`);
  const baseLocale = loadLocaleFile(baseLocalePath);
  const baseKeys = flattenDict(baseLocale).sort();

  const targetLocalePath = path.join(localesPath, `${target}.json`);
  if (!fs.existsSync(targetLocalePath)) {
    console.log(`Locale file '${target}.json' not found in: ${localesPath}`);
    return;
  }
  const targetLocale = loadLocaleFile(targetLocalePath);
  const targetKeys = flattenDict(targetLocale).sort();

  const [missing, extra] = compareKeys(baseKeys, targetKeys);

  if (missing.length === 0 && extra.length === 0) {
    console.log(
      chalk.green(`✅ '${target}.json' is identical to '${base}.json'.`)
    );
  } else {
    console.log(`Comparing ${target} to ${base}:`);
    console.log(`${missing.length} missing, ${extra.length} extra keys`);

    missing.forEach((key) => {
      console.log(chalk.red(`  Missing:   ${key}`));
    });
    extra.forEach((key) => {
      console.log(chalk.green(`  Extra:     ${key}`));
    });

    throw new Error("  Inconsistent locales detected");
  }

  checkTodoValues(targetLocale, target);

  console.log("-".repeat(40));
}

/** @param {string} baseLocaleName */
function compareAll(baseLocaleName) {
  const localesPath = path.join(path.resolve(__dirname), `../../src/locales/`);
  const baseLocalePath = path.join(localesPath, `${baseLocaleName}.json`);

  if (!fs.existsSync(baseLocalePath)) {
    console.log(
      `Locale file '${baseLocaleName}.json' not found in: ${localesPath}`
    );
    return;
  }

  fs.readdirSync(localesPath).forEach((fileName) => {
    if (fileName.endsWith(".json") && fileName !== `${baseLocaleName}.json`) {
      const targetLocaleName = fileName.replace(".json", "");
      compareLocales(baseLocaleName, targetLocaleName);
    }
  });
}

if (targets == null || targets.length === 0) {
  compareAll(base);
} else {
  targets.forEach((target) => compareLocales(base, target));
}
