use crate::error::LauncherError;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use libp2p::{identify, kad, swarm::SwarmEvent, Multiaddr, PeerId, Swarm};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

use super::dht::{dht_instance_key, parse_instance_record_proto};
use super::swarm::{LauncherBehaviour, LauncherEvent};
use super::{NetworkCommand, NetworkDiagnostics, PeerLatency, ResolvedInstance};

pub(super) async fn run_network(
    mut swarm: Swarm<LauncherBehaviour>,
    mut cmd_rx: mpsc::Receiver<NetworkCommand>,
    bootstrap_peers: Vec<String>,
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
                        let key = kad::RecordKey::new(&dht_instance_key(&instance_id));
                        let query_id = swarm.behaviour_mut().kademlia.get_record(key);
                        pending_queries.insert(query_id, respond);
                    }
                    NetworkCommand::GetPeers { respond } => {
                        let peers: Vec<String> = swarm.connected_peers().map(|p| p.to_string()).collect();
                        if respond.send(peers).is_err() {
                            warn!("GetPeers: requester dropped before response");
                        }
                    }
                    NetworkCommand::OpenStream { peer_id, protocol, respond } => {
                        if let Some(ref mut control) = stream_control {
                            let mut control = control.clone();
                            tokio::spawn(async move {
                                let result = control.open_stream(peer_id, protocol).await
                                    .map_err(|e| crate::error::LauncherError::from(e.to_string()));
                                if respond.send(result).is_err() {
                                    warn!("OpenStream: requester dropped before response");
                                }
                            });
                        } else {
                            if respond.send(Err(LauncherError::new("ERROR", "stream control not available"))).is_err() {
                                warn!("OpenStream: requester dropped before error response");
                            }
                        }
                    }
                    NetworkCommand::MeasureLatency { peer_id, respond } => {
                        match peer_id.parse::<PeerId>() {
                            Ok(target) => {
                                let cache_ttl = Duration::from_secs(300);
                                if let Some((ms, at)) = latency_cache.get(&target) {
                                    if at.elapsed() < cache_ttl {
                                        if respond.send(Some(*ms)).is_err() {
                                            warn!(%target, "latency: requester dropped (cached)");
                                        }
                                        continue;
                                    }
                                }
                                if pending_latency.contains_key(&target) {
                                    if respond.send(None).is_err() {
                                        warn!(%target, "latency: requester dropped (pending)");
                                    }
                                    continue;
                                }
                                if swarm.is_connected(&target) {
                                    if respond.send(latency_cache.get(&target).map(|(ms, _)| *ms)).is_err() {
                                        warn!(%target, "latency: requester dropped (connected)");
                                    }
                                    continue;
                                }
                                if let Err(e) = swarm.dial(target) {
                                    warn!(%target, error = %e, "latency measurement dial failed");
                                    if respond.send(None).is_err() {
                                        warn!(%target, "latency: requester dropped (dial error)");
                                    }
                                } else {
                                    pending_latency.insert(target, (Instant::now(), respond));
                                }
                            }
                            Err(e) => {
                                warn!(%peer_id, error = %e, "invalid peer id for latency measurement");
                                if respond.send(None).is_err() {
                                    warn!(%peer_id, "latency: requester dropped (invalid peer id)");
                                }
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
                        let connected = swarm.connected_peers().count() as u32;
                        if respond.send(NetworkDiagnostics {
                            connected_peers: connected,
                            dht_peers: connected,
                            relay_connected,
                            dcutr_holes_punched,
                            dcutr_failures,
                            latencies,
                            active_sessions: 0,
                            total_bytes_rx: 0,
                            total_bytes_tx: 0,
                            bootstrap_peer_count: bootstrap_peers.len() as u32,
                            bootstrap_reachable: connected > 0,
                        }).is_err() {
                            warn!("GetDiagnostics: requester dropped before response");
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
                            if respond.send(Some(rtt_ms)).is_err() {
                                warn!(%peer_id, "latency: requester dropped after connection established");
                            }
                        }
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id: Some(peer_id), error, .. } => {
                        warn!(%peer_id, error = %error, "outgoing connection failed");
                        dcutr_failures += 1;
                        if let Some((_, respond)) = pending_latency.remove(&peer_id) {
                            if respond.send(None).is_err() {
                                warn!(%peer_id, "latency: requester dropped after connection error");
                            }
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
                                    if respond.send(parse_instance_record_proto(&record.record.value)).is_err() {
                                        warn!("resolve instance: requester dropped (found record)");
                                    }
                                }
                            }
                            kad::Event::OutboundQueryProgressed {
                                id,
                                result: kad::QueryResult::GetRecord(Err(_)),
                                ..
                            } => {
                                if let Some(respond) = pending_queries.remove(&id) {
                                    if respond.send(None).is_err() {
                                        warn!("resolve instance: requester dropped (record not found)");
                                    }
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
                    _ => {}
                }
            }
        }
    }
}

fn split_peer_addr(addr: Multiaddr) -> Option<(PeerId, Multiaddr)> {
    let mut protocols: Vec<_> = addr.into_iter().collect();
    let last = protocols.pop()?;
    match last {
        libp2p::multiaddr::Protocol::P2p(peer_id) => {
            Some((peer_id, protocols.into_iter().collect()))
        }
        _ => None,
    }
}
