// Opt-in real Tauri/identity-authority test. All data and keys stay in the caller's disposable profile.
import assert from "node:assert/strict";
import { execFile, spawn } from "node:child_process";
import { once } from "node:events";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createInterface } from "node:readline";
import { promisify } from "node:util";
import { parseRecord, objectValue } from "../json.cjs";

const exec = promisify(execFile);

/** @param {import("../launcher/types").CaptureContext} context */
export async function managementScenario({
  evaluate,
  delay,
  shot,
  temp,
  out,
  repo,
}) {
  const resultPath = resolve(out, "management-result.json");
  await writeFile(
    resultPath,
    JSON.stringify(
      { status: "running", startedAt: new Date().toISOString() },
      null,
      2
    )
  );
  const driver = resolve(
    repo,
    "../federated-server/target/debug/examples/acceptance-driver"
  );
  const server = resolve(
    repo,
    "../federated-server/target/debug/federated-server"
  );
  const locale =
    /** @type {typeof import("../../src/locales/zh-Hans.json")} */ (
      parseRecord(
        await readFile(resolve(repo, "src/locales/zh-Hans.json"), "utf8")
      )
    );
  /** @param {string[]} args */
  const run = async (args) =>
    (await exec(driver, args, { timeout: 30_000 })).stdout.trim();
  /** @param {string} name @param {unknown} value */
  const save = async (name, value) => {
    const file = resolve(temp, name);
    await writeFile(file, JSON.stringify(value), { mode: 0o600 });
    return file;
  };
  /** @param {string} expression @param {string} description */
  async function until(expression, description) {
    for (let i = 0; i < 100; i++) {
      if (await evaluate(`Boolean(${expression})`)) return;
      await delay(200);
    }
    await writeFile(
      resolve(out, "management-failure.json"),
      JSON.stringify(
        { description, body: await evaluate("document.body.innerText") },
        null,
        2
      )
    );
    throw new Error(`Timed out: ${description}`);
  }
  let invocation = 0;
  /** @param {string} command @param {object} [args] */
  async function native(command, args = {}) {
    const key = `__managementResult${++invocation}`;
    await evaluate(
      `void (async()=>{try {window[${JSON.stringify(key)}]={value:await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})}}catch(error){window[${JSON.stringify(key)}]={error:String(error)}}})()`
    );
    await until(`Boolean(window[${JSON.stringify(key)}])`, command);
    const result = objectValue(
      await evaluate(`window[${JSON.stringify(key)}]`)
    );
    assert.ok(!result.error, String(result.error));
    return objectValue(result.value);
  }
  /** @param {string} label @param {boolean} [last] */
  async function button(label, last = false) {
    const expression = `Array.from(document.querySelectorAll('button')).filter(b=>b.textContent.trim()===${JSON.stringify(label)} && b.getClientRects().length && !b.disabled)`;
    await until(`${expression}.length > 0`, `button ${label}`);
    await evaluate(`${expression}.${last ? "at(-1)" : "at(0)"}.click()`);
    await delay(250);
  }
  /** @param {string} label */
  async function expand(label) {
    const expression = `Array.from(document.querySelectorAll('button')).find(b=>b.textContent.trim()===${JSON.stringify(label)} && b.getAttribute('aria-expanded')==='false')`;
    await evaluate(`${expression}?.click()`);
    await delay(300);
  }
  async function submitted() {
    await until(
      `!Array.from(document.querySelectorAll('textarea')).some(e=>e.getAttribute('aria-label')===${JSON.stringify(locale.IdentityFlow.export)})`,
      "submitted record"
    );
  }
  /** @param {string} id @param {unknown} value */
  async function input(id, value) {
    await until(`!!document.getElementById(${JSON.stringify(id)})`, id);
    await evaluate(
      `(()=>{const el=document.getElementById(${JSON.stringify(id)}); const prototype=el.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:el.tagName==='SELECT'?HTMLSelectElement.prototype:HTMLInputElement.prototype;Object.getOwnPropertyDescriptor(prototype,'value').set.call(el,${JSON.stringify(value)});el.dispatchEvent(new Event(el.tagName==='SELECT'?'change':'input',{bubbles:true}));})()`
    );
    await delay(150);
  }
  const exported = () =>
    evaluate(
      `Array.from(document.querySelectorAll('textarea')).find(e=>e.getAttribute('aria-label')===${JSON.stringify(locale.IdentityFlow.export)})?.value || ''`
    );
  /** @param {string} action @param {Record<string, string | string[]>} fields */
  async function draft(action, fields) {
    await input("identity-action-kind", action);
    if (action === "register_service" && !fields.service) {
      await button(locale.IdentityManagement.generateId);
      await until(
        "document.getElementById('identity-action-service').value.length>0",
        "generated service ID"
      );
    }
    for (const [key, value] of Object.entries(fields)) {
      if (key === "evidence") {
        assert.ok(Array.isArray(value));
        for (const choice of value) {
          const label = Object.entries(locale.IdentityManagement.evidence).find(
            ([key]) => key === choice
          )?.[1];
          assert.ok(label);
          await evaluate(
            `Array.from(document.querySelectorAll('label')).find(el=>el.textContent.trim()===${JSON.stringify(label)})?.click()`
          );
        }
      } else await input(`identity-action-${key}`, value);
    }
    await button(locale.IdentityFlow.review);
    await button(locale.IdentityFlow.sign);
    await expand(locale.FederationNav.advanced);
    await until(
      `Array.from(document.querySelectorAll('textarea')).some(e=>e.getAttribute('aria-label')===${JSON.stringify(locale.IdentityFlow.export)} && JSON.parse(e.value).signatures.length>0)`,
      "signed record export"
    );
    const exportedValue = await exported();
    assert.equal(typeof exportedValue, "string");
    if (typeof exportedValue !== "string")
      throw new Error("Exported record required");
    const record = parseRecord(exportedValue);
    assert.ok(Array.isArray(record.signatures) && record.signatures.length > 0);
    return record;
  }
  /** @type {import("node:child_process").ChildProcess | undefined} */
  let host;
  async function stopHost() {
    if (host && host.exitCode === null) {
      const exited = once(host, "exit");
      host.kill("SIGTERM");
      await exited;
    }
  }
  /** @param {object} config @returns {Promise<{addresses: string[]}>} */
  async function start(config) {
    const path = await save("node.json", config);
    const child = spawn(driver, ["run", "--config", path], {
      stdio: ["ignore", "pipe", "pipe"],
    });
    host = child;
    let diagnostics = "";
    child.stderr.on("data", (bytes) => {
      diagnostics = (diagnostics + bytes).slice(-4096);
    });
    const lines = createInterface({ input: child.stdout });
    return new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error(`Host startup timeout: ${diagnostics}`)),
        20_000
      );
      lines.on("line", (line) => {
        try {
          const value = parseRecord(line);
          if (value.peer && Array.isArray(value.addresses)) {
            clearTimeout(timer);
            /** @type {unknown[]} */
            const addresses = value.addresses;
            resolve({
              addresses: addresses.filter(
                (address) => typeof address === "string"
              ),
            });
          }
        } catch {}
      });
      child.once("exit", () => {
        clearTimeout(timer);
        reject(new Error(`Host exited: ${diagnostics}`));
      });
    });
  }
  try {
    await evaluate("window.next.router.push('/settings/federation')");
    await until(
      "location.pathname==='/settings/federation' && document.getElementById('identity-action-kind')",
      "union settings page"
    );
    await expand(locale.FederationNav.advanced);
    const publicIdentity = await native("union_public_identity");
    const root = await native("union_management", {
      request: {
        operation: "build_genesis",
        federation: "desktop-management-test",
        founders: [
          { member: "founder", club: "school", identity: publicIdentity },
        ],
        quorum: 1,
      },
    });
    const genesis = await save("identity-genesis.json", root.genesis);
    const key = resolve(temp, "node.key");
    const peer = await run(["keygen", "--out", key]);
    /** @type {{node: {key: string, listen: string[]}, services: Record<string, unknown>}} */
    const config = {
      node: { key, listen: ["/ip4/127.0.0.1/tcp/0"] },
      services: {
        identity: {
          genesis,
          genesis_hash: root.genesis_hash,
          log: resolve(temp, "identity.jsonl"),
          writer: true,
          max_staleness_seconds: 60,
        },
      },
    };
    const status = await start(config);
    const address = status.addresses.find((a) => a.includes("/tcp/"));
    assert.ok(address);
    await input(
      "identity-configuration",
      JSON.stringify({ ...root, address, peer })
    );
    await button(locale.FederationPage.identity.save);
    await button(locale.IdentityFlow.sync);
    await until(
      `document.body.innerText.includes('founder')`,
      "verified founder"
    );
    const registration = await draft("register_service", {
      audience: peer,
      club: "school",
    });
    const service = objectValue(registration.action).service;
    assert.ok(typeof service === "string");
    const unsigned = await save("registration.json", registration);
    const signedPath = resolve(temp, "registration-signed.json");
    await run([
      "trust",
      "sign",
      "--key",
      key,
      "--record",
      unsigned,
      "--out",
      signedPath,
    ]);
    await input("identity-import", await readFile(signedPath, "utf8"));
    await button(locale.IdentityFlow.review, true);
    await button(locale.IdentityFlow.submit);
    await submitted();
    await native("union_identity_action", { operation: { operation: "sync" } });
    await draft("grant_permission", {
      id: "desktop-admin",
      member: "founder",
      service,
      permission: "admin",
    });
    await button(locale.IdentityFlow.submit);
    await submitted();
    const facts = await native("union_identity_action", {
      operation: { operation: "sync" },
    });
    assert.equal(
      objectValue(objectValue(facts.permissions)["desktop-admin"]).permission,
      "admin"
    );
    await draft("authorize_verifier", {
      id: "desktop-verifier",
      club: "school",
      device: peer,
      evidence: ["institutional_email"],
    });
    await button(locale.IdentityFlow.submit);
    await submitted();
    const verifiedFacts = await native("union_identity_action", {
      operation: { operation: "sync" },
    });
    assert.deepEqual(
      objectValue(objectValue(verifiedFacts.verifiers)["desktop-verifier"])
        .evidence,
      ["institutional_email"]
    );
    await shot(
      "management-identity",
      "真实表单登记服务、合签、授权；通过真实身份节点提交"
    );
    await stopHost();
    const binding = await save("binding.json", {
      genesis: root.genesis_hash,
      namespace: "application",
      service,
      audience: peer,
      club: "school",
    });
    const business = resolve(temp, "application-genesis.json");
    const pin = await run([
      "business-genesis",
      "--binding",
      binding,
      "--out",
      business,
    ]);
    config.node.listen = [address.replace(`/p2p/${peer}`, "")];
    config.services.governance = {
      genesis: business,
      journal: resolve(temp, "application.journal"),
    };
    await start(config);
    const link = `folly://federation?${new URLSearchParams({ address, peer, genesis: pin })}`;
    await evaluate(
      `window.dispatchEvent(new CustomEvent('deeplink:emit',{detail:[${JSON.stringify(link)}]}))`
    );
    await until(
      "location.pathname==='/settings/federation'",
      "federation link route"
    );
    await button(locale.FederationPage.settings.linkConnect);
    await until(
      `document.body.innerText.includes(${JSON.stringify(locale.FederationNav.connected)})`,
      "confirmed application connection"
    );
    await evaluate(
      "window.next.router.push('/settings/federation-management')"
    );
    await until(
      `Array.from(document.querySelectorAll('button')).some(b=>b.textContent.trim()===${JSON.stringify(locale.AuditAnchor.title)})`,
      "authorized management view"
    );
    await expand(locale.AuditAnchor.title);
    const ledger = await save("ledger.json", []);
    const summary = resolve(temp, "audit-summary.json");
    const actionFile = resolve(temp, "audit-action.json");
    await exec(
      server,
      [
        "gateway",
        "audit",
        "--ledger",
        ledger,
        "--out",
        summary,
        "--action",
        actionFile,
      ],
      { timeout: 30_000 }
    );
    const anchor = parseRecord(await readFile(actionFile, "utf8"));
    assert.ok(
      typeof anchor.id === "string" && typeof anchor.digest === "string"
    );
    await input("anchor-id", anchor.id);
    await input("anchor-digest", anchor.digest);
    await button(locale.IdentityFlow.review);
    await button(locale.AuditAnchor.submit);
    const snapshot = await native("union_governance", {
      input: { address, peer, genesis_hash: pin },
    });
    assert.equal(
      objectValue(
        objectValue(objectValue(snapshot.state).digest_anchors)[anchor.id]
      ).digest,
      anchor.digest
    );
    await exec(
      server,
      [
        "gateway",
        "verify-audit",
        "--ledger",
        ledger,
        "--summary",
        summary,
        "--digest",
        String(
          objectValue(
            objectValue(objectValue(snapshot.state).digest_anchors)[anchor.id]
          ).digest
        ),
      ],
      { timeout: 30_000 }
    );
    await expand(locale.BusinessAdmin.title);
    await input("business-member", "founder");
    await input("business-roles", "reviewer");
    await button(locale.BusinessAdmin.reviewRoles);
    await button(locale.BusinessAdmin.submit);
    const updated = await native("union_governance", {
      input: { address, peer, genesis_hash: pin },
    });
    assert.deepEqual(
      objectValue(objectValue(objectValue(updated.state).members).founder)
        .roles,
      ["reviewer"]
    );
    await button(locale.BusinessAdmin.add);
    for (const [label, value] of [
      [locale.BusinessAdmin.nodeId, "1"],
      [locale.IdentityManagement.fields.audience, peer],
      [locale.BusinessAdmin.addresses, address],
    ]) {
      await evaluate(
        `(()=>{const el=Array.from(document.querySelectorAll('input,textarea')).find(e=>e.getAttribute('aria-label')===${JSON.stringify(label)});const proto=el.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype;Object.getOwnPropertyDescriptor(proto,'value').set.call(el,${JSON.stringify(value)});el.dispatchEvent(new Event('input',{bubbles:true}));})()`
      );
      await delay(150);
    }
    await button(locale.BusinessAdmin.reviewReplicas);
    await button(locale.BusinessAdmin.submit);
    const withReplicas = await native("union_governance", {
      input: { address, peer, genesis_hash: pin },
    });
    assert.equal(
      objectValue(
        objectValue(objectValue(withReplicas.state).consensus_members)["1"]
      ).peer,
      peer
    );
    await shot("management-business", "真实联赛治理提交审计摘要与成员业务角色");
    await stopHost();
    await evaluate("window.next.router.push('/settings/federation')");
    await expand(locale.FederationNav.identity);
    await button(locale.IdentityFlow.inspect);
    await until(
      `document.body.innerText.includes(${JSON.stringify(locale.IdentityManagement.cached)})`,
      "offline history label"
    );
    await shot(
      "management-offline",
      "身份节点离线后读取本地已验证历史，明确不声称当前权限"
    );
    await writeFile(
      resultPath,
      JSON.stringify(
        {
          status: "passed",
          checks: [
            "public identity",
            "genesis review",
            "configuration import",
            "service co-signing",
            "permission grant",
            "verifier authorization",
            "federation deep link",
            "audit anchor",
            "server audit export/verification",
            "offline history",
            "business roles",
            "replica metadata update",
          ],
          finishedAt: new Date().toISOString(),
        },
        null,
        2
      )
    );
  } catch (error) {
    await writeFile(
      resultPath,
      JSON.stringify(
        {
          status: "failed",
          error: String(error),
          finishedAt: new Date().toISOString(),
        },
        null,
        2
      )
    );
    throw error;
  } finally {
    await stopHost();
  }
}
