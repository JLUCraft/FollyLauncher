use crate::launch::constants::{GAME_PROCESS_OUTPUT_EVENT, READY_FLAG};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn monitor_process(
    app: AppHandle,
    _id: u64,
    mut child: tokio::process::Child,
    instance_id: String,
    display_log: bool,
) {
    let _pid = child.id().unwrap_or(0);
    let _start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let game_ready = Arc::new(AtomicBool::new(false));
    let log_lines = Arc::new(Mutex::new(Vec::new()));

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Stdout handler
    if let Some(out) = stdout {
        let game_ready_stdout = game_ready.clone();
        let log_lines_stdout = log_lines.clone();
        let app_stdout = app.clone();
        let instance_id_stdout = instance_id.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(out);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if display_log {
                    let _ = app_stdout.emit(
                        GAME_PROCESS_OUTPUT_EVENT,
                        serde_json::json!({
                            "instance_id": &instance_id_stdout,
                            "line": &line,
                        }),
                    );
                }
                log_lines_stdout.lock().unwrap_or_else(|e| e.into_inner()).push(line.clone());

                if !game_ready_stdout.load(Ordering::SeqCst)
                    && READY_FLAG
                        .iter()
                        .any(|p| line.to_lowercase().contains(p))
                {
                    game_ready_stdout.store(true, Ordering::SeqCst);
                    let _ = app_stdout.emit(&format!("game-ready-{}", instance_id_stdout), ());
                }
            }
        });
    }

    // Stderr handler
    if let Some(err) = stderr {
        let log_lines_stderr = log_lines.clone();
        let app_stderr = app.clone();
        let instance_id_stderr = instance_id.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(err);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if display_log {
                    let _ = app_stderr.emit(
                        GAME_PROCESS_OUTPUT_EVENT,
                        serde_json::json!({
                            "instance_id": &instance_id_stderr,
                            "line": &line,
                        }),
                    );
                }
                log_lines_stderr.lock().unwrap_or_else(|e| e.into_inner()).push(line.clone());
            }
        });
    }

    // Exit handler
    let game_ready_exit = game_ready.clone();
    let app_exit = app.clone();
    let instance_id_exit = instance_id.clone();

    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => {
                let exit_ok = status.success();
                let _ = app_exit.emit(
                    &format!("game-exit-{}", instance_id_exit),
                    serde_json::json!({
                        "exit_ok": exit_ok,
                        "code": status.code(),
                        "game_ready": game_ready_exit.load(Ordering::SeqCst),
                    }),
                );
            }
            Err(e) => {
                let _ = app_exit.emit(
                    &format!("game-exit-{}", instance_id_exit),
                    serde_json::json!({
                        "exit_ok": false,
                        "error": e.to_string(),
                    }),
                );
            }
        }
    });
}

pub fn kill_process(pid: u32) -> anyhow::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()?;
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(0x08000000)
            .output()?;
    }

    Ok(())
}
