//! Tauri commands for importing Fantome WAD mods into Flint projects
//!
//! Provides analysis and import of Fantome-packaged mods, with automatic
//! champion/skin detection, refathering, and missing file matching from
//! the live League installation.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use tauri::State;

use crate::core::hash::lmdb_cache::{get_or_open_env, resolve_hashes_lmdb};
use crate::core::wad::reader::WadReader;
use crate::state::LmdbCacheState;

// =============================================================================
// Types
// =============================================================================

/// Analysis result of a Fantome WAD file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantomeAnalysis {
    /// Detected champion name (if any)
    pub champion: Option<String>,
    /// Detected skin IDs from BIN files
    pub skin_ids: Vec<u32>,
    /// Whether this appears to be a champion mod
    pub is_champion_mod: bool,
    /// Total number of files in the WAD
    pub file_count: usize,
    /// Sample of file paths (first 50)
    pub file_paths: Vec<String>,
}

/// Options for importing a Fantome WAD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    /// Whether to perform refathering (path prefixing)
    pub refather: bool,
    /// Creator name for refathering
    pub creator_name: Option<String>,
    /// Project name for refathering
    pub project_name: Option<String>,
    /// Target skin ID for refathering (if remapping)
    pub target_skin_id: Option<u32>,
    /// Whether to clean up unused files after refathering
    pub cleanup_unused: bool,
    /// Whether to match missing files from live League
    pub match_from_league: bool,
    /// Path to League installation (for file matching)
    pub league_path: Option<String>,
}

// =============================================================================
// Champion/Skin Detection
// =============================================================================

/// Extract champion name from a file path
/// E.g., "characters/aurora/..." -> "aurora"
fn extract_champion_from_path(path: &str) -> Option<String> {
    let lower = path.to_lowercase();

    // Try "characters/{champion}/" pattern
    if let Some(start_idx) = lower.find("characters/") {
        let after_characters = &lower[start_idx + 11..];
        if let Some(end_idx) = after_characters.find('/') {
            let champion = &after_characters[..end_idx];
            if !champion.is_empty() {
                return Some(champion.to_string());
            }
        }
    }

    // Try "assets/characters/{champion}/" pattern
    if let Some(start_idx) = lower.find("assets/characters/") {
        let after_characters = &lower[start_idx + 18..];
        if let Some(end_idx) = after_characters.find('/') {
            let champion = &after_characters[..end_idx];
            if !champion.is_empty() {
                return Some(champion.to_string());
            }
        }
    }

    None
}

/// Extract skin ID from a path
/// E.g., "skins/skin5/..." -> Some(5)
fn extract_skin_id_from_path(path: &str) -> Option<u32> {
    let lower = path.to_lowercase();

    // Look for "skin{N}" pattern
    if let Some(start_idx) = lower.find("skin") {
        let after_skin = &lower[start_idx + 4..];
        // Extract digits immediately after "skin"
        let digits: String = after_skin.chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return digits.parse::<u32>().ok();
        }
    }

    None
}

/// Detect champion and skin IDs from BIN file entries
/// Currently simplified - just extracts from paths for now
/// TODO: Implement proper BIN parsing for more accurate detection
fn _detect_from_bin_file(_bin_data: &[u8]) -> (Option<String>, Option<u32>) {
    // Disabled for now due to API complexity
    // Will implement proper BIN analysis in a future update
    (None, None)
}

// =============================================================================
// Commands
// =============================================================================

/// Analyze a Fantome WAD file to detect champion, skin IDs, and mod type
#[tauri::command]
pub async fn analyze_fantome(
    wad_path: String,
    _lmdb_state: State<'_, LmdbCacheState>,
) -> Result<FantomeAnalysis, String> {
    // Get hash directory before spawning blocking task
    let hash_dir = crate::core::hash::downloader::get_ritoshark_hash_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| format!("Hash directory not found: {}", e))?;

    tokio::task::spawn_blocking(move || {
        analyze_fantome_internal(&wad_path, &hash_dir)
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

fn analyze_fantome_internal(
    wad_path: &str,
    hash_dir: &str,
) -> Result<FantomeAnalysis, String> {
    // Open WAD file
    let reader = WadReader::open(wad_path)
        .map_err(|e| format!("Failed to open WAD file: {}", e))?;

    let chunks = reader.chunks();
    let file_count = chunks.len();

    // Collect all path hashes
    let hashes: Vec<u64> = chunks.iter().map(|chunk| chunk.path_hash).collect();

    // Resolve hashes using LMDB
    let env = get_or_open_env(hash_dir)
        .ok_or("Failed to open LMDB environment")?;

    let resolved_paths = resolve_hashes_lmdb(&hashes, &env);

    // Detect champion from paths
    let mut champion_candidates = HashSet::new();
    for path in &resolved_paths {
        if let Some(champ) = extract_champion_from_path(path) {
            champion_candidates.insert(champ);
        }
    }

    // Detect skin IDs from paths
    let mut skin_id_candidates = HashSet::new();
    for path in &resolved_paths {
        if let Some(skin_id) = extract_skin_id_from_path(path) {
            skin_id_candidates.insert(skin_id);
        }
    }

    // BIN-based detection disabled for now - path-based detection above should be sufficient
    // TODO: Implement proper BIN parsing for more accurate champion/skin detection

    // Determine final champion (pick most common or first)
    let champion = champion_candidates.into_iter().next();

    // Determine if this is a champion mod
    let is_champion_mod = champion.is_some();

    // Get skin IDs as sorted vec
    let mut skin_ids: Vec<u32> = skin_id_candidates.into_iter().collect();
    skin_ids.sort_unstable();

    // Get sample file paths (first 50)
    let file_paths = resolved_paths.into_iter().take(50).collect();

    Ok(FantomeAnalysis {
        champion,
        skin_ids,
        is_champion_mod,
        file_count,
        file_paths,
    })
}

/// Import a Fantome WAD into a Flint project
#[tauri::command]
pub async fn import_fantome_wad(
    wad_path: String,
    output_dir: String,
    options: ImportOptions,
    _lmdb_state: State<'_, LmdbCacheState>,
) -> Result<String, String> {
    // Get hash directory before spawning blocking task
    let hash_dir = crate::core::hash::downloader::get_ritoshark_hash_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| format!("Hash directory not found: {}", e))?;

    tokio::task::spawn_blocking(move || {
        import_fantome_internal(&wad_path, &output_dir, &options, &hash_dir)
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

fn import_fantome_internal(
    wad_path: &str,
    output_dir: &str,
    _options: &ImportOptions,
    hash_dir: &str,
) -> Result<String, String> {
    let output_path = Path::new(output_dir);
    std::fs::create_dir_all(output_path)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Open WAD
    let mut reader = WadReader::open(wad_path)
        .map_err(|e| format!("Failed to open WAD file: {}", e))?;

    // Collect chunk hashes and resolve paths before mutably borrowing reader
    let (chunk_hashes, resolved_paths) = {
        let chunks = reader.chunks();
        let hashes: Vec<u64> = chunks.iter().map(|chunk| chunk.path_hash).collect();

        let env = get_or_open_env(hash_dir)
            .ok_or("Failed to open LMDB environment")?;

        let resolved_paths = resolve_hashes_lmdb(&hashes, &env);

        (hashes, resolved_paths)
    };

    // Extract all chunks by hash
    let mut extracted_count = 0;
    for (hash, path) in chunk_hashes.iter().zip(resolved_paths.iter()) {
        // Get chunk by hash (copy it to end the immutable borrow)
        let chunk = *reader.chunks().get(*hash)
            .ok_or_else(|| format!("Chunk not found for hash: {:x}", hash))?;

        // Decompress chunk
        let data = reader.wad_mut().load_chunk_decompressed(&chunk)
            .map_err(|e| format!("Failed to decompress chunk: {}", e))?;

        // Write to disk
        let file_path = output_path.join(path.trim_start_matches('/'));
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        std::fs::write(&file_path, data)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        extracted_count += 1;
    }

    tracing::info!("Extracted {} files from Fantome WAD", extracted_count);

    // TODO: Implement refathering if options.refather is true
    // TODO: Implement missing file matching if options.match_from_league is true

    Ok(format!("Imported {} files to {}", extracted_count, output_dir))
}
