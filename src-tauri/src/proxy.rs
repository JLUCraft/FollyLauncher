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

pub(crate) trait ProxyStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}
impl<T> ProxyStream for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}

#[derive(Clone)]
pub struct InstanceProxy {
    inner: Arc<ProxyInner>,
}

struct ProxyInner {
    listener: TcpListener,
    network: NetworkHandle,
    sessions: RwLock<HashMap<u16, ProxySession>>,
    dht_cache: RwLock<HashMap<String, CachedResolution>>,
    peer_cache: RwLock<HashMap<String, CachedPeerConnection>>,
}

#[derive(Debug, Clone)]
struct CachedPeerConnection {
    cached_at: Instant,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProxySession {
    pub instance_id: String,
    pub local_port: u16,
    pub target_peer_id: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub state: String,
    pub started_at: String,
}

#[derive(Debug, Clone)]
struct CachedResolution {
    resolved: ResolvedInstance,
    cached_at: Instant,
}

impl InstanceProxy {
    pub async fn bind(network: NetworkHandle) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind local proxy")?;
        let local_addr = listener.local_addr()?;
        info!(%local_addr, "instance proxy listening");

        Ok(Self {
            inner: Arc::new(ProxyInner {
                listener,
                network,
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
        let resolved = self.resolve_instance_cached(instance_id.clone()).await
                .with_context(|| format!("instance {} not found in DHT", instance_id))?;

            let target_peer_id = resolved.peer_id.parse::<libp2p::PeerId>()
                .with_context(|| format!("invalid peer id: {}", resolved.peer_id))?;

            let target: Box<dyn ProxyStream> = match self.try_open_libp2p_stream(target_peer_id).await {
                Ok(stream) => {
                    info!(%peer_addr, "connected via libp2p stream tunnel");
                    Box::new(tokio_util::compat::FuturesAsyncReadCompatExt::compat(stream))
                }
                Err(e) => {
                    warn!(%peer_addr, error = %e, "libp2p stream failed, falling back to TCP");
                    let target_addr = pick_target_address(&resolved)
                        .context("no connectable address")?;
                    Box::new(TcpStream::connect(target_addr).await
                        .with_context(|| format!("TCP connect to {target_addr} failed"))?)
                }
            };

            let session = ProxySession {
                instance_id: instance_id.clone(),
                local_port,
                target_peer_id: resolved.peer_id.clone(),
                bytes_in: 0, bytes_out: 0,
                state: "active".to_string(),
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
        ).await;

        {
            let mut sessions = self.inner.sessions.write().await;
            if let Some(s) = sessions.get_mut(&local_port) { s.state = "closed".to_string(); }
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
                if cached.cached_at.elapsed() < cache_ttl {
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

        // Update cache on success
        {
            let mut cache = self.inner.peer_cache.write().await;
            cache.insert(
                peer_id_str,
                CachedPeerConnection {
                    cached_at: Instant::now(),
                },
            );
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
            let mut in_migration = false;
            let mut migration_start = tokio::time::Instant::now();
            loop {
                match target_read.read(&mut buf).await {
                    Ok(0) => break Ok(total),
                    Ok(n) => {
                        // Check for migration control frame prefix
                        if !in_migration && buf[..n.min(17)].windows(16).any(|w| {
                            w == b"MIGRATION_PENDING"
                        }) {
                            in_migration = true;
                            migration_start = tokio::time::Instant::now();
                            info!(peer_addr = %peer_addr, "migration pending, buffering bytes");
                            // Buffer any remaining bytes from this read
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
                                    s.state = "migrating".to_string();
                                }
                            }
                            continue;
                        }

                    if in_migration {
                        if migration_buffer.len() + n <= MIGRATION_BUFFER_SIZE {
                            migration_buffer.extend_from_slice(&buf[..n]);
                            total += n as u64;
                        } else {
                            if migration_start.elapsed() > MIGRATION_TIMEOUT {
                                warn!(peer_addr = %peer_addr, "migration timeout exceeded");
                            }
                            warn!(peer_addr = %peer_addr, "migration buffer overflow");
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
            .await;

        if let Some(ref r) = resolved {
            let mut cache = self.inner.dht_cache.write().await;
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
        _current_peer_id: &str,
    ) -> Result<(Box<dyn ProxyStream>, String)> {
        info!(%instance_id, "migration reconnection");
        let resolved = self
            .resolve_instance_cached(instance_id.to_string())
            .await
            .with_context(|| format!("instance {} not found in DHT during migration", instance_id))?;

        let new_peer_id_str = resolved.peer_id.clone();
        let target_peer_id: libp2p::PeerId = new_peer_id_str
            .parse()
            .with_context(|| format!("invalid peer id: {}", new_peer_id_str))?;

        let stream: Box<dyn ProxyStream> = match self.try_open_libp2p_stream(target_peer_id).await {
            Ok(stream) => {
                info!(new_peer = %new_peer_id_str, "migration libp2p stream established");
                Box::new(tokio_util::compat::FuturesAsyncReadCompatExt::compat(stream))
            }
            Err(e) => {
                warn!(error = %e, "migration libp2p failed, falling back to TCP");
                let target_addr = pick_target_address(&resolved)
                    .context("no connectable address for migration target")?;
                let tcp = TcpStream::connect(target_addr)
                    .await
                    .with_context(|| format!("migration TCP connect to {} failed", target_addr))?;
                Box::new(tcp)
            }
        };

        Ok((stream, new_peer_id_str))
    }

    /// Bind a temporary local port dedicated to a single instance launch.
    /// Returns the local port number that Minecraft should connect to.
    pub async fn bridge_instance(
        &self,
        instance_id: String,
        peer_id: String,
        club: Option<String>,
        has_member_vc: bool,
    ) -> Result<u16> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind temporary instance proxy")?;
        let local_port = listener.local_addr()?.port();
        info!(%local_port, %instance_id, "instance proxy bound for single launch");

        let proxy = self.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let proxy = proxy.clone();
                        let instance_id = instance_id.clone();
                        let peer_id = peer_id.clone();
                        let club = club.clone();
                        tokio::spawn(async move {
                            if let Err(e) = proxy
                                .handle_client_known_instance(
                                    stream,
                                    peer_addr,
                                    instance_id,
                                    peer_id,
                                    club,
                                    has_member_vc,
                                )
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
        instance_id: String,
        peer_id: String,
        club: Option<String>,
        has_member_vc: bool,
    ) -> Result<()> {
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
            proxy = ?resolved.proxy_address,
            "resolved instance location"
        );

        let target_peer_id = resolved
            .peer_id
            .parse::<libp2p::PeerId>()
            .with_context(|| format!("invalid peer id: {}", resolved.peer_id))?;

        let target: Box<dyn ProxyStream> = match self.try_open_libp2p_stream(target_peer_id).await {
            Ok(stream) => {
                info!(%peer_addr, "connected via libp2p stream tunnel");
                Box::new(tokio_util::compat::FuturesAsyncReadCompatExt::compat(
                    stream,
                ))
            }
            Err(e) => {
                warn!(%peer_addr, error = %e, "libp2p stream failed, falling back to TCP");
                let target_addr = pick_target_address(&resolved)
                    .context("no connectable address available for target node")?;
                let tcp = TcpStream::connect(target_addr)
                    .await
                    .with_context(|| format!("failed to connect to target {target_addr}"))?;
                Box::new(tcp)
            }
        };

        let session = ProxySession {
            instance_id: target_handshake.instance_id.to_string(),
            local_port,
            target_peer_id: resolved.peer_id.clone(),
            bytes_in: 0,
            bytes_out: 0,
            state: "active".to_string(),
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

fn pick_target_address(resolved: &ResolvedInstance) -> Option<SocketAddr> {
    // 1. Try public IPs + proxy port (most reliable for direct TCP)
    let proxy_port = resolved
        .proxy_address
        .as_ref()
        .and_then(|addr| addr.parse::<SocketAddr>().ok().map(|a| a.port()));
    if let Some(port) = proxy_port {
        for ip_str in &resolved.public_ips {
            if let Ok(ip) = ip_str.parse::<std::net::IpAddr>() {
                return Some(SocketAddr::new(ip, port));
            }
        }
    }

    // 2. Try proxy_address directly (works for local/loopback testing)
    if let Some(addr) = &resolved.proxy_address {
        if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
            // Replace 0.0.0.0 with 127.0.0.1 for local connections
            if socket_addr.ip().is_unspecified() {
                return Some(SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                    socket_addr.port(),
                ));
            }
            return Some(socket_addr);
        }
    }

    // 3. Parse libp2p multiaddrs with TCP protocol, preferring direct addresses over relay
    let mut relay_fallback = None;
    for addr_str in &resolved.multiaddrs {
        if let Ok(addr) = addr_str.parse::<libp2p::Multiaddr>() {
            let is_relay = addr
                .iter()
                .any(|p| matches!(p, libp2p::multiaddr::Protocol::P2pCircuit));
            let mut ip = None;
            let mut port = None;
            for proto in addr.iter() {
                match proto {
                    libp2p::multiaddr::Protocol::Ip4(v4) => ip = Some(std::net::IpAddr::V4(v4)),
                    libp2p::multiaddr::Protocol::Ip6(v6) => ip = Some(std::net::IpAddr::V6(v6)),
                    libp2p::multiaddr::Protocol::Tcp(p) => port = Some(p),
                    _ => {}
                }
            }
            if let (Some(ip), Some(port)) = (ip, port) {
                if is_relay {
                    if relay_fallback.is_none() {
                        relay_fallback = Some(SocketAddr::new(ip, port));
                    }
                } else {
                    return Some(SocketAddr::new(ip, port));
                }
            }
        }
    }
    relay_fallback
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
    let mut result = 0i32;
    let mut shift = 0u32;
    loop {
        let mut byte = [0u8; 1];
        reader
            .read_exact(&mut byte)
            .await
            .context("failed to read varint byte")?;
        let value = (byte[0] & 0x7F) as i32;
        result |= value << shift;
        if byte[0] & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 32 {
            bail!("varint too long");
        }
    }
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
