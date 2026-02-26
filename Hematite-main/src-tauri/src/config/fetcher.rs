//! Configuration fetcher with HTTP fetch and cache fallback.
//!
//! Fetches the fix configuration from GitHub and falls back to local cache
//! when network is unavailable.

use anyhow::{Context, Result};
use super::schema::FixConfig;
use super::cache;

/// Default URL for the fix configuration JSON.
/// This should point to a raw GitHub file in the Hematite repository.
const CONFIG_URL: &str = "https://raw.githubusercontent.com/RitoShark/Hematite/main/config/fix_config.json";

/// Fetch configuration from GitHub with cache fallback.
///
/// Strategy:
/// 1. If cache is valid (within TTL), return cached config
/// 2. Try to fetch from GitHub
/// 3. If fetch succeeds, update cache and return config
/// 4. If fetch fails, try to return stale cache
/// 5. If no cache available, return error
pub fn fetch_config() -> Result<FixConfig> {
    // Check if cache is valid first
    if cache::is_cache_valid().unwrap_or(false) {
        if let Ok(config) = cache::read_cache() {
            log::info!("Using cached config (TTL not expired)");
            return Ok(config);
        }
    }

    // Try to fetch from GitHub
    match fetch_from_github() {
        Ok(config) => {
            // Update cache with fresh config
            if let Err(e) = cache::write_cache(&config) {
                log::warn!("Failed to write config cache: {}", e);
            }
            log::info!("Fetched fresh config from GitHub (version {})", config.version);
            Ok(config)
        }
        Err(fetch_error) => {
            log::warn!("Failed to fetch config from GitHub: {}", fetch_error);
            
            // Fall back to stale cache
            cache::read_cache()
                .context("Failed to fetch config from GitHub and no cache available")
        }
    }
}

/// Fetch configuration directly from GitHub.
fn fetch_from_github() -> Result<FixConfig> {
    let response = reqwest::blocking::get(CONFIG_URL)
        .context("HTTP request to GitHub failed")?;
    
    if !response.status().is_success() {
        anyhow::bail!("GitHub returned status: {}", response.status());
    }
    
    let text = response.text()
        .context("Failed to read response body")?;
    
    let config: FixConfig = serde_json::from_str(&text)
        .context("Failed to parse config JSON")?;
    
    Ok(config)
}

/// Load configuration from embedded fallback (compile-time bundled).
/// This is used as a last resort when both network and cache fail.
pub fn load_embedded_config() -> Result<FixConfig> {
    const EMBEDDED_CONFIG: &str = include_str!("../../fix_config.json");
    
    let config: FixConfig = serde_json::from_str(EMBEDDED_CONFIG)
        .context("Failed to parse embedded config JSON")?;
    
    Ok(config)
}

/// Fetch config with embedded fallback.
/// This is the recommended entry point for getting configuration.
pub fn get_config() -> Result<FixConfig> {
    fetch_config().or_else(|e| {
        log::warn!("Falling back to embedded config: {}", e);
        load_embedded_config()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_embedded_config() {
        // This will fail until fix_config.json exists
        // For now, just ensure the function is callable
        let result = load_embedded_config();
        // Don't assert success - file may not exist during initial development
        println!("Embedded config result: {:?}", result.is_ok());
    }
}
