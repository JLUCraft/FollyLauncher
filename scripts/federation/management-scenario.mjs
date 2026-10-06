// Opt-in real Tauri/identity-authority test. All data and keys stay in the caller's disposable profile.
import assert from "node:assert/strict";
import { execFile, spawn } from "node:child_process";
import { once } from "node:events";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createInterface } from "node:readline";
import { promisify } from "node:util";

const exec = promisify(execFile);

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
  const locale = JSON.parse(
    await readFile(resolve(repo, "src/locales/zh-Hans.json"), "utf8")
  );
  const run = async (args) =>
    (await exec(driver, args, { timeout: 30_000 })).stdout.trim();
  const save = async (name, value) => {
    const file = resolve(temp, name);
    await writeFile(file, JSON.stringify(value), { mode: 0o600 });
    return file;
  };
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
  async function native(command, args = {}) {
    const key = `__managementResult${++invocation}`;
    await evaluate(
      `void (async()=>{try {window[${JSON.stringify(key)}]={value:await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})}}catch(error){window[${JSON.stringify(key)}]={error:String(error)}}})()`
    );
    await until(`Boolean(window[${JSON.stringify(key)}])`, command);
    const result = await evaluate(`window[${JSON.stringify(key)}]`);
    assert.ok(!result.error, result.error);
    return result.value;
  }
  async function button(label, last = false) {
    const expression = `Array.from(document.querySelectorAll('button')).filter(b=>b.textContent.trim()===${JSON.stringify(label)} && b.getClientRects().length && !b.disabled)`;
    await until(`${expression}.length > 0`, `button ${label}`);
    await evaluate(`${expression}.${last ? "at(-1)" : "at(0)"}.click()`);
    await delay(250);
  }
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
        for (const choice of value) {
          const label = locale.IdentityManagement.evidence[choice];
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
    const record = JSON.parse(await exported());
    assert.ok(record.signatures.length > 0);
    return record;
  }
  let host;
  async function stopHost() {
    if (host && host.exitCode === null) {
      const exited = once(host, "exit");
      host.kill("SIGTERM");
      await exited;
    }
  }
  async function start(config) {
    const path = await save("node.json", config);
    host = spawn(driver, ["run", "--config", path], {
      stdio: ["ignore", "pipe", "pipe"],
    });
    let diagnostics = "";
    host.stderr.on("data", (bytes) => {
      diagnostics = (diagnostics + bytes).slice(-4096);
    });
    const lines = createInterface({ input: host.stdout });
    return new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error(`Host startup timeout: ${diagnostics}`)),
        20_000
      );
      lines.on("line", (line) => {
        try {
          const value = JSON.parse(line);
          if (value.peer) {
            clearTimeout(timer);
            resolve(value);
          }
        } catch {}
      });
      host.once("exit", () => {
        clearTimeout(timer);
        reject(new Error(`Host exited: ${diagnostics}`));
      });
    });
  }
  try {
    await evaluate("window.next.router.push('/federation/identity')");
    await until(
      "location.pathname==='/federation/identity' && document.getElementById('identity-action-kind')",
      "identity page"
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
    const service = registration.action.service;
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
    assert.equal(facts.permissions["desktop-admin"].permission, "admin");
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
    assert.deepEqual(verifiedFacts.verifiers["desktop-verifier"].evidence, [
      "institutional_email",
    ]);
    await shot(
      "management-identity",
      "真实表单登记服务、合签、授权；通过真实身份节点提交"
    );
    await stopHost();
    const binding = await save("binding.json", {
      genesis: root.genesis_hash,
      namespace: "league",
      service,
      audience: peer,
      club: "school",
    });
    const business = resolve(temp, "league-genesis.json");
    const pin = await run([
      "business-genesis",
      "--binding",
      binding,
      "--out",
      business,
    ]);
    config.node.listen = [address.replace(`/p2p/${peer}`, "")];
    config.services.governance = {
      domain: "league",
      genesis: business,
      journal: resolve(temp, "league.journal"),
    };
    await start(config);
    const link = `folly://federation?${new URLSearchParams({ address, peer, genesis: pin })}`;
    await evaluate(
      `window.dispatchEvent(new CustomEvent('deeplink:emit',{detail:[${JSON.stringify(link)}]}))`
    );
    await until(
      "location.pathname==='/federation/overview'",
      "federation link route"
    );
    await evaluate("window.next.router.push('/federation/manage')");
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
    const anchor = JSON.parse(await readFile(actionFile, "utf8"));
    await input("anchor-id", anchor.id);
    await input("anchor-digest", anchor.digest);
    await button(locale.IdentityFlow.review);
    await button(locale.AuditAnchor.submit);
    const snapshot = await native("union_governance", {
      input: { address, peer, genesis_hash: pin },
    });
    assert.equal(
      snapshot.state.digest_anchors[anchor.id].digest,
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
        snapshot.state.digest_anchors[anchor.id].digest,
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
    assert.deepEqual(updated.state.members.founder.roles, ["reviewer"]);
    await button(locale.BusinessAdmin.add);
    for (const [label,value] of [
      [locale.BusinessAdmin.nodeId,"1"],
      [locale.IdentityManagement.fields.audience,peer],
      [locale.BusinessAdmin.addresses,address],
    ]) {
      await evaluate(`(()=>{const el=Array.from(document.querySelectorAll('input,textarea')).find(e=>e.getAttribute('aria-label')===${JSON.stringify(label)});const proto=el.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype;Object.getOwnPropertyDescriptor(proto,'value').set.call(el,${JSON.stringify(value)});el.dispatchEvent(new Event('input',{bubbles:true}));})()`);
      await delay(150);
    }
    await button(locale.BusinessAdmin.reviewReplicas);
    await button(locale.BusinessAdmin.submit);
    const withReplicas = await native("union_governance", {input:{address,peer,genesis_hash:pin}});
    assert.equal(withReplicas.state.consensus_members["1"].peer,peer);
    await shot("management-business", "真实联赛治理提交审计摘要与成员业务角色");
    await stopHost();
    await evaluate("window.next.router.push('/federation/identity')");
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
