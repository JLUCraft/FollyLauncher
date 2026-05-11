use crate::error::LauncherError;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use libp2p::{gossipsub, identify, kad, swarm::SwarmEvent, Multiaddr, PeerId, Swarm};
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, info, warn};

use crate::protos::jlucraft::events::v1::{event_envelope, EventEnvelope};

use super::dht::{dht_instance_key, parse_instance_record_proto};
use super::swarm::{LauncherBehaviour, LauncherEvent};
use super::{
    ClusterMessage, InstanceInfo, NetworkCommand, NetworkDiagnostics, PeerLatency,
    ResolvedInstance, MC_CLUSTER_TOPIC,
};

pub(super) async fn run_network(
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
                                    .map_err(|e| crate::error::LauncherError::from(e.to_string()));
                                let _ = respond.send(result);
                            });
                        } else {
                            let _ = respond.send(Err(LauncherError::new("ERROR", "stream control not available")));
                        }
                    }
                    NetworkCommand::GetInstances { respond } => {
                        let instances: Vec<InstanceInfo> = known_instances.read().await.values().cloned().collect();
                        let _ = respond.send(instances);
                    }
                    NetworkCommand::MeasureLatency { peer_id, respond } => {
                        match peer_id.parse::<PeerId>() {
                            Ok(target) => {
                                let cache_ttl = Duration::from_secs(300);
                                if let Some((ms, at)) = latency_cache.get(&target) {
                                    if at.elapsed() < cache_ttl {
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
                                    pending_latency.insert(target, (Instant::now(), respond));
                                }
                            }
                            Err(e) => {
                                warn!(%peer_id, error = %e, "invalid peer id for latency measurement");
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
                        let connected = swarm.connected_peers().count() as u32;
                        let _ = respond.send(NetworkDiagnostics {
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
                        });
                    }
                    NetworkCommand::SubscribeTopic { topic, respond } => {
                        let ident = gossipsub::IdentTopic::new(topic);
                        match swarm.behaviour_mut().gossipsub.subscribe(&ident) {
                            Ok(true) => {
                                info!(topic = %ident, "subscribed to dynamic topic");
                                let _ = respond.send(Ok(()));
                            }
                            Ok(false) => {
                                debug!(topic = %ident, "already subscribed to topic");
                                let _ = respond.send(Ok(()));
                            }
                            Err(e) => {
                                warn!(topic = %ident, error = %e, "failed to subscribe to topic");
                                let _ = respond.send(Err(LauncherError::new("ERROR", format!("subscribe failed: {e}"))));
                            }
                        }
                    }
                    NetworkCommand::UnsubscribeTopic { topic, respond } => {
                        let ident = gossipsub::IdentTopic::new(topic);
                        let subscribed = swarm.behaviour_mut().gossipsub.unsubscribe(&ident);
                        if subscribed {
                            info!(topic = %ident, "unsubscribed from topic");
                        } else {
                            debug!(topic = %ident, "not subscribed to topic");
                        }
                        let _ = respond.send(Ok(()));
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
                                    let _ = respond.send(parse_instance_record_proto(&record.record.value));
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
                        if let gossipsub::Event::Message { propagation_source: peer_id, message, .. } = *event {
                            let topic = message.topic.to_string();
                            use prost::Message as _;
                            let Some(event) = EventEnvelope::decode(message.data.as_slice()).ok() else {
                                warn!(topic = %topic, peer_id = %peer_id, "discarding non-protobuf GossipSub message");
                                continue;
                            };
                            info!(peer_id = %peer_id, topic = %topic, "received gossipsub message");

                            if topic == MC_CLUSTER_TOPIC && event.event_type.starts_with("instance-") {
                                update_known_instances(&known_instances, &event, &peer_id.to_string()).await;
                            }

                            let mut msgs = messages.write().await;
                            msgs.push_back(ClusterMessage {
                                topic,
                                peer_id: peer_id.to_string(),
                                payload: event_to_payload(&event),
                                received_at: chrono::Utc::now().to_rfc3339(),
                            });
                            if msgs.len() > 256 {
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
    event: &EventEnvelope,
    peer_id: &str,
) {
    let Some(event_envelope::Payload::InstanceUpdate(instance)) = event.payload.as_ref() else {
        return;
    };
    let id = &instance.instance_id;
    let now = chrono::Utc::now().to_rfc3339();

    match event.event_type.as_str() {
        "instance-created" => {
            let info = InstanceInfo {
                id: id.clone(),
                name: coalesce(&instance.name, id),
                kind: coalesce(&instance.kind, "room"),
                status: "created".to_string(),
                host: instance.host_peer_id.clone(),
                mode: coalesce(&instance.mode, "unknown"),
                club: instance.club.clone(),
                players: instance.player_count,
                max_players: instance.max_players,
                version: instance.version.clone(),
                peer_id: peer_id.to_string(),
                discovered_at: now.clone(),
                updated_at: now,
            };
            known_instances.write().await.insert(id.clone(), info);
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
    let mut protocols: Vec<_> = addr.into_iter().collect();
    let last = protocols.pop()?;
    match last {
        libp2p::multiaddr::Protocol::P2p(peer_id) => {
            Some((peer_id, protocols.into_iter().collect()))
        }
        _ => None,
    }
}

fn event_to_payload(event: &EventEnvelope) -> serde_json::Value {
    let mut val = serde_json::json!({ "event_type": event.event_type });
    if let Some(event_envelope::Payload::InstanceUpdate(i)) = event.payload.as_ref() {
        val["payload"] = serde_json::json!({
            "instance_id": i.instance_id,
            "name": i.name,
            "kind": i.kind,
            "host": i.host_peer_id,
            "mode": i.mode,
            "club": i.club,
            "players": i.player_count,
            "max_players": i.max_players,
            "version": i.version,
        });
    }
    val
}

/// Returns `value` if non-empty, otherwise `default`.
fn coalesce(value: &str, default: &str) -> String {
    if value.is_empty() { default } else { value }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protos::jlucraft::events::v1::{event_envelope, InstanceUpdateEvent};
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    fn make_cluster_event(
        event_type: &str,
        instance_id: &str,
        name: &str,
        kind: &str,
        host: &str,
        mode: &str,
        club: &str,
        players: u32,
        max_players: u32,
        version: &str,
    ) -> EventEnvelope {
        EventEnvelope {
            event_type: event_type.to_string(),
            payload: Some(event_envelope::Payload::InstanceUpdate(
                InstanceUpdateEvent {
                    instance_id: instance_id.to_string(),
                    name: name.to_string(),
                    kind: kind.to_string(),
                    host_peer_id: host.to_string(),
                    mode: mode.to_string(),
                    club: club.to_string(),
                    player_count: players,
                    max_players,
                    version: version.to_string(),
                    status: String::new(),
                    last_snapshot_key: String::new(),
                    migration_target: String::new(),
                },
            )),
            ..Default::default()
        }
    }

    fn make_minimal_event(event_type: &str, instance_id: &str) -> EventEnvelope {
        EventEnvelope {
            event_type: event_type.to_string(),
            payload: Some(event_envelope::Payload::InstanceUpdate(
                InstanceUpdateEvent {
                    instance_id: instance_id.to_string(),
                    ..Default::default()
                },
            )),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_update_known_instances_missing_fields() {
        let known = Arc::new(RwLock::new(HashMap::new()));
        let id = "bada550e-e29b-41d4-a716-446655440000";
        let event = make_cluster_event(
            "instance-created",
            id,
            "minimal-instance",
            "",
            "",
            "",
            "",
            0,
            0,
            "",
        );

        update_known_instances(&known, &event, "peer").await;

        let stored = known.read().await;
        let info = stored.get(id).unwrap();
        assert_eq!(info.name, "minimal-instance");
        assert_eq!(info.kind, "room");
        assert_eq!(info.mode, "unknown");
        assert_eq!(info.status, "created");
    }

    #[tokio::test]
    async fn test_update_known_instances_full_payload() {
        let known = Arc::new(RwLock::new(HashMap::new()));
        let id = "cafebabe-e29b-41d4-a716-446655440000";
        let event = make_cluster_event(
            "instance-created",
            id,
            "full-instance",
            "service",
            "node-01",
            "vc-only",
            "builders",
            8,
            32,
            "1.21.4",
        );

        update_known_instances(&known, &event, "peer-full").await;

        let info = known.read().await;
        let info = info.get(id).unwrap();
        assert_eq!(info.kind, "service");
        assert_eq!(info.mode, "vc-only");
        assert_eq!(info.players, 8);
        assert_eq!(info.max_players, 32);
    }

    #[tokio::test]
    async fn test_update_known_instances_status_transitions() {
        let known = Arc::new(RwLock::new(HashMap::new()));
        let id = "status-test-e29b-41d4-a716-446655440000";

        update_known_instances(
            &known,
            &make_cluster_event(
                "instance-created",
                id,
                "t",
                "room",
                "h",
                "public",
                "c",
                1,
                10,
                "1.21",
            ),
            "peer",
        )
        .await;
        assert_eq!(known.read().await.get(id).unwrap().status, "created");

        update_known_instances(&known, &make_minimal_event("instance-started", id), "peer").await;
        assert_eq!(known.read().await.get(id).unwrap().status, "running");

        update_known_instances(&known, &make_minimal_event("instance-stopped", id), "peer").await;
        assert_eq!(known.read().await.get(id).unwrap().status, "stopped");

        update_known_instances(
            &known,
            &make_minimal_event("instance-destroyed", id),
            "peer",
        )
        .await;
        assert!(known.read().await.get(id).is_none());
    }
}
