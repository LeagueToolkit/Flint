//! Analyze Command
//!
//! Tauri command to analyze files for issues.
//! Uses scanner to find files and detector to check against config rules.

use std::collections::HashMap;

use crate::analyzer::detector::{analyze_file, ScanResult};
use crate::analyzer::hash_dict::HashDict;
use crate::analyzer::wad_cache::WadCache;
use crate::config::fetcher::get_config;
use crate::config::schema::FixRule;

/// Analyze a path (file or directory) for issues
///
/// # Arguments
/// * `path` - Path to a .fantome, .zip, .wad.client file or directory
///
/// # Returns
/// * Vector of ScanResult with detected issues for each file
#[tauri::command]
pub async fn analyze_path(path: String) -> Result<Vec<ScanResult>, String> {
    log::info!("Analyzing path: {}", path);

    // Load config
    let config = get_config()
        .map_err(|e| format!("Failed to load config: {:?}", e))?;

    // Load hash dictionary
    let hash_dict = HashDict::load()
        .map_err(|e| format!("Failed to load hash dictionary: {:?}", e))?;

    // Create empty WAD cache for now (TODO: populate from actual WAD)
    let wad_cache = WadCache::new();

    // Get enabled fix rules
    let fix_rules: HashMap<String, FixRule> = config
        .fixes
        .into_iter()
        .filter(|(_, rule)| rule.enabled)
        .collect();

    // For now, try to parse the path as a single BIN file
    // TODO: Implement full scanner for .fantome/.zip/directory traversal
    let result = analyze_file(&path, &wad_cache, &fix_rules, &hash_dict)
        .map_err(|e| format!("Failed to analyze file: {:?}", e))?;

    log::info!(
        "Analysis complete: {} issues found",
        result.detected_issues.len()
    );

    Ok(vec![result])
}

/// Get the current fix configuration
///
/// Returns the list of all enabled fixes for display in the UI
#[tauri::command]
pub async fn get_fix_config() -> Result<HashMap<String, FixRule>, String> {
    let config = get_config()
        .map_err(|e| format!("Failed to load config: {:?}", e))?;

    let enabled_fixes: HashMap<String, FixRule> = config
        .fixes
        .into_iter()
        .filter(|(_, rule)| rule.enabled)
        .collect();

    Ok(enabled_fixes)
}
