pub mod jlucraft {
    pub mod common {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/jlucraft.common.v1.rs"));
        }
    }

    pub mod records {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/jlucraft.records.v1.rs"));
        }
    }

    pub mod events {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/jlucraft.events.v1.rs"));
        }
    }

    pub mod resources {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/jlucraft.resources.v1.rs"));
        }
    }

    pub mod control {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/jlucraft.control.v1.rs"));
        }
    }

    pub mod consensus {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/jlucraft.consensus.v1.rs"));
        }
    }

    pub mod skin {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/jlucraft.skin.v1.rs"));
        }
    }
}

#[cfg(test)]
mod tests {
    use prost::Message;

    #[test]
    fn shared_resource_manifest_roundtrips() {
        let manifest = super::jlucraft::resources::v1::ResourceManifest {
            version: 1,
            instance_id: "instance-1".to_string(),
            files: vec![super::jlucraft::resources::v1::ManifestFile {
                path: "mods/example.jar".to_string(),
                hash: "sha256:example".to_string(),
                size: 42,
                required: true,
                chunks: vec![super::jlucraft::resources::v1::ManifestChunk {
                    index: 0,
                    offset: 0,
                    size: 42,
                    hash: "chunk-example".to_string(),
                }],
            }],
        };

        let encoded = manifest.encode_to_vec();
        let decoded = super::jlucraft::resources::v1::ResourceManifest::decode(encoded.as_slice())
            .expect("shared resource manifest must decode");

        assert_eq!(decoded.instance_id, manifest.instance_id);
        assert_eq!(decoded.files.len(), 1);
        assert_eq!(decoded.files[0].chunks.len(), 1);
    }
}
