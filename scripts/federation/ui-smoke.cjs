// Requires a running Next development server and an available Playwright installation.
const { chromium } = require(
  process.env.PLAYWRIGHT_MODULE || "playwright-core"
);
const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const { readFileSync, rmSync } = require("node:fs");
const path = require("node:path");
const output = path.resolve(__dirname, "../../out");
const bootstrap = path.join(output, "federation-ui-mock.js");
execFileSync("bun", [
  "build",
  path.join(__dirname, "mock-ipc.ts"),
  "--outfile",
  bootstrap,
  "--target",
  "browser",
  "--format",
  "iife",
]);
const base = process.env.FEDERATION_TEST_URL || "http://localhost:3000";
(async () => {
  const browser = await chromium.launch({
    ...(process.env.CHROMIUM_PATH
      ? { executablePath: process.env.CHROMIUM_PATH }
      : {}),
    headless: true,
    args: ["--no-sandbox"],
  });
  try {
    const page = await browser.newPage({
      viewport: { width: 1280, height: 850 },
    });
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.addInitScript({
      content: `${readFileSync(bootstrap, "utf8")}\nwindow.testHandlers = { union_identity_configuration: () => ({ genesis_hash: "a".repeat(64) }) };`,
    });
    // The match link box lives in a collapsed section until a student needs it.
    const openGameLink = async () => {
      const input = page.locator("#game-link");
      if (!(await input.isVisible()))
        await page.getByText("比赛连接", { exact: true }).first().click();
      await input.waitFor();
    };
    const openCompetition = async () => {
      await page.evaluate(() =>
        window.next.router.push(
          "/federation/activities/competition?id=test-league"
        )
      );
      await page.waitForURL(
        "**/federation/activities/competition?id=test-league"
      );
      await openGameLink();
    };
    const nav = async (name) => {
      const paths = {
        连接: "/settings/federation",
        活动: "/federation/activities",
        服务器与房间: "/instances/rooms",
        联盟: "/settings/federation",
        联盟管理: "/settings/federation-management",
      };
      await page.evaluate((path) => window.next.router.push(path), paths[name]);
      await page.waitForURL(`**${paths[name]}`);
      if (name === "服务器与房间")
        await page.getByPlaceholder("搜索房间、社团或版本").waitFor();
    };
    await page.goto(`${base}/federation`);
    await page.waitForURL("**/federation/activities");
    await nav("连接");
    await page.locator("#federation-link").fill("not a link");
    await page.locator("#federation-link").press("Enter");
    await page.getByText("这不是有效的连接链接", { exact: false }).waitFor();
    const connectionLink = `folly://federation?${new URLSearchParams({
      address: "/ip4/127.0.0.1/tcp/4001/p2p/test",
      peer: "test",
      genesis: "a".repeat(64),
    })}`;
    await page.locator("#federation-link").fill(connectionLink);
    await page.locator("#federation-link").press("Enter");
    await page.getByText("已连接", { exact: true }).waitFor();
    assert.equal(
      await page.getByRole("button", { name: "联盟管理", exact: true }).count(),
      0
    );
    await nav("服务器与房间");
    await page
      .getByRole("button", { name: "刷新房间与状态", exact: true })
      .click();
    await page.getByText("Club survival", { exact: true }).waitFor();
    await page.getByPlaceholder("搜索房间、社团或版本").fill("Study");
    assert.equal(
      await page.getByText("Club survival", { exact: true }).count(),
      0
    );
    assert.equal(
      await page.getByText("Study build", { exact: true }).count(),
      1
    );
    await page.screenshot({ path: path.join(output, "federation-rooms.png") });
    await page.getByRole("button", { name: "加入房间", exact: true }).click();
    await page.getByText(/已连接至 127\.0\.0\.1:12345/).waitFor();
    const discoveries = await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "union_rooms").length
    );
    await openCompetition();
    await page.getByText("Study build", { exact: true }).waitFor();
    await nav("服务器与房间");
    assert.equal(
      await page.getByPlaceholder("搜索房间、社团或版本").inputValue(),
      "Study"
    );
    assert.equal(
      await page.evaluate(
        () => window.calls.filter((c) => c.cmd === "union_rooms").length
      ),
      discoveries
    );
    assert.equal(
      await page.evaluate(
        () =>
          window.calls.filter((c) => c.cmd === "union_proxy_start").at(-1).args
            .input.service
      ),
      "room-b"
    );
    await page.getByRole("button", { name: "断开连接", exact: true }).click();
    await nav("联盟");
    assert.equal(await page.locator("textarea:visible").count(), 0);
    await nav("活动");
    await page.waitForTimeout(500);
    const pollCount = await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "union_proxy_status").length
    );
    await page.waitForTimeout(3400);
    assert.equal(
      await page.evaluate(
        () => window.calls.filter((c) => c.cmd === "union_proxy_status").length
      ),
      pollCount
    );
    await nav("连接");
    await page
      .getByRole("button", { name: "手动填写连接信息", exact: true })
      .click();
    assert.equal(await page.locator("#federation-peer").inputValue(), "test");
    await page.evaluate(() => (window.admin = true));
    await page
      .getByRole("button", { name: "连接 / 刷新", exact: true })
      .click();
    await page.getByRole("button", { name: "联盟管理", exact: true }).waitFor();
    const governanceCalls = await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "union_governance").length
    );
    await page.locator("#federation-peer").fill("unapplied-edit");
    assert.equal(await page.getByText("已连接", { exact: true }).count(), 1);
    assert.equal(
      await page.getByRole("button", { name: "联盟管理", exact: true }).count(),
      1
    );
    assert.equal(
      await page.evaluate(
        () => window.calls.filter((c) => c.cmd === "union_governance").length
      ),
      governanceCalls
    );
    assert.equal(
      await page.evaluate(
        () => JSON.parse(localStorage.getItem("federation-connection")).peer
      ),
      "test"
    );
    await nav("联盟管理");
    await page.goto(`${base}/settings/federation-management`);
    await page
      .getByText("请使用管理员身份连接后管理应用。", { exact: true })
      .waitFor();
    // A full reload restores confirmed targets, not the unfinished edit.
    assert.equal(
      await page.evaluate(
        () =>
          window.calls.find((c) => c.cmd === "union_governance").args.input.peer
      ),
      "test"
    );
    await openCompetition();
    await page.locator("#game-link").fill("invalid-link");
    await page
      .getByRole("button", { name: "建立游戏连接", exact: true })
      .click();
    await page
      .getByText("游戏连接未完成，请检查链接或联系组织者", { exact: true })
      .waitFor();
    await nav("连接");
    await page
      .getByText("游戏连接未完成，请检查链接或联系组织者", { exact: true })
      .waitFor({ state: "hidden" });
    const scheme = await page.evaluate(() => window.scheme);
    const link = `${scheme}://connect?peer=match-peer&service=match-service&address=test-address`;
    await page.evaluate(
      (link) =>
        window.dispatchEvent(
          new CustomEvent("deeplink:emit", { detail: [link] })
        ),
      link
    );
    await page.waitForURL("**/federation/activities");
    await openCompetition();
    assert.equal(await page.locator("#game-link").inputValue(), link);
    await page.goto(
      `${base}/launch?test-deep-link=${encodeURIComponent(link)}`
    );
    await page.waitForURL("**/federation/activities");
    await openCompetition();
    assert.equal(await page.locator("#game-link").inputValue(), link);
    assert.deepEqual(errors, []);
    // LAN auto-discovery: a pinned server connects with no manual input at all.
    const autoServer = {
      peer: "auto-peer",
      address: "/ip4/127.0.0.1/tcp/4001/p2p/auto-peer",
      federation: "jilin",
      genesis: { federation: "jilin" },
      genesis_hash: "a".repeat(64),
      application: "b".repeat(64),
      services: [],
    };
    const mockContent = readFileSync(bootstrap, "utf8");
    const autoContext = await browser.newContext({
      viewport: { width: 1280, height: 850 },
    });
    const autoPage = await autoContext.newPage();
    autoPage.on("pageerror", (e) => errors.push(e.message));
    await autoPage.addInitScript({
      content: `${mockContent}\nwindow.testHandlers = { union_identity_configuration: () => ({ genesis_hash: "a".repeat(64) }) };\nwindow.discoverServers = [${JSON.stringify(autoServer)}];`,
    });
    await autoPage.goto(`${base}/settings/federation`);
    await autoPage.getByText("已连接", { exact: true }).waitFor();
    const autoCalls = await autoPage.evaluate(() =>
      window.calls.filter((c) => c.cmd === "union_governance")
    );
    assert.equal(autoCalls.at(-1).args.input.peer, "auto-peer");
    assert.equal(autoCalls.at(-1).args.input.address, autoServer.address);
    await autoContext.close();
    // Auto identity setup: discovery also adopts the identity configuration.
    const setupContext = await browser.newContext({
      viewport: { width: 1280, height: 850 },
    });
    const setupPage = await setupContext.newPage();
    setupPage.on("pageerror", (e) => errors.push(e.message));
    await setupPage.addInitScript({
      content: `${mockContent}\nwindow.savedIdentity = null;\nwindow.testHandlers = { union_identity_configuration: (args) => { if (args.config) window.savedIdentity = args.config; return window.savedIdentity; } };\nwindow.discoverServers = [${JSON.stringify(autoServer)}];`,
    });
    await setupPage.goto(`${base}/settings/federation`);
    await setupPage.getByText("已连接", { exact: true }).waitFor();
    const saved = await setupPage.evaluate(() => window.savedIdentity);
    assert.equal(saved.peer, "auto-peer");
    assert.equal(saved.genesis_hash, autoServer.genesis_hash);
    await setupContext.close();
    assert.deepEqual(errors, []);
    console.log(
      "PASS: game routes, connection drafts, reconnect, deep links, room cache, unified application connection, LAN auto-discovery."
    );
  } finally {
    await browser.close();
    rmSync(bootstrap, { force: true });
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
