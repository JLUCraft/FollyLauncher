import { invoke } from "@tauri-apps/api/core";
import { decodeFile, encodeFile } from "@/utils/federation/base64";

export interface OfficeTarget {
  address: string;
  peer: string;
  genesis: string;
}
export interface OfficeEvent {
  title: string;
  brief_hash: string;
  organizer: string;
  creator: string;
  co_organizers: string[];
  phase: string;
  epoch: number;
  window: { opens_ms: number; closes_ms: number; starts_ms: number };
}
export interface OfficeTask {
  event: string;
  title: string;
  assignee: string | null;
  done: boolean;
  due_ms: number | null;
}
export interface OfficeFlow {
  template: string;
  template_epoch: number;
  payload: Record<string, unknown>;
  proposer: string;
  current: number;
  status: string;
  records: {
    approvals: Record<string, { device: string; comment_hash: string }>;
    delegations: Record<string, { from_device: string; to_member: string }>;
    ai_runs?: { job: string; output: string | null; error: string | null }[];
  }[];
}
export interface OfficeCommit {
  type: "commit";
  parent: { "/": string } | null;
  files: Record<string, { "/": string }>;
  message: string;
}
export interface OfficeSnapshot {
  state: {
    revision: number;
    clock_ms: number;
    events: Record<string, OfficeEvent>;
    tasks: Record<string, OfficeTask>;
    signups: Record<string, string[]>;
    checkins?: Record<string, string[]>;
    flows: Record<string, OfficeFlow>;
    flow_templates: Record<
      string,
      {
        active: boolean;
        versions: {
          name: string;
          stages: { name: string; approver: Record<string, unknown> }[];
          payload_kinds: string[];
        }[];
      }
    >;
    files?: {
      branches: Record<
        string,
        { commit: string; club: string; author: string; revision: number }
      >;
    };
  };
  identity: {
    admins: string[];
    members: Record<
      string,
      {
        club: string;
        device: string;
        additional_devices?: string[];
        roles: string[];
      }
    >;
  };
  commits: Record<string, OfficeCommit>;
  max_file_bytes: number;
  max_commit_paths: number;
}
export class OfficeService {
  static snapshot(target: OfficeTarget): Promise<OfficeSnapshot> {
    return invoke("union_office_snapshot", { target });
  }
  static action(
    target: OfficeTarget,
    action: Record<string, unknown>
  ): Promise<void> {
    return invoke("union_office_action", { target, action });
  }
  static upload(
    target: OfficeTarget,
    input: { branch: string; path: string; message: string; bytes: Uint8Array }
  ): Promise<string> {
    return invoke("union_office_upload", {
      target,
      input: {
        branch: input.branch,
        path: input.path,
        message: input.message,
        data: encodeFile(input.bytes),
      },
    });
  }
  static async fetch(
    target: OfficeTarget,
    cid: string
  ): Promise<Uint8Array<ArrayBuffer>> {
    return decodeFile(
      await invoke<string>("union_office_fetch", { target, cid })
    );
  }
}
