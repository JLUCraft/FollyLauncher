//! Version JSON / libraries / assets / native library validator.
//!
//! Parses version JSON, asset index, checks file existence and hashes,
//! and generates a ValidationSummary with download/repair tasks.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Result of validating a Minecraft installation's completeness.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationSummary {
    pub instance_id: String,
    pub missing_libraries: u32,
    pub missing_assets: u32,
    pub invalid_hashes: u32,
    pub native_actions: u32,
    pub download_group_id: Option<String>,
    pub ready_to_launch: bool,
    /// Detailed download tasks generated from validation.
    #[serde(default)]
    pub download_tasks: Vec<DownloadTask>,
}

/// A single download/repair task produced by validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub kind: DownloadTaskKind,
    pub name: String,
    pub url: String,
    pub dest_path: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DownloadTaskKind {
    Library,
    Asset,
    ClientJar,
    AssetIndex,
    Native,
}

/// Parsed version JSON structure (subset of fields needed for validation).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct VersionJson {
    #[serde(default)]
    pub libraries: Vec<LibraryEntry>,
    #[serde(default)]
    pub downloads: Option<ClientDownloads>,
    #[serde(default)]
    pub asset_index: Option<AssetIndexInfo>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LibraryEntry {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub natives: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<LibraryArtifact>,
    #[serde(default)]
    pub classifiers: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryArtifact {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ClientDownloads {
    #[serde(default)]
    pub client: Option<LibraryArtifact>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AssetIndexInfo {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

/// Parse a version JSON file from the given path.
pub fn parse_version_json(path: &Path) -> Result<VersionJson, crate::error::LauncherError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| crate::error::LauncherError::from(format!("cannot read version JSON at {}: {e}", path.display())))?;
    serde_json::from_str::<VersionJson>(&content)
        .map_err(|e| crate::error::LauncherError::from(format!("invalid version JSON at {}: {e}", path.display())))
}

/// Resolve the library path for a given Maven coordinate name.
pub fn library_artifact_path(name: &str, libraries_dir: &Path) -> Option<PathBuf> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let package = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts.get(3);
    let ext = parts.get(4).unwrap_or(&"jar");

    let filename = if let Some(cls) = classifier {
        format!("{}-{}-{}.{}", artifact, version, cls, ext)
    } else {
        format!("{}-{}.{}", artifact, version, ext)
    };

    Some(
        libraries_dir
            .join(&package)
            .join(artifact)
            .join(version)
            .join(filename),
    )
}

/// Result of checking whether native libraries are ready for launch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeReadiness {
    pub ready: bool,
    pub natives_dir_path: String,
    /// Human-readable reasons why natives are not ready.
    pub reasons: Vec<String>,
    /// Download tasks needed to repair missing natives.
    #[serde(default)]
    pub repair_tasks: Vec<DownloadTask>,
}

/// Check if native libraries are extracted and ready for the given version.
///
/// Parses the version JSON to determine if any library has native classifiers
/// for the current platform. If no native libraries are required, returns
/// ready=true immediately. Otherwise, verifies the natives directory exists
/// and contains at least one platform-native file.
pub fn check_native_readiness(game_dir: &Path, version: &str) -> NativeReadiness {
    let natives_dir = game_dir.join("versions").join(version).join("natives");
    let natives_dir_path = natives_dir.to_string_lossy().to_string();

    let version_json_path = game_dir
        .join("versions")
        .join(version)
        .join(format!("{version}.json"));

    // Cannot determine native requirements without version JSON.
    if !version_json_path.exists() {
        return NativeReadiness {
            ready: false,
            natives_dir_path,
            reasons: vec![
                "version JSON not found; cannot determine native requirements".to_string(),
            ],
            repair_tasks: vec![],
        };
    }

    let vj = match parse_version_json(&version_json_path) {
        Ok(v) => v,
        Err(e) => {
            return NativeReadiness {
                ready: false,
                natives_dir_path,
                reasons: vec![format!("failed to parse version JSON: {e}")],
                repair_tasks: vec![],
            };
        }
    };

    // Determine whether any library requires native extraction for this platform.
    let has_native_libs = vj.libraries.iter().any(|lib| {
        if let Some(ref natives) = lib.natives {
            !extract_native_classifiers(natives).is_empty()
        } else {
            false
        }
    });

    // If no library has platform-native classifiers, natives are not required.
    if !has_native_libs {
        return NativeReadiness {
            ready: true,
            natives_dir_path,
            reasons: vec![],
            repair_tasks: vec![],
        };
    }

    // Natives are required but the directory does not exist.
    if !natives_dir.exists() {
        return NativeReadiness {
            ready: false,
            natives_dir_path,
            reasons: vec![
                "natives directory does not exist but native libraries are required".to_string(),
            ],
            repair_tasks: vec![],
        };
    }

    // Check that the directory contains at least one regular file.
    let has_files = match std::fs::read_dir(&natives_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .any(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)),
        Err(_) => false,
    };

    if !has_files {
        return NativeReadiness {
            ready: false,
            natives_dir_path,
            reasons: vec!["natives directory exists but contains no native files".to_string()],
            repair_tasks: vec![],
        };
    }

    NativeReadiness {
        ready: true,
        natives_dir_path,
        reasons: vec![],
        repair_tasks: vec![],
    }
}

/// Check whether a library at the given path has the expected SHA-1 hash.
pub fn verify_sha1(path: &Path, expected_sha1: &str) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => {
            let actual = sha1_smol::Sha1::from(&bytes).digest().to_string();
            actual.eq_ignore_ascii_case(expected_sha1)
        }
        Err(_) => false,
    }
}

/// Current OS name for native classifier matching.
pub fn current_os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    }
}

/// Current architecture for native matching.
pub fn current_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "64"
    } else if cfg!(target_arch = "x86") {
        "32"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "unknown"
    }
}

/// Extract native library classifiers from a library entry's natives map.
pub fn extract_native_classifiers(natives: &serde_json::Value) -> Vec<(String, String)> {
    // Returns (classifier_key, classifier_value)
    let mut result = Vec::new();
    if let Some(obj) = natives.as_object() {
        for (key, val) in obj {
            if key == current_os_name() || key.contains(current_os_name()) {
                if let Some(s) = val.as_str() {
                    let arch_aware = s.replace("${arch}", current_arch());
                    result.push((key.clone(), arch_aware));
                }
            }
        }
    }
    result
}

/// Validate a single Minecraft version installation and produce a summary.
pub fn validate_version(
    instance_id: &str,
    game_dir: &Path,
    version: &str,
) -> Result<ValidationSummary, crate::error::LauncherError> {
    let version_json_path = game_dir
        .join("versions")
        .join(version)
        .join(format!("{version}.json"));

    if !version_json_path.exists() {
        return Ok(ValidationSummary {
            instance_id: instance_id.to_string(),
            missing_libraries: 0,
            missing_assets: 0,
            invalid_hashes: 0,
            native_actions: 0,
            download_group_id: None,
            ready_to_launch: false,
            download_tasks: vec![],
        });
    }

    let vj = parse_version_json(&version_json_path)?;
    let libraries_dir = game_dir.join("libraries");
    let assets_dir = game_dir.join("assets");

    let mut missing_libraries: u32 = 0;
    let mut missing_assets: u32 = 0;
    let mut invalid_hashes: u32 = 0;
    let mut native_actions: u32 = 0;
    let mut download_tasks: Vec<DownloadTask> = Vec::new();

    // Check client JAR
    let client_jar_path = game_dir
        .join("versions")
        .join(version)
        .join(format!("{version}.jar"));
    if !client_jar_path.exists() {
        if let Some(dl) = &vj.downloads {
            if let Some(client) = &dl.client {
                download_tasks.push(DownloadTask {
                    kind: DownloadTaskKind::ClientJar,
                    name: format!("{version}.jar"),
                    url: client.url.clone(),
                    dest_path: client_jar_path.to_string_lossy().to_string(),
                    sha1: Some(client.sha1.clone()),
                    size: Some(client.size),
                    required: true,
                });
            }
        }
        missing_assets += 1;
    } else if let Some(dl) = &vj.downloads {
        if let Some(client) = &dl.client {
            if !verify_sha1(&client_jar_path, &client.sha1) {
                download_tasks.push(DownloadTask {
                    kind: DownloadTaskKind::ClientJar,
                    name: format!("{version}.jar"),
                    url: client.url.clone(),
                    dest_path: client_jar_path.to_string_lossy().to_string(),
                    sha1: Some(client.sha1.clone()),
                    size: Some(client.size),
                    required: true,
                });
                invalid_hashes += 1;
            }
        }
    }

    // Check libraries
    for lib in &vj.libraries {
        let name = &lib.name;
        if let Some(lib_path) = library_artifact_path(name, &libraries_dir) {
            if !lib_path.exists() {
                // Generate download task if artifact download info is available
                if let Some(dl) = &lib.downloads {
                    if let Some(artifact) = &dl.artifact {
                        download_tasks.push(DownloadTask {
                            kind: DownloadTaskKind::Library,
                            name: name.clone(),
                            url: artifact.url.clone(),
                            dest_path: lib_path.to_string_lossy().to_string(),
                            sha1: Some(artifact.sha1.clone()),
                            size: Some(artifact.size),
                            required: true,
                        });
                    }
                }
                missing_libraries += 1;
            } else if let Some(dl) = &lib.downloads {
                if let Some(artifact) = &dl.artifact {
                    if !verify_sha1(&lib_path, &artifact.sha1) {
                        download_tasks.push(DownloadTask {
                            kind: DownloadTaskKind::Library,
                            name: name.clone(),
                            url: artifact.url.clone(),
                            dest_path: lib_path.to_string_lossy().to_string(),
                            sha1: Some(artifact.sha1.clone()),
                            size: Some(artifact.size),
                            required: true,
                        });
                        invalid_hashes += 1;
                    }
                }
            }
        }

        // Check natives — generate download tasks for missing native JARs.
        // Readiness of the extracted natives directory is determined via
        // check_native_readiness (shared with the launch path) at the end.
        if let Some(natives) = &lib.natives {
            let classifiers = extract_native_classifiers(natives);
            if !classifiers.is_empty() {
                if let Some(dl) = &lib.downloads {
                    if let Some(classifiers_map) = &dl.classifiers {
                        for (_ckey, cval) in &classifiers {
                            if let Some(native_jar) = classifiers_map.get(cval) {
                                if let Some(path_str) =
                                    native_jar.get("path").and_then(|v| v.as_str())
                                {
                                    let native_dest = lib_path(&libraries_dir, path_str);
                                    if !native_dest.exists() {
                                        if let Some(url) =
                                            native_jar.get("url").and_then(|v| v.as_str())
                                        {
                                            download_tasks.push(DownloadTask {
                                                kind: DownloadTaskKind::Native,
                                                name: format!("{name} native"),
                                                url: url.to_string(),
                                                dest_path: native_dest
                                                    .to_string_lossy()
                                                    .to_string(),
                                                sha1: native_jar.get("sha1").and_then(|v| {
                                                    v.as_str().map(|s| s.to_string())
                                                }),
                                                size: native_jar
                                                    .get("size")
                                                    .and_then(|v| v.as_u64()),
                                                required: true,
                                            });
                                            native_actions += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check asset index
    let asset_index_path = assets_dir.join("indexes").join(
        vj.asset_index
            .as_ref()
            .map(|ai| format!("{}.json", ai.id))
            .unwrap_or_else(|| "legacy.json".to_string()),
    );

    if let Some(ai) = &vj.asset_index {
        if !asset_index_path.exists() {
            download_tasks.push(DownloadTask {
                kind: DownloadTaskKind::AssetIndex,
                name: format!("asset index {}", ai.id),
                url: ai.url.clone(),
                dest_path: asset_index_path.to_string_lossy().to_string(),
                sha1: Some(ai.sha1.clone()),
                size: Some(ai.size),
                required: true,
            });
            missing_assets += 1;
        } else if !verify_sha1(&asset_index_path, &ai.sha1) {
            download_tasks.push(DownloadTask {
                kind: DownloadTaskKind::AssetIndex,
                name: format!("asset index {}", ai.id),
                url: ai.url.clone(),
                dest_path: asset_index_path.to_string_lossy().to_string(),
                sha1: Some(ai.sha1.clone()),
                size: Some(ai.size),
                required: true,
            });
            invalid_hashes += 1;
        }

        // Check individual assets if index exists
        if asset_index_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&asset_index_path) {
                if let Ok(index_json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(objects) = index_json.get("objects") {
                        if let Some(obj_map) = objects.as_object() {
                            for (_name, asset_obj) in obj_map {
                                if let Some(hash) = asset_obj.get("hash").and_then(|v| v.as_str()) {
                                    let prefix = &hash[..2];
                                    let asset_path =
                                        assets_dir.join("objects").join(prefix).join(hash);
                                    if !asset_path.exists() {
                                        download_tasks.push(DownloadTask {
                                            kind: DownloadTaskKind::Asset,
                                            name: hash.to_string(),
                                            url: format!(
                                                "https://resources.download.minecraft.net/{}/{}",
                                                prefix, hash
                                            ),
                                            dest_path: asset_path.to_string_lossy().to_string(),
                                            sha1: Some(hash.to_string()),
                                            size: asset_obj.get("size").and_then(|v| v.as_u64()),
                                            required: false,
                                        });
                                        missing_assets += 1;
                                    } else if !verify_sha1(&asset_path, hash) {
                                        download_tasks.push(DownloadTask {
                                            kind: DownloadTaskKind::Asset,
                                            name: hash.to_string(),
                                            url: format!(
                                                "https://resources.download.minecraft.net/{}/{}",
                                                prefix, hash
                                            ),
                                            dest_path: asset_path.to_string_lossy().to_string(),
                                            sha1: Some(hash.to_string()),
                                            size: asset_obj.get("size").and_then(|v| v.as_u64()),
                                            required: false,
                                        });
                                        invalid_hashes += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Use the shared check_native_readiness helper so validate_version
    // and the launch path agree on whether extracted natives are ready.
    let native = check_native_readiness(game_dir, version);
    if !native.ready {
        // Track native readiness failures as actions so consumers can
        // differentiate "missing files" from "natives not extracted".
        if native_actions == 0 {
            native_actions = 1;
        }
    }

    let ready = missing_libraries == 0
        && missing_assets == 0
        && invalid_hashes == 0
        && native.ready
        && client_jar_path.exists();

    Ok(ValidationSummary {
        instance_id: instance_id.to_string(),
        missing_libraries,
        missing_assets,
        invalid_hashes,
        native_actions,
        download_group_id: None,
        ready_to_launch: ready && client_jar_path.exists(),
        download_tasks,
    })
}

/// Helper to construct a library path from a path string within the libraries dir.
fn lib_path(libraries_dir: &Path, path_str: &str) -> PathBuf {
    libraries_dir.join(path_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_artifact_path_vanilla() {
        let path = library_artifact_path("com.mojang:logging:1.0.0", Path::new("/mc/libraries"));
        assert_eq!(
            path,
            Some(PathBuf::from(
                "/mc/libraries/com/mojang/logging/1.0.0/logging-1.0.0.jar"
            ))
        );
    }

    #[test]
    fn test_library_artifact_path_with_classifier() {
        let path = library_artifact_path(
            "org.lwjgl:lwjgl:3.3.1:natives-windows",
            Path::new("/mc/libraries"),
        );
        assert_eq!(
            path,
            Some(PathBuf::from(
                "/mc/libraries/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar"
            ))
        );
    }

    #[test]
    fn test_library_artifact_path_short_name_rejected() {
        assert!(library_artifact_path("short", Path::new("/mc/libraries")).is_none());
    }

    #[test]
    fn test_parse_version_json_invalid_path() {
        let result = parse_version_json(Path::new("/nonexistent/version.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_sha1_match() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.bin");
        std::fs::write(&file_path, b"hello").unwrap();
        // SHA-1 of "hello" is "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
        assert!(verify_sha1(
            &file_path,
            "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
        ));
    }

    #[test]
    fn test_verify_sha1_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.bin");
        std::fs::write(&file_path, b"hello").unwrap();
        assert!(!verify_sha1(
            &file_path,
            "0000000000000000000000000000000000000000"
        ));
    }

    #[test]
    fn test_verify_sha1_nonexistent_file() {
        assert!(!verify_sha1(Path::new("/no/file"), "abc"));
    }

    // ── check_native_readiness ───────────────────────────────────────────

    /// Build a minimal version JSON fixture with the given libraries.
    fn make_version_json(libraries: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "libraries": libraries,
            "mainClass": "net.minecraft.client.main.Main"
        })
    }

    /// Build a library entry that requires native classifiers for the current OS.
    fn native_library_entry() -> serde_json::Value {
        let os = current_os_name();
        serde_json::json!({
            "name": "org.lwjgl:lwjgl:3.3.1",
            "natives": {
                os: "natives-${arch}"
            },
            "downloads": {
                "artifact": {
                    "sha1": "abc123",
                    "size": 100,
                    "url": "https://example.com/lwjgl.jar"
                },
                "classifiers": {
                    "natives-windows": {
                        "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar"
                    }
                }
            }
        })
    }

    /// Build a library entry without native classifiers.
    fn non_native_library_entry() -> serde_json::Value {
        serde_json::json!({
            "name": "com.google.guava:guava:31.0-jre",
            "downloads": {
                "artifact": {
                    "sha1": "def456",
                    "size": 200,
                    "url": "https://example.com/guava.jar"
                }
            }
        })
    }

    #[test]
    fn test_native_readiness_no_native_libs_required() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();

        // Create version JSON with only non-native libraries.
        let version_json = make_version_json(serde_json::json!([non_native_library_entry()]));
        let version_dir = game_dir.join("versions").join("1.21");
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::write(
            version_dir.join("1.21.json"),
            serde_json::to_string(&version_json).unwrap(),
        )
        .unwrap();

        let readiness = check_native_readiness(game_dir, "1.21");
        assert!(
            readiness.ready,
            "should be ready when no native libs are required"
        );
        assert!(readiness.reasons.is_empty());
    }

    #[test]
    fn test_native_readiness_missing_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();

        // Create version JSON with a native library, but no natives directory.
        let version_json = make_version_json(serde_json::json!([native_library_entry()]));
        let version_dir = game_dir.join("versions").join("1.21");
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::write(
            version_dir.join("1.21.json"),
            serde_json::to_string(&version_json).unwrap(),
        )
        .unwrap();

        let readiness = check_native_readiness(game_dir, "1.21");
        assert!(
            !readiness.ready,
            "should not be ready when natives dir is missing"
        );
        assert!(!readiness.reasons.is_empty());
        assert!(
            readiness
                .reasons
                .iter()
                .any(|r| r.contains("natives directory does not exist")),
            "reasons should mention missing natives dir: {:?}",
            readiness.reasons
        );
    }

    #[test]
    fn test_native_readiness_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();

        // Create version JSON with a native library and an empty natives directory.
        let version_json = make_version_json(serde_json::json!([native_library_entry()]));
        let version_dir = game_dir.join("versions").join("1.21");
        let natives_dir = version_dir.join("natives");
        std::fs::create_dir_all(&natives_dir).unwrap();
        std::fs::write(
            version_dir.join("1.21.json"),
            serde_json::to_string(&version_json).unwrap(),
        )
        .unwrap();

        let readiness = check_native_readiness(game_dir, "1.21");
        assert!(
            !readiness.ready,
            "should not be ready when natives dir is empty"
        );
        assert!(!readiness.reasons.is_empty());
        assert!(
            readiness
                .reasons
                .iter()
                .any(|r| r.contains("contains no native files")),
            "reasons should mention empty dir: {:?}",
            readiness.reasons
        );
    }

    #[test]
    fn test_native_readiness_valid_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();

        // Create version JSON with a native library and a populated natives directory.
        let version_json = make_version_json(serde_json::json!([native_library_entry()]));
        let version_dir = game_dir.join("versions").join("1.21");
        let natives_dir = version_dir.join("natives");
        std::fs::create_dir_all(&natives_dir).unwrap();
        // Write a dummy native file to simulate extracted natives.
        std::fs::write(natives_dir.join("lwjgl.dll"), b"mock-native-content").unwrap();
        std::fs::write(
            version_dir.join("1.21.json"),
            serde_json::to_string(&version_json).unwrap(),
        )
        .unwrap();

        let readiness = check_native_readiness(game_dir, "1.21");
        assert!(
            readiness.ready,
            "should be ready when natives dir is populated"
        );
        assert!(readiness.reasons.is_empty());
    }

    #[test]
    fn test_native_readiness_missing_version_json() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();

        let readiness = check_native_readiness(game_dir, "1.21");
        assert!(!readiness.ready);
        assert!(
            readiness
                .reasons
                .iter()
                .any(|r| r.contains("version JSON not found")),
            "reasons: {:?}",
            readiness.reasons
        );
    }
}
