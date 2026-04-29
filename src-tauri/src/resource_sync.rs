use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

const CHUNK_SIZE: usize = 256 * 1024;
const MAX_CONCURRENT_CHUNKS: usize = 4;
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

#[derive(Debug, Clone, Serialize)]
pub struct BitSwapStatus {
    pub total_chunks: usize,
    pub downloaded_chunks: usize,
    pub active_sources: usize,
}

pub struct ResourceSyncService {
    cache_dir: PathBuf,
    client: reqwest::Client,
}

impl ResourceSyncService {
    pub fn new(cache_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&cache_dir);
        let service = Self {
            cache_dir: cache_dir.clone(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("failed to build reqwest client"),
        };
        tokio::spawn(async move {
            let _ = Self::cleanup_stale_files_static(&cache_dir).await;
        });
        service
    }

    pub async fn fetch_manifest(
        &self,
        base_url: &str,
        instance_id: &str,
    ) -> anyhow::Result<ResourceManifest> {
        let url = format!(
            "{}/v1/instances/{}/manifest",
            base_url.trim_end_matches('/'),
            instance_id
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("manifest endpoint returned {}", resp.status());
        }
        let json: serde_json::Value = resp.json().await?;
        let manifest_value = json
            .get("manifest")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("manifest field missing in response"))?;
        let manifest: ResourceManifest = serde_json::from_value(manifest_value).map_err(|e| {
            anyhow::anyhow!(
                "invalid manifest payload for instance {}: {}",
                instance_id,
                e
            )
        })?;
        Ok(manifest)
    }

    pub fn check_cache(
        &self,
        manifest: &ResourceManifest,
    ) -> (Vec<ManifestFile>, Vec<ManifestFile>) {
        let mut cached = Vec::new();
        let mut missing = Vec::new();

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
        let mut handles = Vec::new();

        for file in missing {
            let url = match &file.download_url {
                Some(u) => u.clone(),
                None => {
                    if file.required {
                        warn!(path = %file.path, "no download_url for required file");
                    }
                    continue;
                }
            };

            let sem = semaphore.clone();
            let cache_dir = self.cache_dir.clone();
            let client = self.client.clone();
            let file = file.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await;

                let result = if !file.chunks.is_empty() {
                    Self::download_chunked_static(&client, &cache_dir, &url, &file).await
                } else {
                    Self::download_and_verify_static(&client, &cache_dir, &url, &file.hash, file.size).await
                };

                match result {
                    Ok(_) => (true, false, false),
                    Err(e) => {
                        warn!(path = %file.path, error = %e, "download failed");
                        if file.required {
                            (false, true, false)
                        } else {
                            (false, false, true)
                        }
                    }
                }
            }));
        }

        let mut downloaded = 0usize;
        let mut failed = 0usize;
        let mut skipped = 0usize;

        for handle in handles {
            match handle.await {
                Ok((d, f, s)) => {
                    if d { downloaded += 1; }
                    if f { failed += 1; }
                    if s { skipped += 1; }
                }
                Err(e) => {
                    warn!(error = %e, "download task panicked");
                    failed += 1;
                }
            }
        }

        let result = SyncResult { downloaded, failed, skipped };
        if downloaded > 0 {
            self.enforce_cache_limits().await;
        }
        result
    }
    pub async fn get_bitswap_status(
        &self,
        file: &ManifestFile,
    ) -> BitSwapStatus {
        let total = file.chunks.len();
        let mut downloaded = 0usize;
        for chunk in &file.chunks {
            let chunk_cache = self.cache_path(&chunk.hash);
            if chunk_cache.exists() {
                downloaded += 1;
            }
        }
        BitSwapStatus {
            total_chunks: total,
            downloaded_chunks: downloaded,
            active_sources: if total > 0 { 1 } else { 0 },
        }
    }

    fn cache_path(&self, hash: &str) -> PathBuf {
        let prefix = &hash[..2.min(hash.len())];
        self.cache_dir.join(prefix).join(hash)
    }

    async fn download_and_verify_static(
        client: &reqwest::Client,
        cache_dir: &Path,
        url: &str,
        expected_hash: &str,
        _expected_size: u64,
    ) -> anyhow::Result<()> {
        let resp = client.get(url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("download returned status {}", resp.status());
        }

        let bytes = resp.bytes().await?;

        let hash = sha2::Sha256::digest(&bytes);
        let hash_hex = hex::encode(hash);
        if hash_hex != expected_hash {
            anyhow::bail!(
                "SHA256 mismatch: expected {}, got {}",
                expected_hash,
                hash_hex
            );
        }

        let cache_path = Self::cache_path_static(cache_dir, expected_hash);
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(&cache_path).await?;
        file.write_all(&bytes).await?;
        Ok(())
    }

    async fn download_chunked_static(
        client: &reqwest::Client,
        cache_dir: &Path,
        base_url: &str,
        file: &ManifestFile,
    ) -> anyhow::Result<()> {
        let mut chunks_data: Vec<Vec<u8>> = Vec::new();
        chunks_data.resize(file.chunks.len(), Vec::with_capacity(CHUNK_SIZE));

        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_CHUNKS));
        let mut handles = Vec::new();

        for chunk in &file.chunks {
            let url = format!(
                "{}/chunks/{}?offset={}&size={}",
                base_url.trim_end_matches('/'),
                &file.hash,
                chunk.offset,
                chunk.size,
            );
            let client = client.clone();
            let expected_hash = chunk.hash.clone();
            let sem = semaphore.clone();
            let idx = chunk.index;

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await;
                let resp = client.get(&url).send().await.map_err(|e| {
                    anyhow::anyhow!("chunk {} download failed: {}", idx, e)
                })?;
                if !resp.status().is_success() {
                    anyhow::bail!("chunk {} download returned {}", idx, resp.status());
                }
                let bytes = resp.bytes().await.map_err(|e| {
                    anyhow::anyhow!("chunk {} read failed: {}", idx, e)
                })?;
                let actual_hash = hex::encode(sha2::Sha256::digest(&bytes));
                if actual_hash != expected_hash {
                    anyhow::bail!("chunk {} hash mismatch: expected {}, got {}",
                        idx, expected_hash, actual_hash);
                }
                Ok((idx, bytes.to_vec()))
            }));
        }

        for handle in handles {
            let result = handle.await.map_err(|e| anyhow::anyhow!("join error: {}", e))??;
            let (idx, data) = result;
            chunks_data[idx] = data;
        }

        let mut full_data = Vec::new();
        for chunk in &chunks_data {
            full_data.extend_from_slice(chunk);
        }
        let full_hash = hex::encode(sha2::Sha256::digest(&full_data));
        if full_hash != file.hash {
            anyhow::bail!("full file hash mismatch: expected {}, got {}", file.hash, full_hash);
        }

        let cache_path = Self::cache_path_static(cache_dir, &file.hash);
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut cache_file = fs::File::create(&cache_path).await?;
        cache_file.write_all(&full_data).await?;

        info!(hash = %file.hash, size = full_data.len(), chunks = chunks_data.len(), "cached via chunked download");
        Ok(())
    }

    fn cache_path_static(cache_dir: &std::path::Path, hash: &str) -> PathBuf {
        let prefix = &hash[..2.min(hash.len())];
        cache_dir.join(prefix).join(hash)
    }

    pub async fn enforce_cache_limits(&self) {
        if let Err(e) = self.do_enforce().await {
            warn!(error = %e, "cache limit enforcement failed");
        }
    }

    async fn do_enforce(&self) -> anyhow::Result<()> {
        self.cleanup_stale_files().await?;
        let total_size = self.calculate_cache_size().await?;
        if total_size <= MAX_CACHE_SIZE_BYTES {
            return Ok(());
        }

        info!(total_bytes = total_size, limit_bytes = MAX_CACHE_SIZE_BYTES, "cache exceeds limit, running LRU eviction");
        let mut entries = self.collect_cache_entries().await?;
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

    async fn calculate_cache_size(&self) -> anyhow::Result<u64> {
        let mut total = 0u64;
        let mut dir = tokio::fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = dir.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let mut sub = tokio::fs::read_dir(entry.path()).await?;
                while let Some(file) = sub.next_entry().await? {
                    if let Ok(meta) = file.metadata().await {
                        total += meta.len();
                    }
                }
            }
        }
        Ok(total)
    }

    async fn collect_cache_entries(&self) -> anyhow::Result<Vec<CacheEntry>> {
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = dir.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let mut sub = tokio::fs::read_dir(entry.path()).await?;
                while let Some(file) = sub.next_entry().await? {
                    if let Ok(meta) = file.metadata().await {
                        entries.push(CacheEntry {
                            path: file.path(),
                            size: meta.len(),
                            last_accessed: meta.accessed().unwrap_or(SystemTime::UNIX_EPOCH),
                        });
                    }
                }
            }
        }
        Ok(entries)
    }

    async fn cleanup_stale_files(&self) -> anyhow::Result<()> {
        Self::cleanup_stale_files_static(&self.cache_dir).await
    }

    async fn cleanup_stale_files_static(cache_dir: &Path) -> anyhow::Result<()> {
        let now = SystemTime::now();
        let cutoff = now - Duration::from_secs(MAX_FILE_AGE_SECS);
        let mut removed = 0usize;
        let mut freed = 0u64;

        let mut dir = tokio::fs::read_dir(cache_dir).await?;
        while let Some(entry) = dir.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let mut sub = tokio::fs::read_dir(entry.path()).await?;
                while let Some(file) = sub.next_entry().await? {
                    let meta = file.metadata().await?;
                    let accessed = meta.accessed().unwrap_or(UNIX_EPOCH);
                    if accessed < cutoff {
                        freed += meta.len();
                        tokio::fs::remove_file(file.path()).await?;
                        removed += 1;
                    }
                }
            }
        }

        if removed > 0 {
            info!(removed, freed_bytes = freed, "cleaned up stale cache files (>90 days)");
        }
        Ok(())
    }
}

struct CacheEntry {
    path: PathBuf,
    size: u64,
    last_accessed: SystemTime,
}
