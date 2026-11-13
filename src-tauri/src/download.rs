use std::sync::{Arc, Mutex};

use futures::future::join_all;
use sjmcl_downloader::download::DownloadExecutor;
use sjmcl_downloader::{
  EngineConfig, EngineError, EngineHandle, EngineSetup, GroupState, SubmitGroup, SubmitTask,
  TaskError, TaskExecutor, TokenBucket,
};
use sjmcl_types::error::{SJMCLError, SJMCLResult};
use tauri::{AppHandle, Manager};

use crate::APP_DATA_DIR;
use crate::instance::helpers::loader::common::InstallPlan;
use crate::instance::helpers::loader::postprocess::{
  InstallKind, InstallSpec, InstallTarget, PrepareSpec,
};
use crate::instance::models::misc::{Instance, InstanceError, ModLoaderType};
use crate::launcher_config::models::LauncherConfig;
use crate::resource::helpers::curseforge::misc::{
  CURSEFORGE_API_KEY, is_curseforge_authenticated_url,
};
use crate::utils::fs::is_local_file_valid;

pub fn init_plugin(
  client: tauri_plugin_http::reqwest::Client,
) -> tauri::plugin::TauriPlugin<tauri::Wry> {
  sjmcl_downloader::init_with_setup(move |app| {
    let launcher_config = app.state::<Mutex<LauncherConfig>>();
    let launcher_config = launcher_config.lock().map_err(|error| error.to_string())?;
    let download_concurrency = if launcher_config.download.transmission.auto_concurrent {
      std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
    } else {
      launcher_config
        .download
        .transmission
        .concurrent_count
        .max(1)
    };
    let speed_limit = launcher_config
      .download
      .transmission
      .enable_speed_limit
      .then_some(
        (launcher_config.download.transmission.speed_limit_value as u64).saturating_mul(1024),
      );
    drop(launcher_config);

    let executor = DownloadExecutor {
      client,
      limiter: speed_limit
        .map(|bytes_per_second| Arc::new(TokenBucket::new(bytes_per_second, bytes_per_second))),
      request_decorator: Some(Arc::new(
        |raw_url: &str, request: tauri_plugin_http::reqwest::RequestBuilder| {
          if raw_url
            .parse()
            .is_ok_and(|url| is_curseforge_authenticated_url(&url))
          {
            request.header("x-api-key", CURSEFORGE_API_KEY.as_str())
          } else {
            request
          }
        },
      )),
      ..DownloadExecutor::default()
    };

    Ok(EngineSetup {
      db_path: APP_DATA_DIR
        .get()
        .ok_or_else(|| "APP_DATA_DIR is not initialized".to_string())?
        .join("downloads.db"),
      config: EngineConfig {
        concurrency: download_concurrency,
        ..EngineConfig::default()
      },
      executor,
      extra_executors: vec![
        Arc::new(
          crate::instance::helpers::loader::postprocess::PrepareExecutor { app: app.clone() },
        ) as Arc<dyn TaskExecutor>,
        Arc::new(
          crate::instance::helpers::loader::postprocess::InstallExecutor { app: app.clone() },
        ) as Arc<dyn TaskExecutor>,
        Arc::new(crate::instance::helpers::loader::postprocess::VerifyExecutor { app: app.clone() })
          as Arc<dyn TaskExecutor>,
        Arc::new(
          crate::instance::helpers::loader::postprocess::RefreshJavaExecutor { app: app.clone() },
        ) as Arc<dyn TaskExecutor>,
      ],
    })
  })
}

pub use minecraftd::download::request::DownloadParam as DownloadTask;

fn submit_task(task: DownloadTask) -> SubmitTask {
  let mut sources = task.sources.into_iter();
  let name = task.filename.unwrap_or_else(|| {
    task
      .dest
      .file_name()
      .map(|name| name.to_string_lossy().into_owned())
      .unwrap_or_else(|| task.dest.to_string_lossy().into_owned())
  });
  SubmitTask {
    name,
    executor: "download".into(),
    spec: serde_json::json!({
      "url": sources.next(),
      "fallbackUrls": sources.collect::<Vec<_>>(),
      "size": task.size,
    }),
    dest: Some(task.dest),
    sha1: task.sha1,
    sha256: None,
  }
}

pub async fn get_invalid_download_tasks(
  tasks: Vec<DownloadTask>,
  check_hash: bool,
) -> SJMCLResult<Vec<DownloadTask>> {
  let results = join_all(tasks.into_iter().map(|task| async move {
    let valid = is_local_file_valid(&task.dest, task.sha1.as_deref(), check_hash).await?;
    Ok::<_, SJMCLError>((!valid).then_some(task))
  }))
  .await;

  let mut tasks = Vec::new();
  for result in results {
    if let Some(task) = result? {
      tasks.push(task);
    }
  }
  Ok(tasks)
}

pub async fn submit_download_group(
  app: AppHandle,
  name: String,
  tasks: Vec<DownloadTask>,
  auto_resume: bool,
) -> SJMCLResult<String> {
  let engine = app.state::<EngineHandle>();
  engine
    .0
    .submit_group(SubmitGroup {
      name,
      parent_group_id: None,
      tasks: tasks.into_iter().map(submit_task).collect(),
      auto_resume,
    })
    .await
    .map_err(engine_error)
}

pub async fn submit_java_download_group(
  app: AppHandle,
  name: String,
  tasks: Vec<DownloadTask>,
) -> SJMCLResult<String> {
  let mut tasks: Vec<SubmitTask> = tasks.into_iter().map(submit_task).collect();
  tasks.push(SubmitTask {
    name: "Refresh Java discovery".into(),
    executor: "refresh_java".into(),
    spec: serde_json::json!({}),
    dest: None,
    sha1: None,
    sha256: None,
  });
  app
    .state::<EngineHandle>()
    .0
    .submit_group(SubmitGroup {
      name,
      parent_group_id: None,
      tasks,
      auto_resume: true,
    })
    .await
    .map_err(engine_error)
}

pub async fn submit_instance_download_group(
  app: AppHandle,
  name: String,
  tasks: Vec<DownloadTask>,
  instance: &Instance,
  verify_mod_loader: bool,
) -> SJMCLResult<String> {
  let mut tasks: Vec<SubmitTask> = tasks.into_iter().map(submit_task).collect();
  tasks.push(SubmitTask {
    name: "Prepare installation".into(),
    executor: "prepare_install".into(),
    spec: serde_json::to_value(PrepareSpec {
      instance_id: instance.id.clone(),
      version_path: instance.version_path.clone(),
    })?,
    dest: None,
    sha1: None,
    sha256: None,
  });
  // Fabric and Quilt have no installer processors, but still need dependency verification.
  if verify_mod_loader
    && matches!(
      instance.mod_loader.loader_type,
      ModLoaderType::Fabric | ModLoaderType::Quilt
    )
  {
    tasks.push(SubmitTask {
      name: "Verify".into(),
      executor: "verify".into(),
      spec: serde_json::to_value(InstallTarget {
        instance_id: instance.id.clone(),
        version_path: instance.version_path.clone(),
        kind: InstallKind::ModLoader,
      })?,
      dest: None,
      sha1: None,
      sha256: None,
    });
  }
  app
    .state::<EngineHandle>()
    .0
    .submit_group(SubmitGroup {
      name,
      parent_group_id: None,
      tasks,
      auto_resume: true,
    })
    .await
    .map_err(engine_error)
}

pub async fn submit_install_group(
  app: AppHandle,
  name: String,
  plan: InstallPlan,
  instance: &Instance,
  kind: InstallKind,
  parent_group_id: &str,
) -> Result<String, TaskError> {
  let mut tasks: Vec<SubmitTask> = get_invalid_download_tasks(plan.tasks, false)
    .await
    .map_err(task_error)?
    .into_iter()
    .map(submit_task)
    .collect();
  let target = InstallTarget {
    instance_id: instance.id.clone(),
    version_path: instance.version_path.clone(),
    kind,
  };
  let install_spec = serde_json::to_value(InstallSpec {
    target,
    processors: plan.processors,
  })?;
  tasks.push(SubmitTask {
    name: "Install".into(),
    executor: "install".into(),
    spec: install_spec,
    dest: None,
    sha1: None,
    sha256: None,
  });
  app
    .state::<EngineHandle>()
    .0
    .submit_group(SubmitGroup {
      name,
      parent_group_id: Some(parent_group_id.to_string()),
      tasks,
      auto_resume: true,
    })
    .await
    .map_err(TaskError::from)
}

pub async fn has_active_downloads(app: &AppHandle) -> SJMCLResult<bool> {
  app
    .state::<EngineHandle>()
    .0
    .snapshot()
    .await
    .map(|groups| {
      groups
        .iter()
        .any(|group| group.state != GroupState::Finished)
    })
    .map_err(engine_error)
}

fn engine_error(error: EngineError) -> SJMCLError {
  log::error!("Download engine operation failed: {error}");
  SJMCLError("DOWNLOAD_ENGINE_ERROR".into())
}

pub fn task_error(error: SJMCLError) -> TaskError {
  TaskError::Other { details: error.0 }
}

impl From<InstanceError> for TaskError {
  fn from(error: InstanceError) -> Self {
    Self::Application {
      code: error.to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use sjmcl_downloader::storage::SqliteStore;
  use sjmcl_downloader::{Engine, EngineEvent, EventSink, FinishKind, StateStore};
  use std::path::Path;
  use std::time::Duration;
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  use tokio::net::TcpListener;
  use tokio::sync::mpsc;
  use tokio::task::JoinHandle;

  const BODY: &[u8] = b"abcdef";
  const PART_LEN: usize = 3;
  const MISSING_PATH: &str = "/missing";
  const FILE_PATH: &str = "/artifact";

  struct Events(mpsc::UnboundedSender<EngineEvent>);
  impl EventSink for Events {
    fn emit(&self, event: &EngineEvent) {
      let _ = self.0.send(event.clone());
    }
  }

  type RunningEngine = (Engine, JoinHandle<Result<(), EngineError>>);

  fn start_engine(
    path: &Path,
    events: mpsc::UnboundedSender<EngineEvent>,
  ) -> Result<RunningEngine, Box<dyn std::error::Error>> {
    let store = Arc::new(SqliteStore::open(path)?);
    let mut builder = Engine::builder(
      EngineConfig {
        concurrency: 1,
        max_retries: 0,
        retry_backoff: Duration::from_millis(1),
        ..EngineConfig::default()
      },
      Arc::new(Events(events)),
      store,
    );
    builder.register(Arc::new(DownloadExecutor {
      client: tauri_plugin_http::reqwest::Client::builder()
        .no_proxy()
        .build()?,
      report_interval: Duration::ZERO,
      ..DownloadExecutor::default()
    }));
    let (engine, actor) = builder.build()?;
    Ok((engine, tokio::spawn(actor)))
  }

  async fn wait_event(
    events: &mut mpsc::UnboundedReceiver<EngineEvent>,
    predicate: impl Fn(&EngineEvent) -> bool,
  ) {
    while let Some(event) = events.recv().await {
      if predicate(&event) {
        return;
      }
    }
    panic!("download event channel closed");
  }

  #[tokio::test]
  async fn download_group_retains_partial_data_and_completion_across_restart()
  -> Result<(), Box<dyn std::error::Error>> {
    tokio::time::timeout(Duration::from_secs(10), async {
      let listener = TcpListener::bind("127.0.0.1:0").await?;
      let origin = format!("http://{}", listener.local_addr()?);
      let server = tokio::spawn(async move {
        loop {
          let (mut socket, _) = match listener.accept().await {
            Ok(connection) => connection,
            Err(error) => break Err::<(), _>(error),
          };
          tokio::spawn(async move {
            let mut request = String::new();
            while !request.ends_with("\r\n\r\n") {
              request.push(char::from(socket.read_u8().await?));
            }
            let path = request.split_whitespace().nth(1);
            let response = if path == Some(MISSING_PATH) {
              "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
            } else if request.starts_with("HEAD ") {
              format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", BODY.len())
            } else if request.to_ascii_lowercase().contains(&format!("range: bytes={PART_LEN}-\r\n")) {
              format!("HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {PART_LEN}-{}/{}\r\nConnection: close\r\n\r\n{}", BODY.len()-PART_LEN, BODY.len()-1, BODY.len(), String::from_utf8_lossy(&BODY[PART_LEN..]))
            } else {
              format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", BODY.len(), String::from_utf8_lossy(&BODY[..PART_LEN]))
            };
            socket.write_all(response.as_bytes()).await?;
            socket.flush().await?;
            if path == Some(FILE_PATH) && !request.starts_with("HEAD ") && !request.to_ascii_lowercase().contains("range:") {
              // Hold an incomplete response until the client cancels it.
              let _ = socket.read_u8().await;
            }
            Ok::<_, std::io::Error>(())
          });
        }
      });
      let directory = tempfile::tempdir()?;
      let database = directory.path().join("downloads.db");
      let destination = directory.path().join("resource.jar");
      let (events, mut receiver) = mpsc::unbounded_channel();
      let (engine, actor) = start_engine(&database, events.clone())?;
      let group_id = engine.submit_group(SubmitGroup {
        name: "resource".into(), parent_group_id: None, auto_resume: false,
        tasks: vec![submit_task(DownloadTask {
          sources: vec![format!("{origin}{MISSING_PATH}").parse()?, format!("{origin}{FILE_PATH}").parse()?],
          dest: destination.clone(), filename: None, sha1: None, size: Some(BODY.len() as u64),
        })],
      }).await?;
      wait_event(&mut receiver, |event| matches!(event, EngineEvent::Tick(items) if items.iter().any(|item| item.received == PART_LEN as u64))).await;
      engine.pause(group_id.clone()).await?;
      let stored = SqliteStore::open(&database)?.load_all()?;
      assert_eq!(stored[0].state, GroupState::Paused);
      assert_eq!(stored[0].tasks[0].offset, PART_LEN as u64);
      actor.abort();
      let _ = actor.await;
      drop(engine);
      let (engine, actor) = start_engine(&database, events)?;
      engine.resume(group_id.clone()).await?;
      wait_event(&mut receiver, |event| matches!(event, EngineEvent::GroupFinished { group_id: id, finish: FinishKind::Completed, .. } if id == &group_id)).await;
      assert_eq!(tokio::fs::read(&destination).await?, BODY);
      engine.acknowledge_completion(group_id).await?;
      let stored = SqliteStore::open(&database)?.load_all()?;
      assert!(!stored[0].completion_pending);
      actor.abort();
      let _ = actor.await;
      server.abort();
      let _ = server.await;
      Ok::<_, Box<dyn std::error::Error>>(())
    }).await??;
    Ok(())
  }
}
