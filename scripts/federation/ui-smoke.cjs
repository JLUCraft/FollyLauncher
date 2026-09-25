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
        总览: "overview",
        联赛: "league",
        房间与服务器: "rooms",
        我的身份: "identity",
        活动: "events",
        审批: "approvals",
        文件: "files",
        赛季运营: "manage",
      };
      await page.waitForURL(`**/federation/${paths[name]}`);
      if (paths[name] === "overview")
        await page.locator("#office-genesis").waitFor();
      if (paths[name] === "league") await page.locator("#game-link").waitFor();
      if (paths[name] === "rooms")
        await page.getByPlaceholder("搜索房间、社团或版本").waitFor();
    };
    await page.goto(`${base}/federation`);
    await page.waitForURL("**/federation/overview");
    await page
      .locator("#federation-address")
      .fill("/ip4/127.0.0.1/tcp/4001/p2p/test");
    await page.locator("#federation-peer").fill("test");
    await page.locator("#federation-genesis").fill("a".repeat(64));
    await page.locator("#office-genesis").fill("b".repeat(64));
    await page
      .getByRole("button", { name: "连接 / 刷新", exact: true })
      .click();
    await page.getByText("已同步联赛", { exact: true }).waitFor();
    await page
      .getByRole("button", { name: "连接 / 刷新办公", exact: true })
      .click();
    await page.getByText("已同步办公", { exact: true }).waitFor();
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
    await page.getByRole("button", { name: "选择房间", exact: true }).click();
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
    await nav("活动");
    await page.getByRole("button", { name: "刷新", exact: true }).click();
    await page.getByText("暂无记录。", { exact: true }).waitFor();
    await page.evaluate(() => {
      window.office.state.events = {
        event1: {
          title: "Test event",
          brief_hash: "a".repeat(64),
          organizer: "club-a",
          creator: "member-a",
          co_organizers: [],
          phase: "draft",
          epoch: 1,
          window: { opens_ms: 100, closes_ms: 200, starts_ms: 300 },
        },
      };
    });
    await page.getByRole("button", { name: "刷新", exact: true }).click();
    await page.getByText("Test event", { exact: true }).waitFor();
    await page.getByRole("button", { name: "提交审核", exact: true }).click();
    await page.waitForTimeout(100);
    assert.equal(
      await page.evaluate(
        () =>
          window.calls.filter((c) => c.cmd === "union_office_action").at(-1)
            .args.action.type
      ),
      "events.submit"
    );
    await page.evaluate(() => {
      window.office.state.flow_templates = {
        review: {
          active: true,
          versions: [
            {
              name: "Review template",
              stages: [
                {
                  name: "Chair review",
                  approver: { type: "member", member: "member-a" },
                },
              ],
              payload_kinds: ["events.approve"],
            },
          ],
        },
      };
      window.office.state.flows = {
        flow1: {
          template: "review",
          template_epoch: 1,
          payload: {
            type: "events.approve",
            id: "event1",
            epoch: 1,
            co_organizers: [],
          },
          proposer: "member-a",
          current: 0,
          status: "active",
          records: [{ approvals: {}, delegations: {} }],
        },
      };
    });
    await nav("审批");
    await page.getByRole("button", { name: "刷新", exact: true }).click();
    await page.getByText("Chair review", { exact: true }).waitFor();
    await page.getByPlaceholder("审批意见 / 驳回原因").fill("Approved");
    await page.getByRole("button", { name: "同意", exact: true }).click();
    await page.waitForTimeout(100);
    assert.equal(
      await page.getByPlaceholder("审批意见 / 驳回原因").inputValue(),
      ""
    );
    assert.match(
      await page.evaluate(
        () =>
          window.calls.filter((c) => c.cmd === "union_office_action").at(-1)
            .args.action.comment_hash
      ),
      /^[a-f0-9]{64}$/
    );
    await page.evaluate(() => {
      window.office.state.events.event1.phase = "review";
      window.office.state.events.event1.brief_hash = "input-file-cid";
      window.office.state.flow_templates.review.versions[0].stages[0].approver =
        { type: "ai", worker: "worker-member" };
      window.office.commits = {
        input: {
          type: "commit",
          parent: null,
          files: { "brief.txt": { "/": "input-file-cid" } },
          message: "Brief",
        },
      };
    });
    await nav("活动");
    await page.getByRole("button", { name: "刷新", exact: true }).click();
    await page
      .getByRole("combobox", { name: "审批流程", exact: true })
      .selectOption("review");
    await page.getByRole("button", { name: "发起审批", exact: true }).click();
    await page.waitForTimeout(100);
    assert.equal(
      await page.evaluate(
        () =>
          window.calls.filter((c) => c.cmd === "union_office_action").at(-1)
            .args.action.input
      ),
      "input-file-cid"
    );
    await page.evaluate(() => {
      window.office.state.files = {
        branches: {
          main: {
            commit: "new",
            club: "club-a",
            author: "device-a",
            revision: 2,
          },
        },
      };
      window.office.commits = {
        new: {
          type: "commit",
          parent: { "/": "old" },
          files: { "notes.txt": { "/": "newfile" } },
          message: "New",
        },
        old: {
          type: "commit",
          parent: null,
          files: { "notes.txt": { "/": "oldfile" } },
          message: "Old",
        },
      };
    });
    await nav("文件");
    await page.getByRole("button", { name: "刷新", exact: true }).click();
    await page
      .getByRole("combobox", { name: "目录分支", exact: true })
      .selectOption("main");
    await page
      .getByRole("combobox", { name: "版本历史", exact: true })
      .selectOption("old");
    await page.locator("#file-message").fill("Restore notes");
    await page
      .getByRole("button", { name: "恢复为新版本", exact: true })
      .click();
    await page.waitForTimeout(100);
    assert.equal(
      await page.evaluate(
        () =>
          window.calls.filter((c) => c.cmd === "union_office_fetch").at(-1).args
            .cid
      ),
      "oldfile"
    );
    assert.equal(
      await page.evaluate(
        () =>
          window.calls.filter((c) => c.cmd === "union_office_upload").at(-1)
            .args.input.branch
      ),
      "main"
    );
    const uploadInput = await page.evaluate(
      () =>
        window.calls.filter((c) => c.cmd === "union_office_upload").at(-1).args
          .input
    );
    assert.equal(uploadInput.data, "QUJD");
    assert.equal("bytes" in uploadInput, false);
    await page.evaluate(() => {
      window.office.commits.new.files["second.txt"] = { "/": "secondfile" };
      window.delayFetch = true;
    });
    await page.getByRole("button", { name: "刷新", exact: true }).click();
    await page
      .getByRole("combobox", { name: "版本历史", exact: true })
      .selectOption("new");
    await page.getByText("second.txt", { exact: true }).waitFor();
    const downloads = page.locator("button").filter({ hasText: "下载" });
    await downloads.nth(0).click();
    await page.waitForFunction(
      () =>
        !!window.fetchResolvers.newfile &&
        Array.from(document.querySelectorAll("button")).some(
          (button) => button.textContent.includes("下载") && button.disabled
        )
    );
    assert.equal(await downloads.nth(0).isDisabled(), true);
    assert.equal(await downloads.nth(1).isEnabled(), true);
    await page.evaluate(() => {
      window.fetchResolvers.newfile("QUJD");
      window.delayFetch = false;
    });
    await checkCardIsolation(page, nav);
    assert.equal(
      await page.getByRole("button", { name: "AI 助手", exact: true }).count(),
      0
    );
    await nav("总览");
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
    await page.waitForFunction(() =>
      window.calls.some((c) => c.cmd === "union_office_snapshot")
    );
    await nav("联赛");
    await page.locator("#game-link").fill("invalid-link");
    await page
      .getByRole("button", { name: "建立游戏连接", exact: true })
      .click();
    await page
      .getByText("游戏连接未完成，请检查链接或联系组织者", { exact: true })
      .waitFor();
    await nav("总览");
    assert.equal(
      await page
        .getByText("游戏连接未完成，请检查链接或联系组织者", { exact: true })
        .count(),
      0
    );
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
      "PASS: routes, draft isolation, automatic reconnect, global warm/cold deep links, shared game target, room cache, card isolation, approval eligibility, Base64 IPC, independent downloads, CID restore, workflow-only AI."
    );
  } finally {
    await browser.close();
    rmSync(output, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});

async function checkCardIsolation(page, nav) {
  await page.evaluate(() => {
    const state = window.office.state;
    state.events.event2 = {
      ...structuredClone(state.events.event1),
      title: "Second event",
    };
    state.tasks = {
      task1: {
        event: "event1",
        title: "Task one",
        assignee: null,
        done: false,
        due_ms: null,
      },
      task2: {
        event: "event2",
        title: "Task two",
        assignee: null,
        done: false,
        due_ms: null,
      },
    };
    window.office.identity.members["member-b"] = {
      club: "club-a",
      device: "device-b",
      roles: [],
    };
    state.flow_templates.review.versions[0].stages[0].approver = {
      type: "member",
      member: "member-a",
    };
    state.flow_templates.other = structuredClone(state.flow_templates.review);
    state.flow_templates.other.versions[0].stages[0].approver = {
      type: "member",
      member: "member-b",
    };
    state.flows.flow2 = structuredClone(state.flows.flow1);
    state.flows.other = {
      ...structuredClone(state.flows.flow1),
      template: "other",
    };
  });
  await nav("活动");
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  await page.getByText("Second event", { exact: true }).waitFor();
  const templates = page.getByRole("combobox", {
    name: "审批流程",
    exact: true,
  });
  await templates.first().selectOption("review");
  assert.equal(await templates.nth(1).inputValue(), "");
  const sections = page.getByRole("button", { name: "任务", exact: true });
  await sections.nth(0).click();
  await sections.nth(1).click();
  const titles = page.getByPlaceholder("任务名称");
  await titles.nth(0).fill("Only first event");
  assert.equal(await titles.nth(1).inputValue(), "");
  const assignees = page.getByRole("combobox", {
    name: "负责成员",
    exact: true,
  });
  await assignees.nth(0).selectOption("member-b");
  assert.equal(await assignees.nth(1).inputValue(), "");
  await page
    .getByRole("button", { name: "创建活动", exact: true })
    .first()
    .click();
  assert.equal(await page.locator("select#event-brief").count(), 1);
  assert.equal(await page.locator("input#event-brief").count(), 0);
  await nav("审批");
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  const comments = page.getByPlaceholder("审批意见 / 驳回原因");
  await comments.nth(1).waitFor();
  assert.equal(await comments.count(), 2);
  await comments.nth(0).fill("First comment");
  assert.equal(await comments.nth(1).inputValue(), "");
  await comments.nth(1).fill("Second comment");
  await page.getByRole("button", { name: "同意", exact: true }).nth(0).click();
  await page.waitForFunction(
    () =>
      Array.from(
        document.querySelectorAll('input[placeholder="审批意见 / 驳回原因"]')
      )[0].value === ""
  );
  assert.equal(await comments.nth(1).inputValue(), "Second comment");
}
