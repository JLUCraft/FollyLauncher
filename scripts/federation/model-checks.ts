import assert from "node:assert/strict";
import type { OfficeFlow, OfficeSnapshot } from "../../src/services/office";
import { approvalControls } from "../../src/utils/federation/approval-controls";
import { decodeFile, encodeFile } from "../../src/utils/federation/base64";

const identity: OfficeSnapshot["identity"] = {
  admins: [],
  members: {
    alice: { club: "a", device: "alice-device", roles: ["chair"] },
    bob: { club: "a", device: "bob-device", roles: [] },
    carol: { club: "b", device: "carol-device", roles: ["chair"] },
  },
};
const blank: OfficeFlow["records"][number] = { approvals: {}, delegations: {} };
const member = { type: "member", member: "alice" };
const role = { type: "role", club: "a", role: "chair" };
const quorum = { type: "club_quorum", clubs: ["a", "b"], quorum: 2 };
assert.deepEqual(approvalControls(member, blank, identity, "alice"), {
  approve: true,
  reject: true,
  delegate: true,
});
assert.equal(approvalControls(member, blank, identity, "bob").approve, false);
assert.equal(approvalControls(role, blank, identity, "alice").approve, true);
assert.equal(approvalControls(role, blank, identity, "carol").approve, false);
assert.equal(approvalControls(quorum, blank, identity, "bob").approve, true);
assert.equal(
  approvalControls({ type: "ai", worker: "alice" }, blank, identity, "alice")
    .approve,
  false
);
const delegated = {
  ...blank,
  delegations: { member: { from_device: "alice-device", to_member: "bob" } },
};
assert.deepEqual(approvalControls(member, delegated, identity, "alice"), {
  approve: false,
  reject: false,
  delegate: false,
});
assert.deepEqual(approvalControls(member, delegated, identity, "bob"), {
  approve: true,
  reject: true,
  delegate: false,
});
assert.equal(
  approvalControls(
    member,
    {
      ...blank,
      delegations: {
        member: { from_device: "alice-device", to_member: "carol" },
      },
    },
    identity,
    "carol"
  ).approve,
  false
);
assert.equal(
  approvalControls(
    member,
    {
      ...blank,
      delegations: {
        member: { from_device: "revoked-device", to_member: "bob" },
      },
    },
    identity,
    "bob"
  ).approve,
  false
);
assert.equal(
  approvalControls(
    quorum,
    {
      ...blank,
      approvals: { a: { device: "alice-device", comment_hash: "digest" } },
    },
    identity,
    "bob"
  ).approve,
  false
);
assert.equal(
  approvalControls(member, blank, identity, undefined).approve,
  false
);
for (const length of [0, 1, 2, 3, 16383, 16384, 16385, 2 * 1024 * 1024]) {
  const bytes = Uint8Array.from({ length }, (_, i) => i % 256);
  assert.deepEqual(decodeFile(encodeFile(bytes)), bytes);
  assert.equal(encodeFile(bytes).length, Math.ceil(length / 3) * 4);
}
assert.throws(() => decodeFile("%%%"));
console.log(
  "PASS: approval display eligibility, delegation, revoked devices, quorum slots, binary Base64 boundaries."
);

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
