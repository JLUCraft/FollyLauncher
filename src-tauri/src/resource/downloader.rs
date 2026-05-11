//! Resumable downloader with .part files, Range requests, hash verification,
//! retry/backoff, mirror fallback, bounded concurrency, progress events, and dedup.

use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

/// Maximum concurrent downloads.
const MAX_CONCURRENT_DOWNLOADS: usize = 8;

/// Max retry attempts per URL.
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (in milliseconds).
const BACKOFF_BASE_MS: u64 = 100;

/// Progress event emitted during a download.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub task_id: String,
    pub group_id: Option<String>,
    pub downloaded: u64,
    pub total: u64,
    pub speed_bytes_per_sec: u64,
    pub state: DownloadState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DownloadState {
    Pending,
    Downloading,
    Verifying,
    Completed,
    Failed,
}

/// Hash algorithm for integrity verification.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HashAlgorithm {
    #[default]
    Sha1,
    Sha256,
}

/// A download task for the queue.
#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub id: String,
    pub name: String,
    pub urls: Vec<String>,
    pub dest_path: PathBuf,
    pub expected_sha1: Option<String>,
    pub expected_sha256: Option<String>,
    pub expected_size: Option<u64>,
    /// Hash algorithm to use for integrity check. Defaults to Sha1.
    pub hash_algorithm: HashAlgorithm,
}

/// Result of attempting a single download.
#[derive(Debug)]
pub struct DownloadResult {
    pub task_id: String,
    pub success: bool,
    pub bytes_written: u64,
    pub error: Option<String>,
}

/// Deduplicated download manager.
pub struct DownloadManager {
    client: reqwest::Client,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .connect_timeout(Duration::from_secs(15))
                .build()
                .expect("failed to build downloader reqwest client"),
        }
    }

    /// Deduplicate tasks by dest_path. First task wins, subsequent duplicates are skipped.
    pub fn dedup(tasks: &mut Vec<DownloadTask>) {
        let mut seen: HashSet<PathBuf> = HashSet::new();
        tasks.retain(|t| seen.insert(t.dest_path.clone()));
    }

    /// Download a batch of tasks with bounded concurrency and progress callbacks.
    pub async fn download_batch(
        &self,
        tasks: Vec<DownloadTask>,
        progress_cb: Arc<dyn Fn(DownloadProgress) + Send + Sync>,
    ) -> Vec<DownloadResult> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_DOWNLOADS));
        let mut handles = Vec::new();

        for task in tasks {
            let client = self.client.clone();
            let sem = semaphore.clone();
            let progress = progress_cb.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await;
                Self::download_task(&client, &task, &progress).await
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(DownloadResult {
                    task_id: "join_error".to_string(),
                    success: false,
                    bytes_written: 0,
                    error: Some(format!("task join error: {e}")),
                }),
            }
        }
        let total_bytes: u64 = results.iter().map(|r| r.bytes_written).sum();
        info!(total_bytes, "batch download complete");
        results
    }

    /// Download a single task with retry and mirror fallback.
    async fn download_task(
        client: &reqwest::Client,
        task: &DownloadTask,
        progress: &Arc<dyn Fn(DownloadProgress) + Send + Sync>,
    ) -> DownloadResult {
        // Dedup check: if file already exists and hash matches, skip
        if task.dest_path.exists() {
            let matched = match task.hash_algorithm {
                HashAlgorithm::Sha1 => task
                    .expected_sha1
                    .as_ref()
                    .map(|h| verify_file_sha1(&task.dest_path, h))
                    .unwrap_or(false),
                HashAlgorithm::Sha256 => task
                    .expected_sha256
                    .as_ref()
                    .map(|h| verify_file_sha256(&task.dest_path, h))
                    .unwrap_or(false),
            };
            if matched {
                info!(name = %task.name, "file already exists with valid hash, skipping");
                progress(DownloadProgress {
                    task_id: task.id.clone(),
                    group_id: None,
                    downloaded: 0,
                    total: 0,
                    speed_bytes_per_sec: 0,
                    state: DownloadState::Completed,
                });
                return DownloadResult {
                    task_id: task.id.clone(),
                    success: true,
                    bytes_written: 0,
                    error: None,
                };
            }
        }

        // Ensure parent directory exists
        if let Some(parent) = task.dest_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return DownloadResult {
                    task_id: task.id.clone(),
                    success: false,
                    bytes_written: 0,
                    error: Some(format!("cannot create parent directory: {e}")),
                };
            }
        }

        progress(DownloadProgress {
            task_id: task.id.clone(),
            group_id: None,
            downloaded: 0,
            total: task.expected_size.unwrap_or(0),
            speed_bytes_per_sec: 0,
            state: DownloadState::Downloading,
        });

        // Try each mirror URL
        for (url_idx, url) in task.urls.iter().enumerate() {
            for attempt in 0..MAX_RETRIES {
                if attempt > 0 {
                    let backoff = Duration::from_millis(BACKOFF_BASE_MS * 2u64.pow(attempt));
                    tokio::time::sleep(backoff).await;
                }

                match Self::download_single(client, task, url).await {
                    Ok(bytes) => {
                        progress(DownloadProgress {
                            task_id: task.id.clone(),
                            group_id: None,
                            downloaded: bytes,
                            total: bytes,
                            speed_bytes_per_sec: 0,
                            state: DownloadState::Verifying,
                        });

                        // Verify hash if expected
                        let hash_ok = match task.hash_algorithm {
                            HashAlgorithm::Sha1 => task
                                .expected_sha1
                                .as_ref()
                                .map(|h| verify_file_sha1(&task.dest_path, h))
                                .unwrap_or(true),
                            HashAlgorithm::Sha256 => task
                                .expected_sha256
                                .as_ref()
                                .map(|h| verify_file_sha256(&task.dest_path, h))
                                .unwrap_or(true),
                        };
                        if !hash_ok {
                            let alg_name = match task.hash_algorithm {
                                HashAlgorithm::Sha1 => "SHA-1",
                                HashAlgorithm::Sha256 => "SHA-256",
                            };
                            warn!(name = %task.name, url = %url, "{alg_name} mismatch, will retry with next mirror");
                            let _ = fs::remove_file(&task.dest_path).await;
                            break;
                        }

                        progress(DownloadProgress {
                            task_id: task.id.clone(),
                            group_id: None,
                            downloaded: bytes,
                            total: bytes,
                            speed_bytes_per_sec: 0,
                            state: DownloadState::Completed,
                        });

                        info!(name = %task.name, bytes, url_idx, "download completed");
                        return DownloadResult {
                            task_id: task.id.clone(),
                            success: true,
                            bytes_written: bytes,
                            error: None,
                        };
                    }
                    Err(e) => {
                        warn!(name = %task.name, url = %url, attempt, error = %e, "download attempt failed");
                        if attempt == MAX_RETRIES - 1 {
                            debug!(name = %task.name, url = %url, "exhausted retries for this URL");
                        }
                    }
                }
            }
        }

        progress(DownloadProgress {
            task_id: task.id.clone(),
            group_id: None,
            downloaded: 0,
            total: 0,
            speed_bytes_per_sec: 0,
            state: DownloadState::Failed,
        });

        DownloadResult {
            task_id: task.id.clone(),
            success: false,
            bytes_written: 0,
            error: Some("all mirrors exhausted".to_string()),
        }
    }

    /// Download a single file from a URL with resume support.
    async fn download_single(
        client: &reqwest::Client,
        task: &DownloadTask,
        url: &str,
    ) -> Result<u64, crate::error::LauncherError> {
        let part_path = task.dest_path.with_extension("part");

        // Check for existing partial download
        let resume_offset = if part_path.exists() {
            match fs::metadata(&part_path).await {
                Ok(meta) => meta.len(),
                Err(_) => 0,
            }
        } else {
            0
        };

        // Build request with optional Range header
        let mut req = client.get(url);
        if resume_offset > 0 {
            debug!(offset = resume_offset, url = %url, "resuming partial download");
            req = req.header("Range", format!("bytes={}-", resume_offset));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        let status = resp.status();
        let is_range = status == reqwest::StatusCode::PARTIAL_CONTENT;

        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(crate::error::LauncherError::from(format!("HTTP {}", status)));
        }

        // For resume: if server doesn't support Range, start fresh
        let start_offset = if is_range || resume_offset == 0 {
            resume_offset
        } else {
            // Server didn't honor Range; truncate and restart
            let _ = fs::remove_file(&part_path).await;
            0
        };

        // Open file for writing (append mode if resuming)
        let mut file = if start_offset > 0 {
            fs::OpenOptions::new()
                .append(true)
                .open(&part_path)
                .await
                .map_err(|e| format!("cannot open part file for append: {e}"))?
        } else {
            fs::File::create(&part_path)
                .await
                .map_err(|e| format!("cannot create part file: {e}"))?
        };

        let mut stream = resp.bytes_stream();
        let mut downloaded = start_offset;

        while let Some(chunk) = futures_util::StreamExt::next(&mut stream).await {
            let chunk = chunk.map_err(|e| format!("stream error: {e}"))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("write error: {e}"))?;
            downloaded += chunk.len() as u64;
        }

        file.flush()
            .await
            .map_err(|e| format!("flush error: {e}"))?;
        drop(file);

        // Rename .part -> final destination
        if task.dest_path.exists() {
            let _ = fs::remove_file(&task.dest_path).await;
        }
        fs::rename(&part_path, &task.dest_path)
            .await
            .map_err(|e| format!("rename part file failed: {e}"))?;

        Ok(downloaded)
    }
}

/// Verify a file's SHA-1 hash matches the expected value.
pub fn verify_file_sha1(path: &Path, expected_sha1: &str) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => {
            let actual = sha1_smol::Sha1::from(&bytes).digest().to_string();
            actual.eq_ignore_ascii_case(expected_sha1)
        }
        Err(_) => false,
    }
}

/// Verify a file's SHA-256 hash matches the expected value.
pub fn verify_file_sha256(path: &Path, expected_sha256: &str) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => {
            let actual = hex::encode(sha2::Sha256::digest(&bytes));
            actual.eq_ignore_ascii_case(expected_sha256)
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedup_by_path() {
        let mut tasks = vec![
            DownloadTask {
                id: "1".to_string(),
                name: "a".to_string(),
                urls: vec!["a".to_string()],
                dest_path: PathBuf::from("/tmp/a.jar"),
                expected_sha1: None,
                expected_sha256: None,
                expected_size: None,
                hash_algorithm: HashAlgorithm::Sha1,
            },
            DownloadTask {
                id: "2".to_string(),
                name: "a-dup".to_string(),
                urls: vec!["a".to_string()],
                dest_path: PathBuf::from("/tmp/a.jar"),
                expected_sha1: None,
                expected_sha256: None,
                expected_size: None,
                hash_algorithm: HashAlgorithm::Sha1,
            },
            DownloadTask {
                id: "3".to_string(),
                name: "b".to_string(),
                urls: vec!["b".to_string()],
                dest_path: PathBuf::from("/tmp/b.jar"),
                expected_sha1: None,
                expected_sha256: None,
                expected_size: None,
                hash_algorithm: HashAlgorithm::Sha1,
            },
        ];
        DownloadManager::dedup(&mut tasks);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "1");
        assert_eq!(tasks[1].id, "3");
    }

    #[test]
    fn test_verify_file_sha1_match() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.bin");
        std::fs::write(&path, b"hello").unwrap();
        assert!(verify_file_sha1(
            &path,
            "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
        ));
    }

    #[test]
    fn test_verify_file_sha1_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.bin");
        std::fs::write(&path, b"hello").unwrap();
        assert!(!verify_file_sha1(&path, "0".repeat(40).as_str()));
    }

    #[test]
    fn test_verify_file_sha256_match() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.bin");
        std::fs::write(&path, b"hello").unwrap();
        let expected = hex::encode(sha2::Sha256::digest(b"hello"));
        assert!(verify_file_sha256(&path, &expected));
    }

    #[test]
    fn test_verify_file_sha256_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.bin");
        std::fs::write(&path, b"hello").unwrap();
        assert!(!verify_file_sha256(&path, "0".repeat(64).as_str()));
    }

    #[test]
    fn test_download_result_bytes_written_field() {
        let result = DownloadResult {
            task_id: "t1".into(),
            success: true,
            bytes_written: 12345,
            error: None,
        };
        assert_eq!(result.bytes_written, 12345);
    }
}
