use anyhow::Context;
use futures::StreamExt;
use libp2p::{
    dcutr, gossipsub, identify,
    kad::{self, store::MemoryStore, Mode},
    noise, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    yamux, Multiaddr, PeerId, StreamProtocol, Swarm, SwarmBuilder,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, info, warn};

const MC_DHT_PROTOCOL: &str = "/jlucraft/kad/1.0.0";
const MC_IDENTIFY_PROTOCOL: &str = "/jlucraft/identify/1.0.0";
const MC_CLUSTER_TOPIC: &str = "mc.events.cluster";
const MC_GOVERNANCE_TOPIC: &str = "mc.events.governance";

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
    GetMessages {
        respond: oneshot::Sender<Vec<ClusterMessage>>,
    },
    OpenStream {
        peer_id: PeerId,
        protocol: StreamProtocol,
        respond: oneshot::Sender<Result<libp2p::Stream, String>>,
    },
    GetInstances {
        respond: oneshot::Sender<Vec<InstanceInfo>>,
    },
    MeasureLatency {
        peer_id: String,
        respond: oneshot::Sender<Option<u32>>,
    },
    GetDiagnostics {
        respond: oneshot::Sender<NetworkDiagnostics>,
    },
    SelectBestRelay {
        target_peer_id: String,
        club: Option<String>,
        respond: oneshot::Sender<Option<PeerId>>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerLatency {
    pub peer_id: String,
    pub latency_ms: u32,
    pub stale: bool,
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "LauncherEvent")]
pub struct LauncherBehaviour {
    pub identify: identify::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub relay_client: relay::client::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub stream: libp2p_stream::Behaviour,
}

#[derive(Debug)]
pub enum LauncherEvent {
    Identify(Box<identify::Event>),
    Kademlia(Box<kad::Event>),
    RelayClient(()),
    Dcutr(()),
    Gossipsub(Box<gossipsub::Event>),
    Stream(()),
}

impl From<identify::Event> for LauncherEvent {
    fn from(e: identify::Event) -> Self {
        Self::Identify(Box::new(e))
    }
}
impl From<kad::Event> for LauncherEvent {
    fn from(e: kad::Event) -> Self {
        Self::Kademlia(Box::new(e))
    }
}
impl From<relay::client::Event> for LauncherEvent {
    fn from(_: relay::client::Event) -> Self {
        Self::RelayClient(())
    }
}
impl From<dcutr::Event> for LauncherEvent {
    fn from(_: dcutr::Event) -> Self {
        Self::Dcutr(())
    }
}
impl From<gossipsub::Event> for LauncherEvent {
    fn from(e: gossipsub::Event) -> Self {
        Self::Gossipsub(Box::new(e))
    }
}
impl From<()> for LauncherEvent {
    fn from(_: ()) -> Self {
        Self::Stream(())
    }
}

#[derive(Clone)]
pub struct NetworkHandle {
    cmd_tx: mpsc::Sender<NetworkCommand>,
}

pub struct Network;

impl Network {
    pub async fn start(
        keypair: libp2p::identity::Keypair,
        bootstrap_peers: Vec<String>,
    ) -> anyhow::Result<(Self, NetworkHandle)> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<NetworkCommand>(64);
        let messages = Arc::new(RwLock::new(VecDeque::<ClusterMessage>::new()));
        let known_instances = Arc::new(RwLock::new(HashMap::<String, InstanceInfo>::new()));
        let stream_control = std::sync::Arc::new(std::sync::Mutex::new(None));
        let swarm = build_swarm(keypair, stream_control.clone())?;
        let control = stream_control.lock().unwrap_or_else(|e| e.into_inner()).take();
        let handle = NetworkHandle {
            cmd_tx: cmd_tx.clone(),
        };

        tokio::spawn(run_network(
            swarm,
            cmd_rx,
            bootstrap_peers,
            messages.clone(),
            known_instances.clone(),
            control,
        ));
        Ok((Network, handle))
    }
}

impl NetworkHandle {
    pub async fn resolve_instance(&self, instance_id: String) -> Option<ResolvedInstance> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(NetworkCommand::ResolveInstance {
                instance_id,
                respond: tx,
            })
            .await;
        rx.await.ok().flatten()
    }

    pub async fn get_peers(&self) -> Vec<String> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(NetworkCommand::GetPeers { respond: tx })
            .await;
        rx.await.unwrap_or_else(|_| Vec::new())
    }

    pub async fn get_messages(&self) -> Vec<ClusterMessage> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(NetworkCommand::GetMessages { respond: tx })
            .await;
        rx.await.unwrap_or_else(|_| Vec::new())
    }

    pub async fn open_stream(
        &self,
        peer_id: PeerId,
        protocol: StreamProtocol,
    ) -> Result<libp2p::Stream, String> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(NetworkCommand::OpenStream {
                peer_id,
                protocol,
                respond: tx,
            })
            .await;
        rx.await.unwrap_or(Err("channel closed".to_string()))
    }

    pub async fn list_instances(&self) -> Vec<InstanceInfo> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(NetworkCommand::GetInstances { respond: tx })
            .await;
        rx.await.unwrap_or_else(|_| Vec::new())
    }

    pub async fn measure_latency(&self, peer_id: String) -> Option<u32> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(NetworkCommand::MeasureLatency {
                peer_id,
                respond: tx,
            })
            .await;
        rx.await.unwrap_or(None)
    }

    pub async fn get_diagnostics(&self) -> NetworkDiagnostics {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(NetworkCommand::GetDiagnostics { respond: tx })
            .await;
        rx.await.unwrap_or_else(|_| NetworkDiagnostics {
            connected_peers: 0,
            dht_peers: 0,
            relay_connected: false,
            dcutr_holes_punched: 0,
            dcutr_failures: 0,
            latencies: Vec::new(),
        })
    }

    pub async fn select_best_relay(
        &self,
        target_peer_id: String,
        club: Option<String>,
    ) -> Option<PeerId> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(NetworkCommand::SelectBestRelay {
                target_peer_id,
                club,
                respond: tx,
            })
            .await;
        rx.await.unwrap_or(None)
    }
}

fn build_swarm(
    keypair: libp2p::identity::Keypair,
    stream_control: std::sync::Arc<std::sync::Mutex<Option<libp2p_stream::Control>>>,
) -> anyhow::Result<Swarm<LauncherBehaviour>> {
    let dht_protocol = Box::leak(MC_DHT_PROTOCOL.to_string().into_boxed_str());

    let message_id_fn = |message: &gossipsub::Message| {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        message.data.hash(&mut hasher);
        gossipsub::MessageId::from(hasher.finish().to_string())
    };

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(30))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;

    let cluster_topic = MC_CLUSTER_TOPIC.to_string();
    let governance_topic = MC_GOVERNANCE_TOPIC.to_string();

    let swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_quic_config(|mut config| {
            config.keep_alive_interval = Duration::from_secs(15);
            config.max_idle_timeout = 30000;
            config
        })
        .with_relay_client(noise::Config::new, yamux::Config::default)
        .context("failed to enable relay client")?
        .with_behaviour(move |key: &libp2p::identity::Keypair, relay_client| {
            let local_peer_id = key.public().to_peer_id();

            let mut gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config.clone(),
            )
            .expect("validated gossipsub config");

            let _ = gossipsub.subscribe(&gossipsub::IdentTopic::new(cluster_topic.clone()));
            let _ = gossipsub.subscribe(&gossipsub::IdentTopic::new(governance_topic.clone()));

            let mut kad_config = kad::Config::new(StreamProtocol::new(dht_protocol));
            kad_config.set_query_timeout(Duration::from_secs(30));
            let mut kademlia = kad::Behaviour::with_config(
                local_peer_id,
                MemoryStore::new(local_peer_id),
                kad_config,
            );
            kademlia.set_mode(Some(Mode::Client));

            let identify = identify::Behaviour::new(
                identify::Config::new(MC_IDENTIFY_PROTOCOL.to_string(), key.public())
                    .with_agent_version("FollyLauncher/1.0.0".to_string()),
            );

            let stream_behaviour = libp2p_stream::Behaviour::new();
            *stream_control.lock().unwrap_or_else(|e| e.into_inner()) = Some(stream_behaviour.new_control());

            LauncherBehaviour {
                identify,
                kademlia,
                relay_client,
                dcutr: dcutr::Behaviour::new(local_peer_id),
                gossipsub,
                stream: stream_behaviour,
            }
        })?
        .build();

    Ok(swarm)
}

async fn run_network(
    mut swarm: Swarm<LauncherBehaviour>,
    mut cmd_rx: mpsc::Receiver<NetworkCommand>,
    bootstrap_peers: Vec<String>,
    messages: Arc<RwLock<VecDeque<ClusterMessage>>>,
    known_instances: Arc<RwLock<HashMap<String, InstanceInfo>>>,
    mut stream_control: Option<libp2p_stream::Control>,
) {
    let mut pending_queries: HashMap<kad::QueryId, oneshot::Sender<Option<ResolvedInstance>>> =
        HashMap::new();
    let mut latency_cache: HashMap<PeerId, (u32, Instant)> = HashMap::new();
    let mut pending_latency: HashMap<PeerId, (Instant, oneshot::Sender<Option<u32>>)> =
        HashMap::new();
    let mut dcutr_holes_punched: u32 = 0;
    let mut dcutr_failures: u32 = 0;
    let mut relay_connected = false;

    // Track peer metadata for relay priority
    let peer_clubs: Arc<RwLock<HashMap<PeerId, String>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let peer_public_ips: Arc<RwLock<HashMap<PeerId, bool>>> =
        Arc::new(RwLock::new(HashMap::new()));

    for peer in &bootstrap_peers {
        if let Ok(addr) = peer.parse::<Multiaddr>() {
            if let Some((peer_id, rest)) = split_peer_addr(addr.clone()) {
                swarm.behaviour_mut().kademlia.add_address(&peer_id, rest);
            }
            if let Err(e) = swarm.dial(addr) {
                warn!(error = %e, "failed to dial bootstrap peer");
            }
        }
    }

    if let Err(e) = swarm.behaviour_mut().kademlia.bootstrap() {
        warn!(error = %e, "bootstrap failed");
    }

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    NetworkCommand::ResolveInstance { instance_id, respond } => {
                        let key = kad::RecordKey::new(&format!("/instance/{}", instance_id));
                        let query_id = swarm.behaviour_mut().kademlia.get_record(key);
                        pending_queries.insert(query_id, respond);
                    }
                    NetworkCommand::GetPeers { respond } => {
                        let peers: Vec<String> = swarm.connected_peers()
                            .map(|p| p.to_string())
                            .collect();
                        let _ = respond.send(peers);
                    }
                    NetworkCommand::GetMessages { respond } => {
                        let msgs: Vec<_> = messages.read().await.iter().cloned().collect();
                        let _ = respond.send(msgs);
                    }
                    NetworkCommand::OpenStream { peer_id, protocol, respond } => {
                        if let Some(ref mut control) = stream_control {
                            let mut control = control.clone();
                            tokio::spawn(async move {
                                let result = control.open_stream(peer_id, protocol).await
                                    .map_err(|e| e.to_string());
                                let _ = respond.send(result);
                            });
                        } else {
                            let _ = respond.send(Err("stream control not available".to_string()));
                        }
                    }
                    NetworkCommand::GetInstances { respond } => {
                        let instances: Vec<InstanceInfo> = known_instances.read().await.values().cloned().collect();
                        let _ = respond.send(instances);
                    }
                    NetworkCommand::MeasureLatency { peer_id, respond } => {
                        match peer_id.parse::<PeerId>() {
                            Ok(target) => {
                                let now = Instant::now();
                                let cache_ttl = Duration::from_secs(300);
                                if let Some((ms, cached_at)) = latency_cache.get(&target) {
                                    if cached_at.elapsed() < cache_ttl {
                                        let _ = respond.send(Some(*ms));
                                        continue;
                                    }
                                }
                                if pending_latency.contains_key(&target) {
                                    let _ = respond.send(None);
                                    continue;
                                }
                                if swarm.is_connected(&target) {
                                    let _ = respond.send(latency_cache.get(&target).map(|(ms, _)| *ms));
                                    continue;
                                }
                                if let Err(e) = swarm.dial(target) {
                                    warn!(%target, error = %e, "latency measurement dial failed");
                                    let _ = respond.send(None);
                                } else {
                                    pending_latency.insert(target, (now, respond));
                                }
                            }
                            Err(e) => {
                                warn!(peer_id = %peer_id, error = %e, "invalid peer id for latency measurement");
                                let _ = respond.send(None);
                            }
                        }
                    }
                    NetworkCommand::GetDiagnostics { respond } => {
                        let latencies: Vec<PeerLatency> = latency_cache.iter().map(|(pid, (ms, at))| {
                            PeerLatency {
                                peer_id: pid.to_string(),
                                latency_ms: *ms,
                                stale: at.elapsed() > Duration::from_secs(300),
                            }
                        }).collect();
                        let dht = swarm.connected_peers().count() as u32;
                        let diag = NetworkDiagnostics {
                            connected_peers: swarm.connected_peers().count() as u32,
                            dht_peers: dht,
                            relay_connected,
                            dcutr_holes_punched,
                            dcutr_failures,
                            latencies,
                        };
                        let _ = respond.send(diag);
                    }
                    NetworkCommand::SelectBestRelay { target_peer_id, club, respond } => {
                        if let Ok(target) = target_peer_id.parse::<PeerId>() {
                            let clubs = peer_clubs.read().await;
                            let ips = peer_public_ips.read().await;
                            let connected: Vec<PeerId> = swarm.connected_peers().copied().collect();

                            let mut selected = None;
                            if let Some(ref c) = club {
                                for pid in &connected {
                                    if pid == &target { continue; }
                                    if clubs.get(pid).map(|s| s.as_str()) == Some(c.as_str()) {
                                        selected = Some(*pid);
                                        break;
                                    }
                                }
                            }
                            if selected.is_none() {
                                for pid in &connected {
                                    if pid == &target { continue; }
                                    if ips.get(pid).copied().unwrap_or(false) {
                                        selected = Some(*pid);
                                        break;
                                    }
                                }
                            }
                            if selected.is_none() {
                                for pid in &connected {
                                    if pid == &target { continue; }
                                    selected = Some(*pid);
                                    break;
                                }
                            }
                            let _ = respond.send(selected);
                        } else {
                            let _ = respond.send(None);
                        }
                    }
                }
            }
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        info!(%peer_id, "connected to peer");
                        if endpoint.is_relayed() {
                            relay_connected = true;
                        }
                        if let Some((started_at, respond)) = pending_latency.remove(&peer_id) {
                            let rtt_ms = started_at.elapsed().as_millis() as u32;
                            latency_cache.insert(peer_id, (rtt_ms, Instant::now()));
                            let _ = respond.send(Some(rtt_ms));
                        }
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id: Some(peer_id), error, .. } => {
                        warn!(%peer_id, error = %error, "outgoing connection failed");
                        dcutr_failures += 1;
                        if let Some((_, respond)) = pending_latency.remove(&peer_id) {
                            let _ = respond.send(None);
                        }
                    }
                    SwarmEvent::OutgoingConnectionError { .. } => {}
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        info!(%peer_id, cause = ?cause, "disconnected from peer");
                    }
                    SwarmEvent::Behaviour(LauncherEvent::Kademlia(event)) => {
                        match *event {
                            kad::Event::OutboundQueryProgressed {
                                id,
                                result: kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))),
                                ..
                            } => {
                                if let Some(respond) = pending_queries.remove(&id) {
                                    let resolved = parse_instance_record(&record.record.value);
                                    let _ = respond.send(resolved);
                                }
                            }
                            kad::Event::OutboundQueryProgressed {
                                id,
                                result: kad::QueryResult::GetRecord(Err(_)),
                                ..
                            } => {
                                if let Some(respond) = pending_queries.remove(&id) {
                                    let _ = respond.send(None);
                                }
                            }
                            _ => {}
                        }
                    }
                    SwarmEvent::Behaviour(LauncherEvent::Identify(event)) => {
                        if let identify::Event::Received { peer_id, info, .. } = *event {
                            for addr in &info.listen_addrs {
                                swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                            }
                            // Track peer metadata for relay priority
                            let has_public_ip = info.listen_addrs.iter().any(|a| {
                                a.iter().any(|p| matches!(p, libp2p::multiaddr::Protocol::Ip4(ip) if !ip.is_loopback() && !ip.is_private()))
                            });
                            peer_public_ips.write().await.insert(peer_id, has_public_ip);
                            if !info.agent_version.is_empty() {
                                // Parse club from agent string format: "FollyLauncher/0.1.0/club:<name>"
                                if let Some(club_part) = info.agent_version.split('/').nth(3) {
                                    if let Some(club_name) = club_part.strip_prefix("club:") {
                                        peer_clubs.write().await.insert(peer_id, club_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                    SwarmEvent::Behaviour(LauncherEvent::Dcutr(())) => {
                        dcutr_holes_punched += 1;
                        debug!("DCUtR hole punch completed");
                    }
                    SwarmEvent::Behaviour(LauncherEvent::RelayClient(())) => {
                        relay_connected = true;
                        debug!("relay client event received");
                    }
                    SwarmEvent::Behaviour(LauncherEvent::Gossipsub(event)) => {
                        if let gossipsub::Event::Message {
                            propagation_source: peer_id,
                            message,
                            ..
                        } = *event
                        {
                            let topic = message.topic.to_string();
                            let payload = serde_json::from_slice::<serde_json::Value>(&message.data)
                                .unwrap_or_else(|_| {
                                    use base64::Engine;
                                    serde_json::json!({ "raw": base64::engine::general_purpose::STANDARD.encode(&message.data) })
                                });
                            info!(peer_id = %peer_id, topic = %topic, "received gossipsub message");

                            // Parse instance lifecycle events from cluster topic
                            if topic == MC_CLUSTER_TOPIC || topic == "mc.events.cluster" {
                                if let Some(event_type) = payload.get("event_type").and_then(|v| v.as_str()) {
                                    if event_type.starts_with("instance-") {
                                        if let Some(instance_payload) = payload.get("payload") {
                                            update_known_instances(&known_instances, event_type, instance_payload, &peer_id.to_string()).await;
                                        }
                                    }
                                }
                            }

                            let msg = ClusterMessage {
                                topic,
                                peer_id: peer_id.to_string(),
                                payload,
                                received_at: chrono::Utc::now().to_rfc3339(),
                            };
                            let mut msgs = messages.write().await;
                            msgs.push_back(msg);
                            // Keep last 256 messages
                            while msgs.len() > 256 {
                                msgs.pop_front();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn update_known_instances(
    known_instances: &Arc<RwLock<HashMap<String, InstanceInfo>>>,
    event_type: &str,
    payload: &serde_json::Value,
    peer_id: &str,
) {
    let Some(id) = payload.get("instance_id").and_then(|v| v.as_str()) else {
        warn!("cluster instance event missing instance_id");
        return;
    };

    let now = chrono::Utc::now().to_rfc3339();

    match event_type {
        "instance-created" => {
            let Some(name) = payload.get("name").and_then(|v| v.as_str()) else {
                warn!(instance_id = %id, "cluster instance-created missing name");
                return;
            };
            let Some(kind) = payload.get("kind").and_then(|v| v.as_str()) else {
                warn!(instance_id = %id, "cluster instance-created missing kind");
                return;
            };
            let Some(host) = payload.get("host").and_then(|v| v.as_str()) else {
                warn!(instance_id = %id, "cluster instance-created missing host");
                return;
            };
            let Some(mode) = payload.get("mode").and_then(|v| v.as_str()) else {
                warn!(instance_id = %id, "cluster instance-created missing mode");
                return;
            };
            let Some(club) = payload.get("club").and_then(|v| v.as_str()) else {
                warn!(instance_id = %id, "cluster instance-created missing club");
                return;
            };
            let Some(players) = payload.get("players").and_then(|v| v.as_u64()) else {
                warn!(instance_id = %id, "cluster instance-created missing players");
                return;
            };
            let Some(max_players) = payload.get("max_players").and_then(|v| v.as_u64()) else {
                warn!(instance_id = %id, "cluster instance-created missing max_players");
                return;
            };
            let Some(version) = payload.get("version").and_then(|v| v.as_str()) else {
                warn!(instance_id = %id, "cluster instance-created missing version");
                return;
            };
            let info = InstanceInfo {
                id: id.to_string(),
                name: name.to_string(),
                kind: kind.to_string(),
                status: "created".to_string(),
                host: host.to_string(),
                mode: mode.to_string(),
                club: club.to_string(),
                players: players as u32,
                max_players: max_players as u32,
                version: version.to_string(),
                peer_id: peer_id.to_string(),
                discovered_at: now.clone(),
                updated_at: now,
            };
            known_instances.write().await.insert(id.to_string(), info);
        }
        "instance-started" => {
            if let Some(info) = known_instances.write().await.get_mut(id) {
                info.status = "running".to_string();
                info.updated_at = now;
            }
        }
        "instance-stopped" => {
            if let Some(info) = known_instances.write().await.get_mut(id) {
                info.status = "stopped".to_string();
                info.updated_at = now;
            }
        }
        "instance-destroyed" => {
            known_instances.write().await.remove(id);
        }
        _ => {}
    }
}

fn split_peer_addr(addr: Multiaddr) -> Option<(PeerId, Multiaddr)> {
    let mut protocols = addr.into_iter().collect::<Vec<_>>();
    let last = protocols.pop()?;
    match last {
        libp2p::multiaddr::Protocol::P2p(peer_id) => {
            Some((peer_id, protocols.into_iter().collect()))
        }
        _ => None,
    }
}

fn parse_instance_record(value: &[u8]) -> Option<ResolvedInstance> {
    #[derive(Deserialize)]
    struct RecordPayload {
        instance_id: String,
        peer_id: String,
        proxy_address: String,
        public_ips: Vec<String>,
        listen_addrs: Vec<String>,
    }

    let record: RecordPayload = serde_json::from_slice(value).ok()?;
    let resolved = ResolvedInstance {
        instance_id: record.instance_id.clone(),
        peer_id: record.peer_id.clone(),
        proxy_address: Some(record.proxy_address.clone()),
        public_ips: record.public_ips,
        multiaddrs: record.listen_addrs,
        resolved_at: chrono::Utc::now().to_rfc3339(),
    };
    Some(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── Network diagnostics defaults ────────────────────────────────────

    #[test]
    fn test_network_diagnostics_defaults() {
        let diag = NetworkDiagnostics {
            connected_peers: 0,
            dht_peers: 0,
            relay_connected: false,
            dcutr_holes_punched: 0,
            dcutr_failures: 0,
            latencies: Vec::new(),
        };

        // Round-trip through JSON
        let json = serde_json::to_string(&diag).unwrap();
        let parsed: NetworkDiagnostics = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.connected_peers, 0);
        assert_eq!(parsed.dht_peers, 0);
        assert!(!parsed.relay_connected);
        assert_eq!(parsed.dcutr_holes_punched, 0);
        assert_eq!(parsed.dcutr_failures, 0);
        assert!(parsed.latencies.is_empty());
    }

    #[test]
    fn test_network_diagnostics_with_data() {
        let diag = NetworkDiagnostics {
            connected_peers: 5,
            dht_peers: 12,
            relay_connected: true,
            dcutr_holes_punched: 3,
            dcutr_failures: 1,
            latencies: vec![
                PeerLatency {
                    peer_id: "12D3KooWPeer1".to_string(),
                    latency_ms: 42,
                    stale: false,
                },
                PeerLatency {
                    peer_id: "12D3KooWPeer2".to_string(),
                    latency_ms: 120,
                    stale: true,
                },
            ],
        };

        let json = serde_json::to_string(&diag).unwrap();
        let parsed: NetworkDiagnostics = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.connected_peers, 5);
        assert_eq!(parsed.dht_peers, 12);
        assert!(parsed.relay_connected);
        assert_eq!(parsed.latencies.len(), 2);
        assert_eq!(parsed.latencies[0].latency_ms, 42);
        assert!(!parsed.latencies[0].stale);
        assert!(parsed.latencies[1].stale);
    }

    // ── ResolvedInstance parse ──────────────────────────────────────────

    #[test]
    fn test_resolved_instance_parse() {
        let instance = ResolvedInstance {
            instance_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            peer_id: "12D3KooWHyYqNJxXqRq9HuCvLp5sMQmR8kWjPFQGrWfRAdZ9MdiJ".to_string(),
            proxy_address: Some("192.168.1.100:25566".to_string()),
            public_ips: vec!["203.0.113.5".to_string(), "198.51.100.10".to_string()],
            multiaddrs: vec![
                "/ip4/192.168.1.100/tcp/25565".to_string(),
                "/ip4/192.168.1.100/udp/25565/quic-v1".to_string(),
            ],
            resolved_at: "2025-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&instance).unwrap();
        let parsed: ResolvedInstance = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.instance_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(parsed.peer_id, "12D3KooWHyYqNJxXqRq9HuCvLp5sMQmR8kWjPFQGrWfRAdZ9MdiJ");
        assert_eq!(parsed.public_ips.len(), 2);
        assert_eq!(parsed.multiaddrs.len(), 2);
    }

    // ── ClusterMessage parse ────────────────────────────────────────────

    #[test]
    fn test_cluster_message_parse() {
        let msg = ClusterMessage {
            topic: "mc.events.cluster".to_string(),
            peer_id: "12D3KooWTestPeer".to_string(),
            payload: json!({
                "event_type": "instance-created",
                "payload": {
                    "instance_id": "abc-123",
                    "name": "test-instance"
                }
            }),
            received_at: "2025-01-15T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClusterMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.topic, "mc.events.cluster");
        assert_eq!(parsed.peer_id, "12D3KooWTestPeer");
        assert_eq!(
            parsed.payload["event_type"].as_str().unwrap(),
            "instance-created"
        );
        assert_eq!(parsed.received_at, "2025-01-15T12:00:00Z");
    }

    // ── InstanceInfo deserialize ────────────────────────────────────────

    #[test]
    fn test_instance_info_deserialize() {
        let json = json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "My Lobby Server",
            "kind": "lobby",
            "status": "running",
            "host": "server-host-01",
            "mode": "creative",
            "club": "builders",
            "players": 12,
            "max_players": 50,
            "version": "1.21.4",
            "peer_id": "12D3KooWTestPeerId",
            "discovered_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-02T12:00:00Z"
        });
        let info: InstanceInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(info.name, "My Lobby Server");
        assert_eq!(info.kind, "lobby");
        assert_eq!(info.status, "running");
        assert_eq!(info.host, "server-host-01");
        assert_eq!(info.mode, "creative");
        assert_eq!(info.club, "builders");
        assert_eq!(info.players, 12);
        assert_eq!(info.max_players, 50);
        assert_eq!(info.version, "1.21.4");
        assert_eq!(info.peer_id, "12D3KooWTestPeerId");
    }

    // ── LauncherBehaviour struct smoke test ─────────────────────────────

    #[test]
    fn test_launcher_behaviour_creates() {
        // LauncherBehaviour is generated by the NetworkBehaviour derive macro.
        // We verify the type exists and its associated types are correct at
        // compile time.  We cannot construct the behaviour without a full
        // libp2p swarm, so we exercise the From implementations and type
        // signatures instead.

        // Verify event variant constructors compile:
        let _: fn(identify::Event) -> LauncherEvent = |e| LauncherEvent::Identify(Box::new(e));
        let _: fn(kad::Event) -> LauncherEvent = |e| LauncherEvent::Kademlia(Box::new(e));
        let _: fn() -> LauncherEvent = || LauncherEvent::RelayClient(());
        let _: fn() -> LauncherEvent = || LauncherEvent::Dcutr(());
        let _: fn(gossipsub::Event) -> LauncherEvent = |e| LauncherEvent::Gossipsub(Box::new(e));
        let _: fn() -> LauncherEvent = || LauncherEvent::Stream(());

        // Verify the LauncherBehaviour type is usable:
        let _ = |b: &LauncherBehaviour| {
            let _: &identify::Behaviour = &b.identify;
            let _: &kad::Behaviour<MemoryStore> = &b.kademlia;
            let _: &relay::client::Behaviour = &b.relay_client;
            let _: &dcutr::Behaviour = &b.dcutr;
            let _: &gossipsub::Behaviour = &b.gossipsub;
            let _: &libp2p_stream::Behaviour = &b.stream;
        };
    }
}
