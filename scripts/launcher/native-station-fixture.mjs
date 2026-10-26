// Real station service for the native device-flow case. No SMTP or production config.
import { generateKeyPairSync, randomBytes } from "node:crypto";
import { chmod, mkdir, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { build } from "../../../federated-skinstation/server/app.js";

const directory = resolve(process.argv[2]);
await mkdir(directory, { recursive: true, mode: 0o700 });
const reservation = createServer();
await new Promise((done) => reservation.listen(0, "127.0.0.1", done));
const port = reservation.address().port;
await new Promise((done) => reservation.close(() => done()));
const origin = `http://127.0.0.1:${port}`;
const keyPath = resolve(directory, "signing.pem");
const { privateKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
await writeFile(keyPath, privateKey.export({ type: "pkcs8", format: "pem" }), {
  mode: 0o600,
});
const database = resolve(directory, "station.sqlite");
await writeFile(database, "", { mode: 0o600 });
let mailCode = "";
const app = await build(
  {
    listen: "127.0.0.1",
    port,
    origin,
    database,
    textures: resolve(directory, "textures"),
    signingKey: keyPath,
    schools: { native: { emailDomains: ["school.test"] } },
    smtp: { host: "unused.invalid", port: 465, from: "native@school.test" },
    webRoot: resolve(
      import.meta.dirname,
      "../../../federated-skinstation/dist"
    ),
  },
  {
    mail: async (_email, code) => {
      mailCode = code;
    },
  }
);
await app.listen({ host: "127.0.0.1", port });
const email = "native-device@school.test";
const password = randomBytes(24).toString("hex");
async function post(path, body) {
  const response = await fetch(origin + path, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok)
    throw new Error(`Fixture enrollment ${path} returned ${response.status}`);
}
await post("/api/email/start", {
  email,
  password,
  name: "NativeDevice",
  school: "native",
});
await post("/api/email/confirm", { email, code: mailCode });
mailCode = "";
for (const path of [database, database + "-wal", database + "-shm"])
  await chmod(path, 0o600).catch(() => {});
await writeFile(
  resolve(directory, "ready.json"),
  JSON.stringify({ origin, email, password }),
  { mode: 0o600 }
);
console.log(
  "Isolated real station ready; mail callback only; credentials remain in owner-only profile."
);
const stop = async () => {
  await app.close();
  process.exit(0);
};
process.once("SIGTERM", () => void stop());
process.once("SIGINT", () => void stop());
