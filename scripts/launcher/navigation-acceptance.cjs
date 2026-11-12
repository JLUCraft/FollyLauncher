const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const { mkdirSync, rmSync, writeFileSync } = require("node:fs");
const path = require("node:path");
const { loadResults } = require("./results.cjs");
const { parseRecord } = require("../json.cjs");
const { chromium } = /** @type {typeof import("playwright-core")} */ (
  require(process.env.PLAYWRIGHT_MODULE || "playwright-core")
);
const repo = path.resolve(__dirname, "../..");
const output = path.resolve(
  process.env.LAUNCHER_TEST_OUTPUT || path.join(repo, "out/launcher-navigation")
);
const base = process.env.LAUNCHER_TEST_URL || "http://localhost:3000";
const fixture = path.join(repo, "out/launcher-navigation-mock.js");
const locale = require("../../src/locales/zh-Hans.json");
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
    ? loadResults(path.join(output, "results.json")).filter(
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
  /** @param {string} name @param {string} route @param {boolean} configured @param {(page: import("playwright-core").Page) => Promise<void>} action */
  async function run(name, route, configured, action) {
    if (results.some((result) => result.name === name)) return;
    const page = await browser.newPage({
      viewport: { width: 1100, height: 750 },
    });
    /** @type {string[]} */
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.addInitScript({ path: fixture });
    const url = new URL(route, base);
    url.searchParams.set("test-scenario", configured ? "configured" : "empty");
    try {
      const response = await page.goto(url.href);
      assert.ok(response?.ok());
      await page.getByRole("tab").first().waitFor();
      await action(page);
      assert.deepEqual(errors, []);
      await page.evaluate(() => document.fonts.ready);
      await page.screenshot({ path: path.join(output, `${name}.png`) });
      results.push({ name, status: "passed" });
    } catch (error) {
      results.push({ name, status: "failed", error: String(error) });
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
  /** @param {import("playwright-core").Page} page @param {string} cmd */
  const calls = (page, cmd) =>
    page.evaluate(
      (cmd) => window.calls.filter((call) => call.cmd === cmd),
      cmd
    );
  /** @param {import("playwright-core").Page} page @param {string} pathname */
  const go = async (page, pathname) => {
    await page.waitForURL((url) => url.pathname === pathname);
    const navLabel = pathname.startsWith("/settings/")
      ? locale.HeadNavBar.navList.settings
      : locale.HeadNavBar.navList.instances;
    await page
      .getByRole("tab", { name: navLabel, exact: true, selected: true })
      .waitFor();
  };
  /** @param {string} peer */
  const linkFor = (peer) =>
    `folly://federation?${new URLSearchParams({ address: `/ip4/127.0.0.1/tcp/4001/p2p/${peer}`, peer, genesis: "a".repeat(64) })}`;
  /** @param {import("playwright-core").Page} page @param {string} link */
  const emitLink = (page, link) =>
    page.evaluate(
      (link) =>
        window.dispatchEvent(
          new CustomEvent("deeplink:emit", { detail: [link] })
        ),
      link
    );
  const next = locale.FederationPage.guide.setup;

  try {
    await run(
      "alliance-default-and-public-browsing",
      "/launch",
      false,
      async (page) => {
        assert.equal(
          await page.locator("#head-navbar-tab-federation").count(),
          0
        );
        await page
          .getByRole("tab", {
            name: locale.HeadNavBar.navList.instances,
            exact: true,
          })
          .click();
        await page.waitForURL((url) => url.pathname.startsWith("/instances"));
        await page
          .getByRole("button", {
            name: locale.FederationNav.activities,
            exact: true,
          })
          .click();
        await go(page, "/instances/activities");
        await page
          .getByRole("button", {
            name: locale.FederationNav.rooms,
            exact: true,
          })
          .click();
        await go(page, "/instances/rooms");
        await page.getByText("Club survival", { exact: true }).waitFor();
        assert.equal(
          await page
            .getByRole("button", {
              name: locale.FederationNav.overview,
              exact: true,
            })
            .count(),
          0
        );
        assert.equal(
          await page
            .getByRole("button", {
              name: locale.FederationNav.identity,
              exact: true,
            })
            .count(),
          0
        );
        assert.equal(await page.locator("#room-seeds").count(), 0);
        assert.equal((await calls(page, "union_governance")).length, 0);
        assert.equal((await calls(page, "union_proxy_start")).length, 0);
      }
    );
    await run(
      "filtered-room-settings-return",
      "/instances/rooms",
      false,
      async (page) => {
        const search = page.getByPlaceholder(locale.RoomSearch.search);
        await page.getByText("Study build", { exact: true }).waitFor();
        await search.fill("Study");
        const discoveries = (await calls(page, "union_rooms")).length;
        await page
          .getByRole("link", { name: locale.Rooms.setupToJoin, exact: true })
          .click();
        await go(page, "/settings/federation");
        assert.equal(new URL(page.url()).searchParams.has("returnTo"), false);
        assert.equal(
          await page
            .getByRole("tab", {
              name: locale.HeadNavBar.navList.settings,
              exact: true,
            })
            .getAttribute("aria-selected"),
          "true"
        );
        await page
          .getByRole("button", {
            name: locale.SettingsLayout.settingsDomainList.federation,
            exact: true,
          })
          .waitFor();
        await page.goBack();
        await go(page, "/instances/rooms");
        assert.equal(await search.inputValue(), "Study");
        assert.equal(
          await page.getByText("Club survival", { exact: true }).count(),
          0
        );
        await page.locator("#room-host-b-room-b").waitFor();
        assert.equal((await calls(page, "union_rooms")).length, discoveries);
        assert.equal((await calls(page, "union_proxy_start")).length, 0);
      }
    );
    await run(
      "host-settings-return-context",
      "/instances/rooms",
      false,
      async (page) => {
        await page
          .getByRole("button", { name: locale.Rooms.host, exact: true })
          .click();
        await page
          .getByRole("link", { name: locale.Rooms.setupToHost, exact: true })
          .click();
        await go(page, "/settings/federation");
        await page.goBack();
        await go(page, "/instances/rooms");
        assert.equal((await calls(page, "union_room_host")).length, 0);
      }
    );
    await run(
      "activity-settings-return",
      "/instances/activities",
      false,
      async (page) => {
        await page
          .getByRole("link", {
            name: locale.FederationPage.openIdentity,
            exact: true,
          })
          .click();
        await go(page, "/settings/federation");
        assert.equal(
          await page.getByRole("link", { name: next, exact: true }).count(),
          0
        );
        await page.goBack();
        await go(page, "/instances/activities");
        assert.equal((await calls(page, "union_governance")).length, 0);
      }
    );
    await run(
      "league-detail-settings-return",
      "/instances/activities/competition?id=retained-league",
      false,
      async (page) => {
        await page
          .getByRole("link", {
            name: locale.FederationPage.openIdentity,
            exact: true,
          })
          .click();
        await go(page, "/settings/federation");
        await page.goBack();
        await go(page, "/instances/activities/competition");
        assert.equal(
          new URL(page.url()).searchParams.get("id"),
          "retained-league"
        );
        assert.equal((await calls(page, "union_proxy_start")).length, 0);
      }
    );
    await run(
      "connection-link-explicit-confirmation",
      "/instances/rooms",
      false,
      async (page) => {
        await page.getByText("Study build", { exact: true }).waitFor();
        const link = linkFor("requested-peer");
        await emitLink(page, link);
        await go(page, "/settings/federation");
        await page
          .getByText(locale.ConnectionFlow.linkStaged, { exact: true })
          .waitFor();
        assert.equal((await calls(page, "union_governance")).length, 0);
        assert.equal(
          await page.evaluate(() =>
            localStorage.getItem("federation-connection")
          ),
          null
        );
        await page.goBack();
        await go(page, "/instances/rooms");
        assert.ok(
          (await calls(page, "union_rooms")).every((call) => {
            const seeds = call.args.seeds;
            return (
              Array.isArray(seeds) &&
              !seeds.some(
                (seed) =>
                  typeof seed === "string" && seed.includes("requested-peer")
              )
            );
          })
        );
        await page
          .getByRole("tab", {
            name: locale.HeadNavBar.navList.settings,
            exact: true,
          })
          .click();
        await go(page, "/settings/federation");
        // The setup guide now focuses controls on the same page.
        await page.getByRole("button", { name: next, exact: true }).click();
        await page.locator("#identity-setup").waitFor();
        await page
          .getByRole("button", {
            name: locale.IdentityFlow.importSetup,
            exact: true,
          })
          .click();
        await page.waitForFunction(() => {
          const input = document.querySelector("#identity-setup");
          return (
            input instanceof HTMLTextAreaElement &&
            input.value.includes("acceptance")
          );
        });
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
        // Connection is automatic now; the link form stays available as a fallback.
        await page.locator("#federation-link").waitFor();
        assert.equal(await page.locator("#federation-link").inputValue(), link);
        assert.equal((await calls(page, "union_governance")).length, 0);
        await page.locator("#federation-link").press("Enter");
        await page
          .getByText(locale.FederationNav.connected, { exact: true })
          .waitFor();
        assert.equal((await calls(page, "union_governance")).length, 1);
        assert.equal(
          parseRecord(
            (await page.evaluate(() =>
              localStorage.getItem("federation-connection")
            )) || "null"
          ).peer,
          "requested-peer"
        );
        const replacement = linkFor("another-peer");
        await emitLink(page, replacement);
        await page.locator("#federation-link").waitFor();
        assert.equal(
          await page.locator("#federation-link").inputValue(),
          replacement
        );
        assert.equal((await calls(page, "union_governance")).length, 1);
        assert.equal(
          parseRecord(
            (await page.evaluate(() =>
              localStorage.getItem("federation-connection")
            )) || "null"
          ).peer,
          "requested-peer"
        );
        await page.locator("#federation-link").press("Enter");
        await page.waitForFunction(
          () =>
            window.calls.filter((call) => call.cmd === "union_governance")
              .length === 2
        );
        assert.equal((await calls(page, "union_proxy_start")).length, 0);
      }
    );
    await run(
      "settings-network-and-admin-role",
      "/settings/federation",
      true,
      async (page) => {
        await page.locator("#federation-link").fill(linkFor("test-peer"));
        await page.locator("#federation-link").press("Enter");
        await page
          .getByText(locale.FederationNav.connected, { exact: true })
          .waitFor();
        await page
          .getByRole("tab", {
            name: locale.HeadNavBar.navList.instances,
            exact: true,
          })
          .click();
        // The instances tab restores the last visited instances route.
        await page.waitForURL((url) => url.pathname.startsWith("/instances"));
        await page
          .getByRole("button", {
            name: locale.FederationNav.rooms,
            exact: true,
          })
          .click();
        await go(page, "/instances/rooms");
        await page
          .getByRole("button", {
            name: locale.Rooms.networkSettings,
            exact: true,
          })
          .click();
        await page.locator("#room-seeds").waitFor();
        await page
          .getByRole("tab", {
            name: locale.HeadNavBar.navList.settings,
            exact: true,
          })
          .click();
        await go(page, "/settings/federation");
        assert.equal(
          await page
            .getByRole("button", {
              name: locale.SettingsLayout.settingsDomainList[
                "federation-management"
              ],
              exact: true,
            })
            .count(),
          0
        );
        await page.evaluate(() => (window.admin = true));
        await page
          .getByRole("button", {
            name: locale.FederationPage.settings.manualTitle,
            exact: true,
          })
          .click();
        await page
          .getByRole("button", {
            name: locale.FederationPage.settings.connect,
            exact: true,
          })
          .click();
        const management = page.getByRole("button", {
          name: locale.SettingsLayout.settingsDomainList[
            "federation-management"
          ],
          exact: true,
        });
        await management.waitFor();
        await management.click();
        await go(page, "/settings/federation-management");
        // Route commits before the menu re-renders; poll for the selection state.
        await page.waitForFunction(
          (name) =>
            [...document.querySelectorAll("button")].some(
              (button) =>
                button.textContent === name &&
                button.getAttribute("aria-pressed") === "true"
            ),
          locale.SettingsLayout.settingsDomainList["federation-management"]
        );
        await page
          .getByRole("tab", {
            name: locale.HeadNavBar.navList.instances,
            exact: true,
          })
          .click();
        await page
          .getByRole("button", {
            name: locale.FederationNav.activities,
            exact: true,
          })
          .click();
        await go(page, "/instances/activities");
        assert.equal(
          await page
            .getByRole("button", {
              name: locale.SettingsLayout.settingsDomainList[
                "federation-management"
              ],
              exact: true,
            })
            .count(),
          0
        );
        assert.equal(await page.locator("#room-seeds").count(), 0);
      }
    );
    console.log(`PASS: ${results.length} Alliance navigation scenarios`);
  } finally {
    await browser.close();
    rmSync(fixture, { force: true });
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
