import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";

// Run only in capture-tutorial's disposable native profile.
export async function launcherScenario({ evaluate, delay, temp, out }) {
  const checks = [];
  let serial = 0;
  async function invoke(command, args = {}, expectError = false) {
    const key = `__launcherAcceptance${++serial}`;
    await evaluate(
      `void window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)}).then(value=>window.${key}={value},error=>window.${key}={error:String(error)})`
    );
    for (let attempt = 0; attempt < 150; attempt++) {
      const result = await evaluate(`window.${key}`);
      if (result) {
        await evaluate(`delete window.${key}`);
        if (expectError) assert.ok(result.error, `${command} must reject`);
        else assert.ok(!result.error, `${command}: ${result.error}`);
        return expectError ? result.error : result.value;
      }
      await delay(200);
    }
    throw new Error(`Native command timed out: ${command}`);
  }
  async function record(name, action) {
    try {
      const detail = await action();
      checks.push({ name, status: "passed", detail });
    } catch (error) {
      checks.push({ name, status: "failed", error: String(error) });
      throw error;
    } finally {
      await writeFile(
        resolve(out, "launcher-checks.json"),
        JSON.stringify(checks, null, 2)
      );
    }
  }
  const original = await invoke("retrieve_launcher_config");
  await record("native configuration update and reload", async () => {
    const value = !original.general.functionality.skipFirstScreenOptions;
    await invoke("update_launcher_config", {
      keyPath: "general.functionality.skipFirstScreenOptions",
      value: JSON.stringify(value),
    });
    assert.equal(
      (await invoke("retrieve_launcher_config")).general.functionality
        .skipFirstScreenOptions,
      value
    );
    await evaluate("location.reload()");
    for (let attempt = 0; attempt < 100; attempt++) {
      if (
        await evaluate(
          "Boolean(window.next?.router?.isReady && document.querySelectorAll('button').length)"
        )
      )
        break;
      await delay(200);
    }
    assert.equal(
      (await invoke("retrieve_launcher_config")).general.functionality
        .skipFirstScreenOptions,
      value
    );
    await invoke("update_launcher_config", {
      keyPath: "general.functionality.skipFirstScreenOptions",
      value: JSON.stringify(
        original.general.functionality.skipFirstScreenOptions
      ),
    });
  });
  await record("account and instance empty state", async () => {
    assert.deepEqual(await invoke("retrieve_player_list"), []);
    assert.deepEqual(await invoke("retrieve_instance_list"), []);
    const servers = await invoke("retrieve_auth_server_list");
    assert.ok(
      servers.some((server) => server.authUrl.includes("skin.jlucraft.com"))
    );
    return { configuredAuthenticationServers: servers.length };
  });
  await record("Java discovery and invalid executable rejection", async () => {
    const runtimes = await invoke("retrieve_java_list");
    assert.ok(Array.isArray(runtimes));
    for (const runtime of runtimes)
      await invoke("validate_java", { javaPath: runtime.execPath });
    await invoke(
      "validate_java",
      { javaPath: resolve(temp, "missing-java") },
      true
    );
    return { discoveredRuntimes: runtimes.length };
  });
  await record("missing game instance rejection", async () => {
    return await invoke(
      "launch_game",
      { instanceId: "acceptance-missing" },
      true
    );
  });
  const bytes = Buffer.from(
    "JLUCraft native download acceptance\n".repeat(2048)
  );
  const requests = [];
  const server = createServer((request, response) => {
    requests.push({ method: request.method, range: request.headers.range });
    response.writeHead(200, { "Content-Length": bytes.length });
    response.end(request.method === "HEAD" ? undefined : bytes);
  });
  await new Promise((done, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", done);
  });
  try {
    await record("native download, SHA-1, task list and deletion", async () => {
      const destination = resolve(temp, "acceptance-download.bin");
      const group = "launcher-acceptance";
      await invoke("schedule_progressive_task_group", {
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
      let task;
      for (let attempt = 0; attempt < 100; attempt++) {
        task = (await invoke("retrieve_progressive_task_list")).find(
          (item) => item.taskGroup === group
        );
        if (task?.status === "Completed" || task?.status === "Failed") break;
        await delay(200);
      }
      assert.equal(task?.status, "Completed", JSON.stringify(task));
      assert.deepEqual(await readFile(destination), bytes);
      await invoke("delete_progressive_task_group", { taskGroup: group });
      assert.equal(
        (await invoke("retrieve_progressive_task_list")).some(
          (item) => item.taskGroup === group
        ),
        false
      );
      return { bytes: bytes.length, requests };
    });
  } finally {
    server.closeAllConnections();
    await new Promise((done) => server.close(done));
  }
  console.log(`PASS: ${checks.length} native launcher feature checks`);
}
