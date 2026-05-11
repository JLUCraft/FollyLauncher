use std::path::Path;

/// How to handle process stdout/stderr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogOutput {
    /// Pipe stdout/stderr so a monitor can read them.
    Piped,
    /// Let the child inherit the launcher's stdout/stderr.
    Inherit,
    /// Discard all output.
    Null,
}

/// Build and spawn a Minecraft process with the given command array and options.
///
/// # Arguments
/// * `command` - The full command array (argv\[0\] = executable, argv\[1..\] = args),
///   typically built by `mc_launcher_core::command::get_minecraft_command`.
/// * `game_dir` - Working directory for the process.
/// * `log_output` - How to handle stdout/stderr (Piped, Inherit, or Null).
///
/// Returns the spawned `tokio::process::Child` on success, or a Chinese error string.
pub fn spawn_minecraft_process(
    command: &[String],
    game_dir: &Path,
    log_output: LogOutput,
) -> Result<tokio::process::Child, crate::error::LauncherError> {
    if command.is_empty() {
        return Err(crate::error::LauncherError::from("启动命令为空"));
    }

    let mut cmd = tokio::process::Command::new(&command[0]);
    cmd.args(&command[1..])
        .current_dir(game_dir)
        .stdin(std::process::Stdio::null());

    match log_output {
        LogOutput::Piped => {
            cmd.stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
        }
        LogOutput::Inherit => {
            cmd.stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit());
        }
        LogOutput::Null => {
            cmd.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
        }
    }

    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.spawn()
        .map_err(|e| crate::error::LauncherError::from(format!("启动 Minecraft 进程失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_command_returns_error() {
        let result = spawn_minecraft_process(&[], Path::new("/tmp"), LogOutput::Null);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("为空"));
    }

    #[test]
    fn non_empty_command_does_not_panic_on_build() {
        let cmd = vec!["java".to_string(), "-version".to_string()];
        let result = spawn_minecraft_process(&cmd, Path::new("/tmp"), LogOutput::Null);
        // Will fail at spawn time (no java on test machine) but should not panic
        assert!(result.is_err());
    }
}
