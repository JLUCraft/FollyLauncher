// Browser test fixtures only. Never imported by the application.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { brand } from "@/brand";
import { defaultConfig } from "@/models/config";

export const config = structuredClone(defaultConfig);
config.mocked = false;
config.runCount = 10;
const w = window as any;
w.calls = [];
w.admin = false;
w.proxy = null;
w.scheme = brand.deeplinkScheme;
const member = {
  club: "club-a",
  device: "device-a",
  additional_devices: [],
  roles: [],
};
w.application = {
  revision: 1,
  clock_ms: 1000,
  leagues: {},
  seasons: {},
  disputes: {},
  events: {},
  tasks: {},
  signups: {},
  checkins: {},
  organization: {},
  flow_templates: {},
  flows: {},
  files: { branches: {} },
};
mockWindows("main");
mockIPC(
  async (cmd, args: any) => {
    w.calls.push({ cmd, args });
    if (w.testHandlers?.[cmd]) return await w.testHandlers[cmd](args);
    if (cmd === "update_launcher_config") return;
    if (cmd === "retrieve_launcher_config") return config;
    if (cmd === "union_identity") return "device-a";
    if (cmd === "union_identity_configuration") return null;
    if (cmd === "union_discover") return { servers: w.discoverServers || [] };
    if (cmd === "union_governance")
      return {
        state: structuredClone(w.application),
        activity_briefs: {},
        season_rankings: {},
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
