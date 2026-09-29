// Requires a running Next development server and an available Playwright installation.
const { chromium } = require(
  process.env.PLAYWRIGHT_MODULE || "playwright-core"
);
const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const { mkdtempSync, rmSync } = require("node:fs");
const { tmpdir } = require("node:os");
const path = require("node:path");
const output = mkdtempSync(path.join(tmpdir(), "federation-ui-"));
const bootstrap = path.join(output, "mock-ipc.js");
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
    await page.addInitScript({ path: bootstrap });
    const nav = async (name) => {
      await page.getByRole("button", { name, exact: true }).first().click();
      const paths = {
        连接: "overview",
        联赛: "league",
        房间与服务器: "rooms",
        我的身份: "identity",
        赛季运营: "manage",
      };
      await page.waitForURL(`**/federation/${paths[name]}`);
      if (paths[name] === "league") await page.locator("#game-link").waitFor();
      if (paths[name] === "rooms")
        await page.getByPlaceholder("搜索房间、社团或版本").waitFor();
    };
    await page.goto(`${base}/federation`);
    await page.waitForURL("**/federation/rooms");
    await nav("连接");
    await page
      .locator("#federation-address")
      .fill("/ip4/127.0.0.1/tcp/4001/p2p/test");
    await page.locator("#federation-peer").fill("test");
    await page.locator("#federation-genesis").fill("a".repeat(64));
    await page
      .getByRole("button", { name: "连接 / 刷新", exact: true })
      .click();
    await page.getByText("已同步联赛", { exact: true }).waitFor();
    assert.equal(
      await page.getByRole("button", { name: "赛季运营", exact: true }).count(),
      0
    );
    await nav("房间与服务器");
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
    await page.screenshot({ path: "/tmp/federation-rooms.png" });
    await page.getByRole("button", { name: "加入房间", exact: true }).click();
    await page.getByText(/已连接至 127\.0\.0\.1:12345/).waitFor();
    const discoveries = await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "union_rooms").length
    );
    await nav("联赛");
    await page.locator("#game-link").waitFor();
    await page.getByText("Study build", { exact: true }).waitFor();
    await page.getByText(/服务：room-b/).waitFor();
    await nav("房间与服务器");
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
    await nav("我的身份");
    assert.equal(await page.locator("textarea:visible").count(), 0);
    await nav("联赛");
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
    await page.getByRole("button", { name: "连接设置", exact: true }).click();
    assert.equal(await page.locator("#federation-peer").inputValue(), "test");
    await page.evaluate(() => (window.admin = true));
    await page
      .getByRole("button", { name: "连接 / 刷新", exact: true })
      .click();
    await page.getByRole("button", { name: "赛季运营", exact: true }).waitFor();
    const governanceCalls = await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "union_governance").length
    );
    await page.locator("#federation-peer").fill("unapplied-edit");
    assert.equal(
      await page.getByText("已同步联赛", { exact: true }).count(),
      1
    );
    assert.equal(
      await page.getByRole("button", { name: "赛季运营", exact: true }).count(),
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
        () =>
          JSON.parse(localStorage.getItem("federation-connections")).league.peer
      ),
      "test"
    );
    await nav("赛季运营");
    await page.goto(`${base}/federation/manage`);
    await page
      .getByText("请使用赛季管理员身份连接后管理赛季。", { exact: true })
      .waitFor();
    // A full reload restores confirmed targets, not the unfinished edit.
    assert.equal(
      await page.evaluate(
        () =>
          window.calls.find((c) => c.cmd === "union_governance").args.input.peer
      ),
      "test"
    );
    assert.equal(
      await page.evaluate(() =>
        window.calls.some((c) => c.cmd.startsWith("union_office_"))
      ),
      false
    );
    await nav("联赛");
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
    await page.waitForURL("**/federation/league");
    assert.equal(await page.locator("#game-link").inputValue(), link);
    await page.goto(
      `${base}/launch?test-deep-link=${encodeURIComponent(link)}`
    );
    await page.waitForURL("**/federation/league");
    assert.equal(await page.locator("#game-link").inputValue(), link);
    assert.deepEqual(errors, []);
    console.log(
      "PASS: game routes, connection drafts, reconnect, deep links, room cache, no office calls."
    );
  } finally {
    await browser.close();
    rmSync(output, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
