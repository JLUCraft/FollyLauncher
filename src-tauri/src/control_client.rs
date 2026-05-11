use crate::api;
use crate::network::NetworkHandle;
use crate::protos::jlucraft;

use libp2p::{PeerId, StreamProtocol};
use prost::Message;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tracing::{debug, warn};

const CONTROL_PROTOCOL: &str = "/jlucraft/control/1.0.0";

// ── Error ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ControlError {
    NoPeerAvailable,
    StreamError(String),
    EncodeError(String),
    DecodeError(String),
    Timeout,
    ServerError { code: String, message: String },
    UnexpectedResponse(String),
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlError::NoPeerAvailable => {
                write!(f, "无可用控制节点，请检查 P2P 连接状态")
            }
            ControlError::StreamError(msg) => write!(f, "libp2p 流错误: {msg}"),
            ControlError::EncodeError(msg) => write!(f, "编码错误: {msg}"),
            ControlError::DecodeError(msg) => write!(f, "解码错误: {msg}"),
            ControlError::Timeout => write!(f, "控制请求超时"),
            ControlError::ServerError { code, message } => {
                write!(f, "服务端错误 [{code}]: {message}")
            }
            ControlError::UnexpectedResponse(msg) => {
                write!(f, "意外的响应: {msg}")
            }
        }
    }
}

impl std::error::Error for ControlError {}

impl From<ControlError> for String {
    fn from(e: ControlError) -> Self {
        e.to_string()
    }
}

impl From<ControlError> for crate::error::LauncherError {
    fn from(e: ControlError) -> Self {
        crate::error::LauncherError::new("CONTROL_ERROR", e.to_string())
    }
}

// ── ControlClient ─────────────────────────────────────────────────────────

pub struct ControlClient {
    network: NetworkHandle,
    ed_sk: Option<libp2p::identity::ed25519::Keypair>,
    peer_id: String,
    public_key_b64: String,
    control_peer: Arc<RwLock<Option<PeerId>>>,
    request_seq: AtomicU64,
}

impl ControlClient {
    pub fn new(
        network: NetworkHandle,
        keypair: libp2p::identity::Keypair,
        peer_id: String,
    ) -> Self {
        let (ed_sk, public_key_b64) = keypair
            .clone()
            .try_into_ed25519()
            .map(|kp| {
                let pk = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    kp.public().to_bytes(),
                );
                (Some(kp), pk)
            })
            .unwrap_or_else(|_| {
                warn!("keypair is not ed25519, control requests that require auth will fail");
                (None, String::new())
            });

        Self {
            network,
            ed_sk,
            peer_id,
            public_key_b64,
            control_peer: Arc::new(RwLock::new(None)),
            request_seq: AtomicU64::new(0),
        }
    }

    // ── Peer resolution ──────────────────────────────────────────────────

    async fn resolve_control_peer(&self) -> Result<PeerId, ControlError> {
        // Check cache first.
        if let Some(ref peer) = *self.control_peer.read().await {
            return Ok(*peer);
        }

        let peers = self
            .network
            .get_peers()
            .await
            .map_err(|e| ControlError::StreamError(e.to_string()))?;

        if peers.is_empty() {
            return Err(ControlError::NoPeerAvailable);
        }

        let protocol = StreamProtocol::new(CONTROL_PROTOCOL);
        for peer_str in &peers {
            let peer_id: PeerId = match peer_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };
            match self.network.open_stream(peer_id, protocol.clone()).await {
                Ok(_stream) => {
                    // Stream opened successfully — cache this peer and
                    // return it. The stream we just opened is a disposable
                    // probe; the actual request will open a fresh one.
                    let mut cache = self.control_peer.write().await;
                    *cache = Some(peer_id);
                    debug!(%peer_id, "control peer resolved and cached");
                    return Ok(peer_id);
                }
                Err(e) => {
                    warn!(%peer_id, error = %e, "control peer probe failed");
                }
            }
        }

        Err(ControlError::NoPeerAvailable)
    }

    // ── Auth context builder ──────────────────────────────────────────────

    fn build_auth_context(
        &self,
        request_id: &str,
        body_bytes: &[u8],
    ) -> Option<jlucraft::common::v1::AuthContext> {
        let ed_sk = self.ed_sk.as_ref()?;
        let nonce = {
            let mut buf = [0u8; 32];
            getrandom::fill(&mut buf).unwrap_or(());
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, buf)
        };
        let payload_hash = blake3::hash(body_bytes).to_string();
        let expires_at = {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + 60;
            chrono::DateTime::from_timestamp(now as i64, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        };

        let canonical = format!(
            "JLUCraftAuthV1||{}||{}||control-request||{}||{}",
            request_id, nonce, payload_hash, expires_at,
        );

        let signature = ed_sk.sign(canonical.as_bytes());
        let sig_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &signature[..]);

        Some(jlucraft::common::v1::AuthContext {
            actor: Some(jlucraft::common::v1::PeerIdentity {
                peer_id: self.peer_id.clone(),
                public_key: self.public_key_b64.clone(),
                did: String::new(),
                club: String::new(),
            }),
            signature: Some(jlucraft::common::v1::SignedRequest {
                nonce,
                signature: sig_b64,
                challenge_id: request_id.to_string(),
            }),
        })
    }

    // ── Core send/recv ───────────────────────────────────────────────────

    async fn send_control_request(
        &self,
        body: jlucraft::control::v1::control_request::Body,
        auth_required: bool,
    ) -> Result<jlucraft::control::v1::ControlResponse, ControlError> {
        let peer = self.resolve_control_peer().await?;
        let protocol = StreamProtocol::new(CONTROL_PROTOCOL);
        let stream = self
            .network
            .open_stream(peer, protocol)
            .await
            .map_err(|e| {
                if let Ok(mut cache) = self.control_peer.try_write() {
                    *cache = None;
                }
                ControlError::StreamError(e.to_string())
            })?;

        let mut io = FuturesAsyncReadCompatExt::compat(stream);

        let request_id = uuid::Uuid::new_v4().to_string();
        let _seq = self.request_seq.fetch_add(1, Ordering::Relaxed);

        // Encode a temp request without auth to compute its blake3 for signing.
        let body_bytes = {
            let mut buf = Vec::new();
            jlucraft::control::v1::ControlRequest {
                request_id: request_id.clone(),
                auth: None,
                body: Some(body.clone()),
            }
            .encode(&mut buf)
            .map_err(|e| ControlError::EncodeError(e.to_string()))?;
            buf
        };

        let auth = if auth_required {
            self.build_auth_context(&request_id, &body_bytes)
        } else {
            None
        };

        let request = jlucraft::control::v1::ControlRequest {
            request_id: request_id.clone(),
            auth,
            body: Some(body),
        };

        // Encode with length-delimited framing.
        let frame = request.encode_length_delimited_to_vec();
        io.write_all(&frame)
            .await
            .map_err(|e| ControlError::StreamError(e.to_string()))?;

        // Read response: varint length prefix + message bytes.
        let frame_len = crate::utils::read_varint_u64(&mut io)
            .await
            .map_err(|e| ControlError::StreamError(e.to_string()))?;

        let mut buf = vec![0u8; frame_len as usize];
        io.read_exact(&mut buf)
            .await
            .map_err(|e| ControlError::StreamError(e.to_string()))?;

        let response = jlucraft::control::v1::ControlResponse::decode(buf.as_slice())
            .map_err(|e| ControlError::DecodeError(e.to_string()))?;

        if response.request_id != request_id {
            return Err(ControlError::UnexpectedResponse(format!(
                "request_id mismatch: sent {}, got {}",
                request_id, response.request_id
            )));
        }

        if let Some(error) = response.error {
            return Err(ControlError::ServerError {
                code: error.code,
                message: error.message,
            });
        }

        Ok(response)
    }

    fn unexpected_body<T>(
        name: &str,
        body: Option<jlucraft::control::v1::control_response::Body>,
    ) -> Result<T, ControlError> {
        Err(ControlError::UnexpectedResponse(format!(
            "expected {}, got {:?}",
            name,
            body.map(|_| "some-body")
        )))
    }

    // ── Instance methods ──────────────────────────────────────────────────

    pub async fn create_instance(
        &self,
        args: CreateInstanceArgs<'_>,
    ) -> Result<api::HttpInstance, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::CreateInstance(
            jlucraft::control::v1::CreateInstanceRequest {
                name: args.name.to_string(),
                kind: args.kind.to_string(),
                owner: args.owner.to_string(),
                club: args.club.to_string(),
                runtime: Some(jlucraft::control::v1::RuntimeSpec {
                    image: args.image.to_string(),
                    mc_version: String::new(),
                    env: std::collections::HashMap::new(),
                    labels: std::collections::HashMap::new(),
                }),
                resources: Some(jlucraft::common::v1::ResourceRequest {
                    cpu_cores: args.cpu_cores,
                    memory_gb: args.memory_gb,
                    disk_gb: args.disk_gb,
                }),
                auto_restart: args.auto_restart,
                admission: Some(jlucraft::control::v1::AdmissionPolicy {
                    mode: args.admission_mode.as_str().to_string(),
                    allowed_players: vec![],
                    allowed_clubs: vec![],
                }),
                restore_from_snapshot: String::new(),
            },
        );
        let resp = self.send_control_request(body, true).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::Instance(inst)) => {
                inst.instance.map(|i| convert_instance(&i)).ok_or_else(|| {
                    ControlError::UnexpectedResponse("instance response missing instance".into())
                })
            }
            other => Self::unexpected_body("instance response", other),
        }
    }

    pub async fn get_instance_manifest(
        &self,
        instance_id: &str,
    ) -> Result<crate::resource_sync::ResourceManifest, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::GetInstanceManifest(
            jlucraft::control::v1::GetInstanceManifestRequest {
                instance_id: instance_id.to_string(),
            },
        );
        let resp = self.send_control_request(body, false).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::InstanceManifest(m)) => {
                Ok(crate::resource_sync::ResourceManifest {
                    version: m.version,
                    instance_id: m.instance_id.clone(),
                    files: m
                        .files
                        .iter()
                        .map(|f| crate::resource_sync::ManifestFile {
                            path: f.path.clone(),
                            hash: f.hash.clone(),
                            size: f.size,
                            required: f.required,
                            download_url: None,
                            chunks: f
                                .chunks
                                .iter()
                                .map(|c| crate::resource_sync::ManifestChunk {
                                    index: c.index as usize,
                                    offset: c.offset,
                                    size: c.size,
                                    hash: c.hash.clone(),
                                })
                                .collect(),
                        })
                        .collect(),
                })
            }
            other => Self::unexpected_body("instance manifest", other),
        }
    }

    pub async fn invite_players(
        &self,
        instance_id: &str,
        topic: &str,
        players: Vec<String>,
    ) -> Result<InvitePlayersResult, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::InvitePlayers(
            jlucraft::control::v1::InvitePlayersRequest {
                instance_id: instance_id.to_string(),
                topic: topic.to_string(),
                invitees: players,
                message: String::new(),
            },
        );
        let resp = self.send_control_request(body, true).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::InvitePlayers(r)) => {
                Ok(InvitePlayersResult {
                    instance_id: instance_id.to_string(),
                    invited_count: r.invited_count,
                    missing_recipients: r.missing_recipients,
                })
            }
            other => Self::unexpected_body("invite players response", other),
        }
    }

    pub async fn probe_migration(
        &self,
        instance_id: &str,
        source_peer_id: &str,
    ) -> Result<api::MigrationProbeResponse, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::ProbeMigration(
            jlucraft::control::v1::ProbeMigrationRequest {
                instance_id: instance_id.to_string(),
                source_peer_id: source_peer_id.to_string(),
            },
        );
        let resp = self.send_control_request(body, false).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::MigrationProbe(r)) => {
                Ok(api::MigrationProbeResponse {
                    status: r.status,
                    target_peer_id: r.target_peer_id,
                    supported_protocols: r.supported_protocols,
                    available_disk_mb: r.available_disk_mb,
                    cpu_headroom_pct: r.cpu_headroom_pct,
                    memory_headroom_mb: r.memory_headroom_mb,
                    estimated_rtt_ms: r.estimated_rtt_ms,
                    reason: if r.reason.is_empty() {
                        None
                    } else {
                        Some(r.reason)
                    },
                    checked_at: r.checked_at,
                })
            }
            other => Self::unexpected_body("migration probe response", other),
        }
    }

    // ── Tournament methods ────────────────────────────────────────────────

    pub async fn list_tournaments(&self) -> Result<Vec<api::Tournament>, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::ListTournaments(
            jlucraft::control::v1::ListTournamentsRequest {
                status: String::new(),
            },
        );
        let resp = self.send_control_request(body, false).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::Tournaments(list)) => Ok(list
                .tournaments
                .into_iter()
                .map(|t| convert_tournament(&t))
                .collect()),
            other => Self::unexpected_body("tournament list", other),
        }
    }

    pub async fn get_tournament(
        &self,
        tournament_id: &str,
    ) -> Result<Option<api::Tournament>, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::GetTournament(
            jlucraft::control::v1::GetTournamentRequest {
                tournament_id: tournament_id.to_string(),
            },
        );
        let resp = self.send_control_request(body, false).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::Tournament(t)) => {
                t.tournament.map(|p| Ok(convert_tournament(&p))).transpose()
            }
            other => Self::unexpected_body("tournament", other),
        }
    }

    pub async fn list_matches(&self, tournament_id: &str) -> Result<Vec<api::Match>, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::ListTournamentMatches(
            jlucraft::control::v1::ListTournamentMatchesRequest {
                tournament_id: tournament_id.to_string(),
            },
        );
        let resp = self.send_control_request(body, false).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::Matches(list)) => Ok(list
                .matches
                .into_iter()
                .map(|m| convert_match(&m))
                .collect()),
            other => Self::unexpected_body("match list", other),
        }
    }

    pub async fn register_for_tournament(
        &self,
        tournament_id: &str,
        player_id: &str,
    ) -> Result<(), ControlError> {
        let body = jlucraft::control::v1::control_request::Body::RegisterForTournament(
            jlucraft::control::v1::RegisterForTournamentRequest {
                tournament_id: tournament_id.to_string(),
                player_id: player_id.to_string(),
            },
        );
        self.send_control_request(body, true).await?;
        Ok(())
    }

    // ── Dispute methods ───────────────────────────────────────────────────

    pub async fn create_match_dispute(
        &self,
        tournament_id: &str,
        match_id: &str,
        reason: &str,
        evidence_urls: Vec<String>,
    ) -> Result<api::DisputeMatch, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::CreateMatchDispute(
            jlucraft::control::v1::CreateMatchDisputeRequest {
                tournament_id: tournament_id.to_string(),
                match_id: match_id.to_string(),
                reason: reason.to_string(),
                evidence_urls,
            },
        );
        let resp = self.send_control_request(body, true).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::Dispute(d)) => d
                .dispute
                .map(|p| Ok(convert_dispute(&p)))
                .unwrap_or_else(|| {
                    Err(ControlError::UnexpectedResponse(
                        "dispute response missing dispute".into(),
                    ))
                }),
            other => Self::unexpected_body("dispute response", other),
        }
    }

    pub async fn list_disputes(
        &self,
        tournament_id: Option<&str>,
    ) -> Result<Vec<api::DisputeMatch>, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::ListDisputes(
            jlucraft::control::v1::ListDisputesRequest {
                tournament_id: tournament_id.unwrap_or_default().to_string(),
            },
        );
        let resp = self.send_control_request(body, false).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::Disputes(list)) => Ok(list
                .disputes
                .into_iter()
                .map(|d| convert_dispute(&d))
                .collect()),
            other => Self::unexpected_body("dispute list", other),
        }
    }

    pub async fn get_dispute(
        &self,
        dispute_id: &str,
    ) -> Result<Option<api::DisputeMatch>, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::GetDispute(
            jlucraft::control::v1::GetDisputeRequest {
                dispute_id: dispute_id.to_string(),
            },
        );
        let resp = self.send_control_request(body, false).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::Dispute(d)) => {
                d.dispute.map(|p| Ok(convert_dispute(&p))).transpose()
            }
            other => Self::unexpected_body("dispute", other),
        }
    }

    // ── Team methods ──────────────────────────────────────────────────────

    pub async fn create_team(&self, name: &str, members: Vec<String>) -> Result<(), ControlError> {
        let body = jlucraft::control::v1::control_request::Body::CreateTeam(
            jlucraft::control::v1::CreateTeamRequest {
                name: name.to_string(),
                members,
            },
        );
        self.send_control_request(body, true).await?;
        Ok(())
    }

    pub async fn list_teams(&self) -> Result<Vec<api::Team>, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::ListTeams(
            jlucraft::control::v1::ListTeamsRequest {},
        );
        let resp = self.send_control_request(body, false).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::Teams(list)) => {
                Ok(list.teams.into_iter().map(|t| convert_team(&t)).collect())
            }
            other => Self::unexpected_body("team list", other),
        }
    }

    // ── MUA peer bind ─────────────────────────────────────────────────────

    pub async fn bind_mua_peer(
        &self,
        access_token: &str,
        auth_server_url: &str,
    ) -> Result<(), ControlError> {
        let ed_sk = self
            .ed_sk
            .as_ref()
            .ok_or_else(|| ControlError::StreamError("keypair is not ed25519".into()))?;

        let peer_public_key = ed_sk.public().to_bytes().to_vec();
        let message = format!(
            "JLUCraftPeerBindV1||{}||{}||{}",
            self.peer_id, access_token, auth_server_url,
        );
        let peer_signature = ed_sk.sign(message.as_bytes()).to_vec();

        let body = jlucraft::control::v1::control_request::Body::BindMuaPeer(
            jlucraft::control::v1::BindMuaPeerRequest {
                access_token: access_token.to_string(),
                auth_server_url: auth_server_url.to_string(),
                peer_id: self.peer_id.clone(),
                peer_public_key,
                peer_signature,
            },
        );
        self.send_control_request(body, true).await?;
        Ok(())
    }

    // ── Credential / CRL methods ──────────────────────────────────────────

    pub async fn list_revoked_credentials(&self) -> Result<Vec<String>, ControlError> {
        let body = jlucraft::control::v1::control_request::Body::ListRevokedCredentials(
            jlucraft::control::v1::ListRevokedCredentialsRequest {},
        );
        let resp = self.send_control_request(body, false).await?;
        match resp.body {
            Some(jlucraft::control::v1::control_response::Body::RevokedCredentials(r)) => {
                Ok(r.credential_ids)
            }
            other => Self::unexpected_body("revoked credential list", other),
        }
    }
}

// ── Public arg structs ───────────────────────────────────────────────────

pub struct CreateInstanceArgs<'a> {
    pub name: &'a str,
    pub kind: &'a str,
    pub owner: &'a str,
    pub club: &'a str,
    pub image: &'a str,
    pub cpu_cores: u32,
    pub memory_gb: u64,
    pub disk_gb: u64,
    pub auto_restart: bool,
    pub admission_mode: &'a api::AdmissionMode,
}

// ── Invite result ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct InvitePlayersResult {
    pub instance_id: String,
    pub invited_count: u32,
    #[serde(default)]
    pub missing_recipients: Vec<String>,
}

// ── Proto → Domain converters ─────────────────────────────────────────────

fn convert_instance(p: &jlucraft::control::v1::Instance) -> api::HttpInstance {
    api::HttpInstance {
        id: p.id.clone(),
        name: p.name.clone(),
        kind: p.kind.clone(),
        owner: p.owner.clone(),
        club: p.club.clone(),
        host: p.host_peer_id.clone(),
        status: p.status.clone(),
        created_at: p.created_at.clone(),
        updated_at: p.updated_at.clone(),
        host_port: 0,
        rcon_port: 0,
        auto_restart: false,
        migration_target: if p.migration_target.is_empty() {
            None
        } else {
            Some(p.migration_target.clone())
        },
        player_count: p.player_count,
    }
}

fn convert_tournament(p: &jlucraft::control::v1::Tournament) -> api::Tournament {
    api::Tournament {
        id: p.id.clone(),
        name: p.name.clone(),
        game_type: p.game_type.clone(),
        mode: p.mode.clone(),
        schedule: p
            .schedule
            .as_ref()
            .map(|s| api::TournamentSchedule {
                registration_open: s.registration_open.clone(),
                registration_close: s.registration_close.clone(),
                matches: s
                    .matches
                    .iter()
                    .map(|m| api::MatchSchedule {
                        round: m.round as i32,
                        datetime: m.datetime.clone(),
                        map: m.map.clone(),
                    })
                    .collect(),
            })
            .unwrap_or_default(),
        scoring: p
            .scoring
            .as_ref()
            .map(|s| api::ScoringRules {
                win: s.win,
                kill: s.kill,
                survive_minute: s.survive_minute,
                placement_1: s.placement_1,
                placement_2: s.placement_2,
                placement_3: s.placement_3,
            })
            .unwrap_or_default(),
        min_member_score: p.min_member_score as i32,
        max_participants: p.max_participants as i32,
        participant_count: p.participant_count as i32,
        status: match p.status.as_str() {
            "draft" => api::TournamentStatus::Draft,
            "registration" => api::TournamentStatus::Registration,
            "ongoing" => api::TournamentStatus::Ongoing,
            "paused" => api::TournamentStatus::Paused,
            "cancelled" => api::TournamentStatus::Cancelled,
            "completed" => api::TournamentStatus::Completed,
            _ => api::TournamentStatus::Draft,
        },
        created_at: p.created_at.clone(),
        created_by: p.created_by.clone(),
    }
}

fn convert_match(p: &jlucraft::control::v1::Match) -> api::Match {
    api::Match {
        id: p.id.clone(),
        tournament_id: p.tournament_id.clone(),
        round: p.round as i32,
        participants: p.participants.clone(),
        instance_id: if p.instance_id.is_empty() {
            None
        } else {
            Some(p.instance_id.clone())
        },
        result: p.result.as_ref().map(|r| api::MatchResult {
            rankings: r
                .rankings
                .iter()
                .map(|pr| api::PlayerResult {
                    player_id: pr.player_id.clone(),
                    score: pr.score,
                    kills: pr.kills,
                    deaths: pr.deaths,
                    survive_minutes: pr.survive_minutes,
                })
                .collect(),
        }),
        status: match p.status.as_str() {
            "scheduled" => api::MatchStatus::Scheduled,
            "live" => api::MatchStatus::Live,
            "finished" => api::MatchStatus::Finished,
            "disputed" => api::MatchStatus::Disputed,
            _ => api::MatchStatus::Scheduled,
        },
        scheduled_at: p.scheduled_at.clone(),
    }
}

fn convert_dispute(p: &jlucraft::control::v1::Dispute) -> api::DisputeMatch {
    api::DisputeMatch {
        dispute_id: p.dispute_id.clone(),
        tournament_id: p.tournament_id.clone(),
        match_id: p.match_id.clone(),
        status: p.status.clone(),
        reason: p.reason.clone(),
        evidence_urls: p.evidence_urls.clone(),
        submitted_by: if p.submitted_by.is_empty() {
            None
        } else {
            Some(p.submitted_by.clone())
        },
        resolution: if p.resolution.is_empty() {
            None
        } else {
            Some(p.resolution.clone())
        },
        created_at: p.created_at.clone(),
        resolved_at: if p.resolved_at.is_empty() {
            None
        } else {
            Some(p.resolved_at.clone())
        },
        resolved_by: if p.resolved_by.is_empty() {
            None
        } else {
            Some(p.resolved_by.clone())
        },
    }
}

fn convert_team(p: &jlucraft::control::v1::Team) -> api::Team {
    api::Team {
        id: p.id.clone(),
        name: p.name.clone(),
        members: p.members.clone(),
        total_score: p.total_score,
        tournament_ids: p.tournament_ids.clone(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Varint round-trip ──────────────────────────────────────────────

    #[test]
    fn test_varint_roundtrip() {
        let test_values = [0u64, 1, 127, 128, 255, 65535, 1_000_000];
        for val in test_values {
            let mut buf = Vec::new();
            prost::encoding::encode_varint(val, &mut buf);
            let mut cursor = std::io::Cursor::new(buf);
            let rt = tokio::runtime::Runtime::new().unwrap();
            let decoded = rt
                .block_on(crate::utils::read_varint_u64(&mut cursor))
                .unwrap();
            assert_eq!(decoded, val, "varint roundtrip failed for {}", val);
        }
    }

    // ── Auth context structure ─────────────────────────────────────────

    #[test]
    fn test_auth_context_shape() {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let ed_sk = keypair.clone().try_into_ed25519().ok();
        let public_key_b64 = ed_sk
            .as_ref()
            .map(|kp| {
                base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    kp.public().to_bytes(),
                )
            })
            .unwrap_or_default();
        let client = ControlClient {
            network: crate::network::NetworkHandle::new_for_test(),
            ed_sk,
            peer_id: "12D3KooWTestPeer".into(),
            public_key_b64,
            control_peer: Arc::new(RwLock::new(None)),
            request_seq: AtomicU64::new(0),
        };

        let ctx = client
            .build_auth_context("req-001", b"test-body")
            .expect("auth context should build for ed25519 keypair");

        assert!(!ctx.actor.as_ref().unwrap().peer_id.is_empty());
        assert!(!ctx.actor.as_ref().unwrap().public_key.is_empty());
        assert!(!ctx.signature.as_ref().unwrap().nonce.is_empty());
        assert!(!ctx.signature.as_ref().unwrap().signature.is_empty());
    }

    // ── Proto → Domain converter tests ─────────────────────────────────

    #[test]
    fn test_convert_tournament_maps_status() {
        let p = jlucraft::control::v1::Tournament {
            id: "t1".into(),
            name: "Test".into(),
            game_type: "pvp".into(),
            mode: "solo".into(),
            status: "ongoing".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            created_by: "peer1".into(),
            min_member_score: 100,
            max_participants: 64,
            participant_count: 32,
            schedule: None,
            scoring: None,
        };
        let t = convert_tournament(&p);
        assert_eq!(t.id, "t1");
        assert_eq!(t.status, api::TournamentStatus::Ongoing);
        assert_eq!(t.game_type, "pvp");
    }

    #[test]
    fn test_convert_match_maps_status() {
        let p = jlucraft::control::v1::Match {
            id: "m1".into(),
            tournament_id: "t1".into(),
            round: 1,
            participants: vec!["p1".into(), "p2".into()],
            instance_id: "i1".into(),
            status: "live".into(),
            scheduled_at: "2026-01-01T00:00:00Z".into(),
            result: None,
        };
        let m = convert_match(&p);
        assert_eq!(m.status, api::MatchStatus::Live);
        assert_eq!(m.participants.len(), 2);
    }

    #[test]
    fn test_convert_dispute_maps_optional_fields() {
        let p = jlucraft::control::v1::Dispute {
            dispute_id: "d1".into(),
            tournament_id: "t1".into(),
            match_id: "m1".into(),
            status: "open".into(),
            reason: "test".into(),
            evidence_urls: vec!["url1".into()],
            submitted_by: "peer1".into(),
            resolution: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            resolved_at: String::new(),
            resolved_by: String::new(),
        };
        let d = convert_dispute(&p);
        assert_eq!(d.dispute_id, "d1");
        assert_eq!(d.submitted_by, Some("peer1".into()));
        assert_eq!(d.resolution, None);
        assert_eq!(d.resolved_at, None);
    }

    #[test]
    fn test_convert_instance_maps_fields() {
        let p = jlucraft::control::v1::Instance {
            id: "i1".into(),
            name: "Test Room".into(),
            kind: "room".into(),
            owner: "owner1".into(),
            club: "club1".into(),
            host_peer_id: "host1".into(),
            status: "running".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T01:00:00Z".into(),
            player_count: 5,
            migration_target: "target1".into(),
            runtime: None,
            resources: None,
            admission: None,
        };
        let i = convert_instance(&p);
        assert_eq!(i.id, "i1");
        assert_eq!(i.host, "host1");
        assert_eq!(i.player_count, 5);
        assert_eq!(i.migration_target, Some("target1".into()));
    }

    #[test]
    fn test_convert_team() {
        let p = jlucraft::control::v1::Team {
            id: "team1".into(),
            name: "Dream Team".into(),
            members: vec!["p1".into(), "p2".into()],
            total_score: 500,
            tournament_ids: vec!["t1".into()],
        };
        let t = convert_team(&p);
        assert_eq!(t.name, "Dream Team");
        assert_eq!(t.members.len(), 2);
        assert_eq!(t.total_score, 500);
    }

    // ── ControlError Display ───────────────────────────────────────────

    #[test]
    fn test_control_error_display_contains_cn() {
        assert!(ControlError::NoPeerAvailable
            .to_string()
            .contains("无可用控制节点"));
        assert!(ControlError::Timeout.to_string().contains("超时"));
        let server_err = ControlError::ServerError {
            code: "DENIED".into(),
            message: "vc required".into(),
        };
        assert!(server_err.to_string().contains("DENIED"));
        assert!(server_err.to_string().contains("vc required"));
    }

    #[test]
    fn test_control_error_into_string() {
        let e = ControlError::NoPeerAvailable;
        let s: String = e.into();
        assert!(!s.is_empty());
    }
}
