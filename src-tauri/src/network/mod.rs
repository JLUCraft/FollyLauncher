use crate::error::LauncherError;
use libp2p::{PeerId, StreamProtocol};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

pub mod commands;
mod dht;
mod runner;
mod swarm;



pub(crate) const MC_CLUSTER_TOPIC: &str = "cluster";
pub(crate) const MC_GOVERNANCE_TOPIC: &str = "governance";
pub(crate) const MC_SYSTEM_TOPIC: &str = "system";
pub(crate) const MC_ADMIN_PUSH_TOPIC: &str = "admin.push";
pub(crate) const MC_TOURNAMENT_TOPIC_PREFIX: &str = "tournament.";
pub(crate) const MC_INSTANCE_TOPIC_PREFIX: &str = "instance.";

const NETWORK_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedInstance {
    pub instance_id: String,
    pub peer_id: String,
    pub proxy_address: Option<String>,
    pub public_ips: Vec<String>,
    pub multiaddrs: Vec<String>,
    pub resolved_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMessage {
    pub topic: String,
    pub peer_id: String,
    pub payload: serde_json::Value,
    pub received_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub status: String,
    pub host: String,
    pub mode: String,
    pub club: String,
    pub players: u32,
    pub max_players: u32,
    pub version: String,
    pub peer_id: String,
    pub discovered_at: String,
    pub updated_at: String,
}

#[derive(Debug)]
pub enum NetworkCommand {
    ResolveInstance {
        instance_id: String,
        respond: oneshot::Sender<Option<ResolvedInstance>>,
    },
    GetPeers {
        respond: oneshot::Sender<Vec<String>>,
    },
    OpenStream {
        peer_id: PeerId,
        protocol: StreamProtocol,
        respond: oneshot::Sender<Result<libp2p::Stream, LauncherError>>,
    },
    MeasureLatency {
        peer_id: String,
        respond: oneshot::Sender<Option<u32>>,
    },
    GetDiagnostics {
        respond: oneshot::Sender<NetworkDiagnostics>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDiagnostics {
    pub connected_peers: u32,
    pub dht_peers: u32,
    pub relay_connected: bool,
    pub dcutr_holes_punched: u32,
    pub dcutr_failures: u32,
    pub latencies: Vec<PeerLatency>,
    #[serde(default)]
    pub active_sessions: u32,
    #[serde(default)]
    pub total_bytes_rx: u64,
    #[serde(default)]
    pub total_bytes_tx: u64,
    #[serde(default)]
    pub bootstrap_peer_count: u32,
    #[serde(default)]
    pub bootstrap_reachable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerLatency {
    pub peer_id: String,
    pub latency_ms: u32,
    pub stale: bool,
}



#[derive(Clone)]
pub struct NetworkHandle {
    cmd_tx: mpsc::Sender<NetworkCommand>,
}
pub async fn start(
    keypair: libp2p::identity::Keypair,
    bootstrap_peers: Vec<String>,
) -> anyhow::Result<NetworkHandle> {
    let (cmd_tx, cmd_rx) = mpsc::channel::<NetworkCommand>(64);
    let stream_control = Arc::new(std::sync::Mutex::new(None));
    let swarm = swarm::build_swarm(keypair, stream_control.clone())?;
    let control = stream_control
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();

    tokio::spawn(runner::run_network(swarm, cmd_rx, bootstrap_peers, control));
    Ok(NetworkHandle { cmd_tx })
}




async fn send_command<T>(
    cmd_tx: &mpsc::Sender<NetworkCommand>,
    build: impl FnOnce(oneshot::Sender<T>) -> NetworkCommand,
    timeout: Duration,
) -> Result<T, LauncherError> {
    let (tx, rx) = oneshot::channel();
    cmd_tx
        .send(build(tx))
        .await
        .map_err(|_| LauncherError::from("网络后台任务未运行，请重启启动器或检查网络设置"))?;
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(_)) => Err(LauncherError::new(
            "ERROR",
            "网络后台任务已停止，请重启启动器",
        )),
        Err(_) => Err(LauncherError::new(
            "ERROR",
            "网络操作超时，请检查引导节点或 P2P 连接状态",
        )),
    }
}



impl NetworkHandle {


    #[cfg(test)]
    pub fn new_for_test() -> Self {
        let (cmd_tx, _cmd_rx) = mpsc::channel::<NetworkCommand>(1);
        Self { cmd_tx }
    }

    pub async fn resolve_instance(
        &self,
        instance_id: String,
    ) -> Result<Option<ResolvedInstance>, LauncherError> {
        send_command(
            &self.cmd_tx,
            |tx| NetworkCommand::ResolveInstance {
                instance_id,
                respond: tx,
            },
            NETWORK_COMMAND_TIMEOUT,
        )
        .await
    }

    pub async fn get_peers(&self) -> Result<Vec<String>, LauncherError> {
        send_command(
            &self.cmd_tx,
            |tx| NetworkCommand::GetPeers { respond: tx },
            NETWORK_COMMAND_TIMEOUT,
        )
        .await
    }

    pub async fn open_stream(
        &self,
        peer_id: PeerId,
        protocol: StreamProtocol,
    ) -> Result<libp2p::Stream, LauncherError> {
        send_command(
            &self.cmd_tx,
            |tx| NetworkCommand::OpenStream {
                peer_id,
                protocol,
                respond: tx,
            },
            NETWORK_COMMAND_TIMEOUT,
        )
        .await
        .and_then(|inner| inner)
    }

    pub async fn measure_latency(&self, peer_id: String) -> Result<Option<u32>, LauncherError> {
        send_command(
            &self.cmd_tx,
            |tx| NetworkCommand::MeasureLatency {
                peer_id,
                respond: tx,
            },
            NETWORK_COMMAND_TIMEOUT,
        )
        .await
    }

    pub async fn get_diagnostics(&self) -> Result<NetworkDiagnostics, LauncherError> {
        send_command(
            &self.cmd_tx,
            |tx| NetworkCommand::GetDiagnostics { respond: tx },
            NETWORK_COMMAND_TIMEOUT,
        )
        .await
    }
}



#[cfg(test)]
mod tests {
    use super::*;



    #[tokio::test]
    async fn test_send_command_closed_channel() {
        let (cmd_tx, cmd_rx) = mpsc::channel::<NetworkCommand>(1);
        drop(cmd_rx);
        let err = send_command(
            &cmd_tx,
            |tx| NetworkCommand::GetPeers { respond: tx },
            Duration::from_secs(1),
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("网络后台任务未运行"), "got: {err}");
    }

    #[tokio::test]
    async fn test_send_command_timeout() {
        let (cmd_tx, _cmd_rx) = mpsc::channel::<NetworkCommand>(1);
        let err = send_command(
            &cmd_tx,
            |tx| NetworkCommand::GetPeers { respond: tx },
            Duration::from_millis(50),
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("网络操作超时"), "got: {err}");
    }

    #[tokio::test]
    async fn test_send_command_receiver_dropped() {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<NetworkCommand>(1);
        let handle = tokio::spawn(async move {
            if let Some(NetworkCommand::GetPeers { respond }) = cmd_rx.recv().await {
                drop(respond);
            }
        });
        let err = send_command(
            &cmd_tx,
            |tx| NetworkCommand::GetPeers { respond: tx },
            Duration::from_secs(5),
        )
        .await
        .unwrap_err();
        handle.abort();
        assert!(err.to_string().contains("网络后台任务已停止"), "got: {err}");
    }

    #[tokio::test]
    async fn test_send_command_success() {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<NetworkCommand>(1);
        let handle = tokio::spawn(async move {
            if let Some(NetworkCommand::GetPeers { respond }) = cmd_rx.recv().await {
                let _ = respond.send(vec!["peer-a".to_string(), "peer-b".to_string()]);
            }
        });
        let peers = send_command(
            &cmd_tx,
            |tx| NetworkCommand::GetPeers { respond: tx },
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert_eq!(peers, vec!["peer-a", "peer-b"]);
        handle.abort();
    }
}
