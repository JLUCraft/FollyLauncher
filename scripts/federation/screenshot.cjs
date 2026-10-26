// Screenshot helper for the union surfaces. Not part of the app.
const {
  chromium,
} = require("/home/yizhou/Fughetta/admin/node_modules/playwright-core");
const { readFileSync } = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const mock = readFileSync(path.join(root, "out/federation-ui-mock.js"), "utf8");
const base = process.env.FEDERATION_TEST_URL || "http://localhost:3000";
const scenario = process.argv[2] || "disconnected";
const CHROMIUM =
  process.env.CHROMIUM_PATH ||
  `${process.env.HOME}/.cache/ms-playwright/chromium_headless_shell-1248/chrome-headless-shell-linux64/chrome-headless-shell`;

(async () => {
  const browser = await chromium.launch({
    executablePath: CHROMIUM,
    headless: true,
    args: ["--no-sandbox"],
  });
  const page = await browser.newPage({
    viewport: { width: 1280, height: 850 },
  });
  page.on("pageerror", (e) => console.error("PAGEERROR:", e.message));
  const init = (extra) => `${mock}\nwindow.testHandlers = { ${extra} };`;

  if (scenario === "disconnected") {
    await page.addInitScript({
      content: init(
        `union_identity_configuration: () => ({ genesis_hash: "a".repeat(64) })`
      ),
    });
    await page.goto(`${base}/settings/federation`);
    await page.waitForTimeout(2500);
  } else if (scenario === "no-identity") {
    await page.addInitScript({
      content: init(`union_identity_configuration: () => null`),
    });
    await page.goto(`${base}/settings/federation`);
    await page.waitForTimeout(2500);
  } else if (scenario === "connected") {
    await page.addInitScript({
      content: init(
        `union_identity_configuration: () => ({ genesis_hash: "a".repeat(64) })`
      ),
    });
    await page.goto(`${base}/settings/federation`);
    await page.waitForSelector("#federation-link");
    const link = `folly://federation?${new URLSearchParams({
      address: "/ip4/127.0.0.1/tcp/4001/p2p/test",
      peer: "test",
      genesis: "a".repeat(64),
    })}`;
    await page.locator("#federation-link").fill(link);
    await page.locator("#federation-link").press("Enter");
    await page.waitForTimeout(2500);
  } else if (scenario === "rooms") {
    await page.addInitScript({
      content: init(
        `union_identity_configuration: () => ({ genesis_hash: "a".repeat(64) })`
      ),
    });
    await page.goto(`${base}/instances/rooms`);
    await page.waitForTimeout(2500);
  }
  await page.screenshot({
    path: path.join(root, "out", `union-${scenario}.png`),
    fullPage: true,
  });
  console.log(`saved out/union-${scenario}.png`);
  await browser.close();
})();
