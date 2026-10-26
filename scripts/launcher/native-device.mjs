import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, open, readFile, rm, stat, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

export async function prepareDeviceFixture({ temp, repo }) {
  const directory = resolve(temp, "station");
  await mkdir(directory, { recursive: true, mode: 0o700 });
  const browserBin = resolve(temp, "browser-bin");
  const browserRequest = resolve(temp, "browser-request.txt");
  await mkdir(browserBin, { recursive: true, mode: 0o700 });
  await rm(browserRequest, { force: true });
  const handler = `#!/bin/sh
umask 077
for value; do :; done
printf '%s' "$value" > "$NATIVE_BROWSER_REQUEST"
`;
  for (const name of ["xdg-open", "gio"])
    await writeFile(resolve(browserBin, name), handler, { mode: 0o700 });
  const log = await open(resolve(directory, "station.log"), "w", 0o600);
  await rm(resolve(directory, "ready.json"), { force: true });
  const child = spawn(
    process.execPath,
    [
      "--import",
      resolve(
        repo,
        "../federated-skinstation/node_modules/tsx/dist/loader.mjs"
      ),
      resolve(repo, "scripts/launcher/native-station-fixture.mjs"),
      directory,
    ],
    {
      cwd: resolve(repo, "../federated-skinstation"),
      env: { ...process.env, TSX_DISABLE_CACHE: "1" },
      stdio: ["ignore", log.fd, log.fd],
    }
  );
  const exited = new Promise((done) => child.once("exit", done));
  const cleanup = async () => {
    child.kill("SIGTERM");
    await Promise.race([exited, new Promise((done) => setTimeout(done, 2000))]);
    if (child.exitCode === null) child.kill("SIGKILL");
    await log.close();
  };
  try {
    for (let i = 0; i < 200; i++) {
      const ready = await readFile(resolve(directory, "ready.json"), "utf8")
        .then(JSON.parse)
        .catch(() => null);
      if (ready)
        return {
          station: ready,
          stationDirectory: directory,
          browserBin,
          browserRequest,
          cleanup,
        };
      if (child.exitCode !== null)
        throw new Error(
          `Isolated station exited ${child.exitCode}; inspect its private station.log`
        );
      await new Promise((done) => setTimeout(done, 100));
    }
    throw new Error("Isolated station did not become ready");
  } catch (error) {
    await cleanup();
    throw error;
  }
}

export async function deviceScenario({
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
}) {
  const { station } = fixture;
  assert.ok(station, "Real isolated station must be running");
  const clickText = async (text) => {
    await until(
      `Array.from(document.querySelectorAll('button')).some(b=>b.textContent.trim()===${JSON.stringify(text)})`
    );
    await evaluate(
      `Array.from(document.querySelectorAll('button')).find(b=>b.textContent.trim()===${JSON.stringify(text)}).click()`
    );
  };
  await route("/accounts");
  await clickText("添加认证服务器");
  await until("Boolean(document.querySelector('#serverUrl'))");
  assert.equal(
    await evaluate("document.querySelector('#serverUrl').disabled"),
    false
  );
  await evaluate(
    `(() => {const input=document.querySelector('#serverUrl');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,${JSON.stringify(station.origin)});input.dispatchEvent(new Event('input',{bubbles:true}));})()`
  );
  await clickText("下一步");
  await until(
    "document.querySelector('[aria-modal=true]')?.textContent.includes('JLUCraft Federation')"
  );
  await shot(
    "native-auth-station-metadata",
    "Actual authentication-server form fetched metadata through native IPC",
    { allowDialogs: true }
  );
  await clickText("完成");
  await until("document.querySelectorAll('[aria-modal=true]').length===0");
  const stored = JSON.parse(
    await readFile(resolve(fixture.profile, "sjmcl.account.json"), "utf8")
  );
  const server = stored.authServers.find((s) =>
    s.authUrl.startsWith(station.origin)
  );
  assert.ok(server?.clientId);
  assert.ok(server.metadata.meta["feature.openid_configuration_url"]);
  assert.ok(server.metadata.skinDomains.includes("127.0.0.1"));
  assert.equal(
    (await stat(resolve(fixture.profile, "sjmcl.account.json"))).mode & 0o777,
    0o600
  );
  await until(
    "Array.from(document.querySelectorAll('button')).some(b=>b.textContent.trim()==='JLUCraft Federation')"
  );
  await evaluate(
    "Array.from(document.querySelectorAll('button')).filter(b=>b.textContent.trim()==='JLUCraft Federation').at(-1).click()"
  );
  await clickText("添加角色");
  await until(
    "Array.from(document.querySelectorAll('button')).some(b=>b.textContent.trim()==='开始 OAuth 登录')"
  );
  await clickText("开始 OAuth 登录");
  await until(
    "Array.from(document.querySelectorAll('button')).some(b=>b.textContent.trim()==='继续登录')"
  );
  await evaluate(
    "Array.from(document.querySelectorAll('[aria-modal=true] h2')).filter(e=>/^[A-Z0-9]{4}-[A-Z0-9]{4}$/.test(e.textContent.trim())).forEach(e=>e.style.visibility='hidden')"
  );
  await shot(
    "native-device-code",
    "Real native OAuth interface; authorization code visually redacted",
    { allowDialogs: true }
  );
  await clickText("继续登录");
  let verification;
  for (let i = 0; i < 100; i++) {
    verification = await readFile(fixture.browserRequest, "utf8").catch(
      () => null
    );
    if (verification) break;
    await delay(150);
  }
  assert.ok(
    verification,
    "Native opener must dispatch its actual verification URL to the isolated handler"
  );
  assert.ok(verification.startsWith(station.origin + "/portal/"));
  const before = new Set(targets().map((t) => t.targetId));
  await invoke("create_window", {
    config: {
      label: "native_device_approval",
      url: verification,
      title: "Native device approval",
      width: 1000,
      height: 760,
    },
    customOverlaid: false,
  });
  let browser;
  for (let i = 0; i < 100; i++) {
    await refreshTargets();
    browser = targets().find((t) => !before.has(t.targetId));
    if (browser) break;
    await delay(150);
  }
  assert.ok(browser);
  await until(
    "Boolean(document.querySelector('input[type=email]'))",
    30000,
    browser.targetId
  );
  await evaluate(
    `(() => {const set=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set;for(const [selector,value] of ${JSON.stringify(
      [
        ["input[type=email]", station.email],
        ["input[type=password]", station.password],
      ]
    )}){const input=document.querySelector(selector);set.call(input,value);input.dispatchEvent(new Event('input',{bubbles:true}));}document.querySelector('button[type=submit]').click();})()`,
    browser.targetId
  );
  await until(
    "Boolean(document.querySelector('.device-code'))",
    30000,
    browser.targetId
  );
  await evaluate(
    "document.querySelector('.device-code').style.visibility='hidden'",
    browser.targetId
  );
  const browserWindow = findChildWindow();
  assert.ok(browserWindow);
  await shot(
    "native-device-panel",
    "Actual station DevicePanel before approval; code visually redacted",
    { nativeWindow: browserWindow, allowDialogs: true }
  );
  await evaluate(
    "Array.from(document.querySelectorAll('button')).find(b=>b.textContent.trim()==='授权登录').click()",
    browser.targetId
  );
  await until(
    "document.body.textContent.includes('已授权')",
    15000,
    browser.targetId
  );
  let player;
  for (let i = 0; i < 200; i++) {
    player = (await invoke("retrieve_player_list")).find(
      (p) => p.name === "NativeDevice" && p.playerType === "3rdparty"
    );
    if (player) break;
    await delay(300);
  }
  assert.ok(
    player,
    "Actual Tauri polling and native account persistence must finish"
  );
  await until(
    "document.querySelectorAll('[aria-modal=true]').length===0",
    30000
  );
  const account = JSON.parse(
    await readFile(resolve(fixture.profile, "sjmcl.account.json"), "utf8")
  );
  assert.ok(
    account.players.some(
      (p) => p.name === "NativeDevice" && p.playerType === "3rdparty"
    )
  );
  assert.equal(
    (await stat(resolve(fixture.profile, "sjmcl.account.json"))).mode & 0o777,
    0o600
  );
  await shot(
    "native-device-account-saved",
    "Native third-party player persisted after actual DevicePanel approval"
  );
  return {
    playerId: player.id,
    provider:
      "Real isolated federated-skinstation; school.test and mail callback; no SMTP/MUA Hub",
    authenticationServer: {
      uiAdded: true,
      metadataFetched: true,
      saved: true,
      openidClientDiscovered: true,
      skinDomain: "127.0.0.1",
    },
    nativeAuthorizationUi:
      "Actual native OAuth code and continue controls used",
    browserApproval:
      "Real portal login and DevicePanel approval in isolated WebKit",
    nativePolling:
      "Real Tauri polling completed and third-party player persisted",
    secrets:
      "Private RSA/database/credentials remain in owner-only external profile; no token or device code in evidence",
    browserBoundary:
      "Native system URL dispatch captured by a private runtime handler, then opened in real isolated WebKit. User default browser launching is not claimed.",
    limitations:
      "Does not validate real school email, MUA Hub, dual-client skin rendering, or public multiplayer.",
  };
}
