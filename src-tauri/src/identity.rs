use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ed25519_dalek::{SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

use crate::mua_auth::MuaAccount;

const IDENTITY_FILE: &str = "identity.json";
const KEY_FILE: &str = "key.bin";
const KEYCHAIN_SERVICE: &str = "FollyLauncher";
const KEYCHAIN_KEY_NAME: &str = "ed25519-private-key";
const CRL_CACHE_FILE: &str = "crl_cache.json";
const CRL_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub peer_id: String,
    pub public_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub club: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vc: Option<VerifiableCredential>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mua_account: Option<MuaAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serverless_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiableCredential {
    pub id: String,
    pub issuer: String,
    pub issued_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub subject: CredentialSubject,
    pub proof: CredentialProof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSubject {
    pub peer_id: String,
    pub claims: Vec<CredentialClaim>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialClaim {
    pub claim_type: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialProof {
    pub proof_type: String,
    pub created: String,
    pub verification_method: String,
    pub proof_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VcHolderState {
    Unverified,
    Member,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize)]
pub struct VcStatus {
    pub state: VcHolderState,
    pub role: Option<String>,
    pub club: Option<String>,
    pub issuer: Option<String>,
    pub verified: bool,
    pub crl_stale: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VcImportResult {
    pub state: VcHolderState,
    pub verified: bool,
    pub expired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrlEntry {
    pub vc_id: String,
    pub revoked_at: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CrlCache {
    entries: Vec<CrlEntry>,
    updated_at: Option<String>,
}

pub struct IdentityManager {
    data_dir: PathBuf,
    identity: Identity,
    signing_key: Option<SigningKey>,
    crl: CrlCache,
    crl_http_client: Option<reqwest::Client>,
}

impl IdentityManager {
    pub async fn load_or_create(data_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&data_dir).await?;
        let identity_path = data_dir.join(IDENTITY_FILE);

        if identity_path.exists() {
            let content = fs::read_to_string(&identity_path).await?;
            let identity: Identity = serde_json::from_str(&content)?;

            // Try keychain first, then fallback to file (legacy migration path)
            let (signing_key, migrated) = match load_signing_key_from_keychain() {
                Ok(key) => (Some(key), false),
                Err(keychain_err) => {
                    tracing::warn!(error = %keychain_err, "keychain read failed, trying legacy key.bin");
                    match load_signing_key_from_file(&data_dir).await {
                        Ok(key) => (Some(key), true),
                        Err(file_err) => {
                            tracing::error!(keychain_err = %keychain_err, file_err = %file_err, "unable to load signing key from either keychain or file");
                            (None, false)
                        }
                    }
                }
            };

            // Migrate file-stored key into keychain if possible
            if migrated {
                if let Some(ref key) = signing_key {
                    if let Err(e) = save_signing_key_to_keychain(key) {
                        tracing::warn!(error = %e, "failed to migrate legacy key to keychain");
                    } else {
                        tracing::info!("migrated signing key from file to keychain");
                        // Best-effort remove legacy key file
                        let _ = fs::remove_file(data_dir.join(KEY_FILE)).await;
                    }
                }
            }

            let crl = load_crl_cache(&data_dir).await.unwrap_or_default();

            return Ok(Self {
                data_dir,
                identity,
                signing_key,
                crl,
                crl_http_client: None,
            });
        }

        // Generate new identity
        let mut key_bytes = [0u8; 32];
        getrandom::fill(&mut key_bytes)
            .map_err(|e| anyhow::anyhow!("failed to generate random key: {}", e))?;
        let signing_key = SigningKey::from_bytes(&key_bytes);

        // Save to keychain (primary), fallback to file
        if let Err(e) = save_signing_key_to_keychain(&signing_key) {
            tracing::warn!(error = %e, "keychain unavailable, saving key to legacy file");
            fs::write(data_dir.join(KEY_FILE), &key_bytes).await?;
        }

        let public_key = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            signing_key.verifying_key().to_bytes(),
        );

        let mut key_bytes_mut = key_bytes;
        let libp2p_keypair =
            libp2p::identity::ed25519::Keypair::try_from_bytes(&mut key_bytes_mut)?;
        let peer_id = libp2p::identity::PublicKey::from(libp2p_keypair.public())
            .to_peer_id()
            .to_string();

        let identity = Identity {
            peer_id,
            public_key,
            club: None,
            vc: None,
            mua_account: None,
            serverless_token: None,
        };

        fs::write(&identity_path, serde_json::to_string_pretty(&identity)?).await?;

        Ok(Self {
            data_dir,
            identity,
            signing_key: Some(signing_key),
            crl: CrlCache::default(),
            crl_http_client: None,
        })
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn peer_id(&self) -> &str {
        &self.identity.peer_id
    }

    pub fn libp2p_keypair(&self) -> Result<libp2p::identity::Keypair> {
        let key = self
            .signing_key
            .as_ref()
            .context("signing key not available")?;
        let mut bytes = key.to_bytes();
        let ed25519 = libp2p::identity::ed25519::Keypair::try_from_bytes(&mut bytes)
            .context("failed to construct libp2p keypair")?;
        Ok(libp2p::identity::Keypair::from(ed25519))
    }

    pub fn vc_status(&self) -> VcStatus {
        match &self.identity.vc {
            Some(vc) => {
                let role = vc
                    .subject
                    .claims
                    .iter()
                    .find(|c| c.claim_type == "role")
                    .and_then(|c| c.value.as_str().map(|s| s.to_string()));
                let club = vc
                    .subject
                    .claims
                    .iter()
                    .find(|c| c.claim_type == "club_membership")
                    .and_then(|c| c.value.as_str().map(|s| s.to_string()));

                let verified = self.verify_vc(vc).unwrap_or(false);
                let state = if !verified {
                    VcHolderState::Unverified
                } else if self.is_vc_revoked(vc) {
                    VcHolderState::Revoked
                } else if self.is_vc_expired(vc) {
                    VcHolderState::Expired
                } else {
                    VcHolderState::Member
                };

                VcStatus {
                    state,
                    role,
                    club,
                    issuer: Some(vc.issuer.clone()),
                    verified,
                    crl_stale: self.crl_stale(),
                }
            }
            None => VcStatus {
                state: VcHolderState::Unverified,
                role: None,
                club: None,
                issuer: None,
                verified: false,
                crl_stale: self.crl_stale(),
            },
        }
    }

    pub async fn import_vc(&mut self, vc_json: &str) -> Result<VcImportResult> {
        let vc: VerifiableCredential =
            serde_json::from_str(vc_json).context("invalid VC JSON format")?;

        let verified = self.verify_vc(&vc).unwrap_or(false);
        let expired = self.is_vc_expired(&vc);

        let state = if !verified {
            VcHolderState::Unverified
        } else if self.is_vc_revoked(&vc) {
            VcHolderState::Revoked
        } else if expired {
            VcHolderState::Expired
        } else {
            VcHolderState::Member
        };

        let club = vc
            .subject
            .claims
            .iter()
            .find(|c| c.claim_type == "club_membership")
            .and_then(|c| c.value.as_str().map(|s| s.to_string()));

        self.identity.vc = Some(vc);
        if let Some(c) = club {
            self.identity.club = Some(c);
        }
        self.save().await?;

        Ok(VcImportResult {
            state,
            verified,
            expired,
        })
    }

    pub async fn clear_vc(&mut self) -> Result<()> {
        self.identity.vc = None;
        self.save().await
    }

    fn verify_vc(&self, vc: &VerifiableCredential) -> Result<bool> {
        if vc.proof.proof_type != "Ed25519Signature2020" {
            return Ok(false);
        }

        let signature_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &vc.proof.proof_value,
        )
        .context("invalid base64 in proof_value")?;

        let signature = ed25519_dalek::Signature::from_slice(&signature_bytes)
            .context("invalid signature format")?;

        let message = self.build_signing_message(vc);

        let issuer_pubkey = self.extract_issuer_pubkey(vc)?;

        match issuer_pubkey.verify(&message, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn is_vc_expired(&self, vc: &VerifiableCredential) -> bool {
        if let Some(expires_at) = &vc.expires_at {
            match DateTime::parse_from_rfc3339(expires_at) {
                Ok(expiry) => return Utc::now() > expiry.with_timezone(&Utc),
                Err(e) => {
                    tracing::warn!(error = %e, expires_at = %expires_at, "failed to parse VC expires_at");
                }
            }
        }
        false
    }

    fn build_signing_message(&self, vc: &VerifiableCredential) -> Vec<u8> {
        // Must match federated-server's canonicalize_vc_for_signing:
        // format!("{}|{}|{}|{}", vc.id, vc.issuer, vc.subject.peer_id, claims_json)
        let claims_json = serde_json::to_string(&vc.subject.claims).unwrap_or_default();
        format!(
            "{}|{}|{}|{}",
            vc.id, vc.issuer, vc.subject.peer_id, claims_json
        )
        .into_bytes()
    }

    fn extract_issuer_pubkey(&self, vc: &VerifiableCredential) -> Result<VerifyingKey> {
        let pk_b64 = vc
            .proof
            .verification_method
            .rsplit_once('#')
            .map(|(_, pk)| pk)
            .unwrap_or(&vc.proof.verification_method);

        let pk_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, pk_b64)
            .context("invalid base64 in verification_method")?;

        let pk_array: [u8; 32] = pk_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid public key length"))?;

        VerifyingKey::from_bytes(&pk_array).context("invalid ed25519 public key")
    }

    pub async fn set_mua_account(&mut self, account: MuaAccount) -> Result<()> {
        self.identity.mua_account = Some(account);
        self.save().await
    }

    pub async fn clear_mua_account(&mut self) -> Result<()> {
        self.identity.mua_account = None;
        self.save().await
    }

    pub fn mua_account(&self) -> Option<&MuaAccount> {
        self.identity.mua_account.as_ref()
    }

    pub async fn set_serverless_token(&mut self, token: String) -> Result<()> {
        self.identity.serverless_token = Some(token);
        self.save().await
    }

    pub async fn clear_serverless_token(&mut self) -> Result<()> {
        self.identity.serverless_token = None;
        self.save().await
    }

    pub fn serverless_token(&self) -> Option<&str> {
        self.identity.serverless_token.as_deref()
    }

    fn is_vc_revoked(&self, vc: &VerifiableCredential) -> bool {
        self.crl.entries.iter().any(|e| e.vc_id == vc.id)
    }

    pub fn crl_stale(&self) -> bool {
        match &self.crl.updated_at {
            Some(ts) => {
                if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                    let age = Utc::now().signed_duration_since(dt.with_timezone(&Utc));
                    age.num_seconds() as u64 > CRL_MAX_AGE.as_secs()
                } else {
                    true
                }
            }
            None => true,
        }
    }

    pub fn set_crl_http_client(&mut self, client: reqwest::Client) {
        self.crl_http_client = Some(client);
    }

    pub async fn refresh_crl(&mut self, api_url: &str) -> Result<()> {
        let client = self
            .crl_http_client
            .as_ref()
            .context("CRL HTTP client not configured")?;

        let url = format!("{}/v1/vc/revoked", api_url.trim_end_matches('/'));
        let resp = client
            .get(&url)
            .send()
            .await
            .context("failed to fetch CRL")?;

        if !resp.status().is_success() {
            anyhow::bail!("CRL endpoint returned {}", resp.status());
        }

        let entries: Vec<CrlEntry> = resp.json().await.context("invalid CRL response")?;

        self.crl = CrlCache {
            entries,
            updated_at: Some(Utc::now().to_rfc3339()),
        };
        save_crl_cache(&self.data_dir, &self.crl).await?;

        tracing::info!(
            crl_entries = self.crl.entries.len(),
            "CRL cache refreshed"
        );
        Ok(())
    }

    pub async fn try_auto_clear_revoked_vc(&mut self) -> bool {
        if let Some(vc) = &self.identity.vc {
            if self.is_vc_revoked(vc) {
                tracing::warn!(
                    vc_id = %vc.id,
                    "local VC found in revocation list, auto-clearing"
                );
                self.identity.vc = None;
                self.identity.club = None;
                if let Err(e) = self.save().await {
                    tracing::error!(error = %e, "failed to save identity after VC clear");
                }
                return true;
            }
        }
        false
    }

    async fn save(&self) -> Result<()> {
        let path = self.data_dir.join(IDENTITY_FILE);
        fs::write(&path, serde_json::to_string_pretty(&self.identity)?).await?;
        Ok(())
    }
}

fn save_signing_key_to_keychain(signing_key: &SigningKey) -> Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_KEY_NAME)
        .map_err(|e| anyhow::anyhow!("failed to create keychain entry: {}", e))?;
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        signing_key.to_bytes(),
    );
    entry
        .set_password(&encoded)
        .map_err(|e| anyhow::anyhow!("failed to store key in keychain: {}", e))?;
    Ok(())
}

fn load_signing_key_from_keychain() -> Result<SigningKey> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_KEY_NAME)
        .map_err(|e| anyhow::anyhow!("failed to create keychain entry: {}", e))?;
    let encoded = entry
        .get_password()
        .map_err(|e| anyhow::anyhow!("failed to read key from keychain: {}", e))?;
    let key_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &encoded,
    )?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid key length in keychain"))?;
    Ok(SigningKey::from_bytes(&key_array))
}

async fn load_signing_key_from_file(data_dir: &std::path::Path) -> Result<SigningKey> {
    let key_path = data_dir.join(KEY_FILE);
    let key_bytes = fs::read(&key_path).await?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid key file length"))?;
    Ok(SigningKey::from_bytes(&key_array))
}

async fn load_crl_cache(data_dir: &std::path::Path) -> Result<CrlCache> {
    let path = data_dir.join(CRL_CACHE_FILE);
    if path.exists() {
        let content = fs::read_to_string(&path).await?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(CrlCache::default())
    }
}

async fn save_crl_cache(data_dir: &std::path::Path, crl: &CrlCache) -> Result<()> {
    let path = data_dir.join(CRL_CACHE_FILE);
    fs::write(&path, serde_json::to_string_pretty(crl)?).await?;
    Ok(())
}
