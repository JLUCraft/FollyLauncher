// Browser test fixtures only. Never imported by the application.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { brand } from "@/brand";
import { defaultConfig } from "@/models/config";

const config = structuredClone(defaultConfig);
config.mocked = false;
config.runCount = 10;
const w = window as any;
w.calls = [];
w.admin = false;
w.proxy = null;
w.failOffice = false;
w.scheme = brand.deeplinkScheme;
w.failOfficeAction = false;
w.delayFetch = false;
w.fetchResolvers = {};
const member = {
  club: "club-a",
  device: "device-a",
  additional_devices: [],
  roles: [],
};
const office = {
  state: {
    revision: 1,
    clock_ms: 1000,
    events: {},
    tasks: {},
    signups: {},
    checkins: {},
    flow_templates: {},
    flows: {},
    files: { branches: {} },
  },
  identity: { admins: [], members: { "member-a": member } },
  commits: {},
  max_file_bytes: 2097152,
  max_commit_paths: 24,
};
w.office = office;
mockWindows("main");
mockIPC(
  async (cmd, args: any) => {
    w.calls.push({ cmd, args });
    if (cmd === "update_launcher_config") return;
    if (cmd === "retrieve_launcher_config") return config;
    if (cmd === "union_identity") return "device-a";
    if (cmd === "union_identity_configuration") return null;
    if (cmd === "union_governance")
      return {
        state: { revision: 1, clock_ms: 1000, leagues: {} },
        identity_view: {
          members: { "member-a": member },
          admins: w.admin ? ["device-a"] : [],
        },
        error: null,
      };
    if (cmd === "union_proxy_status") return w.proxy;
    if (cmd === "union_proxy_start") {
      w.proxy = {
        address: "127.0.0.1:12345",
        peer: args.input.peer,
        service: args.input.service,
      };
      return w.proxy;
    }
    if (cmd === "union_proxy_stop") {
      w.proxy = null;
      return;
    }
    if (cmd === "union_room_status") return null;
    if (cmd === "union_rooms")
      return {
        rooms: [
          {
            peer: "host-a",
            service: "room-a",
            name: "Club survival",
            addresses: [],
            metadata: { school: "School A", version: "1.21", players_max: 20 },
          },
          {
            peer: "host-b",
            service: "room-b",
            name: "Study build",
            addresses: [],
            metadata: { school: "School B", version: "1.20", players_max: 10 },
          },
        ],
        unavailable: [],
      };
    if (cmd === "union_office_snapshot") {
      if (w.failOffice) throw "offline";
      return structuredClone(w.office);
    }
    if (cmd === "union_office_action") {
      if (w.failOfficeAction) throw "denied";
      return;
    }
    if (cmd === "union_office_upload") return "bafy-file";
    if (cmd === "union_office_fetch") {
      if (w.delayFetch)
        return new Promise((resolve) => {
          w.fetchResolvers[args.cid] = resolve;
        });
      return "QUJD";
    }
    if (cmd === "plugin:deep-link|get_current")
      return new URL(location.href).searchParams.getAll("test-deep-link");
    if (cmd.includes("get_current")) return null;
    if (
      cmd.includes("plugin:log|") ||
      cmd.includes("plugin:window|") ||
      cmd.includes("plugin:webview|")
    )
      return null;
    if (cmd.startsWith("retrieve_") || cmd.startsWith("list_")) return [];
    throw "TEST_UNAVAILABLE";
  },
  { shouldMockEvents: true }
);
