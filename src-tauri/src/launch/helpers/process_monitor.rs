use crate::instance::models::misc::Instance;
use crate::launch::constants::GAME_PROCESS_OUTPUT_EVENT;
use crate::launch::models::{LaunchError, LaunchingState};
use crate::launcher_config::models::{LauncherVisiablity, ProcessPriority};
use crate::utils::shell::execute_command_line;
use crate::utils::window::create_webview_window;
use sjmcl_types::error::SJMCLResult;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager};
const POLLING_OPERATION_INTERVAL_MS: u64 = 2000;
pub async fn record_play_time(app: AppHandle, play_time_secs: u64, instance_id: String) {
  let instance_in_mem = {
    let binding = app.state::<Mutex<HashMap<String, Instance>>>();

    binding.lock().unwrap().get(&instance_id).cloned()
  };

  if let Some(instance_in_mem) = instance_in_mem {
    // load newest play time in instance config from disk
    let mut instance = instance_in_mem
      .load_json_cfg()
      .await
      .unwrap_or(instance_in_mem);

    let elapsed = u128::from(play_time_secs);
    instance.play_time = instance.play_time.saturating_add(elapsed);

    let _ = instance.save_json_cfg().await;
  }
}

pub async fn monitor_process(
  app: AppHandle,
  id: u64,
  spec: minecraftd::core::process::ProcessSpec,
  instance_id: String,
  display_log_window: bool,
  custom_title: &str,
  launcher_visibility: LauncherVisiablity,
  priority: &ProcessPriority,
  post_exit_command: Option<String>,
) -> SJMCLResult<bool> {
  // create unique log window
  let label = format!("game_log_{id}");
  let log_file_path = app
    .path()
    .resolve::<PathBuf>(format!("game/{label}.log").into(), BaseDirectory::AppLog)?;
  let log_window = if display_log_window {
    create_webview_window(&app, &label, "game_log", None, true)
      .await
      .ok()
  } else {
    None
  };

  let events = app.clone();
  let output_label = label.clone();
  let processes = app.state::<minecraftd::process::client::ClientProcesses>();
  let mut session = processes
    .spawn(&spec, &log_file_path, move |line| {
      if display_log_window {
        let _ = events.emit_to(&output_label, GAME_PROCESS_OUTPUT_EVENT, line);
      }
    })
    .await
    .map_err(|error| sjmcl_types::error::SJMCLError(error.to_string()))?;
  let pid = session.pid;
  let _ = set_process_priority(pid, priority);
  {
    let queue = app.state::<Mutex<Vec<LaunchingState>>>();
    let mut queue = queue.lock()?;
    if let Some(launching) = queue.iter_mut().find(|launching| launching.id == id) {
      launching.pid = pid;
    }
  }
  // polling thread (for changing window title, etc.)
  let stop_polling_flag = Arc::new(AtomicBool::new(false));
  {
    let stop_polling_flag = stop_polling_flag.clone();
    let custom_title = custom_title.to_string();
    thread::spawn(move || {
      while !stop_polling_flag.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_millis(
          POLLING_OPERATION_INTERVAL_MS,
        ));
        let _ = change_process_window_title(pid, &custom_title).is_err();
      }
    });
  };

  // handle game process exit
  let instance_id_clone = instance_id.clone();
  let stop_polling_flag = stop_polling_flag.clone();
  let ready = session.wait_ready().await;

  tokio::spawn(async move {
    let result = session.wait().await;
    stop_polling_flag.store(true, Ordering::SeqCst);
    let exit_ok = result
      .as_ref()
      .is_ok_and(|exit| exit.code == 0 && exit.reached_ready);
    let play_time_secs = result.as_ref().map_or(0, |exit| exit.play_time_secs);
    if let Err(error) = result {
      log::error!("Client process failed: {error}");
    }
    // handle launcher main window visiablity
    match launcher_visibility {
      LauncherVisiablity::RunningHidden => {
        let main_window = app.get_webview_window("main").expect("no main window");
        let _ = main_window.show();
        let _ = main_window.set_focus();
      }
      LauncherVisiablity::StartHidden => {
        // If the main window is still hidden (not shown again due to the single instance plugin when the user runs the launcher again), exit the launcher process
        let main_window = app.get_webview_window("main").expect("no main window");
        if let Ok(is_visible) = main_window.is_visible() {
          if !is_visible {
            app.exit(0);
            return;
          }
        } else {
          app.exit(0);
          return;
        }
      }
      _ => {}
    }

    record_play_time(app.clone(), play_time_secs, instance_id_clone).await;

    if exit_ok {
      if let Some(ref window) = log_window {
        let _ = window.destroy();
      }

      let launching_queue_state = app.state::<Mutex<Vec<LaunchingState>>>();
      let mut launching_queue = launching_queue_state.lock().unwrap();
      launching_queue.retain(|state| state.id != id);
    } else {
      let launching_option = {
        let launching_queue_state = app.state::<Mutex<Vec<LaunchingState>>>();
        let launching_queue = launching_queue_state.lock().unwrap();
        launching_queue.iter().find(|s| s.id == id).cloned()
      };

      if let Some(launching) = launching_option {
        if launching.current_step == 0 {
          // it was marked as manually cancelled, then remove from launching_queue and not show game error window
          let launching_queue_state = app.state::<Mutex<Vec<LaunchingState>>>();
          let mut launching_queue = launching_queue_state.lock().unwrap();
          launching_queue.retain(|state| state.id != id);
        } else {
          let _ =
            create_webview_window(&app, &format!("game_error_{id}"), "game_error", None, true)
              .await
              .unwrap();
        }
      }
    }

    if let Some(cmdline) = post_exit_command.as_ref().filter(|s| !s.trim().is_empty()) {
      let _ = execute_command_line(cmdline);
    }
  });

  Ok(ready)
}

pub fn set_process_priority(pid: u32, priority: &ProcessPriority) -> SJMCLResult<()> {
  #[cfg(any(target_os = "macos", target_os = "linux"))]
  {
    let nice_value = match *priority {
      ProcessPriority::Low => 5,
      ProcessPriority::BelowNormal => 1,
      ProcessPriority::Normal => 0,
      // FIXME: above normal need permission
      ProcessPriority::AboveNormal => -1,
      ProcessPriority::High => -5,
    };

    let _ = Command::new("renice")
      .args([nice_value.to_string(), "-p".to_string(), pid.to_string()])
      .output()
      .map_err(|_| LaunchError::SetProcessPriorityFailed)?;
  }

  #[cfg(target_os = "windows")]
  {
    use winapi::shared::minwindef::{DWORD, FALSE};
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::{OpenProcess, SetPriorityClass};
    use winapi::um::winbase::{
      ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
      IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
    };
    use winapi::um::winnt::PROCESS_SET_INFORMATION;

    unsafe {
      let h_process = OpenProcess(PROCESS_SET_INFORMATION, FALSE, pid as DWORD);
      if h_process.is_null() {
        return Err(LaunchError::SetProcessPriorityFailed.into());
      }

      let priority_class = match *priority {
        ProcessPriority::Low => IDLE_PRIORITY_CLASS,
        ProcessPriority::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS,
        ProcessPriority::Normal => NORMAL_PRIORITY_CLASS,
        ProcessPriority::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
        ProcessPriority::High => HIGH_PRIORITY_CLASS,
      };

      if SetPriorityClass(h_process, priority_class) == 0 {
        CloseHandle(h_process);
        return Err(LaunchError::SetProcessPriorityFailed.into());
      }

      CloseHandle(h_process);
    }
  }

  Ok(())
}

pub fn change_process_window_title(pid: u32, new_title: &str) -> SJMCLResult<()> {
  if new_title.trim().is_empty() {
    return Ok(());
  }
  #[cfg(target_os = "windows")]
  {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use winapi::shared::minwindef::{BOOL, DWORD, LPARAM, TRUE};
    use winapi::shared::windef::HWND;
    use winapi::um::winnt::LPCWSTR;
    use winapi::um::winuser::{EnumWindows, GetWindowThreadProcessId, SetWindowTextW};
    let new_title = new_title.to_string();
    let closure = |hwnd: HWND| unsafe {
      let mut window_pid: DWORD = 0;
      GetWindowThreadProcessId(hwnd, &mut window_pid);
      if window_pid == pid {
        let new_title: Vec<u16> = OsStr::new(&new_title)
          .encode_wide()
          .chain(once(0))
          .collect();
        SetWindowTextW(hwnd, new_title.as_ptr() as LPCWSTR);
      }
    };
    type ForEachCallback<'a> = Box<dyn FnMut(HWND) + 'a>;
    let wrapper: ForEachCallback = Box::new(closure);
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
      if let Some(boxed) = unsafe { (lparam as *mut ForEachCallback).as_mut() } {
        (*boxed)(hwnd);
      }
      TRUE
    }
    unsafe {
      EnumWindows(Some(enum_proc), &wrapper as *const _ as LPARAM);
    }
  }

  #[cfg(any(target_os = "macos", target_os = "linux"))]
  {
    // not support yet.
    let _ = (pid, new_title, LaunchError::ChangeWindowTitleFailed); // avoid unused warning
  }

  Ok(())
}
