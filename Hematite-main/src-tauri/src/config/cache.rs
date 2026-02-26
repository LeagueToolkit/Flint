//! Local cache for fix configuration with TTL-based expiration.
//!
//! Caches the fetched configuration to disk to reduce network requests
//! and provide offline support.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use super::schema::FixConfig;

/// Cache time-to-live in seconds (1 hour).
const CACHE_TTL_SECONDS: u64 = 3600;

/// Cache file name.
const CACHE_FILENAME: &str = "config_cache.json";

/// Get the cache directory path.
/// Uses the system app data directory or falls back to current directory.
fn get_cache_dir() -> Result<PathBuf> {
    // Try to get standard app data directory
    #[cfg(target_os = "windows")]
    {
        if let Some(app_data) = std::env::var_os("APPDATA") {
            let path = PathBuf::from(app_data).join("hematite");
            return Ok(path);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let path = PathBuf::from(home).join("Library/Application Support/hematite");
            return Ok(path);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let path = PathBuf::from(home).join(".config/hematite");
            return Ok(path);
        }
    }

    // Fallback to current directory
    Ok(PathBuf::from(".hematite"))
}

/// Get the full path to the default config cache file.
fn get_config_cache_path() -> Result<PathBuf> {
    let cache_dir = get_cache_dir()?;
    Ok(cache_dir.join(CACHE_FILENAME))
}

/// Check if the cache file exists and is within TTL.
pub fn is_cache_valid() -> Result<bool> {
    let cache_path = get_config_cache_path()?;

    if !cache_path.exists() {
        return Ok(false);
    }

    let metadata = fs::metadata(&cache_path)
        .context("Failed to read cache file metadata")?;

    let modified = metadata
        .modified()
        .context("Failed to get cache modification time")?;

    let age = SystemTime::now()
        .duration_since(modified)
        .map(|d| d.as_secs())
        .unwrap_or(u64::MAX);

    Ok(age < CACHE_TTL_SECONDS)
}

/// Read configuration from cache file.
pub fn read_cache() -> Result<FixConfig> {
    let cache_path = get_config_cache_path()?;
    
    let json = fs::read_to_string(&cache_path)
        .with_context(|| format!("Failed to read cache file: {:?}", cache_path))?;
    
    let config: FixConfig = serde_json::from_str(&json)
        .context("Failed to parse cached config JSON")?;
    
    Ok(config)
}

/// Write configuration to cache file.
pub fn write_cache(config: &FixConfig) -> Result<()> {
    let cache_path = get_config_cache_path()?;

    // Create parent directory if needed
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create cache directory: {:?}", parent))?;
    }

    let json = serde_json::to_string_pretty(config)
        .context("Failed to serialize config to JSON")?;

    fs::write(&cache_path, json)
        .with_context(|| format!("Failed to write cache file: {:?}", cache_path))?;

    log::debug!("Wrote config cache to {:?}", cache_path);
    Ok(())
}

/// Clear the cache file.
pub fn clear_cache() -> Result<()> {
    let cache_path = get_cache_path(CACHE_FILENAME)?;
    
    if cache_path.exists() {
        fs::remove_file(&cache_path)
            .with_context(|| format!("Failed to delete cache file: {:?}", cache_path))?;
        log::info!("Cleared config cache");
    }
    
    Ok(())
}

// =============================================================================
// Generic cache functions for any serializable type
// =============================================================================

/// Get a custom cache file path.
pub fn get_cache_path(filename: &str) -> Result<PathBuf> {
    let cache_dir = get_cache_dir()?;
    Ok(cache_dir.join(filename))
}

/// Read a cached file if it exists and is within TTL.
pub fn read_cached_file<T: serde::de::DeserializeOwned>(
    path: &Path,
    ttl_secs: u64,
) -> Result<T> {
    if !path.exists() {
        anyhow::bail!("Cache file does not exist");
    }

    let metadata = fs::metadata(path)
        .context("Failed to read cache file metadata")?;

    let modified = metadata
        .modified()
        .context("Failed to get cache modification time")?;

    let age = SystemTime::now()
        .duration_since(modified)
        .map(|d| d.as_secs())
        .unwrap_or(u64::MAX);

    if age >= ttl_secs {
        anyhow::bail!("Cache expired (age: {}s, ttl: {}s)", age, ttl_secs);
    }

    let json = fs::read_to_string(path)
        .with_context(|| format!("Failed to read cache file: {:?}", path))?;

    let data: T = serde_json::from_str(&json)
        .context("Failed to parse cached JSON")?;

    Ok(data)
}

/// Write data to a cache file.
pub fn write_cache_file<T: serde::Serialize>(path: &Path, data: &T) -> Result<()> {
    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create cache directory: {:?}", parent))?;
    }

    let json = serde_json::to_string_pretty(data)
        .context("Failed to serialize to JSON")?;

    fs::write(path, json)
        .with_context(|| format!("Failed to write cache file: {:?}", path))?;

    log::debug!("Wrote cache file: {:?}", path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cache_path() {
        let path = get_cache_path(CACHE_FILENAME);
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.ends_with(CACHE_FILENAME));
    }
}
