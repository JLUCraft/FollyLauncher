import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { gzipSync } from "node:zlib";
import { deviceScenario, prepareDeviceFixture } from "./native-device.mjs";
import { modpackScenario } from "./native-modpack.mjs";

const INSTANCE = "NativeAcceptance";
const DIRECTORY = "native-followup";
const EXTENSION = "org.jlucraft.nativeacceptance";
const WORLD_METADATA = "data/minecraft/weather.dat";
const text = (value) => {
  const bytes = Buffer.from(value);
  const size = Buffer.alloc(2);
  size.writeUInt16BE(bytes.length);
  return Buffer.concat([size, bytes]);
};
const tag = (type, name, value) =>
  Buffer.concat([Buffer.from([type]), text(name), value]);
const compound = (...children) =>
  Buffer.concat([...children, Buffer.from([0])]);
const integer = (value) => {
  const b = Buffer.alloc(4);
  b.writeInt32BE(value);
  return b;
};
const long = (value) => {
  const b = Buffer.alloc(8);
  b.writeBigInt64BE(value);
  return b;
};
async function writeWorldMetadata(world) {
  await mkdir(resolve(world, "data/minecraft"), { recursive: true });
  await writeFile(
    resolve(world, WORLD_METADATA),
    gzipSync(
      tag(
        10,
        "",
        compound(tag(8, "Acceptance", text("External metadata file")))
      )
    )
  );
}

export async function prepareNativeFixture({ temp, repo, reuse = false }) {
  const config = JSON.parse(
    execFileSync(
      "bun",
      [
        "-e",
        "import {defaultConfig} from './src/models/config'; process.stdout.write(JSON.stringify(defaultConfig));",
      ],
      { cwd: repo, encoding: "utf8" }
    )
  );
  const identity = JSON.parse(
    await readFile(resolve(repo, "src-tauri/tauri.conf.json"))
  ).identifier;
  const runtime = resolve(temp, "minecraft");
  const versionPath = resolve(runtime, "versions", INSTANCE);
  const world = resolve(versionPath, "saves", "NativeAcceptanceWorld");
  const profile = resolve(temp, "data", identity);
  const device = (process.env.NATIVE_FOLLOWUP_CASES || "")
    .split(",")
    .includes("device")
    ? await prepareDeviceFixture({ temp, repo })
    : {};
  if (reuse) {
    if ((process.env.NATIVE_FOLLOWUP_CASES || "").split(",").includes("world"))
      await writeWorldMetadata(world);
    return {
      runtime,
      versionPath,
      profile,
      extensionZip: resolve(temp, "native-extension.zip"),
      instanceId: `${DIRECTORY}:${INSTANCE}`,
      ...device,
    };
  }
  await mkdir(world, { recursive: true });
  await mkdir(profile, { recursive: true });
  config.mocked = false;
  config.runCount = 2;
  config.localGameDirectories = [{ name: DIRECTORY, dir: runtime }];
  config.download.cache.directory = resolve(temp, "downloads");
  config.general.functionality.autoDownloadJava = false;
  config.general.functionality.resourceTranslation = false;
  config.intelligence.mcpServer.launcher.enabled = false;
  config.suppressedDialogs = ["lastExitedAbnormally"];
  config.globalGameConfig.gameJava = {
    auto: false,
    execPath: "/usr/lib/jvm/java-25-openjdk-amd64/bin/java",
  };
  config.globalGameConfig.launcherVisibility = "always";
  config.globalGameConfig.performance.autoMemAllocation = false;
  config.globalGameConfig.performance.maxMemAllocation = 256;
  config.extraJavaPaths = [config.globalGameConfig.gameJava.execPath];
  config.globalGameConfig.advanced.workaround.useLwjglUnsafeAgent = false;
  config.globalGameConfig.advanced.workaround.gameFileValidatePolicy =
    "disable";
  await writeFile(resolve(profile, "sjmcl.conf.json"), JSON.stringify(config), {
    mode: 0o600,
  });
  await writeFile(
    resolve(profile, "sjmcl.account.json"),
    JSON.stringify({
      players: [],
      authServers: [],
      isOauthProcessing: false,
    }),
    { mode: 0o600 }
  );
  await writeFile(
    resolve(versionPath, `${INSTANCE}.json`),
    JSON.stringify({
      id: INSTANCE,
      version: "1.20.1",
      type: "release",
      mainClass: "jlu.acceptance.DeliberateMissingMain",
      javaVersion: { majorVersion: 21 },
      libraries: [],
      arguments: { game: [], jvm: ["-cp", "${classpath}"] },
      assetIndex: {
        id: "native-acceptance",
        url: "",
        sha1: "",
        size: 0,
        totalSize: 0,
      },
    })
  );
  const extensionZip = resolve(temp, "native-extension.zip");
  // These are local test artifacts, never registered as downloaded Minecraft content.
  execFileSync("python3", [
    "-c",
    `import json,sys,zipfile
with zipfile.ZipFile(sys.argv[1],'w') as z:
 z.writestr('version.json',json.dumps({'id':'1.20.1','name':'1.20.1'}))
with zipfile.ZipFile(sys.argv[2],'w') as z:
 z.writestr('sjmcl.ext.json',json.dumps({'identifier':'${EXTENSION}','name':'Native Acceptance Fixture','frontend':{'entry':'main.js'}}))
 z.writestr('main.js', "window.registerExtension(({React}) => ({settingsPage:{Component:() => React.createElement('div',{'data-native-extension':'executed'},'Native extension executed')}}), document.currentScript.dataset.extensionToken);")
`,
    resolve(versionPath, `${INSTANCE}.jar`),
    extensionZip,
  ]);
  const data = compound(
    tag(8, "LevelName", text("NativeAcceptanceWorld")),
    tag(3, "GameType", integer(0)),
    tag(1, "Difficulty", Buffer.from([2])),
    tag(4, "LastPlayed", long(1700000000000n)),
    tag(4, "RandomSeed", long(9223372036854775807n)),
    tag(
      10,
      "Version",
      compound(tag(8, "Name", text("1.20.1")), tag(3, "Id", integer(3465)))
    )
  );
  const nbt = tag(10, "", compound(tag(10, "Data", data)));
  await writeFile(resolve(world, "level.dat"), gzipSync(nbt));
  await writeWorldMetadata(world);
  return {
    runtime,
    versionPath,
    profile,
    extensionZip,
    instanceId: `${DIRECTORY}:${INSTANCE}`,
    ...device,
  };
}

function nativeKey(windowId, ...keys) {
  execFileSync("python3", [
    "-c",
    `import ctypes,sys,time
x=ctypes.CDLL('libX11.so.6');t=ctypes.CDLL('libXtst.so.6')
x.XOpenDisplay.restype=ctypes.c_void_p;x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong]
x.XStringToKeysym.argtypes=[ctypes.c_char_p];x.XStringToKeysym.restype=ctypes.c_ulong
x.XSetInputFocus.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong];x.XFlush.argtypes=[ctypes.c_void_p]
x.XSync.argtypes=[ctypes.c_void_p,ctypes.c_int]
t.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
d=x.XOpenDisplay(None);assert d
x.XSetInputFocus(d,int(sys.argv[1],0),1,0)
for key in sys.argv[2:]:
 code=x.XKeysymToKeycode(d,x.XStringToKeysym(key.encode()));assert code
 t.XTestFakeKeyEvent(d,code,1,0);x.XSync(d,0);time.sleep(.06);t.XTestFakeKeyEvent(d,code,0,0);x.XSync(d,0);time.sleep(.12)
`,
    windowId,
    ...keys,
  ]);
}

function nativeClick(windowId, point) {
  execFileSync("python3", [
    "-c",
    `import ctypes,sys,time
x=ctypes.CDLL('libX11.so.6');t=ctypes.CDLL('libXtst.so.6');x.XOpenDisplay.restype=ctypes.c_void_p
x.XDefaultRootWindow.argtypes=[ctypes.c_void_p];x.XDefaultRootWindow.restype=ctypes.c_ulong
x.XTranslateCoordinates.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.POINTER(ctypes.c_int),ctypes.POINTER(ctypes.c_int),ctypes.POINTER(ctypes.c_ulong)]
x.XRaiseWindow.argtypes=[ctypes.c_void_p,ctypes.c_ulong];x.XSetInputFocus.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong];x.XSync.argtypes=[ctypes.c_void_p,ctypes.c_int]
t.XTestFakeMotionEvent.argtypes=[ctypes.c_void_p,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_ulong];t.XTestFakeButtonEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
d=x.XOpenDisplay(None);assert d;w=int(sys.argv[1],0);a=ctypes.c_int();b=ctypes.c_int();child=ctypes.c_ulong()
x.XRaiseWindow(d,w);x.XSetInputFocus(d,w,1,0);x.XTranslateCoordinates(d,w,x.XDefaultRootWindow(d),int(sys.argv[2]),int(sys.argv[3]),ctypes.byref(a),ctypes.byref(b),ctypes.byref(child))
t.XTestFakeMotionEvent(d,-1,a.value,b.value,0);x.XSync(d,0);t.XTestFakeButtonEvent(d,1,1,0);x.XSync(d,0);time.sleep(.08);t.XTestFakeButtonEvent(d,1,0,0);x.XSync(d,0)
`,
    windowId,
    String(Math.round(point.x)),
    String(Math.round(point.y)),
  ]);
}

export async function nativeFollowup({
  evaluate,
  delay,
  shot,
  temp,
  out,
  repo,
  fixture,
  windowId,
  appPid,
  targets,
  refreshTargets,
}) {
  const result = {
    startedAt: new Date().toISOString(),
    profileScope:
      "Disposable config/data/cache/state and game runtime outside the workspace; private D-Bus session.",
    reused: [
      "The five earlier native checks",
      "Frontend static checks, locale comparisons, discovery races",
    ],
    checks: [],
    sources: {},
  };
  result.profilePath = temp;
  const previous = await readFile(
    resolve(out, "native-followup-result.json"),
    "utf8"
  )
    .then(JSON.parse)
    .catch(() => null);
  if (previous) {
    result.checks = previous.checks || [];
    result.history = previous.history || [];
    if (previous.policy) result.policy = previous.policy;
  }
  for (const path of [
    "scripts/federation/capture-tutorial.mjs",
    "scripts/launcher/native-followup.mjs",
    "scripts/launcher/native-device.mjs",
    "scripts/launcher/native-station-fixture.mjs",
    "src/components/modals/world-level-data-modal.tsx",
    "src/pages/standalone/game-error.tsx",
    "src-tauri/src/launch/helpers/process_monitor.rs",
    "src-tauri/src/instance/commands.rs",
    "src-tauri/src/instance/helpers/misc.rs",
    "src-tauri/src/lib.rs",
    "src-tauri/src/utils/web.rs",
    "src-tauri/src/tasks/commands.rs",
    "src-tauri/src/tasks/monitor.rs",
    "../minecraftd/src/minecraft/dependencies.rs",
    "../minecraftd/src/minecraft/loader/processors.rs",
    "src/contexts/extension/host.tsx",
  ])
    result.sources[path] = createHash("sha256")
      .update(await readFile(resolve(repo, path)))
      .digest("hex");
  const selected = new Set(
    (
      process.env.NATIVE_FOLLOWUP_CASES ||
      "world,crash,keyboard,download,extension,search,cleanroom"
    ).split(",")
  );
  let serial = 0;
  async function invoke(command, args = {}, timeout = 60000) {
    const key = `__nativeFollowup${++serial}`;
    await evaluate(
      `void window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)}).then(value=>window.${key}={value},error=>window.${key}={error:String(error)})`
    );
    const deadline = Date.now() + timeout;
    while (Date.now() < deadline) {
      const value = await evaluate(`window.${key}`);
      if (value) {
        await evaluate(`delete window.${key}`);
        if (value.error) throw new Error(`${command}: ${value.error}`);
        return value.value;
      }
      await delay(150);
    }
    throw new Error(`${command}: timeout after ${timeout}ms`);
  }
  async function until(expression, timeout = 30000, page) {
    const deadline = Date.now() + timeout;
    while (Date.now() < deadline) {
      const value = await evaluate(expression, page);
      if (value) return value;
      await delay(150);
    }
    throw new Error(`Timed out waiting for ${expression}`);
  }
  async function route(path) {
    await evaluate(`window.next.router.push(${JSON.stringify(path)})`);
    await until(`location.pathname===${JSON.stringify(path.split("?")[0])}`);
    await delay(500);
  }
  async function clickElement(expression) {
    const point = await evaluate(
      `(() => {const element=${expression};element.scrollIntoView({block:'center'});const r=element.getBoundingClientRect();return {x:(r.left+r.width/2)*devicePixelRatio,y:(r.top+r.height/2)*devicePixelRatio};})()`
    );
    nativeClick(windowId, point);
    await delay(300);
  }
  function findChildWindow(pid = appPid) {
    return execFileSync("xwininfo", ["-root", "-tree"], { encoding: "utf8" })
      .split("\n")
      .filter(
        (line) =>
          line.includes('"') &&
          line.match(/(?:[6-9]\d\d|\d{4})x(?:[4-9]\d\d|\d{4})[+-]/)
      )
      .map((line) => line.match(/0x[0-9a-f]+/)?.[0])
      .filter((id) => id && id !== windowId)
      .find(
        (id) =>
          Number(
            execFileSync("xprop", ["-id", id, "_NET_WM_PID"], {
              encoding: "utf8",
            }).match(/=\s*(\d+)/)?.[1]
          ) === pid
      );
  }
  async function record(key, action) {
    if (!selected.has(key)) return;
    const check = { name: key, startedAt: new Date().toISOString() };
    try {
      check.detail = await action();
      check.status = "passed";
    } catch (error) {
      check.status = "failed";
      check.error = String(error);
      check.body = await evaluate("document.body.innerText").catch(
        () => "unavailable"
      );
      await shot(`native-${key}-failure`, `Failure evidence for ${key}`, {
        allowDialogs: true,
      }).catch(() => {});
      nativeKey(windowId, "Escape");
    }
    const prior = result.checks.find((c) => c.name === key);
    if (prior) {
      result.history ||= [];
      result.history.push(prior);
    }
    result.checks = result.checks.filter((c) => c.name !== key);
    check.sourceHashes = { ...result.sources };
    check.nativeProcessPid = appPid;
    check.nativeBinarySha256 = createHash("sha256")
      .update(await readFile(`/proc/${appPid}/exe`))
      .digest("hex");
    result.checks.push(check);
    await writeFile(
      resolve(out, "native-followup-result.json"),
      JSON.stringify(result, null, 2)
    );
    console.log(
      `${check.status.toUpperCase()}: ${key}${check.error ? `: ${check.error}` : ""}`
    );
  }
  await invoke("retrieve_instance_list");
  await record("world", async () => {
    const details = await invoke("retrieve_world_details", {
      instanceId: fixture.instanceId,
      worldName: "NativeAcceptanceWorld",
    });
    assert.ok(JSON.stringify(details).includes("9223372036854775807"));
    assert.ok(Object.keys(details).includes(WORLD_METADATA));
    await route(
      `/instances/details/${encodeURIComponent(fixture.instanceId)}/worlds`
    );
    const label = JSON.parse(
      await readFile(resolve(repo, "src/locales/zh-Hans.json"))
    ).InstanceWorldsPage.worldList.viewLevelData;
    await until(
      `Array.from(document.querySelectorAll('button')).some(b => (b.getAttribute('aria-label')||b.title||b.textContent).includes(${JSON.stringify(label)}))`
    );
    await evaluate(
      `Array.from(document.querySelectorAll('button')).find(b => (b.getAttribute('aria-label')||b.title||b.textContent).includes(${JSON.stringify(label)})).click()`
    );
    await until(
      `Boolean(document.querySelector('[role="dialog"][aria-modal="true"]')?.textContent.includes('level.dat'))`
    );
    assert.ok(
      await evaluate(
        `document.querySelector('[role="dialog"][aria-modal="true"]').textContent.includes(${JSON.stringify(WORLD_METADATA)})`
      )
    );
    await shot(
      "native-world-details",
      "Actual world metadata dialog and external NBT file",
      { allowDialogs: true }
    );
    nativeKey(windowId, "Escape");
    return {
      files: Object.keys(details),
      precision: "9223372036854775807",
      source:
        "Controlled NBT files parsed by actual native IPC; no injected API results.",
    };
  });
  await record("crash", async () => {
    await route("/settings/global-game/advanced");
    const labels = JSON.parse(
      await readFile(resolve(repo, "src/locales/zh-Hans.json"))
    ).GameAdvancedSettingsPage.workaround.settings.autoCloseGameCrashWindow;
    await until(
      `document.body.textContent.includes(${JSON.stringify(labels.title)})`
    );
    await clickElement(
      `Array.from(document.querySelectorAll('button[aria-haspopup="menu"]')).find(b=>Object.values(${JSON.stringify(labels)}).includes(b.textContent.trim()))`
    );
    await until(
      `Array.from(document.querySelectorAll('[role="menuitemradio"]')).some(b => b.textContent.includes(${JSON.stringify(labels.thirtySeconds)}))`
    );
    await clickElement(
      `Array.from(document.querySelectorAll('[role="menuitemradio"]')).find(b => b.textContent.includes(${JSON.stringify(labels.thirtySeconds)}))`
    );
    await delay(350);
    assert.equal(
      (await invoke("retrieve_launcher_config")).globalGameConfig.advanced
        .workaround.autoCloseGameCrashWindow,
      "thirtySeconds"
    );
    await route("/settings/general");
    await route("/settings/global-game/advanced");
    assert.ok(
      await evaluate(
        `document.body.textContent.includes(${JSON.stringify(labels.thirtySeconds)})`
      )
    );
    let players = await invoke("retrieve_player_list");
    if (!players.some((p) => p.name === "NativeAcceptance")) {
      await route("/accounts?add=1");
      await until(
        `Boolean(document.querySelector('[aria-modal="true"] input'))`
      );
      await evaluate(
        `(() => {const input=document.querySelector('[aria-modal="true"] input');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,'NativeAcceptance');input.dispatchEvent(new Event('input',{bubbles:true}));})()`
      );
      await clickElement(
        `Array.from(document.querySelectorAll('[aria-modal="true"] button')).find(b=>b.textContent.trim()==='确定')`
      );
      await until(
        `document.querySelectorAll('[aria-modal="true"]').length===0`
      );
      players = await invoke("retrieve_player_list");
    }
    const offline = players.find((p) => p.name === "NativeAcceptance");
    assert.equal(offline?.playerType, "offline");
    const duplicate = await invoke("add_player_offline", {
      username: "NativeAcceptance",
      uuid: "",
    }).then(
      () => null,
      (error) => String(error)
    );
    assert.ok(
      duplicate?.includes("DUPLICATE"),
      "Duplicate-account validation must remain active"
    );
    const firstId = Math.floor(Date.now() / 1000);
    await invoke("select_suitable_jre", { instanceId: fixture.instanceId });
    let launching;
    for (let id = Math.floor(Date.now() / 1000); id >= firstId; id--) {
      try {
        launching = await invoke("retrieve_game_launching_state", {
          launchingId: id,
        });
        if (launching) break;
      } catch {}
    }
    assert.ok(
      launching,
      "The native JRE selection must produce a real launching state"
    );
    assert.equal(await invoke("validate_selected_player", {}, 90000), true);
    const before = new Set(targets().map((t) => t.targetId));
    await invoke("launch_game", {}, 60000);
    let page;
    for (let i = 0; i < 120; i++) {
      await refreshTargets();
      page = targets().find((t) => !before.has(t.targetId));
      if (page) break;
      await delay(250);
    }
    assert.ok(
      page,
      "The actual failing Java process must automatically open the error page"
    );
    await until(
      `document.body.textContent.includes('秒后自动关闭')`,
      30000,
      page.targetId
    );
    const countdown = await evaluate("document.body.innerText", page.targetId);
    const errorWindow = findChildWindow();
    assert.ok(errorWindow, "Locate only this isolated process's error window");
    await shot(
      "native-crash-countdown",
      "Real native error page and countdown from its launching state",
      { nativeWindow: errorWindow, allowDialogs: true }
    );
    nativeClick(errorWindow, { x: 180, y: 160 });
    await until(
      `!document.body.textContent.includes('秒后自动关闭')`,
      5000,
      page.targetId
    );
    await delay(31000);
    assert.ok(
      targets().some((t) => t.targetId === page.targetId),
      "Click cancellation must keep the error window open beyond its countdown"
    );
    await evaluate(
      `window.__TAURI_INTERNALS__.invoke('plugin:window|close')`,
      page.targetId
    );
    await route("/settings/global-game/advanced");
    await shot(
      "native-crash-settings",
      "Crash policy persisted after reopening settings"
    );
    return {
      policy: "thirtySeconds",
      countdown,
      cancellation:
        "Real XTest mouse click canceled the automatically opened error page for over 30 seconds.",
      fixture:
        "Real offline account admitted through the UI; a real Java process launched by Tauri failed with the missing fixture main class. This is not a successful Minecraft gameplay launch.",
      processLog: (
        await invoke("retrieve_game_log", { launchingId: launching.id })
      ).filter(
        (line) =>
          line.includes("DeliberateMissingMain") ||
          line.includes("ClassNotFound")
      ),
      offlineAdmission:
        "First local offline account was added through the actual native UI; duplicate rejection retained.",
    };
  });
  await record("device", () =>
    deviceScenario({
      fixture,
      invoke,
      evaluate,
      route,
      until,
      shot,
      delay,
      targets,
      refreshTargets,
      findChildWindow,
      clickElement,
    })
  );
  await record("repair", async () => {
    let instance;
    const readyDeadline = Date.now() + 120000;
    while (Date.now() < readyDeadline) {
      instance = (await invoke("retrieve_instance_list")).find(
        (i) => i.name === "NativeCleanroomAcceptance"
      );
      if (instance?.modLoader.status === "Installed") break;
      await delay(400);
    }
    assert.equal(instance?.modLoader.status, "Installed");
    const prior = JSON.parse(
      await readFile(
        resolve(
          repo,
          "src-tauri/target/native-cleanroom-artifact-receipt.json"
        ),
        "utf8"
      )
    );
    const clientPath = resolve(instance.versionPath, `${instance.name}.jar`);
    const beforeClient = createHash("sha256")
      .update(await readFile(clientPath))
      .digest("hex");
    const beforeGroups = new Set(
      (await invoke("retrieve_progressive_task_list")).map((t) => t.taskGroup)
    );
    await invoke(
      "change_mod_loader",
      {
        instanceId: instance.id,
        newModLoader: {
          loaderType: instance.modLoader.loaderType,
          version: instance.modLoader.version,
          description: "Reprocess the current installed loader metadata",
          stable: null,
          branch: instance.modLoader.branch,
        },
        isInstallFabricApi: false,
        isInstallQfApi: false,
      },
      90000
    );
    const concurrentRefreshCalls = 4;
    await Promise.all(
      Array.from({ length: concurrentRefreshCalls }, () => invoke("retrieve_instance_list"))
    );
    const deadline = Date.now() + 240000;
    let groups = [];
    let installed;
    while (Date.now() < deadline) {
      installed = (await invoke("retrieve_instance_list", {}, 60000)).find(
        (i) => i.id === instance.id
      );
      assert.notEqual(installed?.modLoader.status, "DownloadFailed");
      groups = (await invoke("retrieve_progressive_task_list")).filter(
        (t) => !beforeGroups.has(t.taskGroup)
      );
      assert.ok(
        !groups.some((t) => t.status === "Failed"),
        JSON.stringify(groups)
      );
      if (groups.length && groups.every((t) => t.status === "Completed")) {
        const pending = (await invoke("retrieve_progressive_task_list")).filter(
          (t) => !beforeGroups.has(t.taskGroup) && t.status !== "Completed"
        );
        if (installed?.modLoader.status === "Installed" && !pending.length)
          break;
      }
      await delay(400);
    }
    assert.equal(
      installed?.modLoader.status,
      "Installed",
      JSON.stringify(groups.map(({ taskGroup, status }) => ({ taskGroup, status })))
    );
    const manifest = JSON.parse(
      await readFile(
        resolve(instance.versionPath, `${instance.name}.json`),
        "utf8"
      )
    );
    const nativeRows = manifest.libraries.filter(
      (l) => l.natives && l.downloads?.classifiers
    );
    assert.ok(
      nativeRows.some(
        (l) => l.name.startsWith("org.lwjgl:") && l.natives.linux
      ),
      "Actual installer processing must preserve the missing native rows"
    );
    groups = (await invoke("retrieve_progressive_task_list")).filter(
      (t) => !beforeGroups.has(t.taskGroup)
    );
    const libraryGroups = groups.filter((group) =>
      group.taskGroup.startsWith(`cleanroom-libraries?${instance.id}@`)
    );
    assert.equal(libraryGroups.length, 1, "Concurrent refresh must plan one library group");
    const downloads = [];
    for (const file of await readdir(resolve(temp, "downloads"))) {
      if (!/^task-\d+\.json$/.test(file)) continue;
      const task = JSON.parse(
        await readFile(resolve(temp, "downloads", file), "utf8")
      );
      if (groups.some((group) => group.taskGroup === task.taskGroup))
        downloads.push({
          group: task.taskGroup,
          ...task.payload,
          status: task.status,
        });
    }
    const nativeDownloads = downloads.filter((d) =>
      d.dest.includes("natives-linux")
    );
    const nativeArtifacts = [];
    for (const library of nativeRows.filter(
      (row) =>
        row.name.startsWith("org.lwjgl:") &&
        row.natives.linux === "natives-linux"
    )) {
      const [group, name, version] = library.name.split(":");
      const path = `${group.replaceAll(".", "/")}/${name}/${version}/${name}-${version}-natives-linux.jar`;
      const bytes = await readFile(resolve(fixture.runtime, "libraries", path));
      const sha1 = createHash("sha1").update(bytes).digest("hex");
      assert.equal(sha1, library.downloads.classifiers["natives-linux"].sha1);
      nativeArtifacts.push({ path, sha1, bytes: bytes.length });
    }
    assert.ok(nativeArtifacts.length);
    assert.equal(
      createHash("sha256")
        .update(await readFile(clientPath))
        .digest("hex"),
      beforeClient
    );
    const installerProof = [];
    for (const entry of prior.installerJars) {
      const p = resolve(fixture.runtime, entry.path);
      const sha = createHash("sha256")
        .update(await readFile(p))
        .digest("hex");
      assert.equal(sha, entry.sha256);
      installerProof.push({ path: entry.path, sha256: sha });
    }
    return {
      version: installed.modLoader.version,
      entry:
        "Actual change_mod_loader and native task-controller library processing; no final manifest assembled by the fixture",
      interfaceConstraint:
        "The existing public entry schedules its installer prerequisite again. This one affected prerequisite is retained; no new product cache feature was introduced.",
      clientJarUnchanged: beforeClient,
      installerProof,
      nativeRowCount: nativeRows.length,
      concurrentRefreshCalls,
      libraryTaskGroups: libraryGroups.map((group) => group.taskGroup),
      downloads,
      downloadedNativeArtifacts: nativeDownloads.length,
      verifiedNativeArtifacts: nativeArtifacts,
      manifestSha256: createHash("sha256")
        .update(
          await readFile(resolve(instance.versionPath, `${instance.name}.json`))
        )
        .digest("hex"),
    };
  });
  await record("game", async () => {
    if (selected.has("repair"))
      assert.equal(
        result.checks.find((c) => c.name === "repair")?.status,
        "passed",
        "Do not repeat a game launch before the actual library repair succeeds"
      );
    const instances = await invoke("retrieve_instance_list");
    const instance = instances.find(
      (i) =>
        i.name === "NativeCleanroomAcceptance" &&
        i.modLoader.status === "Installed"
    );
    assert.ok(
      instance,
      "Reuse the already installed Cleanroom instance without reinstalling it"
    );
    const offline = (await invoke("retrieve_player_list")).find(
      (p) => p.name === "NativeAcceptance" && p.playerType === "offline"
    );
    assert.ok(
      offline,
      "Use the actual local offline account created through the UI"
    );
    result.gameStartup = {
      instanceId: instance.id,
      stage: "select_jre",
      scope:
        "One offline startup of the existing complete installation; no public multiplayer or dual-client rendering claim",
    };
    const priorMemory = (await invoke("retrieve_launcher_config"))
      .globalGameConfig.performance.maxMemAllocation;
    let launching;
    try {
      await invoke("update_launcher_config", {
        keyPath: "states.shared.selectedPlayerId",
        value: JSON.stringify(offline.id),
      });
      await invoke("update_launcher_config", {
        keyPath: "globalGameConfig.performance.maxMemAllocation",
        value: "1024",
      });
      const start = Math.floor(Date.now() / 1000);
      await invoke("select_suitable_jre", { instanceId: instance.id });
      for (let id = Math.floor(Date.now() / 1000); id >= start; id--) {
        try {
          launching = await invoke("retrieve_game_launching_state", {
            launchingId: id,
          });
          if (launching) break;
        } catch {}
      }
      assert.ok(launching);
      result.gameStartup.launchingId = launching.id;
      result.gameStartup.stage = "prepare_existing_game_files";
      await invoke("validate_game_files", {}, 90000);
      result.gameStartup.stage = "validate_offline_player";
      assert.equal(await invoke("validate_selected_player", {}, 90000), true);
      result.gameStartup.stage = "launch_installed_game";
      await invoke("launch_game", {}, 150000);
      const running = await invoke("retrieve_game_launching_state", {
        launchingId: launching.id,
      });
      assert.ok(running.pid > 0);
      result.gameStartup.javaPid = running.pid;
      let gameWindow;
      for (let i = 0; i < 150; i++) {
        gameWindow = findChildWindow(running.pid);
        if (gameWindow) break;
        await delay(400);
      }
      assert.ok(
        gameWindow,
        "Actual Minecraft Java process must create its own window"
      );
      await delay(10000);
      assert.ok(
        findChildWindow(running.pid),
        "Game window must remain alive during the startup smoke check"
      );
      await shot(
        "native-cleanroom-startup-notice",
        "Cleanroom startup notice before continuing to the game menu",
        { nativeWindow: gameWindow, allowDialogs: true }
      );
      nativeClick(gameWindow, { x: 320, y: 240 });
      await delay(300);
      nativeKey(gameWindow, "space");
      await delay(15000);
      assert.ok(findChildWindow(running.pid), "Game must remain alive after continuing");
      const logs = await invoke("retrieve_game_log", {
        launchingId: launching.id,
      });
      assert.ok(logs.some((line) => /Minecraft|Cleanroom|LWJGL/.test(line)));
      await shot(
        "native-offline-minecraft",
        "Actual installed Cleanroom Minecraft process and window; no authenticated or multiplayer session",
        { nativeWindow: gameWindow, allowDialogs: true }
      );
      result.gameStartup.stage = "window_running";
      return {
        instance: instance.name,
        loader: instance.modLoader,
        java: running.selectedJava,
        javaPid: running.pid,
        window: gameWindow,
        logTail: logs.slice(-35),
        renderingReview:
          "Screenshot requires visual review; no gameplay or public joining claim",
      };
    } finally {
      if (launching) {
        result.gameStartup.logTail = await invoke("retrieve_game_log", {
          launchingId: launching.id,
        })
          .then((logs) => logs.slice(-45))
          .catch(() => []);
        await invoke("cancel_launch_process").catch(() => {});
      }
      await invoke("update_launcher_config", {
        keyPath: "globalGameConfig.performance.maxMemAllocation",
        value: JSON.stringify(priorMemory),
      });
    }
  });
  await record("restart", async () => {
    const prior = previous?.checks?.find(
      (c) => c.name === "crash" && c.status === "passed"
    );
    assert.ok(
      prior?.nativeProcessPid && prior.nativeProcessPid !== appPid,
      "A distinct native process must reopen the same profile"
    );
    const config = await invoke("retrieve_launcher_config");
    assert.equal(
      config.globalGameConfig.advanced.workaround.autoCloseGameCrashWindow,
      "thirtySeconds"
    );
    const players = await invoke("retrieve_player_list");
    assert.ok(players.some((p) => p.name === "NativeAcceptance"));
    const device = previous.checks.find(
      (c) => c.name === "device" && c.status === "passed"
    );
    if (device) {
      assert.notEqual(device.nativeProcessPid, appPid);
      assert.ok(
        players.some(
          (p) =>
            p.id === device.detail.playerId &&
            p.name === "NativeDevice" &&
            p.playerType === "3rdparty"
        )
      );
    }
    await route("/accounts");
    await until("document.body.textContent.includes('NativeAcceptance')");
    await shot(
      "native-process-restart",
      "A distinct native process restored actual saved accounts and settings"
    );
    return {
      previousNativePid: prior.nativeProcessPid,
      currentNativePid: appPid,
      scope:
        "Full launcher process restart with persisted accounts/configuration. Interrupted download recovery and OS reboot are not claimed.",
    };
  });
  await record("modpack", () =>
    modpackScenario({ fixture, temp, invoke, route, shot, delay })
  );
  await record("keyboard", async () => {
    await route("/settings/general");
    const selector =
      "Array.from(document.querySelectorAll('input[type=checkbox]')).find(e=>!e.disabled)";
    await until(`Boolean(${selector})`);
    const original = await evaluate(`${selector}.checked`);
    await clickElement(`${selector}.closest('label')`);
    const before = await evaluate(`${selector}.checked`);
    await evaluate(`${selector}.focus()`);
    nativeKey(windowId, "space");
    await delay(300);
    assert.equal(await evaluate(`${selector}.checked`), !before);
    nativeKey(windowId, "space");
    await delay(250);
    assert.equal(await evaluate(`${selector}.checked`), before);
    await clickElement(`${selector}.closest('label')`);
    assert.equal(await evaluate(`${selector}.checked`), original);
    nativeKey(windowId, "Tab");
    assert.ok(await evaluate("document.activeElement !== document.body"));
    await shot(
      "native-keyboard",
      "Real XTest Space and Tab events in GTK/WebKit"
    );
    return {
      events: ["Space toggle", "Space restore", "Tab focus traversal"],
      transport:
        "XTest into the isolated process's X11 window, not synthetic DOM KeyboardEvent.",
    };
  });
  await record("download", async () => {
    const bytes = Buffer.alloc(4 * 1024 * 1024, 83);
    const requests = [];
    const intervals = new Set();
    const server = createServer((req, res) => {
      const start = Number(req.headers.range?.match(/bytes=(\d+)-/)?.[1] || 0);
      requests.push({ method: req.method, range: req.headers.range || null });
      const headers = {
        "Content-Length": bytes.length - start,
        "Accept-Ranges": "bytes",
        ETag: "native-followup",
      };
      if (start)
        headers["Content-Range"] =
          `bytes ${start}-${bytes.length - 1}/${bytes.length}`;
      res.writeHead(start ? 206 : 200, headers);
      if (req.method === "HEAD") {
        res.end();
        return;
      }
      let offset = start;
      const timer = setInterval(() => {
        const end = Math.min(offset + 65536, bytes.length);
        res.write(bytes.subarray(offset, end));
        offset = end;
        if (offset === bytes.length) {
          clearInterval(timer);
          intervals.delete(timer);
          res.end();
        }
      }, 70);
      intervals.add(timer);
      res.on("close", () => {
        clearInterval(timer);
        intervals.delete(timer);
      });
    });
    await new Promise((r) => server.listen(0, "127.0.0.1", r));
    const group = "native-followup-download";
    const destination = resolve(temp, "paused-download.bin");
    const schedule = () =>
      invoke("schedule_progressive_task_group", {
        taskGroup: group,
        withTimestamp: false,
        params: [
          {
            taskType: "download",
            src: `http://127.0.0.1:${server.address().port}/fixture`,
            dest: destination,
            sha1: createHash("sha1").update(bytes).digest("hex"),
          },
        ],
      });
    const state = async () =>
      (await invoke("retrieve_progressive_task_list")).find(
        (t) => t.taskGroup === group
      );
    try {
      await schedule();
      let item;
      for (let i = 0; i < 100; i++) {
        item = await state();
        if (item?.taskDescs.some((t) => t.current > 0)) break;
        await delay(100);
      }
      assert.ok(item);
      await invoke("stop_progressive_task_group", { taskGroup: group });
      await delay(350);
      const paused = await state();
      assert.equal(paused.status, "Stopped");
      const progress = paused.taskDescs.map((t) => t.current);
      await delay(600);
      assert.deepEqual(
        (await state()).taskDescs.map((t) => t.current),
        progress
      );
      await route("/downloads");
      await shot(
        "native-download-paused",
        "Native task stopped through the production task controller"
      );
      await invoke("resume_progressive_task_group", { taskGroup: group });
      for (let i = 0; i < 200; i++) {
        item = await state();
        if (["Completed", "Failed"].includes(item?.status)) break;
        await delay(100);
      }
      assert.equal(item?.status, "Completed", JSON.stringify(item));
      assert.deepEqual(await readFile(destination), bytes);
      await invoke("delete_progressive_task_group", { taskGroup: group });
      const cancelGroup = "native-followup-cancel";
      await invoke("schedule_progressive_task_group", {
        taskGroup: cancelGroup,
        withTimestamp: false,
        params: [
          {
            taskType: "download",
            src: `http://127.0.0.1:${server.address().port}/cancel`,
            dest: resolve(temp, "cancel-download.bin"),
          },
        ],
      });
      await delay(300);
      await invoke("cancel_progressive_task_group", { taskGroup: cancelGroup });
      await delay(500);
      assert.ok(
        !(await invoke("retrieve_progressive_task_list")).some(
          (t) => t.taskGroup === cancelGroup
        )
      );
      return {
        bytes: bytes.length,
        pausedProgress: progress,
        sha1: createHash("sha1").update(bytes).digest("hex"),
        requests,
        cancellation: "Canceled task removed; no completed output claimed.",
      };
    } finally {
      for (const timer of intervals) clearInterval(timer);
      server.closeAllConnections();
      await new Promise((r) => server.close(r));
    }
  });
  await record("extension", async () => {
    await invoke("add_extension", {
      path: fixture.extensionZip,
      expectedIdentifier: EXTENSION,
    });
    await invoke("update_launcher_config", {
      keyPath: "extension.enabled",
      value: JSON.stringify([EXTENSION]),
    });
    await evaluate("location.reload()");
    await until("Boolean(window.next?.router?.isReady)");
    await route(`/settings/extension/${EXTENSION}`);
    await until(
      `Boolean(document.querySelector('[data-native-extension="executed"]'))`
    );
    await shot(
      "native-extension-executed",
      "Installed local extension executed through the existing host and asset protocol"
    );
    const extensions = await invoke("retrieve_extension_list");
    assert.ok(extensions.some((e) => e.identifier === EXTENSION));
    return {
      identifier: EXTENSION,
      execution:
        "Local package installed through native add_extension; its registered settings Component rendered in actual WebKit.",
    };
  });
  await record("search", async () => {
    const query = {
      resourceType: "mod",
      searchQuery: "sodium",
      gameVersion: "",
      selectedTag: "All",
      sortBy: "relevance",
      page: 0,
      pageSize: 5,
    };
    const response = await invoke(
      "fetch_resource_list_by_name",
      { downloadSource: "Modrinth", query },
      90000
    );
    assert.ok(
      response.list?.length || response.resources?.length || response.total > 0,
      JSON.stringify(response)
    );
    await route("/discover/resource?type=mod");
    await clickElement(
      `Array.from(document.querySelectorAll('button')).find(b=>b.textContent.trim()==='CurseForge')`
    );
    await until(
      `Boolean(Array.from(document.querySelectorAll('[role="menuitemradio"]')).find(b=>b.textContent.trim()==='Modrinth'))`
    );
    await clickElement(
      `Array.from(document.querySelectorAll('[role="menuitemradio"]')).find(b=>b.textContent.trim()==='Modrinth')`
    );
    await delay(300);
    const input = "document.querySelector('input[placeholder]')";
    await until(`Boolean(${input})`);
    await evaluate(`${input}.focus()`);
    for (const key of "sodium") nativeKey(windowId, key);
    nativeKey(windowId, "Return");
    await until(
      `document.body.textContent.toLowerCase().includes('sodium')`,
      90000
    );
    await shot(
      "native-public-resource-search",
      "Actual public Modrinth search and native resource page"
    );
    return {
      query,
      total: response.total,
      resources: (response.list || response.resources || [])
        .slice(0, 5)
        .map((r) => ({ id: r.id, name: r.name })),
    };
  });
  await record("cleanroom", async () => {
    const deadline = Date.now() + 600000;
    result.cleanroom = {
      stage: "game_metadata",
      runtime: fixture.runtime,
      java: "/usr/lib/jvm/java-25-openjdk-amd64/bin/java",
    };
    const game = await invoke(
      "fetch_game_version_specific",
      { gameVersion: "1.12.2" },
      90000
    );
    result.cleanroom.stage = "loader_metadata";
    const versions = await invoke(
      "fetch_mod_loader_version_list",
      { gameVersion: "1.12.2", modLoaderType: "Cleanroom" },
      90000
    );
    assert.ok(versions.length);
    const loader = versions[0];
    result.cleanroom.selectedLoader = loader;
    result.cleanroom.stage = "native_create_instance";
    const name = "NativeCleanroomAcceptance";
    await invoke(
      "create_instance",
      {
        directory: { name: DIRECTORY, dir: fixture.runtime },
        name,
        description: "Isolated native Cleanroom installation acceptance",
        iconSrc: "/images/icons/Cleanroom.png",
        game,
        modLoader: loader,
        optifine: null,
        modpackPath: null,
        isInstallFabricApi: false,
        isInstallQfApi: false,
        modpackVersion: null,
      },
      120000
    );
    result.cleanroom.stage = "download_and_native_processors";
    const completed = [];
    while (Date.now() < deadline) {
      const tasks = (await invoke("retrieve_progressive_task_list")).filter(
        (t) => t.taskGroup.includes(name)
      );
      const failed = tasks.find((t) => t.status === "Failed");
      assert.ok(!failed, JSON.stringify(failed));
      if (tasks.length && tasks.every((t) => t.status === "Completed")) {
        completed.push(...tasks.map((t) => t.taskGroup));
        const instances = await invoke("retrieve_instance_list", {}, 120000);
        const instance = instances.find((i) => i.name === name);
        assert.notEqual(
          instance?.modLoader.status,
          "DownloadFailed",
          "Native loader installation failed"
        );
        if (instance?.modLoader.status === "Installed") {
          const saved = JSON.parse(
            await readFile(
              resolve(fixture.runtime, "versions", name, "sjmclcfg.json")
            )
          );
          assert.equal(saved.modLoader.status, "Installed");
          assert.equal(saved.modLoader.loaderType, "Cleanroom");
          const profile = await stat(
            resolve(fixture.runtime, "versions", name, "install_profile.json")
          );
          assert.ok(profile.size > 0);
          await route(
            `/instances/details/${encodeURIComponent(instance.id)}/overview`
          );
          await shot(
            "native-cleanroom-installed",
            "Production task controller finished the native installer processors and persisted Installed"
          );
          return {
            loader,
            completedTaskGroups: [...new Set(completed)],
            installationProfileBytes: profile.size,
            state: saved.modLoader,
            javaPreference: (await invoke("retrieve_launcher_config"))
              .globalGameConfig.gameJava,
            scope:
              "Complete native installation through the existing task controller; no authenticated Minecraft play session.",
          };
        }
      }
      await delay(500);
    }
    throw new Error(
      "Cleanroom installation exceeded its ten-minute execution budget"
    );
  });
  result.finishedAt = new Date().toISOString();
  result.status = result.checks.every((c) => c.status === "passed")
    ? "passed"
    : "incomplete";
  await writeFile(
    resolve(out, "native-followup-result.json"),
    JSON.stringify(result, null, 2)
  );
}
