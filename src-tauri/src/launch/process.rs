use std::path::Path;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogOutput {

    Piped,

    Inherit,

    Null,
}









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

    #[tokio::test]
    async fn non_empty_command_does_not_panic_on_spawn_error() {
        let cmd = vec!["__jlucraft_missing_executable__".to_string()];
        let result = spawn_minecraft_process(&cmd, Path::new("."), LogOutput::Null);
        assert!(result.is_err());
    }
}
