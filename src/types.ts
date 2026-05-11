/// ADR: Tauri IPC is used as the local API substitute between frontend and Rust backend.
/// gRPC is not introduced at this stage because the launcher is a single-user desktop app
/// with a co-located frontend; Tauri invoke() provides sufficient type safety and
/// serialization at zero additional operational cost. If multi-process or remote
/// launcher management becomes necessary, the command layer can be re-exported via a
/// tonic gRPC server without changing the internal service implementations.
///
/// See DESIGN.md section 3.6 and src-tauri/src/proxy.rs ADR comment.

/// Frontend uses snake_case internally but server returns kebab-case.
/// Both formats are accepted; normalization happens at the Rust boundary.
/// See src-tauri/src/api.rs:normalize_admission_mode() for the Rust-side conversion.
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
  /** Admission mode from DHT/API. Used by AdmissionBadge. */
  admission_mode?: AdmissionMode;
  /** True when the instance requires a club membership VC. */
  requires_vc?: boolean;
  /** Max players as reported by the instance. */
  max_players?: number;
}

/** Maps AdmissionMode to display label. Supports both snake_case and kebab-case. */
export function admissionLabel(mode: AdmissionMode): string {
  switch (mode) {
    case "public": return "公开";
    case "vc_only": case "vc-only": return "VC 成员";
    case "club_only": case "club-only": return "社团成员";
    case "mua_member": case "mua-member": return "MUA 成员";
    default: return "未知";
  }
}

/** Maps AdmissionMode to accent color class. */
export function admissionColor(mode: AdmissionMode): string {
  switch (mode) {
    case "public": return "bg-teal-100 text-teal-800 border-teal-200";
    case "vc_only": case "vc-only": return "bg-blue-100 text-blue-800 border-blue-200";
    case "club_only": case "club-only": return "bg-purple-100 text-purple-800 border-purple-200";
    case "mua_member": case "mua-member": return "bg-amber-100 text-amber-800 border-amber-200";
    default: return "bg-stone-100 text-stone-600 border-stone-200";
  }
}
