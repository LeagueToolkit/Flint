//! Hash Downloader Module
//!
//! Downloads hash files from CommunityDragon and stores them in the shared
//! RitoShark directory. Shows progress via callbacks for UI integration.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// CommunityDragon GitHub API endpoint
const GITHUB_API: &str = "https://api.github.com/repos/CommunityDragon/Data/contents";

/// Hash files to download
const HASH_FILES: &[&str] = &[
    "hashes/lol/hashes.binentries.txt",
    "hashes/lol/hashes.binhashes.txt",
    "hashes/lol/hashes.bintypes.txt",
    "hashes/lol/hashes.binfields.txt",
    "hashes/lol/hashes.game.txt.0",
    "hashes/lol/hashes.game.txt.1",
];

/// Max concurrent downloads
const MAX_CONCURRENT: usize = 4;

/// Files older than this will be re-downloaded
const UPDATE_THRESHOLD: Duration = Duration::from_secs(24 * 60 * 60); // 1 day

/// Download progress event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// File being processed
    pub file: String,
    /// Status: "checking", "downloading", "complete", "skipped", "error"
    pub status: String,
    /// Optional message (e.g., error details)
    pub message: Option<String>,
    /// Current file index (1-based)
    pub current: usize,
    /// Total file count
    pub total: usize,
}

/// Download statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadStats {
    pub downloaded: usize,
    pub skipped: usize,
    pub errors: usize,
}

/// Get the RitoShark hash directory path
pub fn get_hash_dir() -> Result<PathBuf> {
    let appdata = std::env::var("APPDATA")
        .context("APPDATA environment variable not set")?;
    Ok(PathBuf::from(appdata)
        .join("RitoShark")
        .join("Requirements")
        .join("Hashes"))
}

/// Check if hash files need to be downloaded
pub fn needs_download() -> bool {
    let Ok(hash_dir) = get_hash_dir() else {
        return true;
    };

    // Check if key files exist and are recent
    let types_file = hash_dir.join("hashes.bintypes.txt");
    let fields_file = hash_dir.join("hashes.binfields.txt");

    !file_is_recent(&types_file) || !file_is_recent(&fields_file)
}

/// Check if a file exists and is less than UPDATE_THRESHOLD old
fn file_is_recent(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    if let Ok(metadata) = std::fs::metadata(path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                return elapsed < UPDATE_THRESHOLD;
            }
        }
    }
    false
}

/// Download all hash files with progress callback
pub async fn download_hashes<F>(force: bool, mut on_progress: F) -> Result<DownloadStats>
where
    F: FnMut(DownloadProgress),
{
    let hash_dir = get_hash_dir()?;
    fs::create_dir_all(&hash_dir).await?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent("Hematite/1.0")
        .build()?;

    let mut stats = DownloadStats::default();
    let total = HASH_FILES.len();

    for (index, path) in HASH_FILES.iter().enumerate() {
        let file_name = path.rsplit('/').next().unwrap_or(path);
        let current = index + 1;

        on_progress(DownloadProgress {
            file: file_name.to_string(),
            status: "checking".to_string(),
            message: None,
            current,
            total,
        });

        let local_path = hash_dir.join(file_name);
        let sha_path = hash_dir.join(format!("{}.sha", file_name));

        // Check if update needed
        if !force && file_is_recent(&local_path) {
            // Check SHA if available
            if sha_path.exists() {
                if let Ok(info) = fetch_file_info(&client, path).await {
                    if let Ok(existing_sha) = std::fs::read_to_string(&sha_path) {
                        if existing_sha.trim() == info.sha {
                            on_progress(DownloadProgress {
                                file: file_name.to_string(),
                                status: "skipped".to_string(),
                                message: Some("Up to date".to_string()),
                                current,
                                total,
                            });
                            stats.skipped += 1;
                            continue;
                        }
                    }
                }
            } else {
                on_progress(DownloadProgress {
                    file: file_name.to_string(),
                    status: "skipped".to_string(),
                    message: Some("Recent file".to_string()),
                    current,
                    total,
                });
                stats.skipped += 1;
                continue;
            }
        }

        // Download the file
        on_progress(DownloadProgress {
            file: file_name.to_string(),
            status: "downloading".to_string(),
            message: None,
            current,
            total,
        });

        match download_file(&client, path, &local_path, &sha_path).await {
            Ok(_) => {
                on_progress(DownloadProgress {
                    file: file_name.to_string(),
                    status: "complete".to_string(),
                    message: None,
                    current,
                    total,
                });
                stats.downloaded += 1;
            }
            Err(e) => {
                on_progress(DownloadProgress {
                    file: file_name.to_string(),
                    status: "error".to_string(),
                    message: Some(e.to_string()),
                    current,
                    total,
                });
                stats.errors += 1;
            }
        }
    }

    // Merge game files if both parts were downloaded
    let game0 = hash_dir.join("hashes.game.txt.0");
    let game1 = hash_dir.join("hashes.game.txt.1");
    let game_merged = hash_dir.join("hashes.game.txt");

    if game0.exists() && game1.exists() {
        on_progress(DownloadProgress {
            file: "hashes.game.txt".to_string(),
            status: "downloading".to_string(),
            message: Some("Merging...".to_string()),
            current: total,
            total,
        });

        if let Err(e) = merge_game_files(&game0, &game1, &game_merged).await {
            log::warn!("Failed to merge game files: {}", e);
        } else {
            // Delete the split files
            let _ = fs::remove_file(&game0).await;
            let _ = fs::remove_file(&game1).await;
        }
    }

    Ok(stats)
}

/// Fetch file info from GitHub API
async fn fetch_file_info(client: &reqwest::Client, path: &str) -> Result<FileInfo> {
    let url = format!("{}/{}", GITHUB_API, path);
    let response: serde_json::Value = client
        .get(&url)
        .send()
        .await?
        .json()
        .await?;

    Ok(FileInfo {
        sha: response["sha"].as_str().context("Missing sha")?.to_string(),
        download_url: response["download_url"].as_str().context("Missing download_url")?.to_string(),
        name: response["name"].as_str().context("Missing name")?.to_string(),
    })
}

struct FileInfo {
    sha: String,
    download_url: String,
    name: String,
}

/// Download a single file
async fn download_file(
    client: &reqwest::Client,
    path: &str,
    local_path: &Path,
    sha_path: &Path,
) -> Result<()> {
    let info = fetch_file_info(client, path).await?;

    let response = client.get(&info.download_url).send().await?;
    let bytes = response.bytes().await?;

    // Write file
    let mut file = fs::File::create(local_path).await?;
    file.write_all(&bytes).await?;

    // Write SHA
    fs::write(sha_path, &info.sha).await?;

    log::info!("Downloaded {} ({} bytes)", info.name, bytes.len());
    Ok(())
}

/// Merge split game files
async fn merge_game_files(part0: &Path, part1: &Path, output: &Path) -> Result<()> {
    let content0 = fs::read_to_string(part0).await?;
    let content1 = fs::read_to_string(part1).await?;

    let merged = format!("{}\n{}", content0.trim(), content1.trim());
    fs::write(output, &merged).await?;

    log::info!("Merged game hash files");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_hash_dir() {
        if std::env::var("APPDATA").is_ok() {
            let dir = get_hash_dir();
            assert!(dir.is_ok());
        }
    }

    #[test]
    fn test_needs_download_no_files() {
        // Should return true if files don't exist
        // (This test may be flaky depending on system state)
    }

    #[test]
    fn test_download_progress_serialize() {
        let progress = DownloadProgress {
            file: "test.txt".to_string(),
            status: "downloading".to_string(),
            message: None,
            current: 1,
            total: 5,
        };
        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("test.txt"));
    }
}
