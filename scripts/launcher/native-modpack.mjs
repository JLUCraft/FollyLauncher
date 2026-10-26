import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";

export async function modpackScenario({
  fixture,
  temp,
  invoke,
  route,
  shot,
  delay,
}) {
  const installed = resolve(
    fixture.runtime,
    "versions/NativeCleanroomAcceptance"
  );
  const profile = JSON.parse(
    await readFile(resolve(installed, "NativeCleanroomAcceptance.json"))
  );
  const base = structuredClone(
    profile.patches.find((p) => p.downloads?.client)
  );
  assert.ok(base, "Use the genuine cached Minecraft base profile");
  const version = base.version;
  assert.equal(version, "1.12.2");
  const client = await readFile(
    resolve(installed, "NativeCleanroomAcceptance.jar")
  );
  const clientSha256 = createHash("sha256").update(client).digest("hex");
  assert.equal(
    createHash("sha1").update(client).digest("hex"),
    base.downloads.client.sha1
  );
  const requests = [];
  const server = createServer((request, response) => {
    requests.push({ method: request.method, path: request.url });
    const bytes =
      request.url === "/client.jar"
        ? client
        : Buffer.from(JSON.stringify(base));
    response.writeHead(200, { "Content-Length": bytes.length });
    response.end(bytes);
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  const origin = `http://127.0.0.1:${server.address().port}`;
  base.downloads.client.url = origin + "/client.jar";
  const archive = resolve(temp, "native-local.mrpack");
  const override = "Controlled native Modrinth override\n";
  const name = "NativeModpackAcceptance";
  execFileSync("python3", [
    "-c",
    `import json,sys,zipfile
with zipfile.ZipFile(sys.argv[1],'w') as z:
 z.writestr('modrinth.index.json',json.dumps({'formatVersion':1,'game':'minecraft','versionId':'acceptance','name':sys.argv[2],'files':[],'dependencies':{'minecraft':sys.argv[3]}}))
 z.writestr('overrides/native-acceptance.txt',sys.argv[4])
`,
    archive,
    name,
    version,
    override,
  ]);
  try {
    const meta = await invoke(
      "retrieve_modpack_meta_info",
      { path: archive },
      90000
    );
    assert.equal(meta.clientVersion, version);
    const game = await invoke(
      "fetch_game_version_specific",
      { gameVersion: version },
      90000
    );
    game.url = origin + "/client.json";
    const before = new Set(
      (await invoke("retrieve_progressive_task_list")).map((t) => t.taskGroup)
    );
    await invoke(
      "create_instance",
      {
        directory: { name: "native-followup", dir: fixture.runtime },
        name,
        description: "Controlled native local Modrinth import",
        iconSrc: "/images/icons/JEIcon_Release.png",
        game,
        modLoader: {
          loaderType: "Unknown",
          version: "",
          description: "Vanilla",
          stable: null,
          branch: null,
        },
        optifine: null,
        modpackPath: archive,
        isInstallFabricApi: false,
        isInstallQfApi: false,
        modpackVersion: meta.version,
      },
      90000
    );
    let tasks;
    for (let i = 0; i < 300; i++) {
      tasks = (await invoke("retrieve_progressive_task_list")).filter(
        (t) => !before.has(t.taskGroup)
      );
      assert.ok(
        !tasks.some((t) => t.status === "Failed"),
        JSON.stringify(tasks)
      );
      if (tasks.length && tasks.every((t) => t.status === "Completed")) break;
      await delay(200);
    }
    assert.ok(tasks.length && tasks.every((t) => t.status === "Completed"));
    const versionPath = resolve(fixture.runtime, "versions", name);
    assert.equal(
      await readFile(resolve(versionPath, "native-acceptance.txt"), "utf8"),
      override
    );
    assert.equal(
      createHash("sha256")
        .update(await readFile(resolve(versionPath, `${name}.jar`)))
        .digest("hex"),
      clientSha256
    );
    const instance = (await invoke("retrieve_instance_list")).find(
      (i) => i.name === name
    );
    assert.ok(instance);
    assert.equal(instance.version, version);
    await route(
      `/instances/details/${encodeURIComponent(instance.id)}/overview`
    );
    await shot(
      "native-local-modpack-installed",
      "Actual native mrpack installation completed using genuine cached client bytes"
    );
    return {
      archive: "Controlled local Modrinth mrpack",
      minecraft: version,
      entry:
        "Real retrieve_modpack_meta_info and create_instance; production code generated final instance files",
      cachedClientSha256: clientSha256,
      overrideVerified: true,
      tasks: tasks.map((t) => ({
        group: t.taskGroup,
        status: t.status,
        files: t.taskDescs.map((d) => d.payload.dest),
      })),
      transport: requests,
      limits:
        "The genuine cached client was served locally. No Cleanroom reinstall, dependency upgrade, or public pack installation is claimed.",
    };
  } finally {
    server.closeAllConnections();
    await new Promise((done) => server.close(done));
  }
}
