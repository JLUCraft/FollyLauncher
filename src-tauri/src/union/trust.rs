//! Unified identity configuration and signing; private keys remain in the OS vault.
use crate::union::identity;
use client::trust::{Config, Input, Operation, TrustState};
use tauri::{AppHandle, Manager};
fn directory(app: &AppHandle) -> Result<std::path::PathBuf, String> {
  let directory = app
    .path()
    .app_data_dir()
    .map_err(|e| e.to_string())?
    .join("union-identity");
  std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
  Ok(directory)
}
pub fn configuration(app: &AppHandle) -> Result<Option<Config>, String> {
  let path = directory(app)?.join("configuration.json");
  if !path.exists() {
    return Ok(None);
  }
  let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
  if bytes.len() > protocol::MAX_FRAME {
    return Err("Identity configuration too large".into());
  }
  serde_json::from_slice(&bytes)
    .map(Some)
    .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn union_identity_configuration(
  app: AppHandle,
  config: Option<Config>,
) -> Result<Option<Config>, String> {
  if let Some(mut config) = config {
    TrustState::new(&config.genesis, &config.genesis_hash).map_err(|e| e.to_string())?;
    let peer: node::PeerId = config.peer.parse().map_err(|_| "Invalid identity source")?;
    let address: node::Multiaddr = config
      .address
      .parse()
      .map_err(|_| "Invalid identity source")?;
    if !matches!(address.iter().last(),Some(node::AddressProtocol::P2p(id)) if id==peer) {
      return Err("Identity source peer mismatch".into());
    }
    let directory = directory(&app)?;
    config.log = directory.join(format!("{}.jsonl", config.genesis_hash));
    let bytes = serde_json::to_vec(&config).map_err(|e| e.to_string())?;
    let pending = directory.join("configuration.pending");
    std::fs::write(&pending, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(pending, directory.join("configuration.json")).map_err(|e| e.to_string())?;
    return Ok(Some(config));
  }
  configuration(&app)
}
#[tauri::command]
pub async fn union_identity_action(
  app: AppHandle,
  operation: Operation,
) -> Result<serde_json::Value, String> {
  let config = configuration(&app)?.ok_or("Unified identity is not configured")?;
  client::trust::execute_identity(identity(&app)?, Input { config, operation })
    .await
    .map_err(|e| e.to_string())
}
pub async fn synchronize(
  app: &AppHandle,
  node: &node::Node,
) -> Result<Option<client::trust::TrustLog>, String> {
  let Some(config) = configuration(app)? else {
    return Ok(None);
  };
  let log = tokio::sync::Mutex::new(config.open().map_err(|e| e.to_string())?);
  config
    .synchronize(node, &log)
    .await
    .map_err(|e| e.to_string())?;
  Ok(Some(log.into_inner()))
}

/// Bind the distinct hosting device to this member with both device possession signatures.
#[tauri::command]
pub async fn union_bind_host_device(app: AppHandle) -> Result<serde_json::Value, String> {
  let config = configuration(&app)?.ok_or("Unified identity is not configured")?;
  let player = identity(&app)?;
  let host = crate::union::rooms::host_identity(&app)?;
  let node = node::Node::start(player.clone(), node::NodeConfig::default())
    .await
    .map_err(|e| e.to_string())?;
  let result = async {
    let log = tokio::sync::Mutex::new(config.open().map_err(|e| e.to_string())?);
    config
      .synchronize(&node, &log)
      .await
      .map_err(|e| e.to_string())?;
    let log = log.lock().await;
    let member = log
      .state()
      .member_for_device(&player.peer_id().to_string())
      .ok_or("Player device is not bound to a member")?;
    let mut record = log
      .state()
      .draft(client::protocol::TrustAction::BindDevice {
        member: member.into(),
        device: host.peer_id().to_string(),
      });
    client::trust::sign(&mut record, &player).map_err(|e| e.to_string())?;
    client::trust::sign(&mut record, &host).map_err(|e| e.to_string())?;
    // Return for explicit user review/submission in the shared identity panel.
    serde_json::to_value(record).map_err(|e| e.to_string())
  }
  .await;
  node.shutdown().await.map_err(|e| e.to_string())?;
  result
}

/// Skin sites only co-sign scoped attestation actions; the shared Rust verifier checks the result.
#[tauri::command]
pub async fn union_automated_identity(app: AppHandle) -> Result<serde_json::Value, String> {
  use tauri_plugin_http::reqwest;
  let config = configuration(&app)?.ok_or("Unified identity is not configured")?;
  let device = identity(&app)?;
  let player = crate::account::helpers::misc::get_selected_player_info(&app)
    .map_err(|_| "Select an authenticated skin-site account")?;
  let mut url = reqwest::Url::parse(
    player
      .auth_server_url
      .as_deref()
      .ok_or("No skin-site account")?,
  )
  .map_err(|e| e.to_string())?;
  if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
    return Err("Skin site must use HTTPS".into());
  }
  let token = player
    .access_token
    .ok_or("Skin-site account is not logged in")?;
  let http = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .timeout(std::time::Duration::from_secs(30))
    .build()
    .map_err(|e| e.to_string())?;
  async fn post(
    http: &reqwest::Client,
    url: reqwest::Url,
    token: &str,
    body: serde_json::Value,
  ) -> Result<serde_json::Value, String> {
    let mut response = http
      .post(url)
      .bearer_auth(token)
      .json(&body)
      .send()
      .await
      .map_err(|_| "Identity verifier unavailable")?;
    if !response.status().is_success() {
      return Err("Identity verifier rejected the request".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
      .chunk()
      .await
      .map_err(|_| "Invalid identity response")?
    {
      if bytes.len() + chunk.len() > protocol::MAX_FRAME {
        return Err("Identity response too large".into());
      }
      bytes.extend(chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid identity response".into())
  }
  for kind in ["membership", "profile"] {
    url.set_path("/api/identity/proposal");
    url.set_query(None);
    url.set_fragment(None);
    let proposed = post(
      &http,
      url.clone(),
      &token,
      serde_json::json!({"holder":device.peer_id().to_string(),"kind":kind}),
    )
    .await?;
    let record: client::trust::TrustRecord =
      serde_json::from_value(proposed).map_err(|e| e.to_string())?;
    match &record.action {
      client::protocol::TrustAction::ConfirmMember {
        device: Some(holder),
        ..
      } if kind == "membership" && holder == &device.peer_id().to_string() => {}
      client::protocol::TrustAction::BindGameProfile { profile, .. }
        if kind == "profile" && profile == &player.uuid.simple().to_string() => {}
      _ => return Err("Verifier proposed an unrelated identity action".into()),
    }
    let signed = client::trust::execute_identity(
      device.clone(),
      Input {
        config: config.clone(),
        operation: Operation::Sign { record },
      },
    )
    .await
    .map_err(|e| e.to_string())?;
    url.set_path("/api/identity/confirm");
    post(
      &http,
      url.clone(),
      &token,
      serde_json::json!({"record":signed}),
    )
    .await?;
  }
  client::trust::execute_identity(
    device,
    Input {
      config,
      operation: Operation::Sync,
    },
  )
  .await
  .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn union_management(request: client::management::Request) -> Result<serde_json::Value, String> {
  client::management::execute(request).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn union_public_identity(app: AppHandle) -> Result<client::management::PublicIdentity, String> {
  Ok(client::management::PublicIdentity::from_identity(
    &identity(&app)?,
  ))
}
#[tauri::command]
pub fn union_sign_identity_offline(
  app: AppHandle,
  record: client::trust::TrustRecord,
) -> Result<serde_json::Value, String> {
  let config = configuration(&app)?.ok_or("Unified identity is not configured")?;
  client::management::sign_offline(&identity(&app)?, &config, record).map_err(|e| e.to_string())
}
