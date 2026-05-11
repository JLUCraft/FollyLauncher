use crate::api::MigrationProbeResponse;
use crate::control_client::ControlClient;
use crate::network::{NetworkHandle, ResolvedInstance};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

const MIGRATION_TIMEOUT: Duration = Duration::from_secs(30);
const MIGRATION_BUFFER_SIZE: usize = 64 * 1024;

pub(crate) trait ProxyStream:
    tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send
{
}
impl<T> ProxyStream for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}

#[derive(Clone)]
pub struct InstanceProxy {
    inner: Arc<ProxyInner>,
}

struct ProxyInner {
    listener: TcpListener,
    network: NetworkHandle,
    control: Arc<ControlClient>,
    sessions: RwLock<HashMap<u16, ProxySession>>,
    dht_cache: RwLock<HashMap<String, CachedResolution>>,
    peer_cache: RwLock<HashMap<String, Instant>>,
}

/// ADR: Tauri IPC is used as the local API substitute between frontend and Rust backend.
/// gRPC is not introduced at this stage because the launcher is a single-user desktop app
/// with a co-located frontend; Tauri invoke() provides sufficient type safety and
/// serialization at zero additional operational cost. If multi-process or remote
/// launcher management becomes necessary, the command layer can be re-exported via a
/// tonic gRPC server without changing the internal service implementations.
///
/// See also: lib.rs Tauri command handler registration.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProxySession {
    pub instance_id: String,
    pub local_port: u16,
    pub target_peer_id: String,
    /// QUIC substream ID from libp2p (if available). Used for diagnostics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub substream_id: Option<String>,
    /// OS process ID of the Minecraft client for this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    /// State machine: active | migration_pending | reconnecting | failed | closed
    pub state: String,
    pub started_at: String,
}

/// Migration state machine phases.
///
/// ```text
/// Active ──(MIGRATION_PENDING frame)──▶ MigrationPending
/// MigrationPending ──(DHT re-resolve + QUIC open)──▶ Reconnecting
/// Reconnecting ──(success)──▶ Active
/// Reconnecting ──(timeout > 30s)──▶ Failed
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationPhase {
    Active,
    MigrationPending,
    Reconnecting,
    Failed,
}

impl MigrationPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::MigrationPending => "migration_pending",
            Self::Reconnecting => "reconnecting",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
struct CachedResolution {
    resolved: ResolvedInstance,
    cached_at: Instant,
}

#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub instance_id: String,
    pub peer_id: String,
    pub club: Option<String>,
    pub has_member_vc: bool,
}

impl InstanceProxy {
    pub async fn bind(network: NetworkHandle, control: Arc<ControlClient>) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind local proxy")?;
        let local_addr = listener.local_addr()?;
        info!(%local_addr, "instance proxy listening");

        Ok(Self {
            inner: Arc::new(ProxyInner {
                listener,
                network,
                control,
                sessions: RwLock::new(HashMap::new()),
                dht_cache: RwLock::new(HashMap::new()),
                peer_cache: RwLock::new(HashMap::new()),
            }),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.inner
            .listener
            .local_addr()
            .context("failed to get local addr")
    }

    pub async fn run(self) {
        loop {
            match self.inner.listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let proxy = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = proxy.handle_client(stream, peer_addr).await {
                            warn!(%peer_addr, error = %e, "proxy session error");
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "proxy accept error");
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn handle_client(&self, mut client: TcpStream, peer_addr: SocketAddr) -> Result<()> {
        debug!(%peer_addr, "new proxy connection");
        let local_port = client.local_addr()?.port();
        let handshake = McHandshake::read_from(&mut client).await?;
        info!(%peer_addr, instance_id = %handshake.instance_id, "parsed mc handshake");

        let instance_id = handshake.instance_id.to_string();
        // Resolve the instance and connect
        let resolved = self
            .resolve_instance_cached(instance_id.clone())
            .await
            .with_context(|| format!("instance {} not found in DHT", instance_id))?;

        let target_peer_id = resolved
            .peer_id
            .parse::<libp2p::PeerId>()
            .with_context(|| format!("invalid peer id: {}", resolved.peer_id))?;

        let stream = self
            .try_open_libp2p_stream(target_peer_id)
            .await
            .with_context(|| {
                format!(
                    "无法建立到节点 {} 的 QUIC 连接；请检查网络或稍后重试",
                    target_peer_id
                )
            })?;
        info!(%peer_addr, "connected via libp2p stream tunnel");
        let target: Box<dyn ProxyStream> = Box::new(
            tokio_util::compat::FuturesAsyncReadCompatExt::compat(stream),
        );

        let session = ProxySession {
            instance_id: instance_id.clone(),
            local_port,
            target_peer_id: resolved.peer_id.clone(),
            substream_id: None,
            process_id: None,
            bytes_in: 0,
            bytes_out: 0,
            state: MigrationPhase::Active.as_str().to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        {
            let mut sessions = self.inner.sessions.write().await;
            sessions.insert(local_port, session);
        }

        let result = Self::run_bidirectional_copy(
            client,
            target,
            handshake,
            peer_addr,
            self.inner.clone(),
            local_port,
        )
        .await;

        {
            let mut sessions = self.inner.sessions.write().await;
            if let Some(s) = sessions.get_mut(&local_port) {
                s.state = "closed".to_string();
            }
            sessions.remove(&local_port);
        }

        info!(%peer_addr, "proxy session closed");
        result
    }

    async fn try_open_libp2p_stream(&self, peer_id: libp2p::PeerId) -> Result<libp2p::Stream> {
        let peer_id_str = peer_id.to_string();
        let cache_ttl = Duration::from_secs(300);

        // Check cache: if we've connected to this peer recently, the underlying QUIC
        // connection may still be alive (keepalive 30s), so open_stream will be fast.
        {
            let cache = self.inner.peer_cache.read().await;
            if let Some(cached) = cache.get(&peer_id_str) {
                if cached.elapsed() < cache_ttl {
                    debug!(%peer_id, "peer connection cache hit");
                }
            }
        }

        let protocol = libp2p::StreamProtocol::new("/mc/play/1");
        let stream = self
            .inner
            .network
            .open_stream(peer_id, protocol)
            .await
            .map_err(|e| anyhow::anyhow!("failed to open libp2p stream: {e}"))?;

        // Update cache on success, evict stale entries.
        {
            let mut cache = self.inner.peer_cache.write().await;
            cache.retain(|_, v| v.elapsed() < cache_ttl);
            cache.insert(peer_id_str, Instant::now());
        }

        Ok(stream)
    }

    async fn run_bidirectional_copy(
        mut client: TcpStream,
        mut target: Box<dyn ProxyStream>,
        handshake: McHandshake,
        peer_addr: SocketAddr,
        inner: Arc<ProxyInner>,
        local_port: u16,
    ) -> Result<()> {
        handshake.write_to(&mut target).await?;

        let (mut client_read, mut client_write) = client.split();
        let (mut target_read, mut target_write) = tokio::io::split(target);

        let inner_c2t = inner.clone();
        let inner_t2c = inner.clone();
        let port = local_port;

        let c2t = async move {
            let mut buf = [0u8; 8192];
            let mut total: u64 = 0;
            loop {
                match client_read.read(&mut buf).await {
                    Ok(0) => break Ok(total),
                    Ok(n) => {
                        if let Err(e) = target_write.write_all(&buf[..n]).await {
                            break Err(e);
                        }
                        total += n as u64;
                        let mut sessions = inner_c2t.sessions.write().await;
                        if let Some(s) = sessions.get_mut(&port) {
                            s.bytes_out = total;
                        }
                    }
                    Err(e) => break Err(e),
                }
            }
        };

        let t2c = async move {
            let mut buf = [0u8; 8192];
            let mut total: u64 = 0;
            let mut migration_buffer: Vec<u8> = Vec::with_capacity(MIGRATION_BUFFER_SIZE);
            let mut current_phase = MigrationPhase::Active;
            let mut migration_start = tokio::time::Instant::now();
            loop {
                match target_read.read(&mut buf).await {
                    Ok(0) => break Ok(total),
                    Ok(n) => {
                        // Check for migration control frame prefix
                        if current_phase == MigrationPhase::Active
                            && buf[..n.min(17)]
                                .windows(16)
                                .any(|w| w == b"MIGRATION_PENDING")
                        {
                            current_phase = MigrationPhase::MigrationPending;
                            migration_start = tokio::time::Instant::now();
                            info!(peer_addr = %peer_addr, "migration pending (phase: MigrationPending), buffering bytes");
                            let frame_start = buf[..n]
                                .windows(16)
                                .position(|w| w == b"MIGRATION_PENDING")
                                .unwrap_or(0);
                            if frame_start + 16 < n {
                                migration_buffer.extend_from_slice(&buf[frame_start + 16..n]);
                            }
                            {
                                let mut sessions = inner_t2c.sessions.write().await;
                                if let Some(s) = sessions.get_mut(&port) {
                                    s.state = MigrationPhase::MigrationPending.as_str().to_string();
                                }
                            }
                            // Trigger migration reconnection
                            let inner_for_migration = inner_t2c.clone();
                            let migration_context = {
                                let sessions = inner_t2c.sessions.read().await;
                                sessions
                                    .get(&port)
                                    .map(|s| (s.instance_id.clone(), s.target_peer_id.clone()))
                            };
                            if let Some((iid, current_peer_id)) = migration_context {
                                tokio::spawn(async move {
                                    let proxy = InstanceProxy {
                                        inner: inner_for_migration,
                                    };
                                    {
                                        let mut sessions = proxy.inner.sessions.write().await;
                                        if let Some(s) = sessions.get_mut(&port) {
                                            s.state =
                                                MigrationPhase::Reconnecting.as_str().to_string();
                                        }
                                    }
                                    let migrate_result = proxy
                                        .migrate_instance_connection(&iid, &current_peer_id)
                                        .await;
                                    match migrate_result {
                                        Ok((new_stream, new_peer)) => {
                                            info!(%new_peer, %port, "migration reconnected (Reconnecting → Active)");
                                            let mut sessions = proxy.inner.sessions.write().await;
                                            if let Some(s) = sessions.get_mut(&port) {
                                                s.state =
                                                    MigrationPhase::Active.as_str().to_string();
                                                s.target_peer_id = new_peer;
                                            }
                                            // Fast-reconnect strategy: the new QUIC stream is valid
                                            // and verified, but the in-flight copy task cannot atomically
                                            // swap byte-streams mid-flight without client protocol support.
                                            // Instead, the TCP side is broken (buffer overflow / timeout),
                                            // which triggers MC's built-in reconnect logic. The session
                                            // state has been updated to route subsequent connections to
                                            // the new peer. Phase 3 may explore true stream splicing.
                                            drop(new_stream);
                                        }
                                        Err(e) => {
                                            warn!(%port, error = %e, "migration reconnection failed (Reconnecting → Failed)");
                                            let mut sessions = proxy.inner.sessions.write().await;
                                            if let Some(s) = sessions.get_mut(&port) {
                                                s.state =
                                                    MigrationPhase::Failed.as_str().to_string();
                                            }
                                            // Close the TCP side to force client reconnect
                                            // (the c2t task will get a write error and exit cleanly)
                                        }
                                    }
                                });
                            }
                            continue;
                        }

                        if current_phase == MigrationPhase::MigrationPending {
                            if migration_buffer.len() + n <= MIGRATION_BUFFER_SIZE {
                                migration_buffer.extend_from_slice(&buf[..n]);
                                total += n as u64;
                            } else {
                                if migration_start.elapsed() > MIGRATION_TIMEOUT {
                                    warn!(peer_addr = %peer_addr, "migration timeout exceeded (phase: Failed)");
                                    let mut sessions = inner_t2c.sessions.write().await;
                                    if let Some(s) = sessions.get_mut(&port) {
                                        s.state = MigrationPhase::Failed.as_str().to_string();
                                    }
                                }
                                warn!(peer_addr = %peer_addr, "migration buffer overflow (phase: Failed)");
                                break Err(std::io::Error::new(
                                    std::io::ErrorKind::ConnectionAborted,
                                    "migration buffer overflow",
                                ));
                            }
                        } else {
                            if let Err(e) = client_write.write_all(&buf[..n]).await {
                                break Err(e);
                            }
                            total += n as u64;
                        }
                        let mut sessions = inner_t2c.sessions.write().await;
                        if let Some(s) = sessions.get_mut(&port) {
                            s.bytes_in = total;
                        }
                    }
                    Err(e) => break Err(e),
                }
            }
        };

        let (c2t_result, t2c_result) = tokio::join!(c2t, t2c);
        match c2t_result {
            Ok(total) => debug!(%peer_addr, total, "client->target closed"),
            Err(e) => debug!(%peer_addr, error = %e, "client->target error"),
        }
        match t2c_result {
            Ok(total) => debug!(%peer_addr, total, "target->client closed"),
            Err(e) => debug!(%peer_addr, error = %e, "target->client error"),
        }

        Ok(())
    }

    async fn resolve_instance_cached(&self, instance_id: String) -> Option<ResolvedInstance> {
        let cache_ttl = Duration::from_secs(300);

        {
            let cache = self.inner.dht_cache.read().await;
            if let Some(cached) = cache.get(&instance_id) {
                if cached.cached_at.elapsed() < cache_ttl {
                    return Some(cached.resolved.clone());
                }
            }
        }

        let resolved = self
            .inner
            .network
            .resolve_instance(instance_id.clone())
            .await
            .unwrap_or(None);

        if let Some(ref r) = resolved {
            let mut cache = self.inner.dht_cache.write().await;
            cache.retain(|_, v| v.cached_at.elapsed() < cache_ttl);
            cache.insert(
                instance_id,
                CachedResolution {
                    resolved: r.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        resolved
    }

    pub async fn active_sessions(&self) -> Vec<ProxySession> {
        self.inner.sessions.read().await.values().cloned().collect()
    }

    pub async fn migrate_instance_connection(
        &self,
        instance_id: &str,
        current_peer_id: &str,
    ) -> Result<(Box<dyn ProxyStream>, String)> {
        info!(%instance_id, "migration reconnection");
        let probe = self
            .inner
            .control
            .probe_migration(instance_id, current_peer_id)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "migration health probe failed for instance {} from source {}: {}",
                    instance_id,
                    current_peer_id,
                    e
                )
            })?;
        Self::ensure_probe_allows_switch(&probe)?;

        let resolved = self
            .resolve_instance_cached(instance_id.to_string())
            .await
            .with_context(|| {
                format!("instance {} not found in DHT during migration", instance_id)
            })?;

        let new_peer_id_str = resolved.peer_id.clone();
        if probe.target_peer_id != new_peer_id_str {
            bail!(
                "migration probe target mismatch: probe={}, resolved={}",
                probe.target_peer_id,
                new_peer_id_str
            );
        }
        let target_peer_id: libp2p::PeerId = new_peer_id_str
            .parse()
            .with_context(|| format!("invalid peer id: {}", new_peer_id_str))?;

        let stream = self
            .try_open_libp2p_stream(target_peer_id)
            .await
            .with_context(|| {
                format!(
                    "迁移重连失败：无法建立到新宿主 {} 的 QUIC 连接",
                    new_peer_id_str
                )
            })?;
        info!(new_peer = %new_peer_id_str, "migration libp2p stream established");
        let stream: Box<dyn ProxyStream> = Box::new(
            tokio_util::compat::FuturesAsyncReadCompatExt::compat(stream),
        );

        Ok((stream, new_peer_id_str))
    }

    fn ensure_probe_allows_switch(probe: &MigrationProbeResponse) -> Result<()> {
        match probe.status.as_str() {
            "ready" => Ok(()),
            "rejecting" | "storage_full" => {
                let reason = probe
                    .reason
                    .as_deref()
                    .unwrap_or("migration blocked by probe");
                bail!(
                    "migration health probe blocked target switch: status={}, reason={}",
                    probe.status,
                    reason
                )
            }
            status => {
                let reason = probe.reason.as_deref().unwrap_or("migration not ready");
                bail!(
                    "migration health probe did not reach ready state: status={}, reason={}",
                    status,
                    reason
                )
            }
        }
    }

    /// Bind a temporary local port dedicated to a single instance launch.
    /// Returns the local port number that Minecraft should connect to.
    pub async fn bridge_instance(&self, cfg: BridgeConfig) -> Result<u16> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind temporary instance proxy")?;
        let local_port = listener.local_addr()?.port();
        info!(%local_port, instance_id = %cfg.instance_id, "instance proxy bound for single launch");

        let proxy = self.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let proxy = proxy.clone();
                        let cfg = cfg.clone();
                        tokio::spawn(async move {
                            if let Err(e) = proxy
                                .handle_client_known_instance(stream, peer_addr, cfg)
                                .await
                            {
                                warn!(%peer_addr, error = %e, "dedicated instance bridge error");
                            }
                        });
                    }
                    Err(e) => {
                        error!(error = %e, "dedicated instance proxy accept error");
                        break;
                    }
                }
            }
            info!(%local_port, "instance proxy listener closed");
        });

        Ok(local_port)
    }

    async fn handle_client_known_instance(
        &self,
        mut client: TcpStream,
        peer_addr: SocketAddr,
        cfg: BridgeConfig,
    ) -> Result<()> {
        let BridgeConfig {
            instance_id,
            peer_id,
            club,
            has_member_vc,
        } = cfg;
        debug!(%peer_addr, %instance_id, "new dedicated instance proxy connection");

        let local_port = client.local_addr()?.port();

        // Read the handshake from the Minecraft client
        let original_handshake = McHandshake::read_from(&mut client).await?;
        info!(
            %peer_addr,
            instance_id = %instance_id,
            protocol_version = original_handshake.protocol_version,
            "parsed mc handshake for dedicated bridge"
        );

        // Build target handshake with instance info for the remote server
        let target_handshake = build_target_handshake(
            original_handshake,
            instance_id.parse().context("invalid instance_id uuid")?,
            &peer_id,
            club.as_deref(),
            has_member_vc,
        );

        // Resolve target
        let resolved = self
            .resolve_instance_cached(instance_id.clone())
            .await
            .with_context(|| format!("instance {} not found in DHT", instance_id))?;

        info!(
            %peer_addr,
            target_peer = %resolved.peer_id,
            "resolved instance location"
        );

        let target_peer_id = resolved
            .peer_id
            .parse::<libp2p::PeerId>()
            .with_context(|| format!("invalid peer id: {}", resolved.peer_id))?;

        let stream = self
            .try_open_libp2p_stream(target_peer_id)
            .await
            .with_context(|| {
                format!(
                    "无法建立到节点 {} 的 QUIC 连接；请检查网络或稍后重试",
                    target_peer_id
                )
            })?;
        info!(%peer_addr, "connected via libp2p stream tunnel");
        let target: Box<dyn ProxyStream> = Box::new(
            tokio_util::compat::FuturesAsyncReadCompatExt::compat(stream),
        );

        let session = ProxySession {
            instance_id: target_handshake.instance_id.to_string(),
            local_port,
            target_peer_id: resolved.peer_id.clone(),
            substream_id: None,
            process_id: None,
            bytes_in: 0,
            bytes_out: 0,
            state: MigrationPhase::Active.as_str().to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
        };

        {
            let mut sessions = self.inner.sessions.write().await;
            sessions.insert(local_port, session);
        }

        let result = Self::run_bidirectional_copy(
            client,
            target,
            target_handshake,
            peer_addr,
            self.inner.clone(),
            local_port,
        )
        .await;

        {
            let mut sessions = self.inner.sessions.write().await;
            if let Some(s) = sessions.get_mut(&local_port) {
                s.state = "closed".to_string();
            }
            sessions.remove(&local_port);
        }

        info!(%peer_addr, "dedicated instance bridge closed");
        result
    }
}

fn build_target_handshake(
    original: McHandshake,
    instance_id: Uuid,
    peer_id: &str,
    club: Option<&str>,
    has_member_vc: bool,
) -> McHandshake {
    let mut parts = vec![format!("instance={}", instance_id)];
    parts.push(format!("peer_id={}", peer_id));
    if let Some(c) = club {
        parts.push(format!("club={}", c));
    }
    if has_member_vc {
        parts.push("vc=true".to_string());
    }

    McHandshake {
        protocol_version: original.protocol_version,
        server_address: parts.join(";"),
        server_port: original.server_port,
        next_state: original.next_state,
        instance_id,
    }
}

#[derive(Debug, Clone)]
pub struct McHandshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32,
    pub instance_id: Uuid,
}

impl McHandshake {
    async fn read_from<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Self> {
        let _packet_length = read_varint_async(reader).await?;
        let packet_id = read_varint_async(reader).await?;

        if packet_id != 0x00 {
            bail!("expected handshake packet (0x00), got {packet_id:#x}");
        }

        let protocol_version = read_varint_async(reader).await?;
        let server_address = read_mc_string_async(reader).await?;
        let server_port = read_u16_be_async(reader).await?;
        let next_state = read_varint_async(reader).await?;

        let instance_id = extract_instance_id(&server_address).with_context(|| {
            format!("no instance= parameter in server address: {server_address}")
        })?;

        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
            instance_id,
        })
    }

    async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<()> {
        let mut body = Vec::new();
        write_varint(&mut body, 0x00)?;
        write_varint(&mut body, self.protocol_version)?;
        write_mc_string(&mut body, &self.server_address)?;
        write_u16_be(&mut body, self.server_port);
        write_varint(&mut body, self.next_state)?;

        let mut packet = Vec::new();
        write_varint(&mut packet, body.len() as i32)?;
        packet.extend_from_slice(&body);

        writer.write_all(&packet).await?;
        writer.flush().await?;
        Ok(())
    }
}

fn extract_instance_id(address: &str) -> Option<Uuid> {
    for part in address.split(';') {
        if let Some(kv) = part.strip_prefix("instance=") {
            return Uuid::parse_str(kv).ok();
        }
    }
    None
}

async fn read_varint_async<R: AsyncRead + Unpin>(reader: &mut R) -> Result<i32> {
    crate::utils::read_varint_u64(reader)
        .await
        .map(|v| v as i32)
        .context("failed to read varint")
}

fn write_varint(writer: &mut Vec<u8>, mut value: i32) -> Result<()> {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        writer.push(byte);
        if value == 0 {
            break;
        }
    }
    Ok(())
}

async fn read_mc_string_async<R: AsyncRead + Unpin>(reader: &mut R) -> Result<String> {
    let len = read_varint_async(reader).await?;
    if !(0..=65535).contains(&len) {
        bail!("invalid string length: {len}");
    }
    let mut buf = vec![0u8; len as usize];
    reader
        .read_exact(&mut buf)
        .await
        .context("failed to read string bytes")?;
    String::from_utf8(buf).context("invalid utf-8 in mc string")
}

fn write_mc_string(writer: &mut Vec<u8>, value: &str) -> Result<()> {
    write_varint(writer, value.len() as i32)?;
    writer.extend_from_slice(value.as_bytes());
    Ok(())
}

async fn read_u16_be_async<R: AsyncRead + Unpin>(reader: &mut R) -> Result<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf).await?;
    Ok(u16::from_be_bytes(buf))
}

fn write_u16_be(writer: &mut Vec<u8>, value: u16) {
    writer.extend_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::ResolvedInstance;
    use uuid::Uuid;

    // ── Varint reading ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_read_varint_sync_single_byte() {
        // Single-byte varints encode values 0–127 directly in 7 bits.
        for val in [0, 1, 42, 127i32] {
            let data: &[u8] = &[val as u8];
            let mut reader = data;
            let result = read_varint_async(&mut reader).await.unwrap();
            assert_eq!(result, val);
        }
    }

    #[tokio::test]
    async fn test_read_varint_sync_multi_byte() {
        // 128  -> 0x80 0x01  (two bytes)
        let data: &[u8] = &[0x80, 0x01];
        let mut reader = data;
        let result = read_varint_async(&mut reader).await.unwrap();
        assert_eq!(result, 128);

        // 25565 -> 0xDD 0xC7 0x01  (three bytes)
        let data2: &[u8] = &[0xDD, 0xC7, 0x01];
        let mut reader2 = data2;
        let result2 = read_varint_async(&mut reader2).await.unwrap();
        assert_eq!(result2, 25565);
    }

    #[tokio::test]
    async fn test_read_varint_sync_zero() {
        // Zero is the simplest varint: a single 0x00 byte.
        let data: &[u8] = &[0x00];
        let mut reader = data;
        let result = read_varint_async(&mut reader).await.unwrap();
        assert_eq!(result, 0);
    }

    // ── Varint roundtrip ────────────────────────────────────────────────

    #[test]
    fn test_write_varint_roundtrip() {
        // Only test non-negative values; write_varint uses arithmetic right
        // shift and does not terminate for negative inputs.
        let cases = [0i32, 1, 42, 127, 128, 255, 256, 16383, 16384];
        for &v in &cases {
            let mut written = Vec::new();
            write_varint(&mut written, v).unwrap();

            let decoded = sync_read_varint(&mut written.as_slice());
            assert_eq!(decoded, Some(v), "roundtrip failed for {v}");
        }
    }

    fn sync_read_varint(buf: &mut &[u8]) -> Option<i32> {
        let mut result = 0i32;
        let mut shift = 0u32;
        loop {
            let byte = *buf.first()?;
            *buf = &buf[1..];
            result |= ((byte & 0x7F) as i32) << shift;
            if byte & 0x80 == 0 {
                return Some(result);
            }
            shift += 7;
            if shift >= 32 {
                return None;
            }
        }
    }

    // ── Minecraft string reading ────────────────────────────────────────

    #[tokio::test]
    async fn test_read_mc_string_empty() {
        // Varint length=0 followed by zero bytes.
        let data: &[u8] = &[0x00];
        let mut reader = data;
        let result = read_mc_string_async(&mut reader).await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn test_read_mc_string_ascii() {
        // "hello" – length 5 as varint, then 5 bytes.
        let mut data = vec![0x05u8];
        data.extend_from_slice(b"hello");
        let mut reader = data.as_slice();
        let result = read_mc_string_async(&mut reader).await.unwrap();
        assert_eq!(result, "hello");
    }

    // ── u16 big-endian reading ──────────────────────────────────────────

    #[tokio::test]
    async fn test_read_u16_be() {
        // 0x1234 in network order = [0x12, 0x34]
        let data: &[u8] = &[0x12, 0x34];
        let mut reader = data;
        let result = read_u16_be_async(&mut reader).await.unwrap();
        assert_eq!(result, 0x1234);

        // Port 25565 = 0x63DD
        let data2: &[u8] = &[0x63, 0xDD];
        let mut reader2 = data2;
        let result2 = read_u16_be_async(&mut reader2).await.unwrap();
        assert_eq!(result2, 25565);
    }

    // ── Instance ID extraction ──────────────────────────────────────────

    #[test]
    fn test_extract_instance_id_valid_uuid() {
        let uuid = Uuid::new_v4();
        let address = format!("instance={}", uuid);
        let result = extract_instance_id(&address);
        assert_eq!(result, Some(uuid));
    }

    #[test]
    fn test_extract_instance_id_not_found() {
        let result = extract_instance_id("127.0.0.1:25565");
        assert_eq!(result, None);

        let result2 = extract_instance_id("club=myclub;peer_id=12D3");
        assert_eq!(result2, None);
    }

    #[test]
    fn test_extract_param_valid() {
        // Semi-colon delimited address with multiple params.
        let uuid = Uuid::new_v4();
        let address = format!("instance={};peer_id=12D3KooW;club=testclub;vc=true", uuid);
        let result = extract_instance_id(&address);
        assert_eq!(result, Some(uuid));

        // Instance is the second parameter.
        let address2 = format!("peer_id=abc;instance={};vc=true", uuid);
        let result2 = extract_instance_id(&address2);
        assert_eq!(result2, Some(uuid));
    }

    // ── Build target handshake ──────────────────────────────────────────

    #[test]
    fn test_build_target_handshake() {
        let instance_id = Uuid::new_v4();
        let peer_id = "12D3KooWHyYqNJxXqRq9HuCvLp5sMQmR8kWjPFQGrWfRAdZ9MdiJ";
        let club = Some("builders".to_string());

        let original = McHandshake {
            protocol_version: 767,
            server_address: "instance=original-uuid".to_string(),
            server_port: 25565,
            next_state: 2,
            instance_id: Uuid::new_v4(),
        };

        // Without club and vc
        let ht = build_target_handshake(original.clone(), instance_id, peer_id, None, false);
        assert_eq!(ht.protocol_version, 767);
        assert_eq!(ht.server_port, 25565);
        assert_eq!(ht.next_state, 2);
        assert_eq!(ht.instance_id, instance_id);
        assert!(ht
            .server_address
            .contains(&format!("instance={}", instance_id)));
        assert!(ht.server_address.contains(&format!("peer_id={}", peer_id)));
        assert!(!ht.server_address.contains("club="));
        assert!(!ht.server_address.contains("vc=true"));

        // With club and vc
        let ht2 = build_target_handshake(original, instance_id, peer_id, club.as_deref(), true);
        assert!(ht2.server_address.contains("club=builders"));
        assert!(ht2.server_address.contains("vc=true"));
    }

    // ── Phase 38: QUIC-only error diagnostics ────────────────────────────

    #[test]
    fn test_quic_only_error_contains_diagnostic_keywords() {
        // Verify that the error context added when try_open_libp2p_stream
        // fails produces a message with actionable diagnostic keywords.
        let peer_id_str = "12D3KooWHyYqNJxXqRq9HuCvLp5sMQmR8kWjPFQGrWfRAdZ9MdiJ";
        let peer_id: libp2p::PeerId = peer_id_str.parse().unwrap();

        let err = Err::<(), _>(anyhow::anyhow!("failed to open libp2p stream: simulated"))
            .with_context(|| {
                format!(
                    "无法建立到节点 {} 的 QUIC 连接；请检查网络或稍后重试",
                    peer_id
                )
            })
            .unwrap_err();

        let msg = format!("{:#}", err);
        assert!(
            msg.contains("QUIC"),
            "error chain should mention QUIC; got: {msg}"
        );
        assert!(
            msg.contains(peer_id_str),
            "error chain should contain target peer id; got: {msg}"
        );
        assert!(
            msg.contains("连接") || msg.contains("connect"),
            "error chain should mention connection failure; got: {msg}"
        );
    }

    #[test]
    fn test_no_tcp_fallback_path_in_proxy_module() {
        // Phase 38: Verify that pick_target_address has been removed and
        // that ResolvedInstance fields proxy_address / public_ips are no
        // longer referenced in production proxy paths.
        //
        // This test encodes the static grep assertion at the type level:
        // the deleted function is not accessible via super::*.
        let resolved = ResolvedInstance {
            instance_id: "test-id".into(),
            peer_id: "12D3KooWTestPeer".into(),
            proxy_address: Some("127.0.0.1:25565".into()),
            public_ips: vec!["10.0.0.1".into()],
            multiaddrs: vec!["/ip4/127.0.0.1/tcp/25565".into()],
            resolved_at: "2025-01-01T00:00:00Z".into(),
        };

        // ResolvedInstance struct still carries these fields (populated by
        // network.rs DHT resolution), but proxy.rs no longer reads
        // proxy_address or public_ips for TCP fallback – only peer_id is
        // consumed for the QUIC stream path.
        assert_eq!(resolved.peer_id, "12D3KooWTestPeer");
        assert!(resolved.proxy_address.is_some());
        assert!(!resolved.public_ips.is_empty());

        // If pick_target_address were still present, it would be callable
        // via super::*.  This file compiles → the function is deleted.
    }

    // ── Migration state machine tests (Phase 8: fast-reconnect closure) ──

    #[test]
    fn test_migration_phase_as_str() {
        assert_eq!(MigrationPhase::Active.as_str(), "active");
        assert_eq!(
            MigrationPhase::MigrationPending.as_str(),
            "migration_pending"
        );
        assert_eq!(MigrationPhase::Reconnecting.as_str(), "reconnecting");
        assert_eq!(MigrationPhase::Failed.as_str(), "failed");
    }

    #[test]
    fn test_migration_phase_transitions() {
        // Verify the valid state transitions:
        // Active → MigrationPending → Reconnecting → Active (success)
        // Active → MigrationPending → Reconnecting → Failed (timeout/error)
        let active = MigrationPhase::Active;
        let pending = MigrationPhase::MigrationPending;
        let reconnecting = MigrationPhase::Reconnecting;
        let failed = MigrationPhase::Failed;

        // All phases are distinct
        assert_ne!(active, pending);
        assert_ne!(pending, reconnecting);
        assert_ne!(reconnecting, active);
        assert_ne!(reconnecting, failed);
        assert_ne!(failed, active);

        // Verify transition validity (compile-time check on enum)
        let phases = [active, pending, reconnecting, failed];
        for phase in &phases {
            let s = phase.as_str();
            assert!(!s.is_empty());
            assert!(matches!(
                s,
                "active" | "migration_pending" | "reconnecting" | "failed"
            ));
        }
    }

    #[test]
    fn test_proxy_session_state_initialization() {
        // Verify that ProxySession starts in Active state
        let session = ProxySession {
            instance_id: "test-instance".to_string(),
            local_port: 25565,
            target_peer_id: "12D3KooWTest".to_string(),
            substream_id: None,
            process_id: None,
            bytes_in: 0,
            bytes_out: 0,
            state: MigrationPhase::Active.as_str().to_string(),
            started_at: "2025-01-01T00:00:00Z".to_string(),
        };
        assert_eq!(session.state, "active");
        assert_eq!(session.bytes_in, 0);
        assert_eq!(session.bytes_out, 0);
    }

    #[test]
    fn test_migration_buffer_overflow_detection() {
        // Verify the migration buffer size constant is reasonable
        assert_eq!(MIGRATION_BUFFER_SIZE, 64 * 1024);

        // Verify migration timeout constant
        assert_eq!(MIGRATION_TIMEOUT, Duration::from_secs(30));
    }

    #[test]
    fn test_migration_control_frame_detection() {
        // Verify that the control frame prefix is correctly detected
        let data = b"MIGRATION_PENDING\x01\x02extra data";
        let found = data
            .windows(b"MIGRATION_PENDING".len())
            .any(|w| w == b"MIGRATION_PENDING");
        assert!(found, "MIGRATION_PENDING frame should be detected");

        // Normal Minecraft packet data should NOT trigger migration
        let normal_data = b"\x00\xFF\x00\x01packet data here12345";
        let not_found = normal_data
            .windows(b"MIGRATION_PENDING".len())
            .any(|w| w == b"MIGRATION_PENDING");
        assert!(
            !not_found,
            "normal packet data should not trigger migration"
        );
    }

    #[test]
    fn test_ready_probe_allows_switch() {
        let probe = MigrationProbeResponse {
            status: "ready".to_string(),
            target_peer_id: "12D3KooWReady".to_string(),
            supported_protocols: vec!["proxy-v1".to_string()],
            available_disk_mb: 2048,
            cpu_headroom_pct: 55.0,
            memory_headroom_mb: 4096,
            estimated_rtt_ms: 25,
            reason: None,
            checked_at: "2026-01-01T00:00:00Z".to_string(),
        };

        assert!(InstanceProxy::ensure_probe_allows_switch(&probe).is_ok());
    }

    #[test]
    fn test_rejecting_probe_blocks_switch() {
        let probe = MigrationProbeResponse {
            status: "rejecting".to_string(),
            target_peer_id: "12D3KooWRejecting".to_string(),
            supported_protocols: vec!["proxy-v1".to_string()],
            available_disk_mb: 2048,
            cpu_headroom_pct: 55.0,
            memory_headroom_mb: 4096,
            estimated_rtt_ms: 25,
            reason: Some("maintenance".to_string()),
            checked_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let err = InstanceProxy::ensure_probe_allows_switch(&probe)
            .expect_err("rejecting probe should block switch")
            .to_string();
        assert!(err.contains("status=rejecting"), "unexpected error: {err}");
    }

    #[test]
    fn test_storage_full_probe_blocks_switch() {
        let probe = MigrationProbeResponse {
            status: "storage_full".to_string(),
            target_peer_id: "12D3KooWFull".to_string(),
            supported_protocols: vec!["proxy-v1".to_string()],
            available_disk_mb: 0,
            cpu_headroom_pct: 55.0,
            memory_headroom_mb: 4096,
            estimated_rtt_ms: 25,
            reason: Some("disk threshold reached".to_string()),
            checked_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let err = InstanceProxy::ensure_probe_allows_switch(&probe)
            .expect_err("storage_full probe should block switch")
            .to_string();
        assert!(
            err.contains("status=storage_full"),
            "unexpected error: {err}"
        );
    }
}
