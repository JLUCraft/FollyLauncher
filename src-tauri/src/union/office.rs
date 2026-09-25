use crate::union::session::Target;
use base64::{Engine, engine::general_purpose::STANDARD};
use client::office::{self, Office};
use tauri::AppHandle;
use wire::protocol::{Cid, FileNode};

#[derive(serde::Serialize)]
pub struct Snapshot {
  state: serde_json::Value,
  identity: serde_json::Value,
  commits: std::collections::BTreeMap<Cid, FileNode>,
  max_file_bytes: usize,
  max_commit_paths: usize,
}
#[tauri::command]
pub async fn union_office_snapshot(app: AppHandle, target: Target) -> Result<Snapshot, String> {
  let session = target.open(&app).await?;
  let result = async {
    let state = office::snapshot(&session.node, session.peer, &target.genesis)
      .await
      .map_err(|e| e.to_string())?;
    let identity = state
      .identity_core(session.trust.state())
      .map_err(|e| e.to_string())?;
    let store = &state.domain.inner.files;
    let mut commits = std::collections::BTreeMap::new();
    for cid in store.blocks.keys().filter(|cid| !cid.is_raw()) {
      let node = store
        .block(cid)
        .and_then(|block| block.decode())
        .map_err(|e| e.to_string())?;
      if matches!(node, FileNode::Commit { .. }) {
        commits.insert(cid.clone(), node);
      }
    }
    let mut value = serde_json::to_value(&state).map_err(|e| e.to_string())?;
    // File bytes stay native; directory entries and history are enough for browsing.
    if let Some(files) = value.get_mut("files").and_then(|v| v.as_object_mut()) {
      files.remove("blocks");
    }
    Ok(Snapshot {
      state: value,
      identity: serde_json::to_value(identity).map_err(|e| e.to_string())?,
      commits,
      max_commit_paths: events::files::MAX_COMMIT_PATHS,
      max_file_bytes: events::files::MAX_FILE_BYTES,
    })
  }
  .await;
  session.finish(result).await
}
#[tauri::command]
pub async fn union_office_action(
  app: AppHandle,
  target: Target,
  action: flows::FlowAction<events::EventsDomain>,
) -> Result<(), String> {
  let session = target.open(&app).await?;
  let result = async {
    let response = client::trust::execute::<Office>(
      &session.node,
      session.peer,
      &target.genesis,
      &session.identity,
      &session.trust,
      client::governance::Action::Domain(action),
    )
    .await
    .map_err(|e| e.to_string())?;
    if let Some(error) = response.error {
      return Err(error);
    }
    response
      .state
      .ok_or_else(|| "Office action returned no state".into())
      .map(|_| ())
  }
  .await;
  session.finish(result).await
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upload {
  branch: String,
  path: String,
  message: String,
  data: String,
}
#[tauri::command]
pub async fn union_office_upload(
  app: AppHandle,
  target: Target,
  input: Upload,
) -> Result<Cid, String> {
  let bytes = decode_upload(&input.data)?;
  let session = target.open(&app).await?;
  let result = office::upload(
    &session.node,
    session.peer,
    &target.genesis,
    &session.identity,
    office::Upload {
      branch: &input.branch,
      path: &input.path,
      message: &input.message,
      bytes: &bytes,
    },
    &session.trust,
  )
  .await
  .map_err(|e| e.to_string());
  session.finish(result).await
}
#[tauri::command]
pub async fn union_office_fetch(
  app: AppHandle,
  target: Target,
  cid: Cid,
) -> Result<String, String> {
  let session = target.open(&app).await?;
  let result = async {
    let state = office::snapshot(&session.node, session.peer, &target.genesis)
      .await
      .map_err(|e| e.to_string())?;
    state
      .domain
      .inner
      .files
      .read(&cid)
      .map(|bytes| STANDARD.encode(bytes))
      .map_err(|e| e.to_string())
  }
  .await;
  session.finish(result).await
}

fn decode_upload(data: &str) -> Result<Vec<u8>, String> {
  const MAX_ENCODED: usize = events::files::MAX_FILE_BYTES.div_ceil(3) * 4;
  if data.len() > MAX_ENCODED {
    return Err("File too large".into());
  }
  let bytes = STANDARD.decode(data).map_err(|_| "Invalid file encoding")?;
  if bytes.len() > events::files::MAX_FILE_BYTES {
    return Err("File too large".into());
  }
  Ok(bytes)
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn binary_upload_limits_and_encoding() {
    let bytes = vec![255; events::files::MAX_FILE_BYTES];
    assert_eq!(decode_upload(&STANDARD.encode(&bytes)).unwrap(), bytes);
    assert!(decode_upload("%%%").is_err());
    assert!(decode_upload(&STANDARD.encode(vec![0; events::files::MAX_FILE_BYTES + 1])).is_err());
    assert!(decode_upload(&"A".repeat(events::files::MAX_FILE_BYTES * 2)).is_err());
  }
}
