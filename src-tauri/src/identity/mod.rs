pub mod commands;

pub mod w3c_vc {
    use super::*;
    use anyhow::{Context, Result};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct W3cCredential {
        #[serde(rename = "@context")]
        pub context: Vec<String>,
        pub id: Option<String>,
        #[serde(rename = "type")]
        pub type_: Vec<String>,
        pub issuer: serde_json::Value,
        pub valid_from: Option<String>,
        pub valid_until: Option<String>,
        pub credential_subject: serde_json::Value,
        pub credential_status: Option<serde_json::Value>,
        pub proof: Option<W3cProof>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct W3cProof {
        #[serde(rename = "type")]
        pub type_: String,
        pub proof_purpose: Option<String>,
        pub cryptosuite: Option<String>,
        pub created: Option<String>,
        pub verification_method: String,
        pub proof_value: String,
    }

    /// A DID Document for resolving verification methods.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DidDocument {
        pub id: String,
        #[serde(default)]
        pub verification_method: Vec<VerificationMethodEntry>,
    }

    /// A single verification method entry in a DID Document.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VerificationMethodEntry {
        pub id: String,
        #[serde(rename = "type")]
        pub type_: String,
        pub controller: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub public_key_multibase: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub public_key_base58: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub public_key_jwk: Option<serde_json::Value>,
    }

    /// Resolve a DID into a DID Document.
    ///
    /// Supports:
    /// - `did:key` — local/offline Ed25519 key decoding via multibase.
    /// - `did:web` — fetch `https://{domain}/.well-known/did.json` via HTTP
    ///   (timeout-based; tests may inject a mock).
    /// - Unknown methods return `Err` with a "method not supported" message.
    pub async fn resolve_did(did: &str) -> Result<DidDocument> {
        if let Some(key_part) = did.strip_prefix("did:key:") {
            return resolve_did_key(key_part).context("failed to resolve did:key");
        }
        if let Some(web_part) = did.strip_prefix("did:web:") {
            return resolve_did_web(web_part)
                .await
                .context("failed to resolve did:web");
        }
        anyhow::bail!("unsupported DID method: {did}");
    }

    /// Resolve a `did:key` to a DID Document with an Ed25519 verification method.
    fn resolve_did_key(key_part: &str) -> Result<DidDocument> {
        let did = format!("did:key:{key_part}");
        // Multibase: 'z' prefix = base58btc, rest is codec + key material.
        let (_base, decoded) =
            multibase::decode(key_part).context("invalid multibase in did:key")?;
        // Ed25519 multicodec: 0xed 0x01 prefix (varint). Check for 2-byte prefix.
        let raw_key = if decoded.len() >= 2 && decoded[0] == 0xed && decoded[1] == 0x01 {
            &decoded[2..]
        } else {
            anyhow::bail!("did:key multicodec is not Ed25519 (0xed01)");
        };
        if raw_key.len() != 32 {
            anyhow::bail!("invalid Ed25519 key length in did:key");
        }
        let vm_id = format!("{did}#{key_part}");
        let pk_multibase = key_part.to_string();
        let vm = VerificationMethodEntry {
            id: vm_id,
            type_: "Ed25519VerificationKey2020".to_string(),
            controller: did.clone(),
            public_key_multibase: Some(pk_multibase),
            public_key_base58: None,
            public_key_jwk: None,
        };
        Ok(DidDocument {
            id: did,
            verification_method: vec![vm],
        })
    }

    /// Resolve a `did:web` by fetching `https://{domain}/.well-known/did.json`.
    ///
    /// Uses a 10-second HTTP timeout and structured failure.
    /// For testing, an internal static mock can be injected via `DID_WEB_MOCK`.
    async fn resolve_did_web(domain: &str) -> Result<DidDocument> {
        // Check for mock injection (test-only path)
        {
            let mock = DID_WEB_MOCK.lock().unwrap();
            if let Some(ref doc_json) = *mock {
                let doc: DidDocument = serde_json::from_str(doc_json)
                    .context("failed to parse mock did:web document")?;
                return Ok(doc);
            }
        }
        let url = format!("https://{domain}/.well-known/did.json");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("failed to build HTTP client for did:web")?;
        let resp = client
            .get(&url)
            .send()
            .await
            .context("did:web HTTP request failed")?;
        if !resp.status().is_success() {
            anyhow::bail!("did:web returned HTTP {}", resp.status());
        }
        let body = resp
            .text()
            .await
            .context("failed to read did:web response body")?;
        let doc: DidDocument =
            serde_json::from_str(&body).context("invalid did:web DID Document JSON")?;
        if doc.id != format!("did:web:{domain}") {
            anyhow::bail!(
                "did:web document id mismatch: expected did:web:{domain}, got {}",
                doc.id
            );
        }
        Ok(doc)
    }

    /// Test-only global mock for did:web resolution.
    /// Set via `set_mock_did_web()` in test cases.
    static DID_WEB_MOCK: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

    #[cfg(test)]
    pub fn set_mock_did_web(doc_json: &str) {
        *DID_WEB_MOCK.lock().unwrap() = Some(doc_json.to_string());
    }

    #[cfg(test)]
    pub fn clear_mock_did_web() {
        *DID_WEB_MOCK.lock().unwrap() = None;
    }

    /// Find a verification method in a DID Document by id or fragment.
    ///
    /// Matches by full `vm.id == vm_id` or by fragment (`did:...#fragment`).
    pub fn find_verification_method<'a>(
        doc: &'a DidDocument,
        vm_id: &str,
    ) -> Option<&'a VerificationMethodEntry> {
        // Exact match on id
        let exact = doc.verification_method.iter().find(|vm| vm.id == vm_id);
        if exact.is_some() {
            return exact;
        }
        // Fragment match: extract fragment after '#'
        let fragment = vm_id.rsplit_once('#').map(|(_, f)| f);
        if let Some(frag) = fragment {
            return doc
                .verification_method
                .iter()
                .find(|vm| vm.id.rsplit_once('#').map(|(_, f)| f) == Some(frag));
        }
        None
    }

    /// Extract an Ed25519 public key (32 bytes) from a verification method entry.
    ///
    /// Supports:
    /// - `publicKeyMultibase` (multibase-encoded, e.g. `z6Mk...`)
    /// - `publicKeyBase58` (base58btc raw Ed25519 bytes)
    /// - `publicKeyJwk` (JWK with `kty: "OKP"`, `crv: "Ed25519"`, `x` as base64url)
    pub fn extract_ed25519_pubkey(vm: &VerificationMethodEntry) -> Result<[u8; 32]> {
        // 1. publicKeyMultibase
        if let Some(ref mb) = vm.public_key_multibase {
            let (_base, decoded) =
                multibase::decode(mb).context("invalid multibase in verificationMethod")?;
            // Ed25519 multicodec prefix: 0xed 0x01
            let key = if decoded.len() >= 2 && decoded[0] == 0xed && decoded[1] == 0x01 {
                &decoded[2..]
            } else if decoded.len() == 32 {
                // Bare key without multicodec (lenient)
                &decoded[..]
            } else {
                anyhow::bail!("unexpected multibase length/codec in verificationMethod");
            };
            let arr: [u8; 32] = key
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid Ed25519 key length in verificationMethod"))?;
            return Ok(arr);
        }
        // 2. publicKeyBase58
        if let Some(ref b58) = vm.public_key_base58 {
            let decoded = bs58::decode(b58)
                .into_vec()
                .context("invalid base58 in verificationMethod")?;
            let arr: [u8; 32] = decoded.try_into().map_err(|_| {
                anyhow::anyhow!("invalid publicKeyBase58 length in verificationMethod")
            })?;
            return Ok(arr);
        }
        // 3. publicKeyJwk (JWK)
        if let Some(ref jwk) = vm.public_key_jwk {
            let kty = jwk
                .get("kty")
                .and_then(|v| v.as_str())
                .context("JWK missing kty")?;
            let crv = jwk
                .get("crv")
                .and_then(|v| v.as_str())
                .context("JWK missing crv")?;
            if kty != "OKP" || crv != "Ed25519" {
                anyhow::bail!("unsupported JWK: kty={kty}, crv={crv}");
            }
            let x = jwk
                .get("x")
                .and_then(|v| v.as_str())
                .context("JWK missing x coordinate")?;
            let key_bytes =
                base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, x)
                    .context("invalid JWK x base64url")?;
            let arr: [u8; 32] = key_bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid JWK key length"))?;
            return Ok(arr);
        }
        anyhow::bail!("no supported public key material in verificationMethod")
    }

    /// Check if a string looks like a resolvable DID URL
    /// (starts with `did:key:` or `did:web:`). Legacy `did:ex` is
    /// not a real DID method and should fall through to the simplified
    /// base64-based verification method.
    pub fn is_did_url(s: &str) -> bool {
        s.starts_with("did:key:") || s.starts_with("did:web:")
    }

    pub fn normalize_w3c_vc(w3c: &W3cCredential) -> Result<VerifiableCredential> {
        let id = w3c
            .id
            .clone()
            .unwrap_or_else(|| "w3c-vc:unknown".to_string());
        let issuer = normalize_issuer(&w3c.issuer)?;
        let issued_at = w3c
            .valid_from
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        let expires_at = w3c.valid_until.clone();
        let subject = normalize_subject(&w3c.credential_subject)
            .context("failed to normalize credentialSubject")?;
        let proof = match &w3c.proof {
            Some(p) => normalize_proof(p).context("failed to normalize proof")?,
            None => anyhow::bail!("W3C VC is missing proof section"),
        };
        Ok(VerifiableCredential {
            id,
            issuer,
            issued_at,
            expires_at,
            subject,
            proof,
        })
    }

    pub fn is_w3c_format(vc_json: &str) -> bool {
        vc_json.contains("\"@context\"") || vc_json.contains("\"credentialSubject\"")
    }

    pub fn parse_and_normalize_any(vc_json: &str) -> Result<VerifiableCredential> {
        if is_w3c_format(vc_json) {
            let w3c: W3cCredential =
                serde_json::from_str(vc_json).context("invalid W3C VC JSON format")?;
            normalize_w3c_vc(&w3c)
        } else {
            let vc: VerifiableCredential =
                serde_json::from_str(vc_json).context("invalid legacy VC JSON format")?;
            Ok(vc)
        }
    }

    fn normalize_issuer(issuer: &serde_json::Value) -> Result<String> {
        match issuer {
            serde_json::Value::String(s) => Ok(s.clone()),
            serde_json::Value::Object(obj) => obj
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .context("W3C VC issuer object missing 'id' field"),
            _ => anyhow::bail!("W3C VC issuer must be a string or object"),
        }
    }

    fn normalize_subject(subject: &serde_json::Value) -> Result<CredentialSubject> {
        match subject {
            serde_json::Value::Object(obj) => {
                let peer_id = obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .or_else(|| obj.get("peer_id").and_then(|v| v.as_str()))
                    .context("W3C VC credentialSubject missing 'id' or 'peer_id'")?
                    .to_string();
                let claims = extract_claims(obj);
                Ok(CredentialSubject { peer_id, claims })
            }
            serde_json::Value::Array(arr) => {
                let first = arr.first().context("empty credentialSubject array")?;
                normalize_subject(first)
            }
            _ => anyhow::bail!("W3C VC credentialSubject must be an object or array"),
        }
    }

    fn extract_claims(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<CredentialClaim> {
        let mut claims = Vec::new();
        if let Some(role) = obj.get("role") {
            claims.push(CredentialClaim {
                claim_type: "role".to_string(),
                value: role.clone(),
            });
        }
        if let Some(club) = obj.get("club_membership") {
            claims.push(CredentialClaim {
                claim_type: "club_membership".to_string(),
                value: club.clone(),
            });
        }
        if let Some(club) = obj.get("club") {
            if !claims.iter().any(|c| c.claim_type == "club_membership") {
                claims.push(CredentialClaim {
                    claim_type: "club_membership".to_string(),
                    value: club.clone(),
                });
            }
        }
        if let Some(ct) = obj.get("credential_type") {
            claims.push(CredentialClaim {
                claim_type: "credential_type".to_string(),
                value: ct.clone(),
            });
        }
        if let Some(type_val) = obj.get("type") {
            claims.push(CredentialClaim {
                claim_type: "credential_type".to_string(),
                value: type_val.clone(),
            });
        }
        claims
    }

    fn normalize_proof(proof: &W3cProof) -> Result<CredentialProof> {
        let proof_type = match &proof.type_[..] {
            "Ed25519Signature2020" | "DataIntegrityProof" | "Ed25519Signature2018" => {
                "Ed25519Signature2020".to_string()
            }
            other => other.to_string(),
        };
        let created = proof
            .created
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        Ok(CredentialProof {
            proof_type,
            created,
            verification_method: proof.verification_method.clone(),
            proof_value: proof.proof_value.clone(),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use chrono::Utc;

        fn make_w3c_fixture(
            id: &str,
            issuer_str: &str,
            subject_role: &str,
            subject_club: Option<&str>,
            proof_type: &str,
            verification_method: &str,
            proof_value: &str,
        ) -> serde_json::Value {
            let mut subject = serde_json::json!({"id": format!("did:key:test-subject-{}", id), "role": subject_role});
            if let Some(club) = subject_club {
                subject["club_membership"] = serde_json::json!(club);
            }
            serde_json::json!({"@context": ["https://www.w3.org/ns/credentials/v2", "https://w3id.org/security/data-integrity/v2"], "id": id, "type": ["VerifiableCredential", "MemberCredential"], "issuer": issuer_str, "validFrom": Utc::now().to_rfc3339(), "validUntil": (Utc::now() + chrono::Duration::days(365)).to_rfc3339(), "credentialSubject": subject, "proof": {"type": proof_type, "proofPurpose": "assertionMethod", "cryptosuite": "eddsa-rdfc-2022", "created": Utc::now().to_rfc3339(), "verificationMethod": verification_method, "proofValue": proof_value}})
        }

        #[test]
        fn test_parse_server_fixture() {
            let f = make_w3c_fixture(
                "https://jlucraft.org/credentials/vc-test-001",
                "did:key:zIssuer",
                "member",
                Some("builders"),
                "DataIntegrityProof",
                "did:key:zIssuer#k1",
                "zBase58",
            );
            let json = serde_json::to_string(&f).unwrap();
            assert!(is_w3c_format(&json));
            let vc = parse_and_normalize_any(&json).unwrap();
            assert_eq!(vc.proof.proof_type, "Ed25519Signature2020");
        }
        #[test]
        fn test_issuer_string() {
            assert_eq!(
                normalize_issuer(&serde_json::json!("did:key:zIssuer")).unwrap(),
                "did:key:zIssuer"
            );
        }
        #[test]
        fn test_issuer_object() {
            assert_eq!(
                normalize_issuer(&serde_json::json!({"id": "did:key:z"})).unwrap(),
                "did:key:z"
            );
        }
        #[test]
        fn test_issuer_no_id_fails() {
            assert!(normalize_issuer(&serde_json::json!({"name": "X"})).is_err());
        }
        #[test]
        fn test_subject_normalize() {
            let s = normalize_subject(&serde_json::json!({"id": "did:key:subj", "role": "admin"}))
                .unwrap();
            assert_eq!(s.peer_id, "did:key:subj");
        }
        #[test]
        fn test_legacy_vc_accepted() {
            let v = serde_json::json!({"id":"vc:l","issuer":"did:x","issued_at":"2025-01-01T00:00:00Z","subject":{"peer_id":"12D","claims":[]},"proof":{"proof_type":"Ed25519Signature2020","created":"2025-01-01T00:00:00Z","verification_method":"did:x#k","proof_value":""}});
            assert!(parse_and_normalize_any(&serde_json::to_string(&v).unwrap()).is_ok());
        }
        #[test]
        fn test_missing_proof_fails() {
            let v = serde_json::json!({"@context":["https://www.w3.org/ns/credentials/v2"],"id":"vc:x","type":["V"],"issuer":"did:x","credentialSubject":{"id":"did:s"}});
            assert!(parse_and_normalize_any(&serde_json::to_string(&v).unwrap()).is_err());
        }
        #[test]
        fn test_invalid_json() {
            assert!(parse_and_normalize_any("not json").is_err());
        }
    }
}

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ed25519_dalek::{SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

use crate::mua_auth::{MuaAccount, MuaLoginStatus};

const IDENTITY_FILE: &str = "identity.json";
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

#[derive(Debug, Clone, Serialize)]
pub struct OnboardingStatus {
    pub peer_id: String,
    pub has_identity: bool,
    pub vc_state: VcHolderState,
    pub mua_logged_in: bool,
    pub is_guest: bool,
    pub is_member: bool,
    pub club: Option<String>,
    pub crl_stale: bool,
    pub mode_label: String,
    pub next_steps: Vec<String>,
    pub requires_network: bool,
}

pub fn build_onboarding_status(
    identity: &Identity,
    vc_status: &VcStatus,
    mua_status: &MuaLoginStatus,
) -> OnboardingStatus {
    let mut next_steps: Vec<String> = Vec::new();
    let requires_network = vc_status.crl_stale;
    let (mode_label, is_member, is_guest) = if vc_status.state == VcHolderState::Member {
        ("平台身份".to_string(), true, false)
    } else if mua_status.logged_in {
        ("MUA 访客".to_string(), false, true)
    } else {
        ("未验证 / 游客模式".to_string(), false, false)
    };
    match vc_status.state {
        VcHolderState::Member => {}
        VcHolderState::Expired => {
            next_steps.push("你的 VC 已过期，请续签或联系社长申请新 VC".to_string())
        }
        VcHolderState::Revoked => {
            next_steps.push("你的 VC 已被吊销，本地 VC 已清空。如需重新加入请联系社长".to_string())
        }
        VcHolderState::Unverified => {
            if mua_status.logged_in {
                next_steps.push(
                    "当前为 MUA 访客模式——仅游戏功能，无治理/积分。如需完整权限请联系社长申请 VC"
                        .to_string(),
                );
            }
        }
    }
    if vc_status.state != VcHolderState::Member && !mua_status.logged_in {
        next_steps.push("选择 MUA 登录（仅游戏功能），或导入 VC 以获取完整平台身份".to_string());
        next_steps.push("暂无 VC？联系社长并提供你的 PeerID 申请签发".to_string());
    }
    if vc_status.crl_stale
        && (vc_status.state == VcHolderState::Member || vc_status.state == VcHolderState::Expired)
    {
        next_steps.push("吊销列表缓存已过期，请联网刷新以确保安全".to_string());
    }
    next_steps.dedup();
    OnboardingStatus {
        peer_id: identity.peer_id.clone(),
        has_identity: true,
        vc_state: vc_status.state.clone(),
        mua_logged_in: mua_status.logged_in,
        is_guest,
        is_member,
        club: vc_status.club.clone(),
        crl_stale: vc_status.crl_stale,
        mode_label,
        next_steps,
        requires_network,
    }
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
}

impl IdentityManager {
    pub async fn load_or_create(data_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&data_dir).await?;
        let identity_path = data_dir.join(IDENTITY_FILE);
        if identity_path.exists() {
            let content = fs::read_to_string(&identity_path).await?;
            let identity: Identity = serde_json::from_str(&content)?;
            let signing_key = load_signing_key_from_keychain()
                .context("identity exists but signing key is unavailable in keychain")?;
            let crl = load_crl_cache(&data_dir).await.unwrap_or_default();
            return Ok(Self {
                data_dir,
                identity,
                signing_key: Some(signing_key),
                crl,
            });
        }
        let mut key_bytes = [0u8; 32];
        getrandom::fill(&mut key_bytes)
            .map_err(|e| anyhow::anyhow!("failed to generate random key: {e}"))?;
        let signing_key = SigningKey::from_bytes(&key_bytes);
        save_signing_key_to_keychain(&signing_key)
            .context("failed to store signing key in keychain")?;
        let public_key = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            signing_key.verifying_key().to_bytes(),
        );
        let mut key_bytes_mut = key_bytes;
        let pk = libp2p::identity::ed25519::Keypair::try_from_bytes(&mut key_bytes_mut)?;
        let peer_id = libp2p::identity::PublicKey::from(pk.public())
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
        let ed = libp2p::identity::ed25519::Keypair::try_from_bytes(&mut bytes)
            .context("failed to construct libp2p keypair")?;
        Ok(libp2p::identity::Keypair::from(ed))
    }

    pub async fn vc_status(&self) -> VcStatus {
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
                let verified = self.verify_vc(vc).await.unwrap_or(false);
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
        let vc = w3c_vc::parse_and_normalize_any(vc_json)
            .context("failed to parse VC JSON (neither W3C nor legacy format)")?;
        let verified = self.verify_vc(&vc).await?;
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
        self.identity.club = None;
        self.save().await
    }

    async fn verify_vc(&self, vc: &VerifiableCredential) -> Result<bool> {
        if vc.proof.proof_type != "Ed25519Signature2020" {
            return Ok(false);
        }
        let trimmed = vc.proof.verification_method.trim();
        if trimmed.is_empty() {
            return Err(anyhow::anyhow!("missing verification_method in VC proof"));
        }
        let sig_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &vc.proof.proof_value,
        )
        .context("invalid base64 in proof_value")?;
        let sig =
            ed25519_dalek::Signature::from_slice(&sig_bytes).context("invalid signature format")?;
        let claims = serde_json::to_string(&vc.subject.claims).unwrap_or_default();
        let msg = format!("{}|{}|{}|{}", vc.id, vc.issuer, vc.subject.peer_id, claims).into_bytes();
        let pk = self.extract_issuer_pubkey(vc).await?;
        match pk.verify(&msg, &sig) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn is_vc_expired(&self, vc: &VerifiableCredential) -> bool {
        if let Some(exp) = &vc.expires_at {
            if let Ok(exp_dt) = DateTime::parse_from_rfc3339(exp) {
                return Utc::now() > exp_dt.with_timezone(&Utc);
            }
        }
        false
    }

    async fn extract_issuer_pubkey(&self, vc: &VerifiableCredential) -> Result<VerifyingKey> {
        let vm = &vc.proof.verification_method;
        // Phase 5 P0: If verificationMethod is a DID URL, resolve the DID
        // Document and extract the Ed25519 public key from the matching
        // verificationMethod entry. Legacy simplified keys (non-DID) are
        // supported for compatibility but must not override a resolvable DID URL.
        if w3c_vc::is_did_url(vm) {
            // Resolve the issuer DID (the controller DID, before '#')
            let did = if let Some((controller, _fragment)) = vm.rsplit_once('#') {
                controller.to_string()
            } else {
                vm.clone()
            };
            let doc = w3c_vc::resolve_did(&did).await?;
            let vm_entry = w3c_vc::find_verification_method(&doc, vm)
                .context(format!("verificationMethod {vm} not found in DID Document"))?;
            let pk_bytes = w3c_vc::extract_ed25519_pubkey(vm_entry)
                .context("failed to extract Ed25519 public key from verificationMethod")?;
            return VerifyingKey::from_bytes(&pk_bytes)
                .context("invalid Ed25519 pubkey from DID Document");
        }
        // Legacy simplified mode: fragment after '#' or whole string is
        // standard base64-encoded Ed25519 public key bytes.
        let pk_b64 = vm.rsplit_once('#').map(|(_, pk)| pk).unwrap_or(vm);
        let pk_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, pk_b64)
            .context("invalid base64 in legacy verificationMethod")?;
        let arr: [u8; 32] = pk_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid pk length in legacy verificationMethod"))?;
        VerifyingKey::from_bytes(&arr)
            .context("invalid ed25519 pubkey in legacy verificationMethod")
    }

    pub async fn set_mua_account(&mut self, a: MuaAccount) -> Result<()> {
        self.identity.mua_account = Some(a);
        self.save().await
    }
    pub async fn clear_mua_account(&mut self) -> Result<()> {
        self.identity.mua_account = None;
        self.save().await
    }
    pub fn mua_account(&self) -> Option<&MuaAccount> {
        self.identity.mua_account.as_ref()
    }
    pub async fn set_serverless_token(&mut self, t: String) -> Result<()> {
        self.identity.serverless_token = Some(t);
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
            Some(ts) => match DateTime::parse_from_rfc3339(ts) {
                Ok(dt) => {
                    Utc::now()
                        .signed_duration_since(dt.with_timezone(&Utc))
                        .num_seconds() as u64
                        > CRL_MAX_AGE.as_secs()
                }
                Err(_) => true,
            },
            None => true,
        }
    }

    /// Update CRL entries from a list of revoked credential IDs.
    ///
    /// Called after fetching the list via `ControlClient::list_revoked_credentials()`.
    pub async fn update_crl_entries(&mut self, revoked_ids: Vec<String>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.crl = CrlCache {
            entries: revoked_ids
                .into_iter()
                .map(|id| CrlEntry {
                    vc_id: id,
                    revoked_at: now.clone(),
                    reason: None,
                })
                .collect(),
            updated_at: Some(now),
        };
        save_crl_cache(&self.data_dir, &self.crl).await?;
        Ok(())
    }

    pub async fn try_auto_clear_revoked_vc(&mut self) -> bool {
        if let Some(vc) = &self.identity.vc {
            if self.is_vc_revoked(vc) {
                self.identity.vc = None;
                self.identity.club = None;
                let _ = self.save().await;
                return true;
            }
        }
        false
    }

    async fn save(&self) -> Result<()> {
        let p = self.data_dir.join(IDENTITY_FILE);
        fs::write(&p, serde_json::to_string_pretty(&self.identity)?).await?;
        Ok(())
    }
}

fn save_signing_key_to_keychain(k: &SigningKey) -> Result<()> {
    let e = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_KEY_NAME)
        .map_err(|x| anyhow::anyhow!("keychain: {x}"))?;
    let enc = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, k.to_bytes());
    e.set_password(&enc)
        .map_err(|x| anyhow::anyhow!("keychain set: {x}"))
}

fn load_signing_key_from_keychain() -> Result<SigningKey> {
    let e = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_KEY_NAME)
        .map_err(|x| anyhow::anyhow!("keychain: {x}"))?;
    let enc = e
        .get_password()
        .map_err(|x| anyhow::anyhow!("keychain read: {x}"))?;
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &enc)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("bad key len"))?;
    Ok(SigningKey::from_bytes(&arr))
}

async fn load_crl_cache(d: &std::path::Path) -> Result<CrlCache> {
    let p = d.join(CRL_CACHE_FILE);
    if p.exists() {
        Ok(serde_json::from_str(&fs::read_to_string(&p).await?)?)
    } else {
        Ok(CrlCache::default())
    }
}

async fn save_crl_cache(d: &std::path::Path, c: &CrlCache) -> Result<()> {
    fs::write(d.join(CRL_CACHE_FILE), serde_json::to_string_pretty(c)?).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;
    use tempfile::TempDir;

    fn minimal_im(dir: &TempDir) -> IdentityManager {
        IdentityManager {
            data_dir: dir.path().to_path_buf(),
            identity: Identity {
                peer_id: "12D3KooWTest".into(),
                public_key: base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    [0u8; 32],
                ),
                club: None,
                vc: None,
                mua_account: None,
                serverless_token: None,
            },
            signing_key: None,
            crl: CrlCache::default(),
        }
    }
    fn mk_keys() -> (SigningKey, VerifyingKey) {
        let mut s = [0u8; 32];
        getrandom::fill(&mut s).unwrap();
        let sk = SigningKey::from_bytes(&s);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    fn sign_vc(
        vc: &VerifiableCredential,
        sk: &SigningKey,
        vk: &VerifyingKey,
    ) -> VerifiableCredential {
        let claims = serde_json::to_string(&vc.subject.claims).unwrap_or_default();
        let msg = format!("{}|{}|{}|{}", vc.id, vc.issuer, vc.subject.peer_id, claims);
        let sig = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            sk.sign(msg.as_bytes()).to_bytes(),
        );
        let vk_b =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, vk.to_bytes());
        let mut v = vc.clone();
        v.proof.proof_value = sig;
        v.proof.verification_method = format!("did:ex#{}", vk_b);
        v
    }

    #[tokio::test]
    async fn test_vc_verify() {
        let d = TempDir::new().unwrap();
        let m = minimal_im(&d);
        let (sk, vk) = mk_keys();
        let vc = VerifiableCredential {
            id: "vc:ok".into(),
            issuer: "did:j".into(),
            issued_at: Utc::now().to_rfc3339(),
            expires_at: None,
            subject: CredentialSubject {
                peer_id: "12D".into(),
                claims: vec![],
            },
            proof: CredentialProof {
                proof_type: "Ed25519Signature2020".into(),
                created: Utc::now().to_rfc3339(),
                verification_method: "did:ex#k".into(),
                proof_value: String::new(),
            },
        };
        assert!(m.verify_vc(&sign_vc(&vc, &sk, &vk)).await.unwrap());
    }
    #[tokio::test]
    async fn test_vc_bad_sig() {
        let d = TempDir::new().unwrap();
        let m = minimal_im(&d);
        let (_, vk) = mk_keys();
        let (sk_b, _) = mk_keys();
        let vc = VerifiableCredential {
            id: "vc:bad".into(),
            issuer: "did:j".into(),
            issued_at: Utc::now().to_rfc3339(),
            expires_at: None,
            subject: CredentialSubject {
                peer_id: "12D".into(),
                claims: vec![],
            },
            proof: CredentialProof {
                proof_type: "Ed25519Signature2020".into(),
                created: Utc::now().to_rfc3339(),
                verification_method: "did:ex#k".into(),
                proof_value: String::new(),
            },
        };
        assert!(!m.verify_vc(&sign_vc(&vc, &sk_b, &vk)).await.unwrap());
    }

    // ── Phase 5 P0: DID resolution tests ─────────────────────────────

    /// Create a did:key-based VC signed by a known Ed25519 key.
    fn make_did_key_vc(sk: &SigningKey, vk: &VerifyingKey) -> (VerifiableCredential, String) {
        // Construct a multibase did:key
        let mut codec: Vec<u8> = Vec::new();
        codec.push(0xed);
        codec.push(0x01);
        codec.extend_from_slice(&vk.to_bytes());
        let mb = multibase::encode(multibase::Base::Base58Btc, &codec);
        let did = format!("did:key:{mb}");
        let vm_id = format!("{did}#{mb}");

        let mut vc = VerifiableCredential {
            id: "vc:didkey:1".into(),
            issuer: did.clone(),
            issued_at: Utc::now().to_rfc3339(),
            expires_at: None,
            subject: CredentialSubject {
                peer_id: "12D".into(),
                claims: vec![CredentialClaim {
                    claim_type: "role".into(),
                    value: serde_json::json!("member"),
                }],
            },
            proof: CredentialProof {
                proof_type: "Ed25519Signature2020".into(),
                created: Utc::now().to_rfc3339(),
                verification_method: vm_id.clone(),
                proof_value: String::new(),
            },
        };
        let claims = serde_json::to_string(&vc.subject.claims).unwrap_or_default();
        let msg = format!("{}|{}|{}|{}", vc.id, vc.issuer, vc.subject.peer_id, claims);
        let sig = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            sk.sign(msg.as_bytes()).to_bytes(),
        );
        vc.proof.proof_value = sig;
        (vc, did)
    }

    #[tokio::test]
    async fn test_did_key_valid() {
        let d = TempDir::new().unwrap();
        let m = minimal_im(&d);
        let (sk, _vk) = mk_keys();
        let (vc, _did) = make_did_key_vc(&sk, &sk.verifying_key());
        let result = m.verify_vc(&vc).await;
        assert!(result.is_ok(), "verify should succeed, got: {result:?}");
        assert!(result.unwrap(), "did:key VC should verify");
    }

    #[tokio::test]
    async fn test_mock_did_web_valid() {
        let d = TempDir::new().unwrap();
        let m = minimal_im(&d);
        let (sk, vk) = mk_keys();

        // Build a multibase key for the mock DID document
        let mut codec: Vec<u8> = Vec::new();
        codec.push(0xed);
        codec.push(0x01);
        codec.extend_from_slice(&vk.to_bytes());
        let mb = multibase::encode(multibase::Base::Base58Btc, &codec);

        let domain = "example.com";
        let did = format!("did:web:{domain}");
        let vm_id = format!("{did}#{mb}");

        let mock_doc = serde_json::json!({
            "id": did,
            "verificationMethod": [{
                "id": vm_id,
                "type": "Ed25519VerificationKey2020",
                "controller": did,
                "publicKeyMultibase": mb
            }]
        });
        w3c_vc::set_mock_did_web(&mock_doc.to_string());

        let mut vc = VerifiableCredential {
            id: "vc:didweb:1".into(),
            issuer: did.clone(),
            issued_at: Utc::now().to_rfc3339(),
            expires_at: None,
            subject: CredentialSubject {
                peer_id: "12D".into(),
                claims: vec![],
            },
            proof: CredentialProof {
                proof_type: "Ed25519Signature2020".into(),
                created: Utc::now().to_rfc3339(),
                verification_method: vm_id,
                proof_value: String::new(),
            },
        };
        let claims = serde_json::to_string(&vc.subject.claims).unwrap_or_default();
        let msg = format!("{}|{}|{}|{}", vc.id, vc.issuer, vc.subject.peer_id, claims);
        let sig = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            sk.sign(msg.as_bytes()).to_bytes(),
        );
        vc.proof.proof_value = sig;

        let result = m.verify_vc(&vc).await;
        w3c_vc::clear_mock_did_web();
        assert!(result.is_ok(), "verify should succeed: {result:?}");
        assert!(result.unwrap(), "mock did:web VC should verify");
    }

    #[tokio::test]
    async fn test_unknown_did_method() {
        let d = TempDir::new().unwrap();
        let m = minimal_im(&d);
        let vc = VerifiableCredential {
            id: "vc:unknown".into(),
            issuer: "did:unknown:abc".into(),
            issued_at: Utc::now().to_rfc3339(),
            expires_at: None,
            subject: CredentialSubject {
                peer_id: "12D".into(),
                claims: vec![],
            },
            proof: CredentialProof {
                proof_type: "Ed25519Signature2020".into(),
                created: Utc::now().to_rfc3339(),
                verification_method: "did:unknown:abc#key-1".into(),
                proof_value: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==".into(),
            },
        };
        let result = m.verify_vc(&vc).await;
        assert!(
            result.is_err(),
            "unknown DID method should produce an error"
        );
    }

    #[tokio::test]
    async fn test_missing_verification_method() {
        let d = TempDir::new().unwrap();
        let m = minimal_im(&d);
        let (sk, vk) = mk_keys();
        let mut codec: Vec<u8> = Vec::new();
        codec.push(0xed);
        codec.push(0x01);
        codec.extend_from_slice(&vk.to_bytes());
        let mb = multibase::encode(multibase::Base::Base58Btc, &codec);
        let did = format!("did:key:{mb}");

        let vc = VerifiableCredential {
            id: "vc:missingvm".into(),
            issuer: did.clone(),
            issued_at: Utc::now().to_rfc3339(),
            expires_at: None,
            subject: CredentialSubject {
                peer_id: "12D".into(),
                claims: vec![],
            },
            proof: CredentialProof {
                proof_type: "Ed25519Signature2020".into(),
                created: Utc::now().to_rfc3339(),
                verification_method: format!("{did}#non-existent-fragment"),
                proof_value: base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    sk.sign(b"msg").to_bytes(),
                ),
            },
        };
        let result = m.verify_vc(&vc).await;
        assert!(result.is_err(), "missing VM should fail");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("not found"),
            "error should mention not found, got: {msg}"
        );
    }

    #[tokio::test]
    async fn test_key_mismatch() {
        let d = TempDir::new().unwrap();
        let m = minimal_im(&d);
        let (sk, _vk) = mk_keys();
        // Build a did:key that uses vk, but sign with a different key (sk2)
        let (sk2, _vk2) = mk_keys();
        let (vc, _did) = make_did_key_vc(&sk2, &sk.verifying_key());
        // Signature is from sk2, but DID Document contains vk
        let result = m.verify_vc(&vc).await;
        // The signature should not verify against the DID's key
        assert!(result.is_ok(), "verify should not error, just return false");
        assert!(!result.unwrap(), "key mismatch should produce false");
    }

    #[tokio::test]
    async fn test_expired_vc() {
        let d = TempDir::new().unwrap();
        let m = minimal_im(&d);
        let (sk, vk) = mk_keys();
        let (vc, _did) = make_did_key_vc(&sk, &vk);
        let mut expired = vc.clone();
        expired.expires_at = Some((Utc::now() - chrono::Duration::days(1)).to_rfc3339());
        assert!(m.is_vc_expired(&expired));
    }

    #[tokio::test]
    async fn test_revoked_vc() {
        let d = TempDir::new().unwrap();
        let mut m = minimal_im(&d);
        let (sk, vk) = mk_keys();
        let (vc, _did) = make_did_key_vc(&sk, &vk);
        // Even with valid signature from did:key, if CRL marks it as revoked,
        // is_vc_revoked should catch it (verified but revoked).
        m.crl.entries.push(CrlEntry {
            vc_id: vc.id.clone(),
            revoked_at: Utc::now().to_rfc3339(),
            reason: Some("test revocation".into()),
        });
        let result = m.verify_vc(&vc).await;
        assert!(result.is_ok() && result.unwrap(), "signature should verify");
        assert!(m.is_vc_revoked(&vc), "CRL should mark it revoked");
    }

    #[tokio::test]
    async fn test_legacy_simplified_vc_still_works() {
        let d = TempDir::new().unwrap();
        let m = minimal_im(&d);
        let (sk, vk) = mk_keys();
        let vc = VerifiableCredential {
            id: "vc:legacy".into(),
            issuer: "did:j".into(),
            issued_at: Utc::now().to_rfc3339(),
            expires_at: None,
            subject: CredentialSubject {
                peer_id: "12D".into(),
                claims: vec![],
            },
            proof: CredentialProof {
                proof_type: "Ed25519Signature2020".into(),
                created: Utc::now().to_rfc3339(),
                verification_method: "did:ex#k".into(),
                proof_value: String::new(),
            },
        };
        // Legacy VM is "did:ex#k" — not a real DID URL, so it uses
        // legacy simplified mode (treat "k" as base64). Since the sign_vc
        // helper writes the actual base64 pubkey after #, it works.
        assert!(m.verify_vc(&sign_vc(&vc, &sk, &vk)).await.unwrap());
    }

    #[tokio::test]
    async fn test_legacy_cannot_override_resolvable_did() {
        let d = TempDir::new().unwrap();
        let m = minimal_im(&d);
        let (sk, vk) = mk_keys();
        let (vc, _did) = make_did_key_vc(&sk, &vk);
        // vm_id is a real did:key URL; tampering with proof_value should fail
        let mut tampered = vc.clone();
        tampered.proof.proof_value =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, [0u8; 64]);
        let result = m.verify_vc(&tampered).await;
        assert!(result.is_ok());
        assert!(
            !result.unwrap(),
            "tampered sig should not verify against DID key"
        );
    }

    fn make_id(pid: &str) -> Identity {
        Identity {
            peer_id: pid.into(),
            public_key: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                [0u8; 32],
            ),
            club: None,
            vc: None,
            mua_account: None,
            serverless_token: None,
        }
    }
    fn make_mua(logged: bool, mem: bool) -> MuaLoginStatus {
        MuaLoginStatus {
            logged_in: logged,
            username: if logged { Some("u".into()) } else { None },
            uuid: if logged { Some("uid".into()) } else { None },
            auth_server_url: "https://a".into(),
            peer_bound: logged,
            is_guest: logged && !mem,
            is_member: mem,
        }
    }
    fn make_vcs(st: VcHolderState, club: Option<&str>, stale: bool) -> VcStatus {
        let role = if st == VcHolderState::Member {
            Some("member".into())
        } else {
            None
        };
        let issuer = if st != VcHolderState::Unverified {
            Some("did:j".into())
        } else {
            None
        };
        let verified = st == VcHolderState::Member;
        VcStatus {
            state: st,
            role,
            club: club.map(|s| s.into()),
            issuer,
            verified,
            crl_stale: stale,
        }
    }

    #[test]
    fn test_onboard_member() {
        let s = build_onboarding_status(
            &make_id("p"),
            &make_vcs(VcHolderState::Member, Some("b"), false),
            &make_mua(true, true),
        );
        assert!(s.is_member);
        assert_eq!(s.mode_label, "平台身份");
    }
    #[test]
    fn test_onboard_guest() {
        let s = build_onboarding_status(
            &make_id("p"),
            &make_vcs(VcHolderState::Unverified, None, false),
            &make_mua(true, false),
        );
        assert!(s.is_guest);
    }
    #[test]
    fn test_onboard_stranger() {
        let s = build_onboarding_status(
            &make_id("p"),
            &make_vcs(VcHolderState::Unverified, None, false),
            &make_mua(false, false),
        );
        assert!(!s.is_guest);
        assert!(s.next_steps.iter().any(|x| x.contains("MUA")));
    }
}
