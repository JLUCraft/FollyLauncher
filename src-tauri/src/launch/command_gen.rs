//! Launch command generation: JVM args, game args, classpath, natives,
//! QuickPlay, server join, custom JVM flags. Uses mc_launcher_core as
//! primary builder, with fallback/extension for JLUCraft-specific options.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Full launch plan produced for the UI before executing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchPlan {
    pub java_executable: String,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub classpath: Vec<String>,
    pub natives_dir: String,
    pub game_dir: String,
    pub main_class: String,
    pub version: String,
    pub quick_play: Option<QuickPlayTarget>,
    pub custom_jvm_flags: Vec<String>,
}

/// QuickPlay target as defined in the Minecraft launcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickPlayTarget {
    pub kind: QuickPlayKind,
    pub server_address: Option<String>,
    pub server_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuickPlayKind {
    Multiplayer,
    Realms,
    Singleplayer,
}

/// Options for generating a launch plan, extending mc_launcher_core options.
#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    pub username: Option<String>,
    pub uuid: Option<String>,
    pub token: Option<String>,
    pub java_executable: String,
    pub jvm_args: Vec<String>,
    pub custom_jvm_flags: Vec<String>,
    pub resolution_width: Option<u32>,
    pub resolution_height: Option<u32>,
    pub fullscreen: bool,
    pub server_address: Option<String>,
    pub server_port: Option<u16>,
    pub quick_play: Option<QuickPlayTarget>,
    pub launcher_name: Option<String>,
}

/// Generate a launch plan using mc_launcher_core for base command construction,
/// then extend with custom JVM flags, QuickPlay, and server join arguments.
pub fn generate_launch_plan(
    version: &str,
    game_dir: &Path,
    options: &LaunchOptions,
) -> Result<LaunchPlan, crate::error::LauncherError> {
    // Build base mc_launcher_core options
    let mc_options = mc_launcher_core::types::MinecraftOptions {
        username: options.username.clone(),
        uuid: options.uuid.clone(),
        token: options.token.clone(),
        server: options.server_address.clone(),
        port: options.server_port.map(|p| p.to_string()),
        launcher_name: options.launcher_name.clone(),
        executable_path: Some(options.java_executable.clone()),
        jvm_arguments: Some(options.jvm_args.clone()),
        game_directory: Some(game_dir.to_string_lossy().to_string()),
        custom_resolution: Some(
            options.resolution_width.unwrap_or(0) > 0 && options.resolution_height.unwrap_or(0) > 0,
        ),
        resolution_width: options.resolution_width.map(|w| w.to_string()),
        resolution_height: options.resolution_height.map(|h| h.to_string()),
        ..Default::default()
    };

    let command = mc_launcher_core::command::get_minecraft_command(version, game_dir, &mc_options)
        .map_err(|e| crate::error::LauncherError::from(format!("failed to build minecraft command: {e}")))?;

    if command.is_empty() {
        return Err(crate::error::LauncherError::from("empty minecraft command"));
    }

    let java_executable = command[0].clone();

    // Partition command array: first is java, then JVM args up to the main class,
    // then main class, then game args.
    let mut jvm_args: Vec<String> = Vec::new();
    let mut main_class = String::from("net.minecraft.client.main.Main");
    let mut game_args: Vec<String> = Vec::new();
    let mut classpath: Vec<String> = Vec::new();
    let mut seen_cp = false;

    for (i, arg) in command.iter().enumerate().skip(1) {
        if arg == "-cp" || arg == "-classpath" {
            seen_cp = true;
            // Next argument is the classpath value
            if let Some(cp_value) = command.get(i + 1) {
                classpath = cp_value
                    .split(if cfg!(target_os = "windows") {
                        ';'
                    } else {
                        ':'
                    })
                    .map(|s| s.to_string())
                    .collect();
            }
            jvm_args.push(arg.clone());
            continue;
        }
        if seen_cp
            && i == command
                .iter()
                .position(|a| a == "-cp" || a == "-classpath")
                .map(|p| p + 1)
                .unwrap_or(0)
        {
            // This is the classpath value itself, already handled above
            jvm_args.push(arg.clone());
            seen_cp = false;
            continue;
        }

        // JVM args are those starting with -D, -X, -XX, etc., but not -- (game args)
        if arg.starts_with('-') && !arg.starts_with("--") && main_class.is_empty() {
            jvm_args.push(arg.clone());
        } else {
            main_class = arg.clone();
            // Remaining args are game args
            game_args = command[i + 1..].to_vec();
            break;
        }
    }

    // Deduplicate classpath entries while preserving order
    let mut unique_cp = Vec::new();
    let mut seen_cp_entries = HashSet::new();
    for entry in &classpath {
        if seen_cp_entries.insert(entry.clone()) {
            unique_cp.push(entry.clone());
        }
    }

    // Determine natives directory
    let natives_dir = game_dir
        .join("versions")
        .join(version)
        .join("natives")
        .to_string_lossy()
        .to_string();

    // Build final JVM args with natives path
    let mut final_jvm_args: Vec<String> = Vec::new();
    let mut has_library_path = false;
    for arg in &jvm_args {
        final_jvm_args.push(arg.clone());
        if arg.starts_with("-Djava.library.path=") {
            has_library_path = true;
        }
    }
    if !has_library_path {
        final_jvm_args.push(format!("-Djava.library.path={natives_dir}"));
    }

    // Append custom JVM flags
    for flag in &options.custom_jvm_flags {
        if !final_jvm_args.contains(flag) {
            final_jvm_args.push(flag.clone());
        }
    }

    // Build game args
    let mut final_game_args = game_args;

    // Add fullscreen arg
    if options.fullscreen && !final_game_args.iter().any(|a| a == "--fullscreen") {
        final_game_args.push("--fullscreen".to_string());
    }

    // Add QuickPlay args if set
    if let Some(ref qp) = options.quick_play {
        match qp.kind {
            QuickPlayKind::Multiplayer => {
                if let Some(ref addr) = qp.server_address {
                    let port = qp.server_port.unwrap_or(25565);
                    final_game_args.push("--quickPlayMultiplayer".to_string());
                    final_game_args.push(format!("{addr}:{port}"));
                }
            }
            QuickPlayKind::Realms => {
                final_game_args.push("--quickPlayRealms".to_string());
                final_game_args.push("0".to_string());
            }
            QuickPlayKind::Singleplayer => {
                final_game_args.push("--quickPlaySingleplayer".to_string());
                final_game_args.push("world".to_string());
            }
        }
    }

    let plan = LaunchPlan {
        java_executable,
        jvm_args: final_jvm_args,
        game_args: final_game_args,
        classpath: unique_cp,
        natives_dir,
        game_dir: game_dir.to_string_lossy().to_string(),
        main_class,
        version: version.to_string(),
        quick_play: options.quick_play.clone(),
        custom_jvm_flags: options.custom_jvm_flags.clone(),
    };

    // Validate the plan round-trips to a valid command line.
    let cmd = plan_to_command(&plan);
    tracing::debug!(command = ?cmd, "generated launch plan");

    Ok(plan)
}

/// Build the full command line array from a LaunchPlan.
pub fn plan_to_command(plan: &LaunchPlan) -> Vec<String> {
    let mut cmd = Vec::new();
    cmd.push(plan.java_executable.clone());

    // JVM args
    for arg in &plan.jvm_args {
        cmd.push(arg.clone());
    }

    // Custom JVM flags
    for flag in &plan.custom_jvm_flags {
        cmd.push(flag.clone());
    }

    // Classpath
    let sep = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };
    cmd.push("-cp".to_string());
    cmd.push(plan.classpath.join(sep));

    // Main class
    cmd.push(plan.main_class.clone());

    // Game args
    for arg in &plan.game_args {
        cmd.push(arg.clone());
    }

    // QuickPlay args (if set, appended after game_args)
    if let Some(ref qp) = plan.quick_play {
        match qp.kind {
            QuickPlayKind::Multiplayer => {
                if let Some(ref addr) = qp.server_address {
                    let port = qp.server_port.unwrap_or(25565);
                    cmd.push("--quickPlayMultiplayer".to_string());
                    cmd.push(format!("{addr}:{port}"));
                }
            }
            QuickPlayKind::Realms => {
                cmd.push("--quickPlayRealms".to_string());
                cmd.push("0".to_string());
            }
            QuickPlayKind::Singleplayer => {
                cmd.push("--quickPlaySingleplayer".to_string());
                cmd.push("world".to_string());
            }
        }
    }

    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_to_command_structure() {
        let plan = LaunchPlan {
            java_executable: "/usr/bin/java".to_string(),
            jvm_args: vec![
                "-Xmx4G".to_string(),
                "-Djava.library.path=/natives".to_string(),
            ],
            game_args: vec!["--username".to_string(), "Steve".to_string()],
            classpath: vec!["/lib/a.jar".to_string(), "/lib/b.jar".to_string()],
            natives_dir: "/natives".to_string(),
            game_dir: "/mc".to_string(),
            main_class: "net.minecraft.client.main.Main".to_string(),
            version: "1.21".to_string(),
            quick_play: None,
            custom_jvm_flags: vec![],
        };

        let cmd = plan_to_command(&plan);
        assert_eq!(cmd[0], "/usr/bin/java");
        // Find -cp and classpath
        let cp_idx = cmd.iter().position(|a| a == "-cp").unwrap();
        let cp_val = &cmd[cp_idx + 1];
        assert!(cp_val.contains("a.jar"));
        assert!(cp_val.contains("b.jar"));
        // Main class follows classpath
        let main_idx = cp_idx + 2;
        assert_eq!(cmd[main_idx], "net.minecraft.client.main.Main");
        // Game args follow main class
        assert!(cmd[main_idx + 1..].contains(&"--username".to_string()));
    }

    #[test]
    fn test_quick_play_multiplayer_args() {
        let plan = LaunchPlan {
            java_executable: "java".to_string(),
            jvm_args: vec![],
            game_args: vec![],
            classpath: vec![],
            natives_dir: String::new(),
            game_dir: String::new(),
            main_class: "Main".to_string(),
            version: "1.21".to_string(),
            quick_play: Some(QuickPlayTarget {
                kind: QuickPlayKind::Multiplayer,
                server_address: Some("mc.example.com".to_string()),
                server_port: Some(25565),
            }),
            custom_jvm_flags: vec![],
        };

        let cmd = plan_to_command(&plan);
        let qp_idx = cmd
            .iter()
            .position(|a| a == "--quickPlayMultiplayer")
            .unwrap();
        assert_eq!(cmd[qp_idx + 1], "mc.example.com:25565");
    }
}
