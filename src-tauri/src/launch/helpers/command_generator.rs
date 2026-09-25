use base64::Engine;
use base64::engine::general_purpose;
use sjmcl_types::error::{SJMCLError, SJMCLResult};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

use crate::account::helpers::authlib_injector::jar::get_jar_path as get_authlib_injector_jar_path;
use crate::account::models::{AccountError, PlayerType};
use crate::instance::helpers::game_version::compare_game_versions;
use crate::instance::helpers::misc::get_instance_subdir_paths;
use crate::instance::models::misc::{InstanceError, InstanceSubdirType};
use crate::launch::helpers::file_validator::get_nonnative_library_paths;
use crate::launch::helpers::misc::get_separator;
use crate::launch::models::{LaunchError, LaunchingState};
use crate::launcher_config::models::*;
use crate::utils::fs::get_app_resource_filepath;
use crate::utils::sys_info::get_memory_info;
use minecraftd::minecraft::plan::{
  AuthAgent, GameOptions, JvmOptions, LaunchArguments, LaunchPlan, Proxy,
};
pub use minecraftd::minecraft::plan::{LaunchCommand, export_full_launch_command};

#[cfg(target_os = "windows")]
use crate::launch::helpers::file_validator::get_windows_mesa_loader_path;
#[cfg(target_os = "windows")]
use crate::launcher_config::helpers::graphics::mesa_driver_name;
#[cfg(target_os = "macos")]
use crate::launcher_config::helpers::graphics::{find_macos_libvulkan, find_vulkan_icd_file};

pub async fn generate_launch_command(
  app: &AppHandle,
  quick_play_singleplayer: Option<String>,
  quick_play_multiplayer: Option<String>,
) -> SJMCLResult<LaunchCommand> {
  let launcher_config = { app.state::<Mutex<LauncherConfig>>().lock()?.clone() };
  let launching_queue = { app.state::<Mutex<Vec<LaunchingState>>>().lock()?.clone() };

  let LauncherConfig { basic_info, .. } = launcher_config;
  let launching = launching_queue
    .last()
    .ok_or(LaunchError::LaunchingStateNotFound)?
    .clone();
  let LaunchingState {
    selected_java,
    selected_instance,
    game_config,
    client_info,
    auth_server_meta,
    ..
  } = launching;
  let selected_player = launching
    .selected_player
    .clone()
    .ok_or(AccountError::NotFound)?;
  let client_jar_path = selected_instance
    .version_path
    .join(format!("{}.jar", selected_instance.name))
    .to_string_lossy()
    .to_string();

  #[allow(unused_mut)]
  let mut platform_args = Vec::new();

  // -----------------------------------------
  // Part 1: Prepare Arguments
  // -----------------------------------------
  let related_dirs = get_instance_subdir_paths(
    app,
    &selected_instance,
    &[
      &InstanceSubdirType::Root,
      &InstanceSubdirType::Assets,
      &InstanceSubdirType::Libraries,
      &InstanceSubdirType::NativeLibraries,
    ],
  )
  .ok_or(InstanceError::InstanceNotFoundByID)?;

  let [root_dir, assets_dir, libraries_dir, natives_dir] = related_dirs.as_slice() else {
    return Err(InstanceError::InstanceNotFoundByID.into());
  };

  let mut seen_paths = HashSet::new();
  let mut class_paths: Vec<String> = get_nonnative_library_paths(
    &client_info,
    libraries_dir,
    selected_instance.mod_loader.loader_type,
  )?
  .into_iter()
  .filter(|path| seen_paths.insert(path.clone()))
  .map(|p| p.to_string_lossy().to_string())
  .collect();
  class_paths.push(client_jar_path.clone());

  let quickplay_server_url = match quick_play_multiplayer {
    Some(ref url) if !url.is_empty() => url.clone(),
    None if game_config.game_server.auto_join => game_config.game_server.server_url.clone(),
    _ => String::new(),
  };

  let custom_info = game_config.game_window.custom_info.trim();
  let custom_info = (!custom_info.is_empty()).then(|| custom_info.to_string());

  let arguments_value = LaunchArguments {
    game_assets: assets_dir
      .join("virtual/legacy")
      .to_string_lossy()
      .to_string(),
    assets_root: assets_dir.to_string_lossy().to_string(),
    assets_index_name: client_info.asset_index.id.clone(),
    game_directory: root_dir.to_string_lossy().to_string(),

    version_name: selected_instance.name.clone(),
    primary_jar_name: format!("{}.jar", selected_instance.name.clone()),
    version_type: custom_info
      .clone()
      .unwrap_or_else(|| client_info.type_.clone()),
    natives_directory: natives_dir.to_string_lossy().to_string(),
    launcher_name: "SJMC Launcher".to_string(),
    launcher_version: basic_info.launcher_version,
    library_directory: libraries_dir.to_string_lossy().to_string(),
    classpath_separator: get_separator().to_string(),

    auth_access_token: selected_player.access_token.unwrap_or_default(),
    auth_player_name: selected_player.name,
    user_type: "msa".to_string(), // TODO
    auth_uuid: selected_player.uuid.to_string(),
    auth_xuid: "".to_string(), // TODO
    demo: false,
    user_properties: "{}".to_string(),
    clientid: "".to_string(),
    resolution_height: game_config.game_window.resolution.height,
    resolution_width: game_config.game_window.resolution.width,
    quick_play_path: String::new(),
    quick_play_singleplayer: quick_play_singleplayer.clone().unwrap_or_default(),
    quick_play_multiplayer: quickplay_server_url.clone(),
    quick_play_realms: String::new(),
  };

  // -----------------------------------------
  // Part 2: JVM
  // ref: https://github.com/HMCL-dev/HMCL/blob/c33ef5170b2cc726f01674fe6fca28035b1eef8b/HMCLCore/src/main/java/org/jackhuang/hmcl/launch/DefaultLauncher.java#L106
  // -----------------------------------------

  let jvm = &game_config.advanced.jvm;
  let game_proxy = &game_config.advanced.proxy;
  let mut options = JvmOptions {
    max_memory_mb: if game_config.performance.auto_mem_allocation {
      (get_memory_info().suggested_max_alloc / 1024 / 1024) as u32
    } else {
      game_config.performance.max_mem_allocation
    },
    java_major: selected_java.major_version,
    permanent_space_mb: jvm.java_permanent_generation_space,
    garbage_collector: match jvm.garbage_collector {
      GarbageCollector::Auto => minecraftd::minecraft::plan::GarbageCollector::Auto,
      GarbageCollector::G1gc => minecraftd::minecraft::plan::GarbageCollector::G1gc,
      GarbageCollector::Zgc => minecraftd::minecraft::plan::GarbageCollector::Zgc,
      GarbageCollector::Shenandoah => minecraftd::minecraft::plan::GarbageCollector::Shenandoah,
      GarbageCollector::Parallel => minecraftd::minecraft::plan::GarbageCollector::Parallel,
      GarbageCollector::Serial => minecraftd::minecraft::plan::GarbageCollector::Serial,
    },
    extra: jvm.args.clone(),
    proxy: if !game_proxy.enabled {
      Proxy::System
    } else {
      match game_proxy.selected_type {
        ProxyType::Http => Proxy::Http {
          host: game_proxy.host.clone(),
          port: u16::try_from(game_proxy.port).map_err(|e| SJMCLError(e.to_string()))?,
        },
        ProxyType::Socks => Proxy::Socks {
          host: game_proxy.host.clone(),
          port: u16::try_from(game_proxy.port).map_err(|e| SJMCLError(e.to_string()))?,
        },
      }
    },
    client_jar: (!game_config.advanced.workaround.no_jvm_args).then_some(client_jar_path),
    ..Default::default()
  };
  #[cfg(target_os = "windows")]
  if let Some(driver_name) = mesa_driver_name(
    &game_config.advanced.graphics.api,
    &game_config.advanced.graphics.renderer,
  ) {
    if let Some(mesa_loader_path) = get_windows_mesa_loader_path(app, libraries_dir)? {
      platform_args.push(format!(
        "-javaagent:{}={}",
        mesa_loader_path.to_string_lossy(),
        driver_name
      ));
      platform_args.push(format!(
        "-Dorg.glavo.mesa.loader.nativeDir={}",
        natives_dir.join("mesa-loader").to_string_lossy()
      ));
    }
  }

  #[cfg(target_os = "macos")]
  if game_config.advanced.graphics.api == GraphicsApi::Vulkan
    && game_config.advanced.graphics.renderer != "moltenvk"
    && find_vulkan_icd_file(&game_config.advanced.graphics.renderer).is_some()
    && let Some(libvulkan_path) = find_macos_libvulkan()
  {
    platform_args.push(format!(
      "-Dorg.lwjgl.vulkan.libname={}",
      libvulkan_path.to_string_lossy()
    ));
  }

  // login via authlib-injector
  if matches!(
    selected_player.player_type,
    PlayerType::ThirdParty | PlayerType::Offline
  ) && let Some(auth_server_meta) = auth_server_meta.as_ref()
  {
    let auth_server_url = selected_player
      .auth_server_url
      .clone()
      .ok_or(LaunchError::AuthServerNotFound)?;
    let custom = &game_config.advanced.workaround.use_custom_authlib_injector;
    let trimmed_path = custom.path.trim();
    let custom_path = PathBuf::from(trimmed_path);
    let authlib_jar_path = if custom.enabled && !trimmed_path.is_empty() && custom_path.is_file() {
      custom_path
    } else {
      get_authlib_injector_jar_path(app)?
    };
    options.auth_agent = Some(AuthAgent {
      jar: authlib_jar_path.to_string_lossy().into_owned(),
      server: auth_server_url,
      prefetched_base64: general_purpose::STANDARD.encode(auth_server_meta),
    });
  }

  if game_config.advanced.workaround.use_lwjgl_unsafe_agent
    && minecraftd::minecraft::plan::needs_unsafe_agent(&client_info, selected_java.major_version)
  {
    match get_app_resource_filepath(app, "assets/game/lwjgl-unsafe-agent.jar") {
      Ok(agent_path) => options.unsafe_agent = Some(agent_path.to_string_lossy().into_owned()),
      Err(error) => log::warn!("Failed to resolve lwjgl-unsafe-agent.jar: {:?}", error),
    }
  }

  // -----------------------------------------
  // Part 3: Replace JVM and game arguments
  // -----------------------------------------
  // some feature rules defined in client json, add to jvm/game arg templates
  let (jvm_template, game_template) = minecraftd::minecraft::plan::profile_templates(
    &client_info,
    &arguments_value,
    crate::instance::helpers::client_json::rule_platform(),
  )
  .map_err(|error| SJMCLError(error.to_string()))?;
  let legacy_server = if !quickplay_server_url.is_empty()
    && compare_game_versions(app, &selected_instance.version, "23w14a", false)
      .await
      .is_lt()
  {
    Some(quickplay_server_url)
  } else {
    None
  };
  let graphics_backend = if game_config.advanced.graphics.api != GraphicsApi::Default
    && compare_game_versions(app, &selected_instance.version, "26.2-snapshot-2", false)
      .await
      .is_ge()
  {
    match game_config.advanced.graphics.api {
      GraphicsApi::Opengl => Some("opengl".into()),
      GraphicsApi::Vulkan => Some("vulkan".into()),
      GraphicsApi::Default => None,
    }
  } else {
    None
  };
  options.platform_args = platform_args;
  LaunchPlan {
    class_paths,
    parameters: arguments_value,
    custom_version_name: custom_info,
    main_class: client_info
      .main_class
      .ok_or(InstanceError::MainClassNotFound)?,
    jvm_template,
    game_template,
    jvm: options,
    game: GameOptions {
      legacy_server,
      fullscreen: game_config.game_window.resolution.fullscreen,
      graphics_backend,
      extra: game_config
        .advanced
        .custom_commands
        .minecraft_argument
        .clone(),
    },
  }
  .render()
  .map_err(|e| SJMCLError(e.to_string()))
}
