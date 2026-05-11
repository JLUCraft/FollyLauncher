use crate::error::LauncherError;
use crate::network::NetworkHandle;
use crate::resource::downloader::{DownloadManager, DownloadTask, HashAlgorithm};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

#[cfg(test)]
use serde_json::json;

const CHUNK_SIZE: usize = 256 * 1024;
const MAX_CACHE_SIZE_BYTES: u64 = 5 * 1024 * 1024 * 1024;
const MAX_FILE_AGE_SECS: u64 = 90 * 24 * 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManifest {
    pub version: u32,
    pub instance_id: String,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub required: bool,
    pub download_url: Option<String>,
    /// Optional chunk list for BitSwap parallel download
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chunks: Vec<ManifestChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestChunk {
    pub index: usize,
    pub offset: u64,
    pub size: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub manifest_loaded: bool,
    pub total_files: usize,
    pub cached_files: usize,
    pub missing_files: usize,
    pub sync_in_progress: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub downloaded: usize,
    pub failed: usize,
    pub skipped: usize,
}

pub struct ResourceSyncService {
    cache_dir: PathBuf,
    /// Optional network handle for P2P source discovery (BitSwap).
    network: Option<NetworkHandle>,
    /// Download manager for resilient HTTP downloads (Phase 5 P0).
    download_manager: DownloadManager,
}

/// 解析非 chunk 文件的最终下载 URL。
///
/// `download_url` 是必填来源；manifest 不得依赖启动器合成备用 HTTP 路径。
fn resolve_file_download_url(file: &ManifestFile) -> Result<String, LauncherError> {
    file.download_url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| LauncherError::from(format!("manifest file {} missing download_url", file.path)))
}

/// 解析单个 chunk 的下载 URL。
///
/// 规则：
/// `download_url` 非空时视为 chunk base，在其后拼接
/// `?offset={offset}&size={size}`（若已有 query 则追加 `&offset=...&size=...`）。
fn resolve_chunk_download_url(
    file: &ManifestFile,
    offset: u64,
    size: u64,
) -> Result<String, LauncherError> {
    let url = file
        .download_url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .ok_or_else(|| format!("manifest file {} missing chunk download_url", file.path))?;
    let sep = if url.contains('?') { "&" } else { "?" };
    Ok(format!("{}{}offset={}&size={}", url, sep, offset, size))
}

/// Infer the hash algorithm for a manifest file based on its path and hash length.
///
/// Phase 5 P0: DESIGN.md §5.3 — non-chunk files must not hardcode SHA256.
/// Mojang assets and libraries use SHA1; BitSwap chunks use SHA256.
///
/// Inference priority:
/// 1. Path-based: `libraries/` and `assets/objects/` files are SHA1 (Mojang convention).
/// 2. Hash-length-based: 40 hex chars → SHA1, 64 hex chars → SHA256.
/// 3. Default: SHA256 (conservative).
fn infer_hash_algorithm_for_file(file_path: &str, hash: &str) -> HashAlgorithm {
    // Path-based inference for Mojang resources.
    if file_path.starts_with("libraries/") || file_path.starts_with("assets/objects/") {
        return HashAlgorithm::Sha1;
    }

    // Hash-length-based inference.
    let hex_len = hash.trim().len();
    if hex_len == 40 {
        HashAlgorithm::Sha1
    } else if hex_len == 64 {
        HashAlgorithm::Sha256
    } else {
        // Unknown length: default to SHA256 (conservative).
        HashAlgorithm::Sha256
    }
}

impl ResourceSyncService {
    pub fn new(cache_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&cache_dir);
        let service = Self {
            cache_dir: cache_dir.clone(),
            network: None,
            download_manager: DownloadManager::new(),
        };
        tokio::spawn(async move {
            let _ = Self::cleanup_stale_files_static(&cache_dir).await;
        });
        service
    }

    /// Inject a network handle for P2P BitSwap source discovery.
    pub fn set_network(&mut self, network: NetworkHandle) {
        self.network = Some(network);
    }

    pub fn check_cache(
        &self,
        manifest: &ResourceManifest,
    ) -> (Vec<ManifestFile>, Vec<ManifestFile>) {
        let n = manifest.files.len();
        let mut cached = Vec::with_capacity(n);
        let mut missing = Vec::with_capacity(n);

        for file in &manifest.files {
            let cache_path = self.cache_path(&file.hash);
            if cache_path.exists() {
                cached.push(file.clone());
            } else {
                missing.push(file.clone());
            }
        }

        (cached, missing)
    }

    pub async fn sync_files(&self, missing: &[ManifestFile], _base_url: &str) -> SyncResult {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(4));
        let mut handles = Vec::with_capacity(missing.len());
        let mut downloaded = 0usize;
        let mut failed = 0usize;
        let mut skipped = 0usize;

        let mut download_tasks: Vec<DownloadTask> = Vec::with_capacity(missing.len());

        for file in missing {
            if !file.chunks.is_empty() {
                let sem = semaphore.clone();
                let cache_dir = self.cache_dir.clone();
                let file = file.clone();
                handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire().await;
                    let result = Self::download_chunked_static(&cache_dir, &file).await;
                    match result {
                        Ok(_) => (true, false, false),
                        Err(e) => {
                            warn!(path = %file.path, error = %e, "chunked download failed");
                            if file.required {
                                (false, true, false)
                            } else {
                                (false, false, true)
                            }
                        }
                    }
                }));
            } else {
                let url = match resolve_file_download_url(file) {
                    Ok(u) => u,
                    Err(e) => {
                        warn!(path = %file.path, error = %e, "failed to resolve download URL");
                        if file.required {
                            failed += 1;
                        } else {
                            skipped += 1;
                        }
                        continue;
                    }
                };
                let urls = vec![url];
                let dest_path = self.cache_path(&file.hash);
                let algo = infer_hash_algorithm_for_file(&file.path, &file.hash);
                download_tasks.push(DownloadTask {
                    id: file.hash.clone(),
                    name: file.path.clone(),
                    urls,
                    dest_path,
                    expected_sha1: if algo == HashAlgorithm::Sha1 {
                        Some(file.hash.clone())
                    } else {
                        None
                    },
                    expected_sha256: if algo == HashAlgorithm::Sha256 {
                        Some(file.hash.clone())
                    } else {
                        None
                    },
                    expected_size: Some(file.size),
                    hash_algorithm: algo,
                });
            }
        }

        // Use DownloadManager for resilient HTTP downloads
        if !download_tasks.is_empty() {
            let mut deduped = download_tasks;
            DownloadManager::dedup(&mut deduped);
            let progress_cb: Arc<
                dyn Fn(crate::resource::downloader::DownloadProgress) + Send + Sync,
            > = Arc::new(|p| {
                debug!(
                    task_id = %p.task_id,
                    state = ?p.state,
                    downloaded = p.downloaded,
                    total = p.total,
                    "resource sync download progress"
                );
            });
            let results = self
                .download_manager
                .download_batch(deduped, progress_cb)
                .await;
            for r in results {
                if r.success {
                    downloaded += 1;
                } else {
                    warn!(task_id = %r.task_id, error = ?r.error, "resource download failed");
                    failed += 1;
                }
            }
        }

        for handle in handles {
            match handle.await {
                Ok((d, f, s)) => {
                    if d {
                        downloaded += 1;
                    }
                    if f {
                        failed += 1;
                    }
                    if s {
                        skipped += 1;
                    }
                }
                Err(e) => {
                    warn!(error = %e, "download task panicked");
                    failed += 1;
                }
            }
        }

        let result = SyncResult {
            downloaded,
            failed,
            skipped,
        };
        if downloaded > 0 {
            self.enforce_cache_limits().await;
        }
        result
    }

    fn cache_path(&self, hash: &str) -> PathBuf {
        Self::cache_path_static(&self.cache_dir, hash)
    }

    async fn download_chunked_static(cache_dir: &Path, file: &ManifestFile) -> anyhow::Result<()> {
        let num_chunks = file.chunks.len();
        let mut chunk_tasks: Vec<DownloadTask> = Vec::with_capacity(num_chunks);
        // Build a temporary directory for chunk downloads.
        let tmp_root = cache_dir.join(".chunks");
        let _ = std::fs::create_dir_all(&tmp_root);
        let chunk_dest_dir = tmp_root.join(&file.hash);
        let _ = std::fs::create_dir_all(&chunk_dest_dir);

        for chunk in &file.chunks {
            let url = resolve_chunk_download_url(file, chunk.offset, chunk.size)
                .map_err(|e| anyhow::anyhow!("chunk {} URL 解析失败: {}", chunk.index, e))?;
            let urls = vec![url];
            let dest_path = chunk_dest_dir.join(format!("{}.chunk", chunk.index));
            chunk_tasks.push(DownloadTask {
                id: format!("{}-chunk-{}", file.hash, chunk.index),
                name: format!("{} chunk {}", file.path, chunk.index),
                urls,
                dest_path,
                expected_sha1: None,
                expected_sha256: Some(chunk.hash.clone()),
                expected_size: Some(chunk.size),
                hash_algorithm: HashAlgorithm::Sha256,
            });
        }

        // Phase 5 P0: Route chunk downloads through DownloadManager for
        // retry, resume, and hash verification.
        let dm = DownloadManager::new();
        let progress_cb: Arc<dyn Fn(crate::resource::downloader::DownloadProgress) + Send + Sync> =
            Arc::new(|p| {
                debug!(
                    task_id = %p.task_id,
                    state = ?p.state,
                    downloaded = p.downloaded,
                    total = p.total,
                    "chunk download progress"
                );
            });
        DownloadManager::dedup(&mut chunk_tasks);
        let results = dm.download_batch(chunk_tasks, progress_cb).await;

        // Check all chunks succeeded.
        let mut chunks_data: Vec<Vec<u8>> = Vec::with_capacity(num_chunks);
        chunks_data.resize(num_chunks, Vec::with_capacity(CHUNK_SIZE));
        for result in &results {
            if !result.success {
                anyhow::bail!(
                    "chunk {} download failed: {}",
                    result.task_id,
                    result.error.as_deref().unwrap_or("unknown error")
                );
            }
        }

        // Read downloaded chunk files in order.
        for chunk in &file.chunks {
            let chunk_path = chunk_dest_dir.join(format!("{}.chunk", chunk.index));
            let data = tokio::fs::read(&chunk_path)
                .await
                .with_context(|| format!("failed to read chunk {} file", chunk.index))?;
            // Re-verify hash (defense in depth).
            let actual_hash = hex::encode(sha2::Sha256::digest(&data));
            if actual_hash != chunk.hash {
                anyhow::bail!(
                    "chunk {} hash mismatch on re-read: expected {}, got {}",
                    chunk.index,
                    chunk.hash,
                    actual_hash
                );
            }
            chunks_data[chunk.index] = data;
        }

        // Assemble full file and verify full hash.
        let mut full_data = Vec::with_capacity(file.size as usize);
        for chunk in &chunks_data {
            full_data.extend_from_slice(chunk);
        }
        let full_hash = hex::encode(sha2::Sha256::digest(&full_data));
        if full_hash != file.hash {
            anyhow::bail!(
                "full file hash mismatch: expected {}, got {}",
                file.hash,
                full_hash
            );
        }

        let cache_path = Self::cache_path_static(cache_dir, &file.hash);
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut cache_file = fs::File::create(&cache_path).await?;
        cache_file.write_all(&full_data).await?;

        // Clean up temp chunk directory.
        let _ = tokio::fs::remove_dir_all(&chunk_dest_dir).await;

        info!(hash = %file.hash, size = full_data.len(), chunks = chunks_data.len(), "cached via chunked download (DownloadManager)");
        Ok(())
    }

    fn cache_path_static(cache_dir: &std::path::Path, hash: &str) -> PathBuf {
        let prefix = &hash[..2.min(hash.len())];
        cache_dir.join(prefix).join(hash)
    }

    pub async fn enforce_cache_limits(&self) {
        let result: anyhow::Result<()> = async {
            Self::cleanup_stale_files_static(&self.cache_dir).await?;
            let files = list_cache_files(&self.cache_dir).await?;
            let total_size: u64 = files.iter().map(|e| e.size).sum();
            if total_size <= MAX_CACHE_SIZE_BYTES {
                return Ok(());
            }

            info!(
                total_bytes = total_size,
                limit_bytes = MAX_CACHE_SIZE_BYTES,
                "cache exceeds limit, running LRU eviction"
            );
            let mut entries = files;
            entries.sort_by_key(|e| e.last_accessed);

            let mut freed = 0u64;
            for entry in &entries {
                if total_size - freed <= MAX_CACHE_SIZE_BYTES {
                    break;
                }
                if let Err(e) = tokio::fs::remove_file(&entry.path).await {
                    warn!(path = %entry.path.display(), error = %e, "failed to evict cache file");
                } else {
                    freed += entry.size;
                }
            }
            info!(freed_bytes = freed, "LRU cache eviction complete");
            Ok(())
        }
        .await;

        if let Err(e) = result {
            warn!(error = %e, "cache limit enforcement failed");
        }
    }

    async fn cleanup_stale_files_static(cache_dir: &Path) -> anyhow::Result<()> {
        let cutoff = SystemTime::now() - Duration::from_secs(MAX_FILE_AGE_SECS);
        let mut removed = 0usize;
        let mut freed = 0u64;

        for entry in list_cache_files(cache_dir).await? {
            if entry.last_accessed < cutoff {
                freed += entry.size;
                tokio::fs::remove_file(&entry.path).await?;
                removed += 1;
            }
        }

        if removed > 0 {
            info!(
                removed,
                freed_bytes = freed,
                "cleaned up stale cache files (>90 days)"
            );
        }
        Ok(())
    }
}

struct CacheEntry {
    path: PathBuf,
    size: u64,
    last_accessed: SystemTime,
}

async fn list_cache_files(cache_dir: &Path) -> anyhow::Result<Vec<CacheEntry>> {
    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(cache_dir).await?;
    while let Some(entry) = dir.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            let mut sub = tokio::fs::read_dir(entry.path()).await?;
            while let Some(file) = sub.next_entry().await? {
                let meta = file.metadata().await?;
                entries.push(CacheEntry {
                    path: file.path(),
                    size: meta.len(),
                    last_accessed: meta.accessed().unwrap_or(UNIX_EPOCH),
                });
            }
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file(
        hash: &str,
        download_url: Option<&str>,
        chunks: Vec<ManifestChunk>,
    ) -> ManifestFile {
        ManifestFile {
            path: format!("assets/{}", hash),
            hash: hash.to_string(),
            size: 1024,
            required: true,
            download_url: download_url.map(|u| u.to_string()),
            chunks,
        }
    }

    // ── resolve_file_download_url ─────────────────────────────────────

    #[test]
    fn test_file_url_explicit_download_url_priority() {
        let file = make_file("abc123", Some("https://cdn.example.com/file.bin"), vec![]);
        let url = resolve_file_download_url(&file).unwrap();
        assert_eq!(url, "https://cdn.example.com/file.bin");
    }

    #[test]
    fn test_file_url_missing_download_url_fails() {
        let file = make_file("abc123", None, vec![]);
        let err = resolve_file_download_url(&file).unwrap_err();
        assert!(err.contains("missing download_url"));
    }

    #[test]
    fn test_file_url_blank_download_url_fails() {
        let file = make_file("abc123", Some(""), vec![]);
        let err = resolve_file_download_url(&file).unwrap_err();
        assert!(err.contains("missing download_url"));
    }

    // ── resolve_chunk_download_url ─────────────────────────────────────

    #[test]
    fn test_chunk_url_explicit_no_query_appends_offset_size() {
        let file = make_file("hash42", Some("https://cdn.example.com/chunks"), vec![]);
        let url = resolve_chunk_download_url(&file, 0, 262144).unwrap();
        assert_eq!(url, "https://cdn.example.com/chunks?offset=0&size=262144");
    }

    #[test]
    fn test_chunk_url_explicit_with_query_appends_offset_size() {
        let file = make_file(
            "hash42",
            Some("https://cdn.example.com/chunks?token=xyz"),
            vec![],
        );
        let url = resolve_chunk_download_url(&file, 1024, 65536).unwrap();
        assert_eq!(
            url,
            "https://cdn.example.com/chunks?token=xyz&offset=1024&size=65536"
        );
    }

    #[test]
    fn test_chunk_url_missing_download_url_fails() {
        let file = make_file("hash42", None, vec![]);
        let err = resolve_chunk_download_url(&file, 0, 262144).unwrap_err();
        assert!(err.contains("missing chunk download_url"));
    }

    #[test]
    fn test_chunk_url_blank_download_url_fails() {
        let file = make_file("hash42", Some(""), vec![]);
        let err = resolve_chunk_download_url(&file, 0, 262144).unwrap_err();
        assert!(err.contains("missing chunk download_url"));
    }

    // ── SyncStatus contract ────────────────────────────────────────────

    #[test]
    fn test_sync_status_contract() {
        let status = SyncStatus {
            manifest_loaded: true,
            total_files: 10,
            cached_files: 7,
            missing_files: 3,
            sync_in_progress: false,
            last_error: None,
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["manifest_loaded"], true);
        assert_eq!(json["total_files"], 10);
        assert_eq!(json["cached_files"], 7);
        assert_eq!(json["missing_files"], 3);
        assert_eq!(json["sync_in_progress"], false);
        assert_eq!(json["last_error"], json!(null));
    }

    #[test]
    fn test_sync_result_contract() {
        let result = SyncResult {
            downloaded: 5,
            failed: 2,
            skipped: 1,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["downloaded"], 5);
        assert_eq!(json["failed"], 2);
        assert_eq!(json["skipped"], 1);
    }

    // ── Phase 5 P0: Hash algorithm inference for manifest files ────────

    #[test]
    fn test_infer_hash_algorithm_sha1_by_path() {
        // libraries/ files use SHA1 (Mojang convention).
        let algo = infer_hash_algorithm_for_file(
            "libraries/net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar",
            "eeaaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d",
        );
        assert_eq!(algo, HashAlgorithm::Sha1);
    }

    #[test]
    fn test_infer_hash_algorithm_sha1_by_assets_path() {
        let algo = infer_hash_algorithm_for_file(
            "assets/objects/ee/eeaaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d",
            "eeaaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d",
        );
        assert_eq!(algo, HashAlgorithm::Sha1);
    }

    #[test]
    fn test_infer_hash_algorithm_sha1_by_hash_length() {
        // 40-char hex hash = SHA1.
        let algo = infer_hash_algorithm_for_file(
            "mods/some-mod.jar",
            "eeaaf4c61ddcc5e8a2dabede0f3b482cd9aea943",
        );
        assert_eq!(algo, HashAlgorithm::Sha1);
    }

    #[test]
    fn test_infer_hash_algorithm_sha256_by_hash_length() {
        // 64-char hex hash = SHA256.
        let hash64 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let algo = infer_hash_algorithm_for_file("resources/chunk-file.bin", hash64);
        assert_eq!(algo, HashAlgorithm::Sha256);
    }

    #[test]
    fn test_infer_hash_algorithm_default_sha256_for_unknown_length() {
        // Non-standard hash length defaults to SHA256.
        let algo = infer_hash_algorithm_for_file("unknown/resource.bin", "abc123");
        assert_eq!(algo, HashAlgorithm::Sha256);
    }

    #[test]
    fn test_infer_hash_algorithm_sha256_by_path_overrides_length() {
        // Even with a 40-char hash, if path doesn't match Mojang patterns, it depends on length.
        // But a non-Mojang path with 40-char hash → SHA1 by length inference.
        let algo = infer_hash_algorithm_for_file(
            "mods/optifine.jar",
            "abcdef0123456789abcdef0123456789abcdef12",
        );
        assert_eq!(algo, HashAlgorithm::Sha1);
    }

    // ── Phase 6: Dedup regression tests (resource_sync consumption path) ─

    /// Build download tasks from manifest files (mirrors do_sync task-building path).
    fn build_file_download_tasks(files: &[ManifestFile]) -> Vec<DownloadTask> {
        files
            .iter()
            .map(|file| {
                let url = resolve_file_download_url(file).expect("resolve url");
                let urls = vec![url];
                let dest_path = PathBuf::from("cache").join(&file.path);
                let algo = infer_hash_algorithm_for_file(&file.path, &file.hash);
                DownloadTask {
                    id: file.hash.clone(),
                    name: file.path.clone(),
                    urls,
                    dest_path,
                    expected_sha1: if algo == HashAlgorithm::Sha1 {
                        Some(file.hash.clone())
                    } else {
                        None
                    },
                    expected_sha256: if algo == HashAlgorithm::Sha256 {
                        Some(file.hash.clone())
                    } else {
                        None
                    },
                    expected_size: Some(file.size),
                    hash_algorithm: algo,
                }
            })
            .collect()
    }

    /// Build chunk download tasks from a manifest file (mirrors download_chunked_static).
    fn build_chunk_download_tasks(file: &ManifestFile) -> Vec<DownloadTask> {
        file.chunks
            .iter()
            .map(|chunk| {
                let url = resolve_chunk_download_url(file, chunk.offset, chunk.size)
                    .expect("resolve chunk url");
                let urls = vec![url];
                let dest_path = PathBuf::from("cache/.chunks")
                    .join(&file.hash)
                    .join(format!("{}.chunk", chunk.index));
                DownloadTask {
                    id: format!("{}-chunk-{}", file.hash, chunk.index),
                    name: format!("{} chunk {}", file.path, chunk.index),
                    urls,
                    dest_path,
                    expected_sha1: None,
                    expected_sha256: Some(chunk.hash.clone()),
                    expected_size: Some(chunk.size),
                    hash_algorithm: HashAlgorithm::Sha256,
                }
            })
            .collect()
    }

    /// Normal file tasks: dedup must remove duplicate dest_path and leave
    /// the vector mutated in-place (no clone regression).
    #[test]
    fn test_dedup_file_tasks_no_clone_regression() {
        let file_a = ManifestFile {
            path: "libs/a.jar".to_string(),
            hash: "aaa".to_string(),
            size: 100,
            required: true,
            download_url: Some("https://cdn.example.com/a.jar".to_string()),
            chunks: vec![],
        };
        let file_a_dup = ManifestFile {
            path: "libs/a.jar".to_string(), // same dest_path
            hash: "aaa".to_string(),
            size: 100,
            required: true,
            download_url: Some("https://cdn.example.com/a.jar".to_string()),
            chunks: vec![],
        };
        let file_b = ManifestFile {
            path: "libs/b.jar".to_string(),
            hash: "bbb".to_string(),
            size: 200,
            required: true,
            download_url: Some("https://cdn.example.com/b.jar".to_string()),
            chunks: vec![],
        };

        let mut tasks = build_file_download_tasks(&[file_a, file_a_dup, file_b]);
        assert_eq!(tasks.len(), 3, "three tasks before dedup");
        DownloadManager::dedup(&mut tasks);
        assert_eq!(tasks.len(), 2, "duplicate dest_path must be removed");
        assert!(tasks.iter().any(|t| t.dest_path.ends_with("libs/a.jar")));
        assert!(tasks.iter().any(|t| t.dest_path.ends_with("libs/b.jar")));
    }

    /// Chunk tasks: dedup must remove duplicate chunk dest_path entries
    /// and leave the vector mutated in-place.
    #[test]
    fn test_dedup_chunk_tasks_no_clone_regression() {
        let chunk_file = ManifestFile {
            path: "assets/big.dat".to_string(),
            hash: "ff".to_string(),
            size: 200,
            required: true,
            download_url: Some("https://cdn.example.com/big.dat".to_string()),
            chunks: vec![
                ManifestChunk {
                    index: 0,
                    offset: 0,
                    size: 100,
                    hash: "c0".to_string(),
                },
                // Duplicate chunk 0 (same dest_path) — should be removed by dedup.
                ManifestChunk {
                    index: 0,
                    offset: 0,
                    size: 100,
                    hash: "c0".to_string(),
                },
                ManifestChunk {
                    index: 1,
                    offset: 100,
                    size: 100,
                    hash: "c1".to_string(),
                },
            ],
        };

        let mut tasks = build_chunk_download_tasks(&chunk_file);
        assert_eq!(tasks.len(), 3, "three chunk tasks before dedup");
        DownloadManager::dedup(&mut tasks);
        assert_eq!(tasks.len(), 2, "duplicate chunk dest_path must be removed");
    }

    /// Mixed file-style and chunk-style tasks must not collide when they
    /// have distinct dest_path prefixes.
    #[test]
    fn test_dedup_mixed_file_and_chunk_no_collision() {
        let regular = ManifestFile {
            path: "mods/example.jar".to_string(),
            hash: "abc".to_string(),
            size: 50,
            required: true,
            download_url: Some("https://cdn.example.com/example.jar".to_string()),
            chunks: vec![],
        };
        let chunked = ManifestFile {
            path: "mods/example.jar".to_string(),
            hash: "xyz".to_string(),
            size: 100,
            required: true,
            download_url: Some("https://cdn.example.com/example-chunks".to_string()),
            chunks: vec![ManifestChunk {
                index: 0,
                offset: 0,
                size: 100,
                hash: "c0".to_string(),
            }],
        };

        let mut file_tasks = build_file_download_tasks(&[regular]);
        let chunk_tasks = build_chunk_download_tasks(&chunked);

        // Regular dest: cache/mods/example.jar
        // Chunk dest:   cache/.chunks/xyz/0.chunk
        // These are different paths → dedup should not remove either.
        file_tasks.extend(chunk_tasks);
        assert_eq!(file_tasks.len(), 2, "one file task + one chunk task");
        DownloadManager::dedup(&mut file_tasks);
        assert_eq!(file_tasks.len(), 2, "file and chunk paths must not collide");
    }
}
