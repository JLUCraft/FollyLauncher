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
