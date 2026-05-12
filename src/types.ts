











export type AdmissionMode =
  | "public"
  | "vc_only" | "vc-only"
  | "club_only" | "club-only"
  | "mua_member" | "mua-member"
  | "unknown";

export interface Instance {
  id: string;
  name: string;
  mode: string;
  club: string;
  type: "service" | "room";
  players: string;
  latency: number | null;
  state: string;
  version: string;
  peer_id: string | null;

  admission_mode?: AdmissionMode;

  requires_vc?: boolean;

  max_players?: number;
}


export function admissionLabel(mode: AdmissionMode): string {
  switch (mode) {
    case "public": return "公开";
    case "vc_only": case "vc-only": return "VC 成员";
    case "club_only": case "club-only": return "社团成员";
    case "mua_member": case "mua-member": return "MUA 成员";
    default: return "未知";
  }
}


export function admissionColor(mode: AdmissionMode): string {
  switch (mode) {
    case "public": return "bg-teal-100 text-teal-800 border-teal-200";
    case "vc_only": case "vc-only": return "bg-blue-100 text-blue-800 border-blue-200";
    case "club_only": case "club-only": return "bg-purple-100 text-purple-800 border-purple-200";
    case "mua_member": case "mua-member": return "bg-amber-100 text-amber-800 border-amber-200";
    default: return "bg-stone-100 text-stone-600 border-stone-200";
  }
}
