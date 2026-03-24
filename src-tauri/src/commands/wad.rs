use flint_ltk::hash::{resolve_hashes_lmdb, resolve_hashes_lmdb_bulk};
use flint_ltk::wad::extractor::{extract_all, extract_chunk};
use flint_ltk::wad::reader::WadReader;
use crate::state::{LmdbCacheState, WadCacheState};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tauri::State;
use walkdir::WalkDir;

/// Information about a WAD archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WadInfo {
    pub path: String,
    pub chunk_count: usize,
}

/// Information about a chunk within a WAD archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub hash: String,
    pub path: Option<String>,
    pub size: u32,
}

/// Result of a WAD extraction operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub extracted_count: usize,
    pub failed_count: usize,
}

/// Opens a WAD file and returns metadata about it
#[tauri::command]
pub async fn read_wad(path: String) -> Result<WadInfo, String> {
    let reader = WadReader::open(&path)?;
    Ok(WadInfo {
        path,
        chunk_count: reader.chunk_count(),
    })
}

/// Returns a list of all chunks in a WAD archive with resolved paths.
///
/// Uses LMDB for O(log N) point lookups — no full hashtable loaded into RAM.
#[tauri::command]
pub async fn get_wad_chunks(
    path: String,
    lmdb: State<'_, LmdbCacheState>,
    wad_cache_state: State<'_, WadCacheState>,
) -> Result<Vec<ChunkInfo>, String> {
    let cache = wad_cache_state.get();

    // WAD metadata cache (avoids re-parsing headers)
    let chunks = if let Some(cached) = cache.get(&path) {
        tracing::debug!("WAD cache hit: {}", path);
        cached
    } else {
        tracing::debug!("WAD cache miss: {}", path);
        let reader = WadReader::open(&path)?;
        let chunks: Vec<_> = reader.chunks().iter().cloned().collect();
        let chunks = Arc::new(chunks);
        let _ = cache.insert(&path, Arc::clone(&chunks));
        chunks
    };

    // Bulk-resolve all hashes in a single LMDB read txn (microseconds)
    let hash_u64s: Vec<u64> = chunks.iter().map(|c| c.path_hash()).collect();
    let resolved: Vec<String> = if let Some(env) = lmdb.get_env(
        // Determine hash dir from the LMDB cache (uses whatever was last primed)
        &flint_ltk::hash::downloader::get_ritoshark_hash_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    ) {
        resolve_hashes_lmdb(&hash_u64s, &env)
    } else {
        hash_u64s.iter().map(|h| format!("{:016x}", h)).collect()
    };

    let chunk_infos = chunks
        .iter()
        .zip(resolved.into_iter())
        .map(|(chunk, resolved_path)| {
            let path_hash = chunk.path_hash();
            // Hex-only 16-char strings are unresolved hashes — treat as None
            let path = if resolved_path.len() == 16
                && resolved_path.bytes().all(|b| b.is_ascii_hexdigit())
            {
                None
            } else {
                Some(resolved_path)
            };
            ChunkInfo {
                hash: format!("{:016x}", path_hash),
                path,
                size: chunk.uncompressed_size() as u32,
            }
        })
        .collect();

    Ok(chunk_infos)
}

/// Result of loading one WAD in a batch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WadChunkBatch {
    pub path: String,
    pub chunks: Vec<ChunkInfo>,
    pub error: Option<String>,
}

/// Loads chunk metadata for multiple WAD files in one call.
///
/// Phase 1: parallel WAD header parsing (I/O-bound, rayon).
/// Phase 2: collect ALL unique hashes across every WAD, deduplicate.
/// Phase 3: single LMDB read txn resolves every unique hash once.
/// Phase 4: O(1) HashMap lookup distributes resolved paths back to each WAD.
///
/// This is dramatically faster than per-WAD resolution because:
/// - One LMDB transaction instead of N (avoids N × txn + db open overhead)
/// - Deduplication saves 30-50% of B-tree lookups (shared assets across WADs)
#[tauri::command]
pub async fn load_all_wad_chunks(
    paths: Vec<String>,
    lmdb: State<'_, LmdbCacheState>,
    wad_cache_state: State<'_, WadCacheState>,
) -> Result<Vec<WadChunkBatch>, String> {
    let cache = wad_cache_state.get();

    // Resolve hash dir once
    let hash_dir = flint_ltk::hash::downloader::get_ritoshark_hash_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let env_opt = lmdb.get_env(&hash_dir);

    // Phase 1: parallel WAD header reads (rayon — I/O-bound)
    let toc_results: Vec<(String, Result<Vec<_>, String>)> = paths
        .par_iter()
        .map(|wad_path| {
            let result: Result<Vec<_>, String> = (|| {
                let chunks = if let Some(cached) = cache.get(wad_path) {
                    cached
                } else {
                    let reader = WadReader::open(wad_path).map_err(|e| e.to_string())?;
                    let chunks: Vec<_> = reader.chunks().iter().cloned().collect();
                    let chunks = Arc::new(chunks);
                    let _ = cache.insert(wad_path, Arc::clone(&chunks));
                    chunks
                };
                Ok((*chunks).clone())
            })();
            (wad_path.clone(), result)
        })
        .collect();

    // Phase 2: separate successes from errors, collect ALL unique hashes
    let mut wad_chunks: Vec<(String, Vec<_>)> = Vec::with_capacity(toc_results.len());
    let mut error_batches: Vec<WadChunkBatch> = Vec::new();
    let mut unique_hashes: HashSet<u64> = HashSet::new();

    for (wad_path, result) in toc_results {
        match result {
            Err(e) => error_batches.push(WadChunkBatch {
                path: wad_path,
                chunks: vec![],
                error: Some(e),
            }),
            Ok(chunks) => {
                for c in &chunks {
                    unique_hashes.insert(c.path_hash());
                }
                wad_chunks.push((wad_path, chunks));
            }
        }
    }

    // Phase 3: single LMDB read txn for ALL unique hashes (deduped)
    let unique_vec: Vec<u64> = unique_hashes.into_iter().collect();
    tracing::info!(
        "Resolving {} unique hashes across {} WADs (single LMDB txn)",
        unique_vec.len(),
        wad_chunks.len()
    );
    let resolved_map: HashMap<u64, String> = if let Some(ref env) = env_opt {
        resolve_hashes_lmdb_bulk(&unique_vec, env)
    } else {
        unique_vec.iter().map(|h| (*h, format!("{:016x}", h))).collect()
    };

    // Phase 4: distribute resolved paths back to each WAD via O(1) HashMap lookup
    let mut batches: Vec<WadChunkBatch> = Vec::with_capacity(error_batches.len() + wad_chunks.len());
    batches.append(&mut error_batches);

    for (wad_path, chunks) in wad_chunks {
        let chunk_infos = chunks
            .iter()
            .map(|chunk| {
                let path_hash = chunk.path_hash();
                let resolved = resolved_map
                    .get(&path_hash)
                    .cloned()
                    .unwrap_or_else(|| format!("{:016x}", path_hash));
                let path = if resolved.len() == 16
                    && resolved.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    None
                } else {
                    Some(resolved)
                };
                ChunkInfo {
                    hash: format!("{:016x}", path_hash),
                    path,
                    size: chunk.uncompressed_size() as u32,
                }
            })
            .collect();

        batches.push(WadChunkBatch {
            path: wad_path,
            chunks: chunk_infos,
            error: None,
        });
    }

    // Log category stats
    let mut category_stats: HashMap<String, (usize, usize)> = HashMap::new();
    for batch in &batches {
        if batch.error.is_some() { continue; }
        let category = Path::new(&batch.path)
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Other")
            .to_string();
        let entry = category_stats.entry(category).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += batch.chunks.len();
    }
    for (cat, (wads, chunks)) in &category_stats {
        tracing::info!("Loaded \"{}\" folder: {} wads, {} chunks", cat, wads, chunks);
    }

    Ok(batches)
}

/// Extracts chunks from a WAD archive to the specified output directory.
///
/// Uses LMDB for path resolution — no full hashtable loaded into memory.
#[tauri::command]
pub async fn extract_wad(
    wad_path: String,
    output_dir: String,
    chunk_hashes: Option<Vec<String>>,
    lmdb: State<'_, LmdbCacheState>,
) -> Result<ExtractionResult, String> {
    let mut reader = WadReader::open(&wad_path)?;

    // Get resolver via LMDB (point lookup, no RAM spike)
    let hash_dir = flint_ltk::hash::downloader::get_ritoshark_hash_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let env_opt = lmdb.get_env(&hash_dir);

    let resolve = |hash: u64| -> String {
        if let Some(ref env) = env_opt {
            let resolved = resolve_hashes_lmdb(&[hash], env);
            resolved.into_iter().next().unwrap_or_else(|| format!("{:016x}", hash))
        } else {
            format!("{:016x}", hash)
        }
    };

    let mut extracted_count = 0;
    let mut failed_count = 0;

    if let Some(hashes) = chunk_hashes {
        let out_dir = std::path::Path::new(&output_dir);
        for hash_str in hashes {
            let path_hash = u64::from_str_radix(&hash_str, 16)
                .map_err(|e| format!("Invalid hash format '{}': {}", hash_str, e))?;

            let chunk_exists = reader.get_chunk(path_hash).is_some();
            if chunk_exists {
                let chunk = reader.get_chunk(path_hash).unwrap();
                let resolved_path = resolve(path_hash);

                // Check if the full output path exceeds Windows MAX_PATH (260).
                // If so, fall back to {hash}.{ext} in the same parent directory.
                let candidate = out_dir.join(&resolved_path);
                let output_path = if candidate.to_string_lossy().len() > 240 {
                    let ext = std::path::Path::new(&resolved_path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("bin");
                    let parent = std::path::Path::new(&resolved_path)
                        .parent()
                        .filter(|p| !p.as_os_str().is_empty());
                    let hash_name = format!("{:016x}.{}", path_hash, ext);
                    match parent {
                        Some(p) => out_dir.join(p).join(&hash_name),
                        None => out_dir.join(&hash_name),
                    }
                } else {
                    candidate
                };

                let chunk_copy = *chunk;
                match extract_chunk(reader.wad_mut(), &chunk_copy, &output_path, None) {
                    Ok(_) => extracted_count += 1,
                    Err(_) => failed_count += 1,
                }
            } else {
                failed_count += 1;
            }
        }
    } else {
        match extract_all(reader.wad_mut(), &output_dir, resolve) {
            Ok(count) => extracted_count = count,
            Err(e) => return Err(e.into()),
        }
    }

    Ok(ExtractionResult { extracted_count, failed_count })
}

/// Invalidate a WAD entry from the metadata cache so the next read re-parses it.
#[tauri::command]
pub async fn invalidate_wad_cache(
    path: String,
    wad_cache_state: State<'_, WadCacheState>,
) -> Result<(), String> {
    wad_cache_state.get().remove(&path);
    tracing::info!("Invalidated WAD cache for: {}", path);
    Ok(())
}

/// Result of extracting a WAD model preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WadModelPreviewResult {
    pub skn_path: String,
    pub temp_dir: String,
}

/// Extract an SKN chunk and its companion files from a WAD to a temp directory
/// for inline 3D preview. Companion files: .skl, .bin, .dds, .tex in the same
/// skin folder. Also auto-generates .ritobin cache for .bin files.
#[tauri::command]
pub async fn extract_wad_model_preview(
    wad_path: String,
    skn_hash: String,
    lmdb: State<'_, LmdbCacheState>,
    wad_cache_state: State<'_, WadCacheState>,
) -> Result<WadModelPreviewResult, String> {
    let target_hash = u64::from_str_radix(&skn_hash, 16)
        .map_err(|e| format!("Invalid hash '{}': {}", skn_hash, e))?;

    let cache = wad_cache_state.get();

    // Get chunks (from cache or fresh parse)
    let chunks = if let Some(cached) = cache.get(&wad_path) {
        cached
    } else {
        let reader = WadReader::open(&wad_path)?;
        let chunks: Vec<_> = reader.chunks().iter().cloned().collect();
        let chunks = Arc::new(chunks);
        let _ = cache.insert(&wad_path, Arc::clone(&chunks));
        chunks
    };

    // Resolve all hashes via LMDB
    let hash_u64s: Vec<u64> = chunks.iter().map(|c| c.path_hash()).collect();
    let hash_dir = flint_ltk::hash::downloader::get_ritoshark_hash_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let resolved_map: HashMap<u64, String> = if let Some(ref env) = lmdb.get_env(&hash_dir) {
        resolve_hashes_lmdb_bulk(&hash_u64s, env)
    } else {
        hash_u64s.iter().map(|h| (*h, format!("{:016x}", h))).collect()
    };

    // Find the SKN chunk's resolved path
    let skn_resolved = resolved_map.get(&target_hash)
        .ok_or_else(|| format!("SKN chunk {:016x} not found in WAD", target_hash))?;

    // Determine the skin folder prefix (e.g., "data/characters/ahri/skins/skin01/")
    let skn_normalized = skn_resolved.replace('\\', "/");
    let skn_folder = skn_normalized.rsplit_once('/')
        .map(|(folder, _)| format!("{}/", folder))
        .unwrap_or_default();

    // Companion extensions to extract alongside the SKN
    let companion_exts = [".skn", ".skl", ".bin", ".dds", ".tex"];

    // Find all companion chunks in the same folder
    let mut to_extract: Vec<(u64, String)> = Vec::new();
    for chunk in chunks.iter() {
        let h = chunk.path_hash();
        if let Some(resolved) = resolved_map.get(&h) {
            let norm = resolved.replace('\\', "/");
            if !skn_folder.is_empty() && norm.starts_with(&skn_folder) {
                let lower = norm.to_lowercase();
                if companion_exts.iter().any(|ext| lower.ends_with(ext)) {
                    to_extract.push((h, norm));
                }
            } else if h == target_hash {
                // Always include the target SKN even if folder matching fails
                to_extract.push((h, norm));
            }
        }
    }

    if to_extract.is_empty() {
        return Err("No extractable files found for SKN preview".to_string());
    }

    // Create temp directory
    let uuid = uuid::Uuid::new_v4();
    let temp_dir = std::env::temp_dir().join("flint-wad-preview").join(uuid.to_string());
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    // Extract companion files
    let mut reader = WadReader::open(&wad_path)?;
    let mut skn_path = String::new();

    for (hash, rel_path) in &to_extract {
        if let Some(chunk) = reader.get_chunk(*hash) {
            let output_path = temp_dir.join(rel_path);
            if let Some(parent) = output_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let chunk_copy = *chunk;
            if let Err(e) = flint_ltk::wad::extractor::extract_chunk(
                reader.wad_mut(), &chunk_copy, &output_path, None,
            ) {
                tracing::warn!("Failed to extract {}: {}", rel_path, e);
                continue;
            }
            if *hash == target_hash {
                skn_path = output_path.to_string_lossy().to_string();
            }
        }
    }

    if skn_path.is_empty() {
        // Cleanup on failure
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err("Failed to extract SKN chunk".to_string());
    }

    // Auto-generate .ritobin cache for all extracted .bin files
    for (_, rel_path) in &to_extract {
        if rel_path.to_lowercase().ends_with(".bin") {
            let bin_path = temp_dir.join(rel_path);
            let ritobin_path = std::path::PathBuf::from(format!("{}.ritobin", bin_path.display()));
            if let Err(e) = crate::commands::mesh::create_ritobin_cache(&bin_path, &ritobin_path) {
                tracing::warn!("Failed to create ritobin cache for {}: {}", rel_path, e);
            }
        }
    }

    tracing::info!(
        "Extracted {} files for SKN preview to {}",
        to_extract.len(),
        temp_dir.display()
    );

    Ok(WadModelPreviewResult {
        skn_path,
        temp_dir: temp_dir.to_string_lossy().to_string(),
    })
}

/// Clean up a temporary WAD model preview directory.
/// Validates the path starts with the expected temp prefix.
#[tauri::command]
pub async fn cleanup_wad_model_preview(temp_dir: String) -> Result<(), String> {
    let path = std::path::Path::new(&temp_dir);
    let expected_prefix = std::env::temp_dir().join("flint-wad-preview");

    if !path.starts_with(&expected_prefix) {
        return Err("Invalid temp dir path — must be inside flint-wad-preview".to_string());
    }

    if path.exists() {
        std::fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to cleanup temp dir: {}", e))?;
        tracing::debug!("Cleaned up WAD preview temp: {}", temp_dir);
    }

    Ok(())
}

/// Info about a WAD file found on disk (for game WAD scanning)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWadInfo {
    pub path: String,
    pub name: String,
    pub category: String,
}

/// Read decompressed chunk data from a WAD archive into memory — no disk write.
#[tauri::command]
pub async fn read_wad_chunk_data(
    wad_path: String,
    hash: String,
) -> Result<Vec<u8>, String> {
    let path_hash = u64::from_str_radix(&hash, 16)
        .map_err(|e| format!("Invalid hash '{}': {}", hash, e))?;

    let mut reader = WadReader::open(&wad_path)?;
    let chunk = *reader
        .get_chunk(path_hash)
        .ok_or_else(|| format!("Chunk {:016x} not found in WAD", path_hash))?;

    reader
        .wad_mut()
        .load_chunk_decompressed(&chunk)
        .map(|b| b.into())
        .map_err(|e| format!("Failed to decompress chunk {:016x}: {}", path_hash, e))
}

/// Scan a game installation directory for all WAD archive files.
#[tauri::command]
pub async fn scan_game_wads(game_path: String) -> Result<Vec<GameWadInfo>, String> {
    let root = std::path::Path::new(&game_path).join("DATA").join("FINAL");

    if !root.exists() {
        return Err(format!(
            "WAD directory not found: {} — make sure this is the League Game/ folder",
            root.display()
        ));
    }

    let mut wads: Vec<GameWadInfo> = WalkDir::new(&root)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let path = e.path();
            let name = path.file_name()?.to_str()?;
            if !name.ends_with(".wad.client") && !name.ends_with(".wad") {
                return None;
            }
            let category = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("Other")
                .to_string();
            Some(GameWadInfo {
                path: path.to_string_lossy().to_string(),
                name: name.to_string(),
                category,
            })
        })
        .collect();

    wads.sort_unstable_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));

    Ok(wads)
}
