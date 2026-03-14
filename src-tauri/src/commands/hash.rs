use crate::core::hash::{build_hash_db, force_rebuild_hash_db, downloader::get_ritoshark_hash_dir, download_hashes as core_download_hashes, DownloadStats};
use crate::state::LmdbCacheState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Status information about the loaded hash database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashStatus {
    /// Number of entries in the LMDB database (approximate, via file size heuristic)
    pub loaded_count: usize,
    pub last_updated: Option<String>,
}

/// Downloads hash files from CommunityDragon repository
#[tauri::command]
pub async fn download_hashes(force: bool) -> Result<DownloadStats, String> {
    let hash_dir = get_ritoshark_hash_dir()
        .map_err(|e| format!("Failed to get hash directory: {}", e))?;
    let stats = core_download_hashes(&hash_dir, force)
        .await
        .map_err(|e| format!("Failed to download hashes: {}", e))?;
    Ok(stats)
}

/// Returns information about the current LMDB hash database
#[tauri::command]
pub async fn get_hash_status(lmdb: State<'_, LmdbCacheState>) -> Result<HashStatus, String> {
    let hash_dir = get_ritoshark_hash_dir()
        .map_err(|e| format!("Failed to get hash directory: {}", e))?;

    // Approximate entry count from data.mdb file size (heuristic: ~40 bytes/entry)
    let lmdb_dir = hash_dir.join("hashes.lmdb");
    let loaded_count = std::fs::metadata(lmdb_dir.join("data.mdb"))
        .map(|m| (m.len() / 40) as usize)
        .unwrap_or(0);

    let last_updated = if hash_dir.exists() {
        std::fs::metadata(&hash_dir)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|time| {
                use std::time::SystemTime;
                time.duration_since(SystemTime::UNIX_EPOCH)
                    .ok()
                    .map(|d| {
                        let secs = d.as_secs();
                        chrono::DateTime::from_timestamp(secs as i64, 0)
                            .unwrap_or_default()
                            .format("%Y-%m-%dT%H:%M:%SZ")
                            .to_string()
                    })
            })
    } else {
        None
    };

    // Warm the env (open if not already open)
    let hash_dir_str = hash_dir.to_string_lossy().into_owned();
    let _ = lmdb.get_env(&hash_dir_str);

    Ok(HashStatus { loaded_count, last_updated })
}

/// Rebuild hashes.lmdb from .txt files if stale, then open the env
#[tauri::command]
pub async fn reload_hashes(lmdb: State<'_, LmdbCacheState>) -> Result<(), String> {
    let hash_dir = get_ritoshark_hash_dir()
        .map_err(|e| format!("Failed to get hash directory: {}", e))?;
    let hash_dir_str = hash_dir.to_string_lossy().into_owned();

    // Clear old env first (needed on Windows before deleting/overwriting LMDB files)
    lmdb.clear();

    // Rebuild LMDB from .txt files if any source is newer
    if !build_hash_db(&hash_dir_str) {
        return Err("Failed to build hash database".to_string());
    }

    // Open the fresh env
    if lmdb.prime(&hash_dir_str).is_some() {
        tracing::info!("LMDB hash database reloaded from {}", hash_dir_str);
        Ok(())
    } else {
        Err("Failed to open hash database after rebuild".to_string())
    }
}

/// Force rebuild hashes.lmdb from .txt files regardless of timestamps
///
/// Use this when hash resolution logic has changed and databases need regeneration
#[tauri::command]
pub async fn force_rebuild_hashes(lmdb: State<'_, LmdbCacheState>) -> Result<(), String> {
    let hash_dir = get_ritoshark_hash_dir()
        .map_err(|e| format!("Failed to get hash directory: {}", e))?;
    let hash_dir_str = hash_dir.to_string_lossy().into_owned();

    // Clear old env first (needed on Windows before deleting/overwriting LMDB files)
    lmdb.clear();

    // Force rebuild LMDB from .txt files
    if !force_rebuild_hash_db(&hash_dir_str) {
        return Err("Failed to force rebuild hash database".to_string());
    }

    // Reload BIN hash cache to pick up new asset path hashes
    crate::core::bin::reload_bin_hash_cache();

    // Open the fresh env
    if lmdb.prime(&hash_dir_str).is_some() {
        tracing::info!("LMDB hash database force rebuilt from {}", hash_dir_str);
        Ok(())
    } else {
        Err("Failed to open hash database after force rebuild".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_status_serialization() {
        let status = HashStatus {
            loaded_count: 100,
            last_updated: Some("2024-01-01T00:00:00Z".to_string()),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("loaded_count"));
        assert!(json.contains("100"));
        assert!(json.contains("last_updated"));
    }

    #[test]
    fn test_download_stats_serialization() {
        let stats = DownloadStats {
            downloaded: 5,
            skipped: 2,
            errors: 1,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("downloaded"));
        assert!(json.contains("5"));
        assert!(json.contains("skipped"));
        assert!(json.contains("2"));
        assert!(json.contains("errors"));
        assert!(json.contains("1"));
    }
}
