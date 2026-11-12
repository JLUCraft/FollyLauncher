import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { mkdir, mkdtemp, open, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseRecord, objectValue } from "../json.cjs";

const repo = fileURLToPath(new URL("../../", import.meta.url));
if (process.platform !== "linux")
  throw new Error("Native capture currently requires Linux GTK/X11");
if (!process.env.TUTORIAL_OUTPUT_DIR)
  throw new Error("Set TUTORIAL_OUTPUT_DIR to a new evidence directory");
const out = resolve(process.env.TUTORIAL_OUTPUT_DIR);
await mkdir(out, { recursive: true });
/** @param {number} ms @returns {Promise<void>} */
const delay = (ms) => new Promise((r) => setTimeout(r, ms));
const temp =
  process.env.NATIVE_FOLLOWUP_PROFILE ||
  (await mkdtemp(resolve(tmpdir(), "jlucraft-e2e-desktop-")));
await mkdir(`${temp}/config`, { recursive: true });
await mkdir(`${temp}/data`, { recursive: true });
await mkdir(`${temp}/cache`, { recursive: true });
await mkdir(`${temp}/state`, { recursive: true });
const followup = process.env.NATIVE_FOLLOWUP === "1";
const followupModule = followup
  ? await import("../launcher/native-followup.mjs")
  : undefined;
const fixture = await followupModule?.prepareNativeFixture({
  temp,
  repo,
  reuse: Boolean(process.env.NATIVE_FOLLOWUP_PROFILE),
});
const reservation = createServer();
await new Promise((r, j) => {
  reservation.once("error", j);
  reservation.listen(0, "127.0.0.1", () => r(undefined));
});
const address = reservation.address();
assert.ok(address && typeof address !== "string");
const port = address.port;
await new Promise((r) => reservation.close(r));
const inspector = `127.0.0.1:${port}`;
const log = await open(
  `${out}/${followup ? "native-followup" : "native"}.log`,
  "w",
  0o600
);
const app = spawn(
  process.env.TAURI_BINARY || resolve(repo, "src-tauri/target/debug/SJMCL"),
  [],
  {
    cwd: repo,
    env: {
      ...process.env,
      ...(fixture?.browserBin
        ? {
            PATH: `${fixture.browserBin}:${process.env.PATH}`,
            BROWSER: `${fixture.browserBin}/xdg-open`,
            NATIVE_BROWSER_REQUEST: fixture.browserRequest,
            XDG_CURRENT_DESKTOP: "",
            DE: "generic",
            GTK_USE_PORTAL: "0",
          }
        : {}),
      GDK_BACKEND: "x11",
      XDG_CONFIG_HOME: `${temp}/config`,
      XDG_DATA_HOME: `${temp}/data`,
      XDG_CACHE_HOME: `${temp}/cache`,
      XDG_STATE_HOME: `${temp}/state`,
      WEBKIT_INSPECTOR_HTTP_SERVER: inspector,
    },
    stdio: ["ignore", log.fd, log.fd],
  }
);
const exited = new Promise((resolve) => app.once("exit", resolve));
/** @type {unknown} */
let launchError;
app.on("error", (error) => {
  launchError = error;
});
/** @type {WebSocket | undefined} */
let socket;
let serial = 0;
/** @type {string | undefined} */
let target;
/** @type {Map<string, import("../launcher/types").Target>} */
const targets = new Map();
/** @type {Map<string, WebSocket>} */
const targetSockets = new Map();
/** @type {Map<string, WebSocket>} */
const inspectorSockets = new Map();
/** @type {Map<number, (value: Record<string, unknown>) => void>} */
const pending = new Map();
/** @type {{file: string, step: string, scope: string}[]} */
const shots = [];
try {
  let path;
  for (let i = 0; i < 60; i++) {
    try {
      const html = await (
        await fetch(`http://${inspector}/`, {
          signal: AbortSignal.timeout(2000),
        })
      ).text();
      path = html.match(/\/socket\/[^']+/)?.[0];
      if (path) break;
    } catch {}
    if (launchError) throw launchError;
    if (app.exitCode !== null)
      throw new Error(`Native process exited ${app.exitCode}`);
    await delay(1000);
  }
  assert.ok(path, "Native inspector must become ready");
  socket = new WebSocket(`ws://${inspector}${path}`);
  inspectorSockets.set(path, socket);
  /** @param {MessageEvent<unknown>} event @param {WebSocket} connection */
  const receive = (event, connection) => {
    assert.equal(typeof event.data, "string");
    if (typeof event.data !== "string")
      throw new Error("Inspector text frame required");
    const message = parseRecord(event.data);
    if (message.method === "Target.targetCreated") {
      const info = objectValue(objectValue(message.params).targetInfo);
      if (info.type !== "page") return;
      assert.equal(typeof info.targetId, "string");
      if (typeof info.targetId !== "string")
        throw new Error("Inspector target required");
      const channel = [...inspectorSockets].find(
        ([, owner]) => owner === connection
      )?.[0];
      assert.ok(channel);
      const key = `${channel}:${info.targetId}`;
      targets.set(key, {
        targetId: key,
        protocolTargetId: info.targetId,
        url: typeof info.url === "string" ? info.url : undefined,
        title: typeof info.title === "string" ? info.title : undefined,
        type: "page",
      });
      targetSockets.set(key, connection);
      target ||= key;
    }
    if (message.method === "Target.targetDestroyed") {
      for (const [key, info] of targets)
        if (
          targetSockets.get(key) === connection &&
          info.protocolTargetId === objectValue(message.params).targetId
        )
          targets.delete(key);
    }
    if (message.method === "Target.dispatchMessageFromTarget") {
      const inner = parseRecord(String(objectValue(message.params).message));
      if (typeof inner.id !== "number") return;
      const resolve = pending.get(inner.id);
      if (resolve) {
        pending.delete(inner.id);
        resolve(inner);
      }
    }
  };
  const mainSocket = socket;
  socket.onmessage = (event) => receive(event, mainSocket);
  await new Promise((r, j) => {
    mainSocket.onopen = r;
    mainSocket.onerror = j;
  });
  for (let i = 0; i < 50 && !target; i++) await delay(100);
  assert.ok(target);
  async function refreshTargets() {
    const html = await (await fetch(`http://${inspector}/`)).text();
    const paths = [...new Set(html.match(/\/socket\/[^'"<>\s]+/g) || [])];
    for (const pagePath of paths) {
      if (inspectorSockets.has(pagePath)) continue;
      const connection = new WebSocket(`ws://${inspector}${pagePath}`);
      inspectorSockets.set(pagePath, connection);
      connection.onmessage = (event) => receive(event, connection);
      connection.onclose = () => {
        for (const [id, owner] of targetSockets)
          if (owner === connection) targets.delete(id);
      };
      await new Promise((done, reject) => {
        connection.onopen = done;
        connection.onerror = reject;
      });
      await delay(100);
    }
    return [...targets.values()];
  }
  /** @param {string} expression @param {string} [selectedTarget] @returns {Promise<unknown>} */
  async function evaluate(expression, selectedTarget = target) {
    assert.ok(selectedTarget);
    const targetInfo = targets.get(selectedTarget);
    assert.ok(targetInfo);
    const id = ++serial;
    /** @type {Promise<Record<string, unknown>>} */
    const result = new Promise((r, j) => {
      const timer = setTimeout(() => {
        pending.delete(id);
        j(new Error("Inspector timeout"));
      }, 15000);
      pending.set(id, (value) => {
        clearTimeout(timer);
        r(value);
      });
    });
    const connection = targetSockets.get(selectedTarget) || socket;
    assert.ok(connection);
    connection.send(
      JSON.stringify({
        id: ++serial,
        method: "Target.sendMessageToTarget",
        params: {
          targetId: targetInfo.protocolTargetId,
          message: JSON.stringify({
            id,
            method: "Runtime.evaluate",
            params: { expression, returnByValue: true },
          }),
        },
      })
    );
    const response = await result;
    const resultValue =
      response.result === undefined ? undefined : objectValue(response.result);
    if (response.error || resultValue?.wasThrown)
      throw new Error(JSON.stringify(response));
    return resultValue?.result === undefined
      ? undefined
      : objectValue(resultValue.result).value;
  }
  for (let i = 0; i < 60; i++) {
    if (
      await evaluate(
        "Boolean(window.__TAURI_INTERNALS__ && window.next?.router)"
      )
    )
      break;
    await delay(1000);
  }
  const tree = execFileSync("xwininfo", ["-root", "-tree"], {
    encoding: "utf8",
  });
  await writeFile(
    `${out}/${followup ? "native-followup-windows" : "windows"}.txt`,
    tree
  );
  const candidates = tree
    .split("\n")
    .filter((line) => line.includes('"Folly Launcher"'))
    .map((line) => ({
      id: line.match(/0x[0-9a-f]+/)?.[0],
      size: line.match(/(\d+)x(\d+)[+-]/),
    }))
    .filter(
      (window) =>
        window.id &&
        window.size &&
        Number(window.size[1]) > 400 &&
        Number(window.size[2]) > 300
    );
  const windowId =
    candidates
      .map((window) => window.id)
      .filter((id) => typeof id === "string")
      .find((id) => {
        const pid = execFileSync("xprop", ["-id", id, "_NET_WM_PID"], {
          encoding: "utf8",
        });
        return Number(pid.match(/=\s*(\d+)/)?.[1]) === app.pid;
      }) || "";
  assert.ok(windowId, "Native window must carry the selected Folly brand");
  execFileSync("python3", [
    "-c",
    `import ctypes
x=ctypes.CDLL('libX11.so.6');x.XOpenDisplay.restype=ctypes.c_void_p
x.XMoveResizeWindow.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]
x.XFlush.argtypes=[ctypes.c_void_p]
d=x.XOpenDisplay(None);x.XMoveResizeWindow(d,${Number(windowId)},0,0,1200,780);x.XFlush(d)
`,
  ]);
  await evaluate(
    `window.__TAURI_INTERNALS__.invoke('update_launcher_config',{keyPath:'general.general.language',value:JSON.stringify('zh-Hans')})`
  );
  await delay(1500);
  /** @param {string} name @param {string} step @param {import("../launcher/types").ShotOptions} [options] */
  async function shot(
    name,
    step,
    { allowDialogs = false, nativeWindow = windowId } = {}
  ) {
    await delay(1200);
    if (name !== "desktop-01-start") {
      await evaluate(
        `Array.from(document.querySelectorAll('[role="alertdialog"]')).filter(d => d.textContent.includes('上次运行异常退出')).forEach(d => Array.from(d.querySelectorAll('button')).find(b => b.textContent.trim() === '确定')?.click())`
      );
      await delay(400);
      if (!allowDialogs)
        assert.equal(
          await evaluate(
            `document.querySelectorAll('[aria-modal="true"]').length`
          ),
          0,
          "Capture must not be obscured by a dialog"
        );
    }
    const body = await evaluate("document.body.innerText");
    assert.equal(typeof body, "string");
    if (typeof body !== "string") throw new Error("Page text required");
    assert.ok(
      !body.includes("Unhandled Runtime Error"),
      "Page must not contain a runtime error overlay"
    );
    execFileSync("python3", [
      "-c",
      `import ctypes
x=ctypes.CDLL('libX11.so.6');x.XOpenDisplay.restype=ctypes.c_void_p
x.XRaiseWindow.argtypes=[ctypes.c_void_p,ctypes.c_ulong];x.XFlush.argtypes=[ctypes.c_void_p]
d=x.XOpenDisplay(None);x.XRaiseWindow(d,${Number(nativeWindow)});x.XFlush(d)
`,
    ]);
    await delay(300);
    execFileSync("import", ["-window", nativeWindow, `${out}/${name}.png`]);
    shots.push({
      file: `${name}.png`,
      step,
      scope: "真实 Tauri/WebKit/原生 IPC；未配置业务节点的页面仅验证入口与空态",
    });
    await writeFile(
      `${out}/${followup ? "native-followup-screenshots" : "desktop-manifest"}.json`,
      JSON.stringify(shots, null, 2)
    );
  }
  if (!followup)
    await shot("desktop-01-start", "启动 Folly；确认原生窗口名称与前端品牌");
  // Set only local startup preferences in this disposable profile.
  await evaluate(`void (async () => {
    await window.__TAURI_INTERNALS__.invoke('update_launcher_config',{keyPath:'runCount',value:'1'});
    await window.__TAURI_INTERNALS__.invoke('update_launcher_config',{keyPath:'suppressedDialogs',value:JSON.stringify(['lastExitedAbnormally'])});
    window.__tutorialPrepared = true;
  })()`);
  for (let i = 0; i < 30; i++) {
    if (await evaluate("window.__tutorialPrepared === true")) break;
    await delay(500);
  }
  assert.equal(await evaluate("window.__tutorialPrepared === true"), true);
  await evaluate("location.reload()");
  for (let i = 0; i < 30; i++) {
    await delay(500);
    if (
      await evaluate(
        'Boolean(window.next?.router && document.querySelectorAll("button").length)'
      )
    )
      break;
  }
  await delay(1500);
  await evaluate(
    `Array.from(document.querySelectorAll('button')).filter(b=>['OK','确定','关闭'].includes(b.textContent.trim())).forEach(b=>b.click())`
  );
  if (
    followup &&
    (await evaluate(
      `document.querySelectorAll('[aria-modal="true"]').length > 0`
    ))
  ) {
    await writeFile(
      `${out}/native-followup-startup-dialog.json`,
      JSON.stringify(
        await evaluate(
          `Array.from(document.querySelectorAll('[aria-modal="true"]')).map(d=>({text:d.textContent,buttons:Array.from(d.querySelectorAll('button')).map(b=>({text:b.textContent,label:b.getAttribute('aria-label')}))}))`
        ),
        null,
        2
      )
    );
    await shot(
      "native-startup-dialog",
      "Actual startup modal blocking the new scenarios",
      { allowDialogs: true }
    );
    await evaluate(
      `Array.from(document.querySelectorAll('[role="dialog"] button[aria-label="Close"]')).forEach(b=>b.click())`
    );
  }
  assert.equal(
    await evaluate("document.querySelectorAll('[aria-modal=\"true\"]').length"),
    0,
    "Tutorial pages must not be obscured by a modal"
  );
  if (followup) {
    assert.ok(followupModule && fixture && app.pid);
    await followupModule.nativeFollowup({
      evaluate,
      delay,
      shot,
      temp,
      out,
      repo,
      fixture,
      windowId,
      appPid: app.pid,
      targets: () => [...targets.values()],
      refreshTargets,
    });
  }
  if (!followup) {
    if (process.env.MANAGEMENT_CHECK === "1") {
      const { managementScenario } = await import("./management-scenario.mjs");
      await managementScenario({ evaluate, delay, shot, temp, out, repo });
    }
    if (process.env.LAUNCHER_CHECK === "1") {
      const { launcherScenario } =
        await import("../launcher/native-scenario.mjs");
      await launcherScenario({ evaluate, delay, temp, out });
    }
    await evaluate(`window.next.router.push('/launch')`);
    await shot("desktop-02-launch", "启动页：当前没有配置账号和游戏实例");
    for (const [slug, path, title] of [
      ["accounts", "/accounts", "账号管理"],
      ["instances", "/instances/list", "游戏实例"],
      ["federation", "/settings/federation", "联盟设置"],
      ["rooms", "/instances/rooms", "服务器与房间"],
      ["activities", "/instances/activities", "联赛与组队"],
      ["settings", "/settings/general", "常规设置"],
      ["java", "/settings/java", "Java 环境"],
      ["about", "/settings/about", "Folly 与上游 SJMCL 来源说明"],
    ]) {
      await evaluate(`window.next.router.push(${JSON.stringify(path)})`);
      for (let i = 0; i < 30; i++) {
        if (await evaluate(`location.pathname===${JSON.stringify(path)}`))
          break;
        await delay(500);
      }
      assert.equal(await evaluate("location.pathname"), path);
      await shot(`desktop-${slug}`, title);
      if (slug === "federation") {
        await evaluate(
          `Array.from(document.querySelectorAll('button')).find(b => b.textContent.trim() === '服务器与房间网络设置')?.click()`
        );
        await shot(
          "desktop-federation-settings",
          "设置中的服务器与房间网络参数"
        );
      }
    }
    await writeFile(
      `${out}/desktop-result.json`,
      JSON.stringify(
        {
          status: "passed",
          nativeTitle: "Folly Launcher",
          screenshots: shots.length,
          finishedAt: new Date().toISOString(),
          scope: "入口与原生 IPC 冒烟；业务写入未验收",
        },
        null,
        2
      )
    );
    socket.send(
      JSON.stringify({
        id: ++serial,
        method: "Target.sendMessageToTarget",
        params: {
          targetId: targets.get(target)?.protocolTargetId,
          message: JSON.stringify({
            id: ++serial,
            method: "Runtime.evaluate",
            params: {
              expression: `window.__TAURI_INTERNALS__.invoke('plugin:process|exit',{code:0})`,
            },
          }),
        },
      })
    );
    await Promise.race([exited, delay(3000)]);
    console.log(`PASS: ${shots.length} native desktop screenshots`);
  }
} finally {
  socket?.close();
  for (const connection of inspectorSockets.values()) connection.close();
  if (app.exitCode === null && !launchError) {
    app.kill("SIGTERM");
    await Promise.race([exited, delay(3000)]);
    if (app.exitCode === null) app.kill("SIGKILL");
  }
  await log.close();
  await fixture?.cleanup?.();
  if (!followup) await rm(temp, { recursive: true, force: true });
}
