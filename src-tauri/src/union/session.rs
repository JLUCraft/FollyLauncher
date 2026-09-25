//! Authenticated, short-lived federation clients; every command closes its node.
use crate::union::{identity, trust};
use client::{Identity, Multiaddr, Node, NodeConfig, PeerId, trust::TrustLog};
use tauri::AppHandle;

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
  pub address: String,
  pub peer: String,
  pub genesis: String,
}
pub struct Session {
  pub node: Node,
  pub peer: PeerId,
  pub identity: Identity,
  pub trust: TrustLog,
}
impl Session {
  pub async fn open(app: &AppHandle, address: &str, peer: &str) -> Result<Self, String> {
    let peer: PeerId = peer.parse().map_err(|_| "Invalid peer")?;
    let address: Multiaddr = address.parse().map_err(|_| "Invalid address")?;
    if !matches!(address.iter().last(), Some(client::AddressProtocol::P2p(id)) if id == peer) {
      return Err("Address peer mismatch".into());
    }
    trust::configuration(app)?.ok_or("Configure identity first")?;
    let identity = identity(app)?;
    let node = Node::start(identity.clone(), NodeConfig::default())
      .await
      .map_err(|e| e.to_string())?;
    let result = async {
      let trust = trust::synchronize(app, &node)
        .await?
        .ok_or("Configure identity first")?;
      node.dial(address).await.map_err(|e| e.to_string())?;
      Ok::<_, String>(trust)
    }
    .await;
    match result {
      Ok(trust) => Ok(Self {
        node,
        peer,
        identity,
        trust,
      }),
      Err(error) => {
        let _ = node.shutdown().await;
        Err(error)
      }
    }
  }
  pub async fn finish<T>(self, result: Result<T, String>) -> Result<T, String> {
    let closed = self.node.shutdown().await.map_err(|e| e.to_string());
    match result {
      Ok(value) => closed.map(|_| value),
      Err(error) => Err(error),
    }
  }
}
impl Target {
  pub async fn open(&self, app: &AppHandle) -> Result<Session, String> {
    if self.genesis.len() != 64
      || !self
        .genesis
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
      return Err("Invalid office genesis hash".into());
    }
    Session::open(app, &self.address, &self.peer).await
  }
}
