const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const { mkdirSync, readFileSync, rmSync, writeFileSync } = require("node:fs");
const path = require("node:path");
const { chromium } = require(
  process.env.PLAYWRIGHT_MODULE || "playwright-core"
);
const repo = path.resolve(__dirname, "../..");
const output = path.resolve(
  process.env.LAUNCHER_TEST_OUTPUT || path.join(repo, "out/launcher-workflows")
);
const base = process.env.LAUNCHER_TEST_URL || "http://localhost:3000";
const fixture = path.join(repo, "out/launcher-workflow-mock.js");
const locale = JSON.parse(
  readFileSync(path.join(repo, "src/locales/zh-Hans.json"))
);
mkdirSync(output, { recursive: true });
execFileSync("bun", [
  "build",
  path.join(__dirname, "workflow-mock.ts"),
  "--outfile",
  fixture,
  "--target",
  "browser",
  "--format",
  "iife",
]);
const results =
  process.env.LAUNCHER_TEST_RESUME === "1"
    ? JSON.parse(readFileSync(path.join(output, "results.json"))).filter(
        (result) => result.status === "passed"
      )
    : [];
(async () => {
  const browser = await chromium.launch({
    ...(process.env.CHROMIUM_PATH
      ? { executablePath: process.env.CHROMIUM_PATH }
      : {}),
    headless: true,
    args: ["--no-sandbox"],
  });
  async function run(name, scenario, route, action) {
    if (
      results.some(
        (result) => result.name === name && result.status === "passed"
      )
    )
      return;
    const page = await browser.newPage({
      viewport: { width: 1100, height: 750 },
    });
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    let consoleErrors = 0;
    page.on("console", (message) => {
      if (message.type() === "error" && consoleErrors++ < 5)
        console.error(`[${name}] ${message.text()}`);
    });
    await page.addInitScript({ path: fixture });
    try {
      const response = await page.goto(
        `${base}${route}?test-scenario=${scenario}`
      );
      assert.ok(response.ok());
      await page.getByRole("button").first().waitFor();
      await action(page);
      assert.deepEqual(errors, []);
      await page.evaluate(() => document.fonts.ready);
      await page.screenshot({ path: path.join(output, `${name}.png`) });
      results.push({ name, status: "passed" });
    } catch (error) {
      const diagnostic = await page
        .evaluate(() => ({
          calls: window.calls?.slice(-15),
          selection: window.testConfig?.states.shared,
        }))
        .catch(() => null);
      results.push({
        name,
        status: "failed",
        error: String(error),
        diagnostic,
      });
      await page
        .screenshot({
          path: path.join(output, `${name}-failure.png`),
          timeout: 3000,
        })
        .catch(() => {});
      throw error;
    } finally {
      writeFileSync(
        path.join(output, "results.json"),
        JSON.stringify(results, null, 2)
      );
      await page.close();
    }
  }
  const calls = (page, cmd) =>
    page.evaluate(
      (cmd) => window.calls.filter((call) => call.cmd === cmd),
      cmd
    );
  try {
    await run("home-empty-setup", "empty", "/launch", async (page) => {
      const main = page.locator("#main-launch-button");
      await page.waitForFunction(
        () => !document.querySelector("#main-launch-button").disabled
      );
      assert.ok(
        (await main.innerText()).includes(
          locale.AllInstancesPage.button.addAndImport
        )
      );
      await page.screenshot({ path: path.join(output, "home-empty.png") });
      await main.click();
      await page.waitForURL("**/instances/add-import?returnTo=launch");
      await page
        .getByRole("link", { name: locale.LaunchPage.button.returnToLaunch })
        .click();
      await page.waitForURL("**/launch");
      await page
        .getByRole("button", {
          name: locale.AccountsPage.button.addPlayer,
          exact: true,
        })
        .last()
        .click();
      const dialog = page.getByRole("dialog");
      await dialog.waitFor();
      await dialog
        .getByRole("button", {
          name: locale.AddPlayerModal.oauthCommon.button.start.microsoft,
          exact: true,
        })
        .waitFor();
      await page.keyboard.press("Escape");
      await dialog.waitFor({ state: "hidden" });
      await page
        .getByRole("link", { name: locale.LaunchPage.button.returnToLaunch })
        .waitFor();
      assert.equal((await calls(page, "add_player_offline")).length, 0);
      assert.equal((await calls(page, "fetch_oauth_code")).length, 0);
    });
    await run("account-empty-actions", "empty", "/accounts", async (page) => {
      await page
        .getByText(locale.AccountsPage.empty, { exact: true })
        .waitFor();
      await page
        .getByRole("button", { name: "https://skin.example.test", exact: true })
        .waitFor();
      await page
        .getByRole("button", {
          name: locale.AccountsPage.button.addPlayer,
          exact: true,
        })
        .last()
        .click();
      await page.getByRole("dialog").waitFor();
      await page.keyboard.press("Escape");
      await page.getByRole("dialog").waitFor({ state: "hidden" });
      await page
        .getByRole("button", {
          name: locale.AccountsPage.button.importFromOtherLaunchers,
          exact: true,
        })
        .last()
        .click();
      await page.getByRole("dialog").waitFor();
    });
    await run(
      "instance-empty-keyboard",
      "empty",
      "/instances/list",
      async (page) => {
        await page
          .getByText(locale.AllInstancesPage.empty, { exact: true })
          .waitFor();
        await page
          .getByRole("link", {
            name: locale.AllInstancesPage.button.addAndImport,
            exact: true,
          })
          .click();
        const row = page.getByRole("button", {
          name: new RegExp(
            `^${locale.AddAndImportInstancePage.addAndImportOptions.new.title}`
          ),
        });
        await row.focus();
        await row.press("Enter");
        const dialog = page.getByRole("dialog", {
          name: locale.CreateInstanceModal.header.title,
          exact: true,
        });
        await dialog.waitFor();
        await dialog.getByRole("button", { name: /release 1\.21/ }).waitFor();
        const before = (await calls(page, "fetch_game_version_list")).length;
        await page.waitForTimeout(250);
        assert.equal(
          (await calls(page, "fetch_game_version_list")).length,
          before
        );
        await dialog
          .getByRole("button", { name: locale.General.refresh, exact: true })
          .click();
        await page.waitForFunction(
          (before) =>
            window.calls.filter(
              (call) => call.cmd === "fetch_game_version_list"
            ).length > before,
          before
        );
        await page.waitForTimeout(250);
        assert.equal(
          (await calls(page, "fetch_game_version_list")).length,
          before + 1
        );
      }
    );
    await run(
      "existing-instance-account-setup",
      "instances",
      "/launch",
      async (page) => {
        const main = page.locator("#main-launch-button");
        await page.waitForFunction(
          () => !document.querySelector("#main-launch-button").disabled
        );
        assert.ok(
          (await main.innerText()).includes(
            locale.AccountsPage.button.addPlayer
          )
        );
        await main.click();
        await page.getByRole("dialog").waitFor();
        assert.equal(
          await page.evaluate(
            () => window.testConfig.states.shared.selectedInstanceId
          ),
          "acceptance:1.21"
        );
        assert.equal((await calls(page, "select_suitable_jre")).length, 0);
        await page.keyboard.press("Escape");
        await page.getByRole("dialog").waitFor({ state: "hidden" });
        await page
          .getByRole("link", { name: locale.LaunchPage.button.returnToLaunch })
          .waitFor();
      }
    );
    await run(
      "local-selection-before-launch",
      "selection",
      "/launch",
      async (page) => {
        await page.waitForFunction(
          () => !document.querySelector("#main-launch-button").disabled
        );
        await page.locator("#main-launch-button").click();
        const picker = page.getByRole("dialog", {
          name: locale.SelectInstanceModal.header.titleForLaunch,
          exact: true,
        });
        await picker.waitFor();
        await page.mouse.click(600, 8);
        assert.equal(await picker.isVisible(), true);
        assert.equal(
          await page.evaluate(
            () => window.testConfig.states.shared.selectedInstanceId
          ),
          ""
        );
        const row = picker.getByRole("button", { name: /Acceptance instance/ });
        await row.focus();
        await row.press("Space");
        const players = page.getByRole("dialog", {
          name: locale.SelectPlayerModal.header.titleForLaunch,
          exact: true,
        });
        await players.waitFor();
        const playerRow = players.getByRole("button", {
          name: /Acceptance player/,
        });
        await playerRow.focus();
        await playerRow.press("Enter");
        await page.waitForFunction(() =>
          window.calls.some((call) => call.cmd === "select_suitable_jre")
        );
        assert.equal(
          (await calls(page, "select_suitable_jre"))[0].args.instanceId,
          "acceptance:1.21"
        );
        assert.equal((await calls(page, "launch_game")).length, 0);
      }
    );
    await run(
      "returning-user-direct-launch",
      "ready",
      "/launch",
      async (page) => {
        await page.waitForFunction(
          () => !document.querySelector("#main-launch-button").disabled
        );
        await page.locator("#main-launch-button").click();
        await page.waitForFunction(() =>
          window.calls.some((call) => call.cmd === "select_suitable_jre")
        );
        assert.equal(
          (await calls(page, "select_suitable_jre"))[0].args.instanceId,
          "acceptance:1.21"
        );
        assert.equal((await calls(page, "launch_game")).length, 0);
      }
    );
    await run(
      "organizer-file-explicit-save",
      "empty",
      "/settings/federation",
      async (page) => {
        const next = locale.FederationPage.guide.setup;
        // The guide acts on the controls of the same page.
        await page.getByRole("button", { name: next, exact: true }).waitFor();
        assert.equal(await page.locator("#federation-link").count(), 0);
        assert.equal((await calls(page, "union_governance")).length, 0);
        await page.screenshot({
          path: path.join(output, "federation-prerequisite.png"),
        });
        await page.getByRole("button", { name: next, exact: true }).click();
        await page
          .locator("#identity-setup")
          .waitFor({ state: "visible" });
        await page.waitForFunction(
          () =>
            document.getElementById("identity-setup") ===
            document.activeElement
        );
        await page
          .getByRole("button", {
            name: locale.IdentityFlow.importSetup,
            exact: true,
          })
          .click();
        await page.waitForFunction(() =>
          document
            .querySelector("#identity-setup")
            ?.value.includes("acceptance")
        );
        assert.equal(
          (await calls(page, "union_identity_configuration")).filter(
            (call) => call.args.config !== null
          ).length,
          0
        );
        await page
          .getByRole("button", {
            name: locale.FederationPage.identity.save,
            exact: true,
          })
          .click();
        const input = page.locator("#federation-link");
        await input.waitFor();
        const link = await page.evaluate(
          () =>
            `${window.scheme}://federation?address=test-address&peer=test-peer&genesis=${"a".repeat(64)}`
        );
        await input.fill(link);
        await input.press("Enter");
        await page
          .getByText(locale.FederationNav.connected, { exact: true })
          .waitFor();
        assert.equal(new URL(page.url()).pathname, "/settings/federation");
      }
    );
    await run(
      "invalid-file-and-cancel-preserve-draft",
      "empty",
      "/settings/federation",
      async (page) => {
        const input = page.locator("#identity-setup");
        await input.waitFor();
        await input.fill("retained draft");
        await page.evaluate(() => (window.testFilePath = null));
        await page
          .getByRole("button", {
            name: locale.IdentityFlow.importSetup,
            exact: true,
          })
          .click();
        assert.equal(await input.inputValue(), "retained draft");
        await page.evaluate(() => {
          window.testFilePath = "/bad.json";
          window.testFileContent = "invalid json";
        });
        await page
          .getByRole("button", {
            name: locale.IdentityFlow.importSetup,
            exact: true,
          })
          .click();
        await page
          .getByText(locale.FederationPage.identity.saveInvalid, {
            exact: true,
          })
          .waitFor();
        await input.fill("null");
        await page
          .getByRole("button", {
            name: locale.FederationPage.identity.save,
            exact: true,
          })
          .click();
        await page
          .getByText(locale.FederationPage.identity.saveInvalid, {
            exact: true,
          })
          .waitFor();
        assert.equal(
          (await calls(page, "union_identity_configuration")).filter(
            (call) => call.args.config !== null
          ).length,
          0
        );
      }
    );
    await run(
      "activity-single-prerequisite",
      "empty",
      "/federation/activities",
      async (page) => {
        await page
          .getByRole("link", {
            name: locale.FederationPage.openIdentity,
            exact: true,
          })
          .waitFor();
        assert.equal(
          await page
            .getByText(locale.FederationPage.identity.unavailable, {
              exact: true,
            })
            .count(),
          0
        );
      }
    );
    console.log(`PASS: ${results.length} ergonomic workflow scenarios`);
  } finally {
    await browser.close();
    rmSync(fixture, { force: true });
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
