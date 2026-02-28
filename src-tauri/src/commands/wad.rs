use crate::core::hash::{resolve_hashes_lmdb, resolve_hashes_lmdb_bulk};
use crate::core::wad::extractor::{extract_all, extract_chunk};
use crate::core::wad::reader::WadReader;
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
        &crate::core::hash::downloader::get_ritoshark_hash_dir()
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
    let hash_dir = crate::core::hash::downloader::get_ritoshark_hash_dir()
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
    let hash_dir = crate::core::hash::downloader::get_ritoshark_hash_dir()
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
        for hash_str in hashes {
            let path_hash = u64::from_str_radix(&hash_str, 16)
                .map_err(|e| format!("Invalid hash format '{}': {}", hash_str, e))?;

            let chunk_exists = reader.get_chunk(path_hash).is_some();
            if chunk_exists {
                let chunk = reader.get_chunk(path_hash).unwrap();
                let resolved_path = resolve(path_hash);
                let output_path = std::path::Path::new(&output_dir).join(&resolved_path);
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
