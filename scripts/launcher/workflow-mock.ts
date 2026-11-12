// Explicit browser fixtures only; never imported by the application.
import { emit } from "@tauri-apps/api/event";
import { updateByKeyPath } from "@/utils/partial";
import { config, fixtureWindow } from "../federation/mock-ipc";

const w = fixtureWindow;
const scenario = new URL(location.href).searchParams.get("test-scenario");
const instance = {
  id: "acceptance:1.21",
  name: "Acceptance instance",
  version: "1.21",
  majorVersion: "1.21",
  versionPath: "/acceptance/versions/1.21",
  iconSrc: "/images/icons/JEIcon_Release.png",
  starred: false,
  playTime: 0,
  modLoader: { loaderType: "Unknown", status: "Installed" },
  supportQuickPlay: true,
  useSpecGameConfig: false,
  isVersionIsolated: true,
};
const player = {
  id: "acceptance-player",
  name: "Acceptance player",
  uuid: "00000000-0000-4000-8000-000000000001",
  playerType: "microsoft",
  avatar: [],
  textures: [],
};
const hasInstances = ["instances", "selection", "ready"].includes(
  scenario ?? ""
);
const hasPlayers = ["selection", "ready"].includes(scenario ?? "");
if (scenario === "instances" || scenario === "ready")
  config.states.shared.selectedInstanceId = instance.id;
if (scenario === "ready") config.states.shared.selectedPlayerId = player.id;
w.testConfig = config;
w.testFilePath = "/acceptance/organizer.json";
w.testFileContent = JSON.stringify({
  genesis: { federation: "acceptance" },
  genesis_hash: "a".repeat(64),
});
w.testIdentityConfiguration =
  scenario === "configured" ? (JSON.parse(w.testFileContent) as unknown) : null;
w.testHandlers = {
  retrieve_launcher_config: () => structuredClone(config),
  retrieve_player_list: () => (hasPlayers ? [player] : []),
  retrieve_instance_list: () => (hasInstances ? [instance] : []),
  retrieve_other_launcher_account_info: () => [[], []],
  fetch_game_version_list: () => [
    {
      id: "1.21",
      gameType: "release",
      releaseTime: "2026-01-01T00:00:00Z",
      url: "https://game.example.test/version.json",
    },
  ],
  retrieve_auth_server_list: () => [
    {
      name: "",
      authUrl: "https://skin.example.test",
      homepageUrl: "https://skin.example.test",
      registerUrl: "https://skin.example.test/register",
      mutable: true,
      features: {},
      clientId: "acceptance",
    },
  ],
  cancel_oauth: () => undefined,
  cancel_launch_process: () => undefined,
  select_suitable_jre: () => {
    throw "ACCEPTANCE_PREFLIGHT_STOP";
  },
  union_public_identity: () => ({ peer: "acceptance-device" }),
  union_identity_configuration: (args) => {
    if (args.config !== null) w.testIdentityConfiguration = args.config;
    return w.testIdentityConfiguration;
  },
  "plugin:dialog|open": () => w.testFilePath,
  read_file: () => w.testFileContent,
  update_launcher_config: async (args) => {
    if (typeof args.keyPath !== "string" || typeof args.value !== "string")
      throw new Error("Config update requires a path and encoded value");
    updateByKeyPath(config, args.keyPath, JSON.parse(args.value) as unknown);
    await emit("config:partial-update", {
      path: args.keyPath,
      value: args.value,
    });
  },
};
