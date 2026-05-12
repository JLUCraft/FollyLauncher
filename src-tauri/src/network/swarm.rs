use anyhow::Context;
use libp2p::{
    dcutr, identify,
    kad::{self, store::MemoryStore, Mode},
    noise, relay,
    swarm::NetworkBehaviour,
    yamux, StreamProtocol, Swarm, SwarmBuilder,
};
use std::time::Duration;

const MC_DHT_PROTOCOL: &str = "/jlucraft/kad/1.0.0";
const MC_IDENTIFY_PROTOCOL: &str = "/jlucraft/identify/1.0.0";

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "LauncherEvent")]
pub struct LauncherBehaviour {
    pub identify: identify::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub relay_client: relay::client::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub stream: libp2p_stream::Behaviour,
}

#[derive(Debug)]
pub enum LauncherEvent {
    Identify(Box<identify::Event>),
    Kademlia(Box<kad::Event>),
    RelayClient(()),
    Dcutr(()),
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
impl From<()> for LauncherEvent {
    fn from(_: ()) -> Self {
        Self::Stream(())
    }
}

pub(super) fn build_swarm(
    keypair: libp2p::identity::Keypair,
    stream_control: std::sync::Arc<std::sync::Mutex<Option<libp2p_stream::Control>>>,
) -> anyhow::Result<Swarm<LauncherBehaviour>> {
    let dht_protocol = Box::leak(MC_DHT_PROTOCOL.to_string().into_boxed_str());

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
            *stream_control.lock().unwrap_or_else(|e| e.into_inner()) =
                Some(stream_behaviour.new_control());

            LauncherBehaviour {
                identify,
                kademlia,
                relay_client,
                dcutr: dcutr::Behaviour::new(local_peer_id),
                stream: stream_behaviour,
            }
        })?
        .build();

    Ok(swarm)
}
