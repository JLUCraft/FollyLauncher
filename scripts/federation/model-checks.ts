import assert from "node:assert/strict";
import { decodeFile, encodeFile } from "../../src/utils/federation/base64";

for (const length of [0, 1, 2, 3, 16383, 16384, 16385, 2 * 1024 * 1024]) {
  const bytes = Uint8Array.from({ length }, (_, i) => i % 256);
  assert.deepEqual(decodeFile(encodeFile(bytes)), bytes);
  assert.equal(encodeFile(bytes).length, Math.ceil(length / 3) * 4);
}
assert.throws(() => decodeFile("%%%"));
console.log("PASS: binary Base64 boundaries.");

const { parseDeltas } = await import("../../src/utils/federation/score-deltas");
for (const invalid of [
  "missing",
  "a 1.5",
  "a Infinity",
  "a 9007199254740992",
  "a 1\na 2",
])
  assert.equal(parseDeltas(invalid), null);
assert.equal(parseDeltas("a -2\nb 3")?.a, -2);
assert.equal(parseDeltas("__proto__ 1")?.__proto__, 1);

// A new season must offer unassigned leagues, before any team registers.
const { schedulableLeagues } =
  await import("../../src/utils/federation/season-schedule");
const freshLeague = {
  phase: "Registration" as const,
  rules: { seats: 1, cross_club: false },
  teams: {},
  player_scores: {},
  team_scores: {},
};
const scheduleState = {
  revision: 0,
  clock_ms: 0,
  members: {},
  events: {},
  signups: {},
  checkins: {},
  tasks: {},
  seasons: {
    fresh: { title: "Fresh", leagues: [] },
    other: { title: "Other", leagues: ["assigned"] },
  },
  leagues: {
    available: freshLeague,
    assigned: freshLeague,
    playing: { ...freshLeague, phase: "Playing" as const },
  },
};
assert.deepEqual(schedulableLeagues(scheduleState, "fresh"), ["available"]);
assert.deepEqual(schedulableLeagues(scheduleState, "missing"), []);
assert.deepEqual(schedulableLeagues(scheduleState, "other"), [
  "available",
  "assigned",
]);
console.log("PASS: new-season scheduling and existing-season ownership.");

const { CONNECTION_LINK_TRIGGER, buildConnectionLink, parseConnectionLink } =
  await import("../../src/utils/federation/connection-link");
const target = {
  address: "/ip4/10.0.0.8/tcp/29637/p2p/12D3KooWtest?x=1&y=2",
  peer: "peer-org",
  genesis: "a".repeat(64),
};
assert.deepEqual(
  parseConnectionLink(buildConnectionLink(target, "folly"), "folly"),
  target
);
assert.deepEqual(
  parseConnectionLink(`  ${buildConnectionLink(target, "folly")}\n`, "folly"),
  target
);
const query = new URL(buildConnectionLink(target, "folly")).search;
for (const route of ["federation", "federation/"]) {
  assert.equal(CONNECTION_LINK_TRIGGER.test(`${route}${query}`), true);
  assert.deepEqual(
    parseConnectionLink(`folly://${route}${query}`, "folly"),
    target
  );
}
assert.deepEqual(
  parseConnectionLink(`folly://federation${query}&peer=ignored`, "folly"),
  target
);
assert.deepEqual(
  parseConnectionLink(
    `folly://federation${query.replace("address=", "%61ddress=")}`,
    "folly"
  ),
  target
);
for (const route of [
  "connect",
  "launch",
  "add-auth-server",
  "import-modpack",
  "reload-extension",
  "federation/extra",
  "other/federation",
]) {
  assert.equal(CONNECTION_LINK_TRIGGER.test(`${route}${query}`), false);
}
for (const invalid of [
  "",
  "not a link",
  "folly://connect?address=a&peer=b&genesis=" + "a".repeat(64),
  "other://federation?address=a&peer=b&genesis=" + "a".repeat(64),
  "folly://federation?address=a&peer=b",
  "folly://federation?address=a&peer=b&genesis=" + "A".repeat(64),
  `sjmcl://federation${query}`,
  `folly:///federation${query}`,
  `folly://federation/extra${query}`,
  `folly://user@federation${query}`,
  `folly://federation:1234${query}`,
  `folly://federation?address&${query.slice(1)}`,
  `folly://federation${query.replace(/address=[^&]*/, "address=++")}`,
])
  assert.equal(parseConnectionLink(invalid, "folly"), null);
console.log("PASS: connection links.");
