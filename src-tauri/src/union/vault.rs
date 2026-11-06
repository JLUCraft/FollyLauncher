//! OS credential storage is mandatory; an unavailable vault never creates a new identity.
use identity::Identity;
use std::{fs::OpenOptions, path::Path, sync::Mutex};
use zeroize::Zeroizing;
static SERIAL: Mutex<()> = Mutex::new(());
pub async fn identity(directory: &Path) -> Result<Identity, String> {
  identity_for(directory, "launcher").await
}
pub(super) async fn identity_for(
  directory: &Path,
  account: &'static str,
) -> Result<Identity, String> {
  let directory = directory.to_owned();
  // OS prompts and both locks may wait indefinitely; keep them off UI and runtime threads.
  tauri::async_runtime::spawn_blocking(move || load_identity(&directory, account))
    .await
    .map_err(|_| "Identity storage task failed".to_string())?
}
fn load_identity(directory: &Path, account: &str) -> Result<Identity, String> {
  let _serial = SERIAL
    .lock()
    .map_err(|_| "Identity storage is temporarily unavailable".to_string())?;
  std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
  let lock = OpenOptions::new()
    .create(true)
    .truncate(false)
    .read(true)
    .write(true)
    .open(directory.join("union-device.lock"))
    .map_err(|e| e.to_string())?;
  lock.lock().map_err(|e| e.to_string())?;
  let entry = keyring::Entry::new("com.jlucraft.union.device", account)
    .map_err(|_| "Cannot open system credential vault".to_string())?;
  load(directory, &entry)
}
fn load(directory: &Path, entry: &keyring::Entry) -> Result<Identity, String> {
  if directory.join("union-device.key").exists() {
    return Err("Plaintext identity files are not supported; register the device again".into());
  }
  let marker = directory.join("union-device.peer");
  let stored = match entry.get_secret() {
    Ok(secret) => Some(Zeroizing::new(secret)),
    Err(keyring::Error::NoEntry) => None,
    Err(_) => return Err("Unlock the system credential vault and retry".into()),
  };
  if stored.is_none() && marker.exists() {
    return Err(
      "Federation identity is missing from the credential vault; use member device recovery".into(),
    );
  }
  let identity = match stored {
    Some(secret) => Identity::import_secret(&secret)
      .map_err(|_| "Federation identity in the credential vault is corrupt".to_string())?,
    None => {
      let identity = Identity::generate();
      let secret = Zeroizing::new(identity.export_secret().map_err(|e| e.to_string())?);
      entry
        .set_secret(&secret)
        .map_err(|_| "Cannot save to system credential vault".to_string())?;
      let saved = Zeroizing::new(
        entry
          .get_secret()
          .map_err(|_| "Cannot verify credential vault write".to_string())?,
      );
      if *saved != *secret {
        return Err("Credential vault write verification failed".into());
      }
      identity
    }
  };
  let peer = identity.peer_id().to_string();
  if marker.exists()
    && std::fs::read_to_string(&marker)
      .map_err(|e| e.to_string())?
      .trim()
      != peer
  {
    return Err("Device identity in the credential vault changed".into());
  }
  std::fs::write(marker, peer).map_err(|e| e.to_string())?;
  Ok(identity)
}
#[cfg(test)]
mod tests {
  use crate::union::vault::load;
  use identity::Identity;
  #[tokio::test(flavor = "current_thread")]
  async fn waiting_for_identity_storage_does_not_block_the_runtime() {
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
      let _guard = super::SERIAL.lock().unwrap();
      ready_tx.send(()).unwrap();
      release_rx.recv_timeout(std::time::Duration::from_secs(5))
    });
    ready_rx.recv().unwrap();
    // A file cannot be an identity directory, so this never accesses an OS vault.
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().to_owned();
    let request = tokio::spawn(async move { super::identity_for(&path, "launcher").await });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(!request.is_finished());
    let released = release_tx.send(());
    assert!(
      holder.join().unwrap().is_ok(),
      "identity loading blocked the runtime"
    );
    released.unwrap();
    assert!(request.await.unwrap().is_err());
  }
  #[test]
  fn vault_is_required_and_unsupported_files_are_rejected() {
    keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    let entry = keyring::Entry::new("union-vault-test", "device").unwrap();
    let directory = tempfile::tempdir().unwrap();
    let old = Identity::generate();
    old
      .save_new(directory.path().join("union-device.key"))
      .unwrap();
    assert!(load(directory.path(), &entry).is_err());
    let fresh = tempfile::tempdir().unwrap();
    let peer = load(fresh.path(), &entry).unwrap().peer_id();
    assert_eq!(load(fresh.path(), &entry).unwrap().peer_id(), peer);
    entry.delete_credential().unwrap();
    assert!(load(fresh.path(), &entry).is_err());
  }
}
