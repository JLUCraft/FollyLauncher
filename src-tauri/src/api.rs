use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Manager;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tournament {
    pub id: String,
    pub name: String,
    #[serde(rename = "game_type")]
    pub game_type: String,
    pub mode: String,
    pub status: String,
    #[serde(rename = "participant_count", default)]
    pub participant_count: i32,
    pub max_participants: i32,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "created_by")]
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub id: String,
    #[serde(rename = "tournament_id")]
    pub tournament_id: String,
    pub round: i32,
    pub participants: Vec<String>,
    pub status: String,
    #[serde(rename = "scheduled_at")]
    pub scheduled_at: String,
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

/// Resources and admission configuration used when creating a new instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConfig {
    pub image_prefix: String,
    pub cpu_cores: u16,
    pub memory_gb: u64,
    pub disk_gb: u64,
    pub auto_restart: bool,
    /// Must be one of: "public", "vc-only", "mua-member", "club-only"
    pub admission_mode: String,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            image_prefix: "registry.jlucraft.local/mc/".to_string(),
            cpu_cores: 2,
            memory_gb: 4,
            disk_gb: 20,
            auto_restart: true,
            admission_mode: "public".to_string(),
        }
    }
}

impl LauncherConfig {
    pub fn load_from_resource(app_handle: &tauri::AppHandle) -> anyhow::Result<Self> {
        use std::io::Read;

        let resource_dir = app_handle.path().resource_dir()
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

    pub fn default_federated_server(&self) -> Option<&FederatedServer> {
        self.federated_servers.iter().find(|s| s.default).or_else(|| self.federated_servers.first())
    }

    pub fn default_yggdrasil_server(&self) -> Option<&YggdrasilServer> {
        self.yggdrasil_servers.iter().find(|s| s.default).or_else(|| self.yggdrasil_servers.first())
    }
}

#[derive(Clone)]
pub struct FederatedApiClient {
    client: reqwest::Client,
    base_url: Arc<String>,
    launcher_name: Arc<String>,
}

impl FederatedApiClient {
    pub fn new(base_url: String, launcher_name: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build reqwest client"),
            base_url: Arc::new(base_url),
            launcher_name: Arc::new(launcher_name),
        }
    }

    pub fn get_base_url(&self) -> String {
        self.base_url.to_string()
    }

    pub fn get_launcher_name(&self) -> String {
        self.launcher_name.to_string()
    }

    async fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    pub async fn list_tournaments(&self) -> Result<Vec<Tournament>, String> {
        let url = self.url("/v1/tournaments").await;
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("api returned status {}", resp.status()));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_tournament(&self, id: &str) -> Result<Option<Tournament>, String> {
        let url = self.url(&format!("/v1/tournaments/{}", id)).await;
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(format!("api returned status {}", resp.status()));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn list_matches(&self, tournament_id: &str) -> Result<Vec<Match>, String> {
        let url = self
            .url(&format!("/v1/tournaments/{}/matches", tournament_id))
            .await;
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("api returned status {}", resp.status()));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn register_for_tournament(&self, tournament_id: &str) -> Result<(), String> {
        let url = self
            .url(&format!("/v1/tournaments/{}/register", tournament_id))
            .await;
        let resp = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("api returned status {}", resp.status()));
        }
        Ok(())
    }

    pub async fn create_team(&self, name: &str, members: Vec<String>) -> Result<(), String> {
        let url = self.url("/v1/teams").await;
        let body = serde_json::json!({ "name": name, "members": members });
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("api returned status {}", resp.status()));
        }
        Ok(())
    }

    pub async fn list_teams(&self) -> Result<Vec<Team>, String> {
        let url = self.url("/v1/teams").await;
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("api returned status {}", resp.status()));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn list_instances(&self) -> Result<Vec<HttpInstance>, String> {
        let url = self.url("/v1/instances").await;
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("api returned status {}", resp.status()));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

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
pub struct HttpInstance {
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
}
