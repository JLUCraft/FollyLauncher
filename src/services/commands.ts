import type { AccountService } from "@/services/account";
import type { ConfigService } from "@/services/config";
import type { ExtensionService } from "@/services/extension";
import type { InstanceService } from "@/services/instance";
import type { LaunchService } from "@/services/launch";
import type { ResourceService } from "@/services/resource";
import type { TaskService } from "@/services/task";
import type { UtilsService } from "@/services/utils";

export const nativeCommands = {
  account: {
    retrievePlayerList: "retrieve_player_list",
    addPlayerOffline: "add_player_offline",
    fetchOAuthCode: "fetch_oauth_code",
    retrieveAuthServerList: "retrieve_auth_server_list",
  },
  config: {
    retrieveLauncherConfig: "retrieve_launcher_config",
    updateLauncherConfig: "update_launcher_config",
    retrieveJavaList: "retrieve_java_list",
    validateJava: "validate_java",
  },
  extension: {
    retrieveExtensionList: "retrieve_extension_list",
    addExtension: "add_extension",
  },
  instance: {
    retrieveInstanceList: "retrieve_instance_list",
    createInstance: "create_instance",
    retrieveWorldDetails: "retrieve_world_details",
    changeModLoader: "change_mod_loader",
    retrieveModpackMetaInfo: "retrieve_modpack_meta_info",
  },
  launch: {
    selectSuitableJRE: "select_suitable_jre",
    validateGameFiles: "validate_game_files",
    validateSelectedPlayer: "validate_selected_player",
    launchGame: "launch_game",
    cancelLaunchProcess: "cancel_launch_process",
    retrieveGameLog: "retrieve_game_log",
    retrieveGameLaunchingState: "retrieve_game_launching_state",
  },
  resource: {
    fetchGameVersionSpecific: "fetch_game_version_specific",
    fetchModLoaderVersionList: "fetch_mod_loader_version_list",
    fetchOptiFineVersionList: "fetch_optifine_version_list",
    fetchResourceListByName: "fetch_resource_list_by_name",
  },
  task: {
    scheduleProgressiveTaskGroup: "schedule_progressive_task_group",
    cancelProgressiveTaskGroup: "cancel_progressive_task_group",
    stopProgressiveTaskGroup: "stop_progressive_task_group",
    resumeProgressiveTaskGroup: "resume_progressive_task_group",
    deleteProgressiveTaskGroup: "delete_progressive_task_group",
    retrieveProgressiveTaskList: "retrieve_progressive_task_list",
  },
  utils: {
    retrieveFontList: "retrieve_truetype_font_list",
    createWindow: "create_window",
  },
} as const satisfies {
  account: Partial<Record<keyof typeof AccountService, string>>;
  config: Partial<Record<keyof typeof ConfigService, string>>;
  extension: Partial<Record<keyof typeof ExtensionService, string>>;
  instance: Partial<Record<keyof typeof InstanceService, string>>;
  launch: Partial<Record<keyof typeof LaunchService, string>>;
  resource: Partial<Record<keyof typeof ResourceService, string>>;
  task: Partial<Record<keyof typeof TaskService, string>>;
  utils: Partial<Record<keyof typeof UtilsService, string>>;
};
