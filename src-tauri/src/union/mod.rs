//! Federation commands shared with the Android manager through union-core.
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest;
use union_core::{Identity, client};

mod vault;
fn identity(app: &AppHandle) -> Result<Identity, String> {
  let directory = app.path().app_data_dir().map_err(|e| e.to_string())?;
  vault::identity(&directory)
}
#[tauri::command]
pub fn union_identity(app: AppHandle) -> Result<String, String> {
  Ok(identity(&app)?.peer_id().to_string())
}
#[tauri::command]
pub async fn union_governance(
  app: AppHandle,
  input: client::Input,
) -> Result<union_core::journal::Response, String> {
  client::execute(identity(&app)?, input)
    .await
    .map_err(|e| e.to_string())
}

#[derive(Default)]
pub struct ProxyState(pub tokio::sync::Mutex<Option<GameConnection>>);
pub struct GameConnection {
  proxy: terminal_runtime::LocalProxy,
  refresh: Option<tokio::task::JoinHandle<()>>,
}
impl Drop for GameConnection {
  fn drop(&mut self) {
    if let Some(task) = &self.refresh {
      task.abort();
    }
  }
}
async fn student_proof(
  app: &AppHandle,
  holder: &str,
  expected_profile: Option<&str>,
) -> Result<union_core::federation::StudentPresentation, String> {
  let player = crate::account::helpers::misc::get_selected_player_info(app)
    .map_err(|_| "Select a school account first".to_string())?;
  if player.player_type != crate::account::models::PlayerType::ThirdParty {
    return Err("A school account is required".into());
  }
  let profile = player.uuid.simple().to_string();
  if expected_profile.is_some_and(|p| p != profile) {
    return Err("Selected game account changed".into());
  }
  let mut url = reqwest::Url::parse(
    player
      .auth_server_url
      .as_deref()
      .ok_or("Missing identity provider")?,
  )
  .map_err(|e| e.to_string())?;
  if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
    return Err("School identity provider must use HTTPS".into());
  }
  url.set_path("/api/student-credential");
  url.set_query(None);
  url.set_fragment(None);
  let client = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .timeout(std::time::Duration::from_secs(10))
    .build()
    .map_err(|e| e.to_string())?;
  let mut response = client
    .post(url)
    .bearer_auth(
      player
        .access_token
        .ok_or("School account is not logged in")?,
    )
    .json(&serde_json::json!({"holder":holder}))
    .send()
    .await
    .map_err(|_| "School identity service unavailable".to_string())?;
  if !response.status().is_success() {
    return Err("School identity verification failed".into());
  }
  let mut bytes = Vec::new();
  while let Some(chunk) = response
    .chunk()
    .await
    .map_err(|_| "Invalid school credential".to_string())?
  {
    if bytes.len() + chunk.len() > 65536 {
      return Err("School credential too large".into());
    }
    bytes.extend_from_slice(&chunk);
  }
  let credential =
    serde_json::from_slice(&bytes).map_err(|_| "Invalid school credential".to_string())?;
  Ok(union_core::federation::StudentPresentation {
    credential,
    profile,
  })
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyInput {
  pub peer: String,
  pub addresses: Vec<String>,
  pub service: String,
  pub student: Option<union_core::federation::StudentPresentation>,
  #[serde(default)]
  pub request_student: bool,
}

#[tauri::command]
pub async fn union_proxy_start(
  app: AppHandle,
  state: tauri::State<'_, ProxyState>,
  input: ProxyInput,
) -> Result<String, String> {
  if input.addresses.is_empty()
    || input.addresses.len() > 8
    || input.addresses.iter().any(|a| a.len() > 2048)
  {
    return Err("Invalid game addresses".into());
  }
  let mut active = state.0.lock().await;
  if active.as_ref().is_some_and(|p| p.proxy.is_running()) {
    return Err("Disconnect the current game connection first".into());
  }
  let target = terminal_runtime::TargetArgs {
    peer: input
      .peer
      .parse::<union_core::PeerId>()
      .map_err(|e| e.to_string())?,
    address: input
      .addresses
      .iter()
      .map(|a| a.parse::<union_core::Multiaddr>())
      .collect::<Result<_, _>>()
      .map_err(|e| e.to_string())?,
  };
  let service = input
    .service
    .parse()
    .map_err(|e: union_core::Error| e.to_string())?;
  let device = identity(&app)?;
  let holder = device.peer_id().to_string();
  let proof = if input.request_student {
    Some(student_proof(&app, &holder, None).await?)
  } else {
    input.student
  };
  let profile = proof.as_ref().map(|p| p.profile.clone());
  let node = union_core::Node::start(device, union_core::NodeConfig::default())
    .await
    .map_err(|e| e.to_string())?;
  let proxy = terminal_runtime::LocalProxy::start(
    node,
    target,
    service,
    None,
    proof,
    std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
  )
  .await
  .map_err(|e| e.to_string())?;
  let address = proxy.address().to_string();
  let refresh = if input.request_student {
    let proofs = proxy.proof_store();
    let abort = proxy.abort_handle();
    Some(tokio::spawn(async move {
      loop {
        tokio::time::sleep(std::time::Duration::from_secs(600)).await;
        match student_proof(&app, &holder, profile.as_deref()).await {
          Ok(proof) => {
            *proofs.write().await = Some(proof);
          }
          Err(_) => {
            abort.abort();
            break;
          }
        }
      }
    }))
  } else {
    None
  };
  *active = Some(GameConnection { proxy, refresh });
  Ok(address)
}
#[tauri::command]
pub async fn union_proxy_stop(state: tauri::State<'_, ProxyState>) -> Result<(), String> {
  state.0.lock().await.take();
  Ok(())
}
#[tauri::command]
pub async fn union_proxy_status(
  state: tauri::State<'_, ProxyState>,
) -> Result<Option<String>, String> {
  Ok(
    state
      .0
      .lock()
      .await
      .as_ref()
      .filter(|p| p.proxy.is_running())
      .map(|p| p.proxy.address().to_string()),
  )
}

pub async fn active_game_address(app: &AppHandle) -> Option<String> {
  app
    .state::<ProxyState>()
    .0
    .lock()
    .await
    .as_ref()
    .filter(|c| c.proxy.is_running())
    .map(|c| c.proxy.address().to_string())
}
