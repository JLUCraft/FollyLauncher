use crate::launch::constants::{
    GAME_EXITED_EVENT, GAME_PROCESS_OUTPUT_EVENT, GAME_READY_EVENT, READY_FLAG,
};
use crate::launch::models::LaunchStep;
use crate::launch::models::LaunchingState;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn monitor_process(
    app: AppHandle,
    id: u64,
    mut child: tokio::process::Child,
    instance_id: String,
    display_log: bool,
    launch_states: Arc<tokio::sync::Mutex<HashMap<u64, LaunchingState>>>,
) {
    let game_ready = Arc::new(AtomicBool::new(false));

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Stdout handler
    if let Some(out) = stdout {
        let game_ready_stdout = game_ready.clone();
        let app_stdout = app.clone();
        let instance_id_stdout = instance_id.clone();
        let launch_states_stdout = launch_states.clone();

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

                // Append to launch_states.recent_logs
                {
                    let mut states = launch_states_stdout.lock().await;
                    if let Some(state) = states.get_mut(&id) {
                        push_recent_log(&mut state.recent_logs, line.clone(), 200);
                    }
                }

                if !game_ready_stdout.load(Ordering::SeqCst)
                    && READY_FLAG.iter().any(|p| line.to_lowercase().contains(p))
                {
                    game_ready_stdout.store(true, Ordering::SeqCst);
                    let _ = app_stdout.emit(
                        &format!("{GAME_READY_EVENT}-{instance_id_stdout}"),
                        serde_json::json!({ "session_id": &instance_id_stdout }),
                    );
                    // Update game_ready in launch_states
                    let mut states = launch_states_stdout.lock().await;
                    if let Some(state) = states.get_mut(&id) {
                        state.game_ready = true;
                    }
                }
            }
        });
    }

    // Stderr handler
    if let Some(err) = stderr {
        let app_stderr = app.clone();
        let instance_id_stderr = instance_id.clone();
        let launch_states_stderr = launch_states.clone();

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

                // Append to launch_states.recent_logs
                {
                    let mut states = launch_states_stderr.lock().await;
                    if let Some(state) = states.get_mut(&id) {
                        push_recent_log(&mut state.recent_logs, line.clone(), 200);
                    }
                }
            }
        });
    }

    // Exit handler
    let game_ready_exit = game_ready.clone();
    let app_exit = app.clone();
    let instance_id_exit = instance_id.clone();
    let launch_states_exit = launch_states.clone();

    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => {
                let now_secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let exit_ok = status.success();
                let duration_secs = {
                    let states = launch_states_exit.lock().await;
                    states
                        .get(&id)
                        .and_then(|s| s.start_time)
                        .map(|start| now_secs.saturating_sub(start))
                        .unwrap_or(0)
                };
                let crash_summary = if !exit_ok {
                    let states = launch_states_exit.lock().await;
                    states
                        .get(&id)
                        .map(|s| {
                            let last_lines: Vec<&String> =
                                s.recent_logs.iter().rev().take(20).collect();
                            last_lines
                                .into_iter()
                                .rev()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let _ = app_exit.emit(
                    &format!("{GAME_EXITED_EVENT}-{instance_id_exit}"),
                    serde_json::json!({
                        "session_id": instance_id_exit,
                        "code": status.code(),
                        "duration_secs": duration_secs,
                        "exit_ok": exit_ok,
                        "game_ready": game_ready_exit.load(Ordering::SeqCst),
                        "crash_summary": crash_summary,
                    }),
                );
                // Update launch_states on exit
                let mut states = launch_states_exit.lock().await;
                if let Some(state) = states.get_mut(&id) {
                    state.current_step = classify_exit_step(exit_ok) as usize;
                    state.exit_code = status.code();
                    state.exit_ok = Some(exit_ok);
                    state.game_ready = game_ready_exit.load(Ordering::SeqCst);
                    state.end_time = Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    );
                } else {
                    tracing::debug!(launching_id = %id, "launching state not found for exit handler");
                }
            }
            Err(e) => {
                let now_secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let duration_secs = {
                    let states = launch_states_exit.lock().await;
                    states
                        .get(&id)
                        .and_then(|s| s.start_time)
                        .map(|start| now_secs.saturating_sub(start))
                        .unwrap_or(0)
                };
                let _ = app_exit.emit(
                    &format!("{GAME_EXITED_EVENT}-{instance_id_exit}"),
                    serde_json::json!({
                        "session_id": instance_id_exit,
                        "code": null,
                        "duration_secs": duration_secs,
                        "exit_ok": false,
                        "error": e.to_string(),
                    }),
                );
                // Update launch_states for error case
                let mut states = launch_states_exit.lock().await;
                if let Some(state) = states.get_mut(&id) {
                    state.current_step = LaunchStep::Crashed as usize;
                    state.exit_code = None;
                    state.exit_ok = Some(false);
                    state.game_ready = game_ready_exit.load(Ordering::SeqCst);
                    state.end_time = Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    );
                } else {
                    tracing::debug!(launching_id = %id, "launching state not found for exit error handler");
                }
            }
        }
    });
}

/// Append a line to the log buffer, keeping at most `limit` lines.
/// Drops the oldest lines when the buffer exceeds the limit.
pub fn push_recent_log(lines: &mut Vec<String>, line: String, limit: usize) {
    lines.push(line);
    let excess = lines.len().saturating_sub(limit);
    if excess > 0 {
        lines.drain(0..excess);
    }
}

/// Classify the final LaunchStep based on whether the process exited successfully.
pub fn classify_exit_step(exit_ok: bool) -> LaunchStep {
    if exit_ok {
        LaunchStep::Exited
    } else {
        LaunchStep::Crashed
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_recent_log_respects_limit() {
        let mut lines = Vec::new();
        for i in 0..205 {
            push_recent_log(&mut lines, format!("line {}", i), 200);
        }
        assert_eq!(lines.len(), 200);
        assert_eq!(lines[0], "line 5");
        assert_eq!(lines[199], "line 204");
    }

    #[test]
    fn push_recent_log_under_limit_keeps_all() {
        let mut lines = Vec::new();
        push_recent_log(&mut lines, "hello".to_string(), 200);
        push_recent_log(&mut lines, "world".to_string(), 200);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "hello");
        assert_eq!(lines[1], "world");
    }

    #[test]
    fn push_recent_log_at_exact_limit() {
        let mut lines = Vec::new();
        for i in 0..200 {
            push_recent_log(&mut lines, format!("line {}", i), 200);
        }
        assert_eq!(lines.len(), 200);
        assert_eq!(lines[0], "line 0");
        assert_eq!(lines[199], "line 199");
    }

    #[test]
    fn classify_exit_ok_is_exited() {
        assert_eq!(classify_exit_step(true), LaunchStep::Exited);
    }

    #[test]
    fn classify_exit_fail_is_crashed() {
        assert_eq!(classify_exit_step(false), LaunchStep::Crashed);
    }
}
