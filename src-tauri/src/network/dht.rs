use base64::Engine;
use prost::Message;
use tracing::warn;

use super::ResolvedInstance;



pub(crate) fn dht_instance_key(instance_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(format!("instance:{instance_id}").as_bytes());
    format!("/instance/sha256/{}", hex::encode(hasher.finalize()))
}


pub const INSTANCE_RECORD_TTL_SECS: u64 = 1800;


pub fn instance_record_is_fresh(published_at: &str) -> bool {
    let Ok(published) = chrono::DateTime::parse_from_rfc3339(published_at) else {
        return false;
    };
    let age = chrono::Utc::now()
        .signed_duration_since(published.with_timezone(&chrono::Utc))
        .num_seconds();
    age < 0 || (age as u64) <= INSTANCE_RECORD_TTL_SECS
}


pub(crate) fn parse_instance_record_proto(value: &[u8]) -> Option<ResolvedInstance> {
    let record = crate::protos::jlucraft::records::v1::InstanceRecord::decode(value).ok()?;

    if !record.published_at.is_empty() && !instance_record_is_fresh(&record.published_at) {
        warn!(
            instance_id = %record.instance_id,
            published_at = %record.published_at,
            "DHT instance record expired, rejecting"
        );
        return None;
    }

    if record.public_key.is_empty() || record.signature.is_empty() {
        warn!(
            instance_id = %record.instance_id,
            "DHT instance record missing pubkey or signature, rejecting"
        );
        return None;
    }

    let pubkey_b64 = base64::engine::general_purpose::STANDARD.encode(&record.public_key);
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(&record.signature);

    if !verify_dht_record_signature(
        &pubkey_b64,
        &sig_b64,
        &record.instance_id,
        &record.peer_id,
        &record.published_at,
    ) {
        warn!(
            instance_id = %record.instance_id,
            peer_id = %record.peer_id,
            "DHT instance record signature verification failed, rejecting"
        );
        return None;
    }

    Some(ResolvedInstance {
        instance_id: record.instance_id,
        peer_id: record.peer_id,
        proxy_address: record.proxy_address.filter(|s| !s.is_empty()),
        public_ips: record.public_ips,
        multiaddrs: record.listen_addrs,
        resolved_at: chrono::Utc::now().to_rfc3339(),
    })
}





fn verify_dht_record_signature(
    pubkey_b64: &str,
    sig_b64: &str,
    instance_id: &str,
    peer_id: &str,
    published_at: &str,
) -> bool {
    if pubkey_b64.is_empty() || sig_b64.is_empty() {
        return false;
    }
    let Ok(pubkey_bytes) = base64::engine::general_purpose::STANDARD.decode(pubkey_b64) else {
        return false;
    };
    let Ok(public_key) = libp2p::identity::PublicKey::try_decode_protobuf(&pubkey_bytes) else {
        return false;
    };
    let derived = public_key.to_peer_id().to_string();
    if derived != peer_id {
        warn!(
            claimed_peer_id = %peer_id,
            derived_peer_id = %derived,
            "DHT instance record: pubkey peer_id mismatch, rejecting"
        );
        return false;
    }
    let Ok(sig_bytes) = base64::engine::general_purpose::STANDARD.decode(sig_b64) else {
        return false;
    };
    let canon = format!("instance-record|{instance_id}|{peer_id}|{published_at}");
    public_key.verify(canon.as_bytes(), &sig_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;



    #[test]
    fn test_dht_instance_key_matches_contract_hash() {
        assert_eq!(
            dht_instance_key("550e8400-e29b-41d4-a716-446655440000"),
            "/instance/sha256/00631932b7793af9adc23ae5db7a0fbcff7f2a6fd2a0e24411aac092fbc1c1c0"
        );
    }



    #[test]
    fn test_expired_instance_record_rejected() {
        let ts = (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        assert!(!instance_record_is_fresh(&ts));
    }

    #[test]
    fn test_fresh_instance_record_accepted() {
        assert!(instance_record_is_fresh(&chrono::Utc::now().to_rfc3339()));
    }

    #[test]
    fn test_future_published_at_is_fresh() {
        let ts = (chrono::Utc::now() + chrono::Duration::seconds(10)).to_rfc3339();
        assert!(instance_record_is_fresh(&ts));
    }



    #[test]
    fn test_verify_dht_record_signature_empty_inputs() {
        assert!(!verify_dht_record_signature(
            "",
            "",
            "inst",
            "peer",
            "2026-05-05T00:00:00Z"
        ));
        assert!(!verify_dht_record_signature(
            "abc",
            "",
            "inst",
            "peer",
            "2026-05-05T00:00:00Z"
        ));
        assert!(!verify_dht_record_signature(
            "",
            "abc",
            "inst",
            "peer",
            "2026-05-05T00:00:00Z"
        ));
    }

    #[test]
    fn test_verify_dht_record_signature_valid() {
        use base64::Engine;
        let kp = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = kp.public().to_peer_id().to_string();
        let canon = format!("instance-record|inst|{peer_id}|2026-05-05T00:00:00Z");
        let sig = kp.sign(canon.as_bytes()).unwrap();
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(&sig);
        let pk_b64 =
            base64::engine::general_purpose::STANDARD.encode(kp.public().encode_protobuf());
        assert!(verify_dht_record_signature(
            &pk_b64,
            &sig_b64,
            "inst",
            &peer_id,
            "2026-05-05T00:00:00Z"
        ));
    }

    #[test]
    fn test_peer_id_mismatch_rejected() {
        use base64::Engine;
        let kp = libp2p::identity::Keypair::generate_ed25519();
        let real_peer_id = kp.public().to_peer_id().to_string();
        let canon =
            format!("instance-record|inst-peer-mismatch|{real_peer_id}|2026-05-05T00:00:00Z");
        let sig = kp.sign(canon.as_bytes()).unwrap();
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(&sig);
        let pk_b64 =
            base64::engine::general_purpose::STANDARD.encode(kp.public().encode_protobuf());

        assert!(!verify_dht_record_signature(
            &pk_b64,
            &sig_b64,
            "inst-peer-mismatch",
            "12D3KooWFake",
            "2026-05-05T00:00:00Z"
        ));
        assert!(verify_dht_record_signature(
            &pk_b64,
            &sig_b64,
            "inst-peer-mismatch",
            &real_peer_id,
            "2026-05-05T00:00:00Z"
        ));
    }



    fn make_signed_record(instance_id: &str) -> (Vec<u8>, libp2p::identity::Keypair, String) {
        let kp = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = kp.public().to_peer_id().to_string();
        let published_at = chrono::Utc::now().to_rfc3339();
        let canon = format!("instance-record|{instance_id}|{peer_id}|{published_at}");
        let sig = kp.sign(canon.as_bytes()).unwrap();
        let record = crate::protos::jlucraft::records::v1::InstanceRecord {
            instance_id: instance_id.to_string(),
            peer_id: peer_id.clone(),
            proxy_address: Some("127.0.0.1:25565".to_string()),
            public_ips: vec!["203.0.113.10".to_string()],
            listen_addrs: vec!["/ip4/203.0.113.10/udp/4001/quic-v1".to_string()],
            published_at,
            public_key: kp.public().encode_protobuf(),
            signature: sig,
        };
        (record.encode_to_vec(), kp, peer_id)
    }

    #[test]
    fn test_valid_signed_dht_record_accepted() {
        let (payload, _kp, peer_id) = make_signed_record("inst-valid");
        let resolved =
            parse_instance_record_proto(&payload).expect("valid record should be accepted");
        assert_eq!(resolved.instance_id, "inst-valid");
        assert_eq!(resolved.peer_id, peer_id);
        assert_eq!(resolved.proxy_address.as_deref(), Some("127.0.0.1:25565"));
        assert_eq!(resolved.public_ips, vec!["203.0.113.10"]);
    }

    #[test]
    fn test_tampered_record_rejected() {
        let (payload, _kp, _) = make_signed_record("inst-ok");
        let mut record =
            crate::protos::jlucraft::records::v1::InstanceRecord::decode(payload.as_slice())
                .unwrap();
        record.instance_id = "tampered-inst".to_string();
        assert!(parse_instance_record_proto(&record.encode_to_vec()).is_none());
    }

    #[test]
    fn test_missing_signature_record_rejected() {
        let record = crate::protos::jlucraft::records::v1::InstanceRecord {
            instance_id: "inst-nosig".to_string(),
            peer_id: "12D3KooWNoSig".to_string(),
            published_at: "2026-05-05T00:00:00Z".to_string(),
            public_key: vec![],
            signature: vec![],
            proxy_address: Some("127.0.0.1:25565".to_string()),
            public_ips: vec![],
            listen_addrs: vec![],
        };
        assert!(parse_instance_record_proto(&record.encode_to_vec()).is_none());
    }

    #[test]
    fn test_wrong_signer_record_rejected() {
        let kp_good = libp2p::identity::Keypair::generate_ed25519();
        let kp_wrong = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = kp_good.public().to_peer_id().to_string();
        let canon = format!("instance-record|inst-wrong-signer|{peer_id}|2026-05-05T00:00:00Z");
        let record = crate::protos::jlucraft::records::v1::InstanceRecord {
            instance_id: "inst-wrong-signer".to_string(),
            peer_id,
            published_at: "2026-05-05T00:00:00Z".to_string(),
            public_key: kp_wrong.public().encode_protobuf(),
            signature: kp_good.sign(canon.as_bytes()).unwrap(),
            proxy_address: None,
            public_ips: vec![],
            listen_addrs: vec![],
        };
        assert!(parse_instance_record_proto(&record.encode_to_vec()).is_none());
    }

    #[test]
    fn test_unsigned_legacy_record_rejected() {
        let record = crate::protos::jlucraft::records::v1::InstanceRecord {
            instance_id: "inst-legacy".to_string(),
            peer_id: "legacy-peer".to_string(),
            proxy_address: Some("evil.example.com:25565".to_string()),
            published_at: String::new(),
            public_key: vec![],
            signature: vec![],
            public_ips: vec![],
            listen_addrs: vec![],
        };
        assert!(parse_instance_record_proto(&record.encode_to_vec()).is_none());
    }

    #[test]
    fn test_expired_parse_rejected() {
        let kp = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = kp.public().to_peer_id().to_string();
        let expired = (chrono::Utc::now() - chrono::Duration::hours(3)).to_rfc3339();
        let canon = format!("instance-record|inst-expired|{peer_id}|{expired}");
        let record = crate::protos::jlucraft::records::v1::InstanceRecord {
            instance_id: "inst-expired".to_string(),
            peer_id,
            published_at: expired,
            public_key: kp.public().encode_protobuf(),
            signature: kp.sign(canon.as_bytes()).unwrap(),
            proxy_address: None,
            public_ips: vec![],
            listen_addrs: vec![],
        };
        assert!(parse_instance_record_proto(&record.encode_to_vec()).is_none());
    }
}
