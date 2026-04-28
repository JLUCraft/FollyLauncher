use base64::Engine;
use libp2p::identity::Keypair;
use serde::Deserialize;

#[derive(Deserialize)]
struct ChallengeResponse {
    nonce: String,
}

pub struct ApiAuth {
    client: reqwest::Client,
    base_url: String,
    keypair: Keypair,
}

impl ApiAuth {
    pub fn new(base_url: String, keypair: Keypair) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build reqwest client"),
            base_url,
            keypair,
        }
    }

    pub async fn auth_headers(
        &self,
        cmd_type: &str,
        payload_json: &str,
    ) -> Result<(String, String), String> {
        let ed_kp = self
            .keypair
            .clone()
            .try_into_ed25519()
            .map_err(|_| "keypair is not ed25519".to_string())?;
        let public_key = base64::engine::general_purpose::STANDARD
            .encode(ed_kp.public().to_bytes());
        let payload_hash = blake3::hash(payload_json.as_bytes()).to_string();

        let auth_body = serde_json::json!({
            "cmd_type": cmd_type,
            "payload": serde_json::from_str::<serde_json::Value>(payload_json).unwrap_or_default(),
            "public_key": public_key,
        });

        let url = format!("{}/v1/auth/challenge", self.base_url.trim_end_matches('/'));
        let resp = self
            .client
            .post(&url)
            .json(&auth_body)
            .send()
            .await
            .map_err(|e| format!("challenge request: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("challenge returned {}", resp.status()));
        }

        let challenge: ChallengeResponse =
            resp.json().await.map_err(|e| format!("parse challenge: {e}"))?;

        let message = format!("{}|{}|{}", challenge.nonce, cmd_type, payload_hash);
        let signature = ed_kp.sign(message.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(&signature[..]);

        Ok((challenge.nonce, sig_b64))
    }

    pub async fn post_with_auth(
        &self,
        path: &str,
        json_body: &serde_json::Value,
        cmd_type: &str,
    ) -> Result<reqwest::Response, String> {
        let payload_json = serde_json::to_string(json_body).map_err(|e| format!("json: {e}"))?;
        let (nonce, sig) = self.auth_headers(cmd_type, &payload_json).await?;

        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        self.client
            .post(&url)
            .json(json_body)
            .header("X-Auth-Nonce", &nonce)
            .header("X-Auth-Signature", &sig)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))
    }
}
