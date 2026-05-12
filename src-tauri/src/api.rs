use serde::{Deserialize, Serialize};
use tauri::Manager;
use tracing::info;


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TournamentStatus {
    Draft,
    Registration,
    Ongoing,
    Paused,
    Cancelled,
    Completed,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MatchStatus {
    Scheduled,
    Live,
    Finished,
    Disputed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tournament {
    pub id: String,
    pub name: String,
    #[serde(rename = "game_type")]
    pub game_type: String,
    pub mode: String,
    #[serde(default)]
    pub schedule: TournamentSchedule,
    #[serde(default)]
    pub scoring: ScoringRules,
    #[serde(rename = "min_member_score")]
    pub min_member_score: i32,
    pub max_participants: i32,
    #[serde(rename = "participant_count", default)]
    pub participant_count: i32,
    pub status: TournamentStatus,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "created_by")]
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TournamentSchedule {
    pub registration_open: String,
    pub registration_close: String,
    pub matches: Vec<MatchSchedule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchSchedule {
    pub round: i32,
    pub datetime: String,
    pub map: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScoringRules {
    pub win: i32,
    pub kill: i32,
    pub survive_minute: f64,
    pub placement_1: i32,
    pub placement_2: i32,
    pub placement_3: i32,
}

impl Default for ScoringRules {
    fn default() -> Self {
        Self {
            win: 10,
            kill: 2,
            survive_minute: 0.5,
            placement_1: 10,
            placement_2: 7,
            placement_3: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerResult {
    pub player_id: String,
    pub score: f64,
    pub kills: i32,
    pub deaths: i32,
    #[serde(rename = "survive_minutes")]
    pub survive_minutes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub rankings: Vec<PlayerResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub id: String,
    #[serde(rename = "tournament_id")]
    pub tournament_id: String,
    pub round: i32,
    pub participants: Vec<String>,
    #[serde(rename = "instance_id", default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub result: Option<MatchResult>,
    pub status: MatchStatus,
    #[serde(rename = "scheduled_at")]
    pub scheduled_at: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeMatch {
    pub dispute_id: String,
    pub tournament_id: String,
    pub match_id: String,
    pub status: String,
    pub reason: String,
    #[serde(default)]
    pub evidence_urls: Vec<String>,
    #[serde(default)]
    pub submitted_by: Option<String>,
    #[serde(default)]
    pub resolution: Option<String>,

    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub resolved_at: Option<String>,
    #[serde(default)]
    pub resolved_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedServer {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YggdrasilServer {
    pub name: String,
    pub url: String,
    pub auth_server_url: String,
    pub client_id: String,
    pub scope: String,
    #[serde(default)]
    pub default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub launcher_name: String,
    pub federated_servers: Vec<FederatedServer>,
    pub yggdrasil_servers: Vec<YggdrasilServer>,
    #[serde(default)]
    pub discover_source_endpoints: Vec<(String, bool)>,
    #[serde(default)]
    pub bootstrap_peers: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AdmissionMode {
    Public,
    #[serde(alias = "vc_only")]
    VcOnly,
    #[serde(alias = "mua_member")]
    MuaMember,
    #[serde(alias = "club_only")]
    ClubOnly,
}

impl AdmissionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::VcOnly => "vc-only",
            Self::MuaMember => "mua-member",
            Self::ClubOnly => "club-only",
        }
    }

    pub fn normalize(raw: &str) -> Self {
        match raw {
            "vc_only" | "vc-only" => Self::VcOnly,
            "mua_member" | "mua-member" => Self::MuaMember,
            "club_only" | "club-only" => Self::ClubOnly,
            _ => Self::Public,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConfig {
    pub image_prefix: String,
    pub cpu_cores: u16,
    pub memory_gb: u64,
    pub disk_gb: u64,
    pub auto_restart: bool,
    pub admission_mode: AdmissionMode,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            image_prefix: "registry.jlucraft.local/mc/".to_string(),
            cpu_cores: 2,
            memory_gb: 4,
            disk_gb: 20,
            auto_restart: true,
            admission_mode: AdmissionMode::Public,
        }
    }
}

impl LauncherConfig {
    pub fn load_from_resource(app_handle: &tauri::AppHandle) -> anyhow::Result<Self> {
        use std::io::Read;

        let resource_dir = app_handle
            .path()
            .resource_dir()
            .map_err(|e| anyhow::anyhow!("failed to get resource dir: {e}"))?;
        let path = resource_dir.join("defaults.toml");
        let mut file = std::fs::File::open(&path)
            .map_err(|e| anyhow::anyhow!("failed to open {}: {}", path.display(), e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {}", path.display(), e))?;
        let config: Self = toml::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", path.display(), e))?;
        info!(name = %config.launcher_name, federated = config.federated_servers.len(), yggdrasil = config.yggdrasil_servers.len(), "launcher config loaded");
        Ok(config)
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationProbeResponse {
    pub status: String,
    pub target_peer_id: String,
    pub supported_protocols: Vec<String>,
    pub available_disk_mb: u64,
    pub cpu_headroom_pct: f64,
    pub memory_headroom_mb: u64,
    pub estimated_rtt_ms: u64,
    #[serde(default)]
    pub reason: Option<String>,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub members: Vec<String>,
    pub total_score: i32,
    pub tournament_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInstance {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub owner: String,
    pub club: String,
    #[serde(rename = "current_host")]
    pub host: String,
    pub status: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
    #[serde(rename = "host_port")]
    pub host_port: u16,
    #[serde(rename = "rcon_port", default)]
    pub rcon_port: u16,
    #[serde(rename = "auto_restart", default)]
    pub auto_restart: bool,
    #[serde(
        rename = "migration_target",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub migration_target: Option<String>,
    #[serde(default)]
    pub player_count: u32,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub max_players: u32,
    #[serde(default)]
    pub version: String,
}
