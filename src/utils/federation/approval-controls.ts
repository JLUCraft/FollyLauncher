import type { OfficeFlow, OfficeSnapshot } from "@/services/office";

// Display hints only. The server independently revalidates every signature and permission.
export function approvalControls(
  approver: Record<string, unknown> | undefined,
  record: OfficeFlow["records"][number] | undefined,
  identity: OfficeSnapshot["identity"],
  member: string | undefined
) {
  const none = { approve: false, reject: false, delegate: false };
  if (!approver || !record || !member || !identity.members[member]) return none;
  const members = identity.members;
  const eligible = (id: string) => {
    const m = members[id];
    if (!m) return undefined;
    switch (approver.type) {
      case "member":
        return approver.member === id ? "member" : undefined;
      case "role":
        return approver.club === m.club &&
          typeof approver.role === "string" &&
          m.roles.includes(approver.role)
          ? "role"
          : undefined;
      case "club_quorum":
        return Array.isArray(approver.clubs) && approver.clubs.includes(m.club)
          ? m.club
          : undefined;
      default:
        return undefined;
    }
  };
  const ownSlot = eligible(member);
  const delegatedSlot = Object.entries(record.delegations).find(([slot, d]) => {
    if (d.to_member !== member) return false;
    const owner = Object.entries(members).find(
      ([, m]) =>
        m.device === d.from_device ||
        m.additional_devices?.includes(d.from_device)
    );
    return (
      !!owner &&
      owner[1].club === members[member].club &&
      eligible(owner[0]) === slot
    );
  })?.[0];
  const slot =
    delegatedSlot ??
    (ownSlot && !record.delegations[ownSlot] ? ownSlot : undefined);
  return {
    approve: !!slot && !record.approvals[slot],
    reject: !!slot && !record.approvals[slot],
    delegate:
      !!ownSlot &&
      !record.delegations[ownSlot] &&
      !record.approvals[ownSlot] &&
      !Object.values(record.delegations).some((d) => d.to_member === member),
  };
}
