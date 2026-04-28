use crate::api::{RoomConfig, YggdrasilServer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuaAccount {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub auth_server_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuaLoginStatus {
    pub logged_in: bool,
    pub username: Option<String>,
    pub uuid: Option<String>,
    pub auth_server_url: String,
    pub peer_bound: bool,
    pub is_guest: bool,
    pub is_member: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartAuthResponse {
    pub user_code: String,
    pub verification_uri: String,
}

#[derive(Debug, Deserialize)]
struct OpenIdConfig {
    #[serde(rename = "device_authorization_endpoint")]
    device_auth_endpoint: String,
    #[serde(rename = "token_endpoint")]
    token_endpoint: String,
}

#[derive(Debug, Deserialize)]
struct DeviceAuthResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(rename = "verification_uri_complete")]
    verification_uri_complete: Option<String>,
    interval: Option<u64>,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct OAuthTokens {
    access_token: String,
    #[serde(default)]
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
struct RefreshRequest {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    #[serde(rename = "selectedProfile")]
    selected_profile: Option<YggdrasilProfile>,
}

#[derive(Debug, Deserialize)]
struct YggdrasilProfile {
    id: String,
    name: String,
}

fn form_body(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&")
}

pub struct MuaAuthService {
    client: reqwest::Client,
    pending: Arc<Mutex<Option<PendingAuth>>>,
    auth_server_url: String,
    client_id: String,
    scope: String,
    room: RoomConfig,
}

struct PendingAuth {
    device_code: String,
    token_endpoint: String,
    interval: u64,
    expires_at: std::time::Instant,
    auth_server_url: String,
    client_id: String,
}

impl MuaAuthService {
    pub fn new(server: &YggdrasilServer, room: RoomConfig) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build reqwest client"),
            pending: Arc::new(Mutex::new(None)),
            auth_server_url: server.auth_server_url.clone(),
            client_id: server.client_id.clone(),
            scope: server.scope.clone(),
            room,
        }
    }

    pub fn auth_server_url(&self) -> &str {
        &self.auth_server_url
    }

    pub fn room_config(&self) -> RoomConfig {
        self.room.clone()
    }

    pub async fn start_device_auth(&self) -> anyhow::Result<StartAuthResponse> {
        let openid_config = self
            .fetch_openid_config(&self.auth_server_url)
            .await?;

        let body = form_body(&[
            ("client_id", &self.client_id),
            ("scope", &self.scope),
        ]);
        let resp = self
            .client
            .post(&openid_config.device_auth_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.map_err(|e| {
                anyhow::anyhow!(
                    "device auth request failed and response body was unreadable: {}",
                    e
                )
            })?;
            anyhow::bail!("device auth request failed: {}", text);
        }

        let data: DeviceAuthResponse = resp.json().await?;

        info!(
            user_code = %data.user_code,
            verification_uri = %data.verification_uri,
            "MUA device auth started"
        );

        let pending = PendingAuth {
            device_code: data.device_code.clone(),
            token_endpoint: openid_config.token_endpoint.clone(),
            interval: data
                .interval
                .ok_or_else(|| anyhow::anyhow!("device auth response missing interval"))?,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(data.expires_in),
            auth_server_url: self.auth_server_url.clone(),
            client_id: self.client_id.clone(),
        };

        *self.pending.lock().await = Some(pending);

        Ok(StartAuthResponse {
            user_code: data.user_code,
            verification_uri: data.verification_uri_complete.ok_or_else(|| {
                anyhow::anyhow!("device auth response missing verification_uri_complete")
            })?,
        })
    }

    pub async fn poll_token(&self) -> anyhow::Result<MuaAccount> {
        let pending = self
            .pending
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow::anyhow!("no pending auth session"))?;

        let tokens = self.do_oauth_polling(&pending).await?;

        let profile = self
            .fetch_yggdrasil_profile(&pending.auth_server_url, &tokens.access_token)
            .await?;

        info!(
            username = %profile.name,
            uuid = %profile.id,
            "MUA login successful"
        );

        Ok(MuaAccount {
            username: profile.name,
            uuid: profile.id,
            access_token: tokens.access_token,
            refresh_token: if tokens.refresh_token.is_empty() {
                None
            } else {
                Some(tokens.refresh_token)
            },
            auth_server_url: pending.auth_server_url,
        })
    }

    async fn fetch_openid_config(&self, auth_server_url: &str) -> anyhow::Result<OpenIdConfig> {
        let url = format!(
            "{}/.well-known/openid-configuration",
            auth_server_url.trim_end_matches('/')
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("failed to fetch OpenID configuration: {}", resp.status());
        }
        Ok(resp.json().await?)
    }

    async fn do_oauth_polling(&self, pending: &PendingAuth) -> anyhow::Result<OAuthTokens> {
        let body = form_body(&[
            ("client_id", &pending.client_id),
            ("device_code", &pending.device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ]);
        let sender = self
            .client
            .post(&pending.token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body);

        let mut interval = pending.interval;
        let start = std::time::Instant::now();

        loop {
            let resp = sender
                .try_clone()
                .ok_or_else(|| anyhow::anyhow!("failed to clone request"))?
                .send()
                .await?;

            if resp.status().is_success() {
                return Ok(resp.json().await?);
            }

            if resp.status().as_u16() != 400 {
                anyhow::bail!("token endpoint error: {}", resp.status());
            }

            let err: OAuthErrorResponse = resp.json().await?;
            match err.error.as_str() {
                "authorization_pending" => {}
                "slow_down" => {
                    interval += 5;
                }
                "access_denied" => {
                    anyhow::bail!("user denied authorization");
                }
                "expired_token" => {
                    anyhow::bail!("device code expired");
                }
                _ => {
                    anyhow::bail!("oauth error: {}", err.error);
                }
            }

            if start.elapsed() >= pending.expires_at.duration_since(start) {
                anyhow::bail!("device auth expired");
            }

            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        }
    }

    pub async fn get_skin_textures(
        &self,
        auth_server_url: &str,
        access_token: &str,
        uuid: &str,
    ) -> anyhow::Result<SkinTextures> {
        let url = format!(
            "{}/sessionserver/session/minecraft/profile/{}",
            auth_server_url.trim_end_matches('/'),
            uuid
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.map_err(|e| {
                anyhow::anyhow!(
                    "failed to fetch skin profile and response body was unreadable: {}",
                    e
                )
            })?;
            anyhow::bail!("failed to fetch skin profile: {}", text);
        }

        let profile: SessionProfile = resp.json().await?;

        let mut skin_url = None;
        let mut cape_url = None;

        for prop in profile.properties {
            if prop.name == "textures" {
                let decoded = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    &prop.value,
                )?;
                let textures: TexturesPayload = serde_json::from_slice(&decoded)?;
                if let Some(skin) = textures.textures.skin {
                    skin_url = Some(skin.url);
                }
                if let Some(cape) = textures.textures.cape {
                    cape_url = Some(cape.url);
                }
                break;
            }
        }

        Ok(SkinTextures { skin_url, cape_url })
    }

    async fn fetch_yggdrasil_profile(
        &self,
        auth_server_url: &str,
        access_token: &str,
    ) -> anyhow::Result<YggdrasilProfile> {
        let url = format!(
            "{}/authserver/refresh",
            auth_server_url.trim_end_matches('/')
        );

        let resp = self
            .client
            .post(&url)
            .json(&RefreshRequest {
                access_token: access_token.to_string(),
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.map_err(|e| {
                anyhow::anyhow!(
                    "failed to fetch Yggdrasil profile and response body was unreadable: {}",
                    e
                )
            })?;
            anyhow::bail!("failed to fetch Yggdrasil profile: {}", text);
        }

        let data: RefreshResponse = resp.json().await?;

        data.selected_profile
            .ok_or_else(|| anyhow::anyhow!("no selected profile in response"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinTextures {
    pub skin_url: Option<String>,
    pub cape_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionProfile {
    properties: Vec<ProfileProperty>,
}

#[derive(Debug, Deserialize)]
struct ProfileProperty {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct TexturesPayload {
    textures: TextureMap,
}

#[derive(Debug, Deserialize)]
struct TextureMap {
    skin: Option<TextureInfo>,
    cape: Option<TextureInfo>,
}

#[derive(Debug, Deserialize)]
struct TextureInfo {
    url: String,
}
