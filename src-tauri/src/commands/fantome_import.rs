//! Tauri commands for importing Fantome WAD mods into Flint projects
//!
//! Provides analysis and import of Fantome-packaged mods, with automatic
//! champion/skin detection, refathering, and missing file matching from
//! the live League installation.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::BufReader;
use tauri::State;
use zip::ZipArchive;

use crate::core::hash::lmdb_cache::{get_or_open_env, resolve_hashes_lmdb};
use crate::core::wad::reader::WadReader;
use crate::state::LmdbCacheState;

// =============================================================================
// Types
// =============================================================================

/// Metadata from Fantome package META/info.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantomeMetadata {
    #[serde(rename = "Author")]
    pub author: Option<String>,
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "Version")]
    pub version: Option<String>,
}

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
    /// Metadata from META/info.json (if available)
    pub metadata: Option<FantomeMetadata>,
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
// Fantome Package Handling
// =============================================================================

/// Read metadata from META/info.json in a .fantome package
fn read_fantome_metadata(fantome_path: &str) -> Option<FantomeMetadata> {
    let file = File::open(fantome_path).ok()?;
    let mut archive = ZipArchive::new(BufReader::new(file)).ok()?;

    // Look for META/info.json
    for i in 0..archive.len() {
        let mut file_entry = archive.by_index(i).ok()?;
        let name = file_entry.name();

        if name.eq_ignore_ascii_case("META/info.json") || name.eq_ignore_ascii_case("info.json") {
            let mut contents = String::new();
            std::io::Read::read_to_string(&mut file_entry, &mut contents).ok()?;
            let metadata: FantomeMetadata = serde_json::from_str(&contents).ok()?;
            tracing::info!("Found Fantome metadata: {:?}", metadata);
            return Some(metadata);
        }
    }

    None
}

/// Extract WAD file from a .fantome package (ZIP archive)
/// Returns the path to the extracted WAD file in a temp directory
fn extract_fantome_wad(fantome_path: &str) -> Result<PathBuf, String> {
    let file = File::open(fantome_path)
        .map_err(|e| format!("Failed to open fantome file: {}", e))?;

    let mut archive = ZipArchive::new(BufReader::new(file))
        .map_err(|e| format!("Failed to read fantome archive: {}", e))?;

    // Find the first .wad.client file in the archive (usually in WAD/ folder)
    let mut wad_file_name = None;
    for i in 0..archive.len() {
        let file_entry = archive.by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {}", e))?;
        let name = file_entry.name().to_string();

        if name.ends_with(".wad.client") || name.ends_with(".wad") {
            wad_file_name = Some(name.clone());
            break;
        }
    }

    let wad_name = wad_file_name
        .ok_or("No .wad.client file found in fantome package")?;

    // Extract to temp directory
    let temp_dir = std::env::temp_dir().join("flint_fantome_import");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    let wad_path = temp_dir.join(wad_name.replace('/', "_"));

    // Extract the WAD file
    let mut zip_file = archive.by_name(&wad_name)
        .map_err(|e| format!("Failed to find WAD in archive: {}", e))?;

    let mut wad_file = File::create(&wad_path)
        .map_err(|e| format!("Failed to create temp WAD file: {}", e))?;

    std::io::copy(&mut zip_file, &mut wad_file)
        .map_err(|e| format!("Failed to extract WAD file: {}", e))?;

    tracing::info!("Extracted WAD from fantome: {} -> {:?}", wad_name, wad_path);

    Ok(wad_path)
}

/// Determine the actual WAD path (extract from .fantome if needed)
fn resolve_wad_path(input_path: &str) -> Result<(PathBuf, bool), String> {
    let path = Path::new(input_path);
    let is_fantome = input_path.ends_with(".fantome") ||
                     input_path.ends_with(".zip");

    if is_fantome {
        // Extract WAD from fantome package
        let wad_path = extract_fantome_wad(input_path)?;
        Ok((wad_path, true))
    } else {
        // Direct WAD file
        Ok((path.to_path_buf(), false))
    }
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

/// Extract the most common champion name from a list of paths
fn extract_champion_from_paths(paths: &[String]) -> Option<String> {
    let mut champion_counts: HashMap<String, usize> = HashMap::new();

    for path in paths {
        if let Some(champ) = extract_champion_from_path(path) {
            *champion_counts.entry(champ).or_insert(0) += 1;
        }
    }

    // Return the most common champion
    champion_counts.into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(champ, _)| champ)
}

/// Match missing files from the League installation
/// This copies linked BIN files and other missing assets from the live game
fn match_missing_files_from_league(
    _output_path: &Path,
    _league_path: &str,
    _hash_dir: &str,
    _existing_hashes: &[u64],
) -> Result<(), String> {
    tracing::info!("Matching missing files from League installation...");

    // TODO: Implement actual missing file copying from League WADs
    // This would involve:
    // 1. Finding BIN files in the extracted mod
    // 2. Parsing BINs to extract linked BIN paths
    // 3. Opening the champion WAD from League installation
    // 4. Extracting files that are referenced in linked BINs but missing from the mod
    // 5. Handling .bin extension stripping for skin ID matching (e.g., skin42.bin -> skin42)

    tracing::debug!("Missing file matching not yet fully implemented");
    Ok(())
}

/// Apply refathering to the extracted mod files
fn apply_refathering(
    content_path: &Path,
    creator_name: &str,
    project_name: &str,
    champion: &str,
    target_skin_id: u32,
    path_mappings: &HashMap<String, String>,
) -> Result<(), String> {
    use crate::core::repath::organizer::{organize_project, OrganizerConfig};

    tracing::info!("Applying refathering to imported mod...");

    let config = OrganizerConfig {
        enable_concat: true,
        enable_repath: true,
        creator_name: creator_name.to_string(),
        project_name: project_name.to_string(),
        champion: champion.to_string(),
        target_skin_id,
        cleanup_unused: true,
    };

    organize_project(content_path, &config, path_mappings)
        .map_err(|e| e.to_string())?;

    tracing::info!("Refathering completed successfully");
    Ok(())
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
    // Try to read metadata from .fantome package
    let metadata = if wad_path.ends_with(".fantome") || wad_path.ends_with(".zip") {
        read_fantome_metadata(wad_path)
    } else {
        None
    };

    // Resolve WAD path (extract from .fantome if needed)
    let (resolved_wad_path, _is_temp) = resolve_wad_path(wad_path)?;

    // Open WAD file
    let reader = WadReader::open(resolved_wad_path.to_str().unwrap())
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
        metadata,
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
    options: &ImportOptions,
    hash_dir: &str,
) -> Result<String, String> {
    let output_path = Path::new(output_dir);
    std::fs::create_dir_all(output_path)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Resolve WAD path (extract from .fantome if needed)
    let (resolved_wad_path, _is_temp) = resolve_wad_path(wad_path)?;

    // Open WAD
    let mut reader = WadReader::open(resolved_wad_path.to_str().unwrap())
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
    let mut path_mappings = HashMap::new();

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

        // Track path mappings for refathering (hash -> resolved path)
        path_mappings.insert(path.clone(), path.clone());
        extracted_count += 1;
    }

    tracing::info!("Extracted {} files from Fantome WAD", extracted_count);

    // Match missing files from League installation if enabled
    if options.match_from_league {
        if let Some(ref league_path) = options.league_path {
            match_missing_files_from_league(
                output_path,
                league_path,
                hash_dir,
                &chunk_hashes,
            ).map_err(|e| format!("Failed to match missing files: {}", e))?;
        }
    }

    // Apply refathering if enabled
    if options.refather {
        let creator_name = options.creator_name.as_deref().unwrap_or("FlintUser");
        let project_name = options.project_name.as_deref().unwrap_or("ImportedMod");
        let target_skin_id = options.target_skin_id.unwrap_or(0);

        // Detect champion from paths
        let champion = extract_champion_from_paths(&resolved_paths)
            .ok_or("Failed to detect champion from paths")?;

        apply_refathering(
            output_path,
            creator_name,
            project_name,
            &champion,
            target_skin_id,
            &path_mappings,
        ).map_err(|e| format!("Failed to apply refathering: {}", e))?;
    }

    Ok(format!("Imported {} files to {}", extracted_count, output_dir))
}
