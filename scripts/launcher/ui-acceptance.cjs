// Requires the current Next development server; native behavior is checked separately.
const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const { mkdirSync, readFileSync, rmSync, writeFileSync } = require("node:fs");
const path = require("node:path");
const { chromium } = require(
  process.env.PLAYWRIGHT_MODULE || "playwright-core"
);
const repo = path.resolve(__dirname, "../..");
const output = path.resolve(
  process.env.LAUNCHER_TEST_OUTPUT ||
    path.join(repo, "out/launcher-ui-acceptance")
);
mkdirSync(output, { recursive: true });
const fixture = path.join(repo, "out/launcher-ui-acceptance-mock.js");
execFileSync("bun", [
  "build",
  path.join(__dirname, "mock-ipc.ts"),
  "--outfile",
  fixture,
  "--target",
  "browser",
  "--format",
  "iife",
]);
const locale = JSON.parse(
  readFileSync(path.join(repo, "src/locales/zh-Hans.json"))
);
const base = process.env.LAUNCHER_TEST_URL || "http://localhost:3000";
// Resume only after confirming that successful ranges have unchanged inputs.
const results =
  process.env.LAUNCHER_TEST_RESUME === "1"
    ? JSON.parse(readFileSync(path.join(output, "results.json")))
    : [];
const routes = [
  "/launch",
  "/accounts",
  "/instances/list",
  "/instances/add-import",
  "/downloads",
  "/settings/global-game",
  "/settings/global-game/advanced",
  "/settings/general",
  "/settings/appearance",
  "/settings/java",
  "/settings/download",
  "/settings/intelligence",
  "/settings/extension",
  "/settings/help",
  "/settings/about",
  "/settings/federation",
  "/settings/federation-management",
  "/instances/rooms",
  "/federation/activities",
];

function contrast(foreground, background) {
  const luminance = (color) => {
    const values = color
      .match(/[\d.]+/g)
      .slice(0, 3)
      .map(Number)
      .map((n) => {
        const s = n / 255;
        return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
      });
    return values[0] * 0.2126 + values[1] * 0.7152 + values[2] * 0.0722;
  };
  const a = luminance(foreground);
  const b = luminance(background);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

(async () => {
  const browser = await chromium.launch({
    ...(process.env.CHROMIUM_PATH
      ? { executablePath: process.env.CHROMIUM_PATH }
      : {}),
    headless: true,
    args: ["--no-sandbox"],
  });
  try {
    for (const mode of ["light", "dark"]) {
      const page = await browser.newPage({
        viewport: { width: 1100, height: 750 },
      });
      await page.addInitScript({ path: fixture });
      const errors = [];
      page.on("pageerror", (error) => errors.push(error.message));
      page.on("console", (message) => {
        if (/unique.*key|same key/i.test(message.text()))
          errors.push(message.text());
      });
      for (const route of routes) {
        if (
          results.some(
            (result) =>
              result.mode === mode &&
              result.route === route &&
              result.status === "passed"
          )
        )
          continue;
        const start = errors.length;
        const response = await page.goto(
          `${base}${route}?test-color-mode=${mode}`
        );
        assert.ok(response.ok(), `${route}: HTTP ${response.status()}`);
        await page.waitForFunction(() => window.next?.router?.isReady);
        await page.waitForFunction(
          (mode) => document.documentElement.dataset.theme === mode,
          mode
        );
        await page.getByRole("button").first().waitFor();
        await page.waitForTimeout(250);
        assert.equal(
          await page.getByText("404", { exact: true }).count(),
          0,
          route
        );
        assert.deepEqual(errors.slice(start), [], route);
        const layout = await page.evaluate(() => ({
          width: innerWidth,
          scrollWidth: document.documentElement.scrollWidth,
        }));
        assert.ok(
          layout.scrollWidth <= layout.width,
          `${route}: horizontal overflow`
        );
        const screenshot = `${mode}-${route.slice(1).replaceAll("/", "-")}.png`;
        await page.screenshot({ path: path.join(output, screenshot) });
        results.push({ mode, route, status: "passed", screenshot, layout });
        writeFileSync(
          path.join(output, "results.json"),
          JSON.stringify(results, null, 2)
        );
      }
      if (
        !results.some(
          (result) =>
            result.mode === mode &&
            result.name ===
              "contrast, sidebar selection, keyboard focus and setting update"
        )
      ) {
        await page.goto(`${base}/settings/general?test-color-mode=${mode}`);
        const selected = page.getByRole("button", {
          name: locale.SettingsLayout.settingsDomainList.general,
          exact: true,
        });
        await selected.waitFor();
        assert.equal(await selected.getAttribute("aria-pressed"), "true");
        const description = page.locator(".secondary-text:visible").first();
        const colors = await description.evaluate((element) => {
          let parent = element.parentElement;
          while (
            parent &&
            getComputedStyle(parent).backgroundColor === "rgba(0, 0, 0, 0)"
          )
            parent = parent.parentElement;
          return {
            foreground: getComputedStyle(element).color,
            background: getComputedStyle(parent).backgroundColor,
          };
        });
        const ratio = contrast(colors.foreground, colors.background);
        assert.ok(ratio >= 4.5, `${mode} secondary text: ${ratio}`);
        // Reach the selected sidebar entry through real keyboard navigation.
        await page.locator("body").click({ position: { x: 5, y: 5 } });
        let focused = false;
        for (let attempt = 0; attempt < 80; attempt++) {
          await page.keyboard.press("Tab");
          focused = await selected.evaluate(
            (element) => element === document.activeElement
          );
          if (focused) break;
        }
        assert.ok(focused, "Sidebar must be keyboard reachable");
        await page.waitForTimeout(250);
        const focusStyle = await selected.evaluate((element) => ({
          visible: element.matches(":focus-visible"),
          shadow: getComputedStyle(element).boxShadow,
        }));
        assert.equal(focusStyle.visible, true);
        assert.notEqual(focusStyle.shadow, "none");
        assert.ok(
          focusStyle.shadow.includes("3px"),
          "Focus ring must finish its transition"
        );
        await page.screenshot({
          path: path.join(output, `${mode}-keyboard-focus.png`),
        });
        // A setting must still send its boolean value through the shared row layout.
        const toggle = page.getByRole("checkbox").first();
        const checked = await toggle.isChecked();
        await toggle.press("Space");
        const call = await page.evaluate(() =>
          window.calls
            .filter((item) => item.cmd === "update_launcher_config")
            .at(-1)
        );
        assert.equal(JSON.parse(call.args.value), !checked);
        results.push({
          mode,
          name: "contrast, sidebar selection, keyboard focus and setting update",
          status: "passed",
          ratio,
          colors,
          focusStyle,
        });
      }
      if (
        !results.some(
          (result) =>
            result.mode === mode &&
            result.name === "account dialog open and keyboard dismissal"
        )
      ) {
        await page.goto(`${base}/accounts?test-color-mode=${mode}`);
        await page
          .getByRole("button", {
            name: locale.AccountsPage.button.addPlayer,
            exact: true,
          })
          .last()
          .click();
        const dialog = page.getByRole("dialog");
        await dialog.waitFor();
        await page.keyboard.press("Escape");
        await dialog.waitFor({ state: "hidden" });
        results.push({
          mode,
          name: "account dialog open and keyboard dismissal",
          status: "passed",
        });
      }
      if (
        !results.some(
          (result) =>
            result.mode === mode && result.name === "minimum window layout"
        )
      ) {
        await page.setViewportSize({ width: 800, height: 550 });
        for (const route of [
          "/settings/general",
          "/settings/intelligence",
                ]) {
          await page.goto(`${base}${route}?test-color-mode=${mode}`);
          await page.getByRole("button").first().waitFor();
          await page.waitForTimeout(250);
          const overflow = await page
            .locator(".content-full-y")
            .evaluateAll((elements) =>
              elements.map((element) => ({
                width: element.clientWidth,
                scrollWidth: element.scrollWidth,
              }))
            );
          assert.ok(
            overflow.every(
              (element) => element.scrollWidth <= element.width + 1
            ),
            JSON.stringify({ route, overflow })
          );
          await page.screenshot({
            path: path.join(
              output,
              `${mode}-minimum-${route.slice(1).replaceAll("/", "-")}.png`
            ),
          });
        }
        results.push({
          mode,
          name: "minimum window layout",
          status: "passed",
          width: 800,
          height: 550,
        });
      }
      await page.close();
    }
    writeFileSync(
      path.join(output, "results.json"),
      JSON.stringify(results, null, 2)
    );
    rmSync(path.join(output, "failure.txt"), { force: true });
    console.log(`PASS: ${results.length} launcher UI checks`);
  } finally {
    await browser.close();
    rmSync(fixture, { force: true });
  }
})().catch((error) => {
  writeFileSync(path.join(output, "failure.txt"), String(error.stack));
  console.error(error);
  process.exitCode = 1;
});
