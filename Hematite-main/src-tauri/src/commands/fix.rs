//! Fix Command
//!
//! Tauri command to apply selected fixes to files.
//! Uses fixer module to modify BIN files.

use std::collections::HashMap;

use crate::analyzer::bin_parser::{parse_bin_file, write_bin_file};
use crate::analyzer::hash_dict::HashDict;
use crate::analyzer::wad_cache::WadCache;
use crate::config::fetcher::get_config;
use crate::config::schema::FixRule;
use crate::fixer::{apply_transforms, FixContext, FixResult};

use serde::Deserialize;

/// Request to apply fixes
#[derive(Debug, Clone, Deserialize)]
pub struct ApplyFixesRequest {
    /// Path to the file to fix
    pub file_path: String,
    /// List of fix IDs to apply
    pub fix_ids: Vec<String>,
    /// Path to write the fixed file (optional, defaults to overwriting original)
    pub output_path: Option<String>,
}

/// Apply selected fixes to a file
///
/// # Arguments
/// * `request` - Contains file path, selected fix IDs, and optional output path
///
/// # Returns
/// * FixResult with success/failure details
#[tauri::command]
pub async fn apply_fixes(request: ApplyFixesRequest) -> Result<FixResult, String> {
    log::info!(
        "Applying {} fixes to: {}",
        request.fix_ids.len(),
        request.file_path
    );

    // Load config
    let config = get_config()
        .map_err(|e| format!("Failed to load config: {:?}", e))?;

    // Load hash dictionary
    let hash_dict = HashDict::load()
        .map_err(|e| format!("Failed to load hash dictionary: {:?}", e))?;

    // Create empty WAD cache for now (TODO: populate from actual WAD)
    let wad_cache = WadCache::new();

    // Parse the BIN file
    let bin_tree = parse_bin_file(&request.file_path)
        .map_err(|e| format!("Failed to parse BIN file: {:?}", e))?;

    // Get all fix rules
    let fix_rules: HashMap<String, FixRule> = config.fixes;

    // Create fix context
    let mut context = FixContext::new(bin_tree, &wad_cache, &hash_dict);

    // Apply transforms
    let result = apply_transforms(
        &request.file_path,
        &mut context,
        &fix_rules,
        &request.fix_ids,
    );

    log::info!(
        "Fix complete: {} applied, {} failed",
        result.fixes_applied.len(),
        result.fixes_failed.len()
    );

    // Write modified BIN tree back to file if any fixes were applied
    if !result.fixes_applied.is_empty() {
        let output_path = request.output_path.as_ref().unwrap_or(&request.file_path);
        write_bin_file(&context.bin_tree, output_path)
            .map_err(|e| format!("Failed to write fixed BIN file: {:?}", e))?;
        log::info!("Wrote fixed BIN to: {}", output_path);
    }

    Ok(result)
}

