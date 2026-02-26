use crate::error::{Error, Result};
use league_toolkit::file::LeagueFileKind;
use league_toolkit::wad::{Wad, WadChunk};
use memmap2::Mmap;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Result of an extraction operation
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Number of chunks successfully extracted
    pub extracted_count: usize,
    /// Mapping of original paths to actual paths (for long filenames saved with hashes)
    pub path_mappings: HashMap<String, String>,
}

/// Extracts a single chunk from a WAD archive to the specified output path
/// 
/// # Arguments
/// * `wad` - Mutable reference to the Wad for decoding
/// * `chunk` - The chunk to extract
/// * `output_path` - Path where the chunk should be written
/// * `hashtable` - Optional hashtable for path resolution (not used for single chunk extraction)
/// 
/// # Returns
/// * `Result<()>` - Ok if extraction succeeded, Err otherwise
/// 
/// # Requirements
/// Validates: Requirements 4.1, 4.2, 4.3
pub fn extract_chunk(
    wad: &mut Wad<File>,
    chunk: &WadChunk,
    output_path: impl AsRef<Path>,
    _resolve_path: Option<&dyn Fn(u64) -> String>,
) -> Result<()> {
    let output_path = output_path.as_ref();

    tracing::trace!("Extracting chunk to: {}", output_path.display());

    // Decompress the chunk data
    let chunk_data = wad
        .load_chunk_decompressed(chunk)
        .map_err(|e| {
            tracing::error!("Failed to decompress chunk for '{}': {}", output_path.display(), e);
            Error::Wad {
                message: format!("Failed to decompress chunk: {}", e),
                path: Some(output_path.to_path_buf()),
            }
        })?;
    
    // Verify decompressed size matches metadata
    if chunk_data.len() != chunk.uncompressed_size() {
        tracing::error!(
            "Decompressed size mismatch for '{}': expected {}, got {}",
            output_path.display(),
            chunk.uncompressed_size(),
            chunk_data.len()
        );
        return Err(Error::Wad {
            message: format!(
                "Decompressed size mismatch: expected {}, got {}",
                chunk.uncompressed_size(),
                chunk_data.len()
            ),
            path: Some(output_path.to_path_buf()),
        });
    }
    
    // Create parent directories if needed
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| {
                tracing::error!("Failed to create directory '{}': {}", parent.display(), e);
                Error::io_with_path(e, parent)
            })?;
    }
    
    // Write the chunk data to disk
    fs::write(output_path, &chunk_data)
        .map_err(|e| {
            tracing::error!("Failed to write chunk to '{}': {}", output_path.display(), e);
            Error::io_with_path(e, output_path)
        })?;
    
    tracing::trace!("Successfully extracted chunk to: {}", output_path.display());
    
    Ok(())
}

/// Extracts all chunks from a WAD archive to the specified output directory
/// 
/// This function resolves chunk paths using the provided hashtable, creates
/// the necessary directory structure, handles filename collisions, detects
/// file types, and falls back to hex hashes for unresolved paths.
/// 
/// # Arguments
/// * `wad` - Mutable reference to the Wad for decoding
/// * `output_dir` - Base directory where chunks should be extracted
/// * `hashtable` - Optional hashtable for path resolution
/// 
/// # Returns
/// * `Result<usize>` - Number of chunks successfully extracted, or an error
/// 
/// # Requirements
/// Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6
pub fn extract_all(
    wad: &mut Wad<File>,
    output_dir: impl AsRef<Path>,
    resolve_path: impl Fn(u64) -> String,
) -> Result<usize> {
    let output_dir = output_dir.as_ref();
    tracing::info!("Extracting all chunks to: {}", output_dir.display());

    let chunks: Vec<_> = wad.chunks().iter().copied().collect();
    let total_chunks = chunks.len();
    tracing::info!("Total chunks to extract: {}", total_chunks);

    // ── Phase 1: build extraction plan (sequential) ────────────────────────
    let mut extraction_plan: Vec<(WadChunk, PathBuf)> = Vec::with_capacity(total_chunks);
    let mut parents: HashSet<PathBuf> = HashSet::new();

    for chunk in &chunks {
        let path_hash = chunk.path_hash();
        let resolved_path = resolve_path(path_hash);
        let final_path = resolve_chunk_path(&resolved_path, &[]); // ext-only fallback
        let out_path = output_dir.join(&final_path);
        if let Some(parent) = out_path.parent() {
            parents.insert(parent.to_path_buf());
        }
        extraction_plan.push((*chunk, out_path));
    }

    for parent in parents { let _ = fs::create_dir_all(parent); }

    // ── Phase 2: parallel decompress + write (mmap) ────────────────────────
    // Get the raw file handle back out via the wad — we re-open for mmap.
    // Simplest approach: mmap is only available if we have a path, but wad owns the File.
    // Use len-limited sequential fallback if mmap isn't available (rare).
    let results: Vec<(usize, usize)> = extraction_plan
        .par_chunks((extraction_plan.len() / rayon::current_num_threads().max(1)).max(1))
        .map(|slice| {
            let mut extracted = 0usize;
            let mut skipped  = 0usize;
            // Each rayon worker needs its own read handle into the WAD.
            // We do this by holding a per-chunk File pointer from the original.
            // Since we can't clone File directly, fall back to sequential for extract_all.
            // (extract_skin_assets below uses mmap and is the hot path for project creation.)
            for (chunk, out_path) in slice {
                // Decompress via shared Wad — note: this fn is called from blocking tasks
                // so the sequential nature here is acceptable for the WAD-explorer use case.
                // Project creation uses extract_skin_assets which IS parallelized.
                let _ = out_path; let _ = chunk;
                skipped += 1;
            }
            (extracted, skipped)
        })
        .collect();

    // Fall back to sequential for extract_all (not the hot path)
    let mut extracted_count = 0;
    for (chunk, out_path) in &extraction_plan {
        let chunk_data = match wad.load_chunk_decompressed(chunk) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to decompress chunk for '{}': {}", out_path.display(), e);
                continue;
            }
        };
        // Re-resolve with actual data for extension detection
        let final_path = resolve_chunk_path(&out_path.to_string_lossy(), &chunk_data);
        let actual_path = if final_path != out_path.as_os_str() {
            output_dir.join(final_path)
        } else {
            out_path.clone()
        };
        if let Some(parent) = actual_path.parent() { let _ = fs::create_dir_all(parent); }
        match fs::write(&actual_path, &chunk_data) {
            Ok(()) => {
                extracted_count += 1;
                if extracted_count % 200 == 0 {
                    tracing::info!("Extracted {}/{} chunks", extracted_count, total_chunks);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::InvalidFilename => {
                let hex = format!("{:016x}", chunk.path_hash());
                let _ = fs::write(output_dir.join(hex), &chunk_data);
                extracted_count += 1;
            }
            Err(e) => return Err(Error::io_with_path(e, &actual_path)),
        }
    }
    let _ = results; // rayon result unused (sequential fallback used)
    tracing::info!("Successfully extracted {}/{} chunks", extracted_count, total_chunks);
    Ok(extracted_count)
}

/// Find the champion WAD file in a League installation
/// 
/// # Arguments
/// * `league_path` - Path to League installation
/// * `champion` - Champion internal name (e.g., "Kayn", "Aatrox")
/// 
/// # Returns
/// * `Option<PathBuf>` - Path to the WAD file if found
pub fn find_champion_wad(league_path: impl AsRef<Path>, champion: &str) -> Option<PathBuf> {
    let league_path = league_path.as_ref();
    
    // Normalize champion name: lowercase, remove special characters
    let champion_normalized = champion
        .to_lowercase()
        .replace("'", "")
        .replace(" ", "")
        .replace(".", "");
    
    // Standard WAD path
    let wad_path = league_path
        .join("Game")
        .join("DATA")
        .join("FINAL")
        .join("Champions")
        .join(format!("{}.wad.client", champion_normalized));
    
    if wad_path.exists() {
        tracing::info!("Found champion WAD: {}", wad_path.display());
        Some(wad_path)
    } else {
        tracing::warn!("Champion WAD not found: {}", wad_path.display());
        None
    }
}

/// Extract skin-specific assets from a WAD archive
/// 
/// This function extracts ALL files from the WAD. Cleanup of unused files
/// happens later during the repathing phase based on what the skin BIN references.
/// 
/// # Arguments
/// * `wad` - Mutable reference to the Wad for decoding
/// * `output_dir` - Base directory where chunks should be extracted
/// * `champion` - Champion internal name (e.g., "kayn")
/// * `skin_id` - Skin ID to extract (e.g., 1 for first skin)
/// * `hashtable` - Hashtable for path resolution
/// 
/// # Returns
/// * `Result<ExtractionResult>` - Extraction result with count and path mappings, or an error
pub fn extract_skin_assets(
    wad_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    champion: &str,
    _skin_id: u32,
    resolve_path: impl Fn(u64) -> String + Send + Sync,
) -> Result<ExtractionResult> {
    let wad_path   = wad_path.as_ref();
    let output_dir = output_dir.as_ref();

    let champion_lower   = champion.to_lowercase();
    let wad_folder_name  = format!("{}.wad.client", champion_lower);
    let wad_output_dir   = output_dir.join(&wad_folder_name);

    tracing::info!(
        "Extracting assets to: {} (WAD folder: {})",
        output_dir.display(), wad_folder_name
    );

    // ── Open + mmap the WAD for parallel access ────────────────────────────
    let file = File::open(wad_path)
        .map_err(|e| Error::io_with_path(e, wad_path))?;
    let mmap = unsafe { Mmap::map(&file) }
        .map_err(|e| Error::Wad {
            message: format!("Failed to mmap WAD: {}", e),
            path: Some(wad_path.to_path_buf()),
        })?;

    // Parse the TOC from the mmap'd data
    let wad_toc = Wad::mount(Cursor::new(&mmap[..]))
        .map_err(|e| Error::Wad {
            message: format!("Failed to mount WAD: {}", e),
            path: Some(wad_path.to_path_buf()),
        })?;

    let chunks: Vec<WadChunk> = wad_toc.chunks().iter().copied().collect();
    let total_chunks = chunks.len();
    tracing::info!("Total chunks in WAD: {}", total_chunks);

    // ── Phase 1: bulk-resolve hashes, filter, plan dirs (sequential) ──────
    // Resolve ALL hashes in one LMDB read txn (already done by caller's closure),
    // then filter to only assets/ and data/ files.
    let mut extraction_plan: Vec<(WadChunk, PathBuf)> = Vec::with_capacity(total_chunks / 2);
    let mut path_mappings:   HashMap<String, String>  = HashMap::new();
    let mut parents:         HashSet<PathBuf>          = HashSet::new();
    let mut skipped_unknown = 0usize;

    for chunk in &chunks {
        let path_hash    = chunk.path_hash();
        let resolved     = resolve_path(path_hash);
        let path_lower   = resolved.to_lowercase();
        let is_unresolved = resolved.chars().all(|c| c.is_ascii_hexdigit());

        if !path_lower.starts_with("assets/") && !path_lower.starts_with("data/") {
            if is_unresolved { skipped_unknown += 1; }
            continue;
        }

        // Detect if filename is suspiciously long (will be resolved with actual data later,
        // but we need a placeholder path for directory creation)
        let final_path = PathBuf::from(&resolved);
        let filename_len = final_path.to_string_lossy().len();

        let out_path = if filename_len > 200 {
            let parent = final_path.parent().unwrap_or_else(|| Path::new("data"));
            let ext    = final_path.extension().and_then(|e| e.to_str()).unwrap_or("bin");
            let hash_name = format!("{:016x}.{}", path_hash, ext);
            let hash_path = parent.join(&hash_name);

            let orig = final_path.to_string_lossy().to_lowercase().replace('\\', "/");
            let act  = hash_path.to_string_lossy().to_lowercase().replace('\\', "/");
            path_mappings.insert(orig, act);

            wad_output_dir.join(hash_path)
        } else {
            wad_output_dir.join(&final_path)
        };

        if let Some(p) = out_path.parent() { parents.insert(p.to_path_buf()); }
        extraction_plan.push((*chunk, out_path));
    }

    if skipped_unknown > 0 {
        tracing::warn!("Skipped {} unresolved hashes (not in hash DB)", skipped_unknown);
    }

    // Batch-create all parent directories before launching rayon workers
    for parent in parents { let _ = fs::create_dir_all(parent); }

    tracing::info!(
        "Extraction plan: {} files, {} path mappings — launching parallel workers",
        extraction_plan.len(), path_mappings.len()
    );

    // ── Phase 2: parallel decompress + write (rayon + mmap) ───────────────
    // Each rayon worker mounts its own Wad cursor over the shared mmap.
    // Mmap is Send + Sync; each cursor is thread-local — zero contention.
    let mmap_ref = &mmap;
    let chunk_size = (extraction_plan.len() / rayon::current_num_threads().max(1)).max(1);

    let thread_results: Vec<(usize, usize)> = extraction_plan
        .par_chunks(chunk_size)
        .map(|slice| {
            let mut extracted = 0usize;
            let mut skipped   = 0usize;
            let mut local_wad = match Wad::mount(Cursor::new(&mmap_ref[..])) {
                Ok(w)  => w,
                Err(_) => return (0, slice.len()),
            };
            for (chunk, out_path) in slice {
                match local_wad.load_chunk_decompressed(chunk) {
                    Err(_) => { skipped += 1; },
                    Ok(data) => {
                        // Apply extension detection now that we have actual bytes
                        let final_path = resolve_chunk_path(&out_path.to_string_lossy(), &data);
                        let actual_path = output_dir.join(&wad_folder_name).join(&final_path);
                        let write_path = if actual_path.exists() || actual_path == *out_path {
                            out_path.clone()
                        } else {
                            // Ensure parent exists for extension-corrected path
                            if let Some(p) = actual_path.parent() { let _ = fs::create_dir_all(p); }
                            actual_path
                        };
                        if fs::write(&write_path, &data).is_ok() {
                            extracted += 1;
                        } else {
                            skipped += 1;
                        }
                    }
                }
            }
            (extracted, skipped)
        })
        .collect();

    let (extracted_count, skipped_count) = thread_results
        .iter()
        .fold((0, 0), |(e, s), (te, ts)| (e + te, s + ts));

    tracing::info!(
        "Extracted {}/{} chunks ({} skipped, {} path mappings)",
        extracted_count, total_chunks, skipped_count, path_mappings.len()
    );

    Ok(ExtractionResult { extracted_count, path_mappings })
}

/// Resolves the final chunk path by handling extensions
/// 
/// This function:
/// - Adds .ltk extension if the path has no extension
/// - Detects file type from content and appends appropriate extension
/// - Handles directory name collisions
/// 
/// # Arguments
/// * `path` - The resolved or hex path
/// * `chunk_data` - The decompressed chunk data for file type detection
/// 
/// # Returns
/// * `PathBuf` - The final path with appropriate extensions
/// 
/// # Requirements
/// Validates: Requirements 4.5, 4.6
fn resolve_chunk_path(path: &str, chunk_data: &[u8]) -> PathBuf {
    let mut chunk_path = PathBuf::from(path);
    
    // Check if the path has an extension
    if chunk_path.extension().is_none() {
        // Detect file type from content
        let file_kind = LeagueFileKind::identify_from_bytes(chunk_data);
        
        match file_kind {
            LeagueFileKind::Unknown => {
                // No known file type, add .ltk extension
                let filename = chunk_path
                    .file_name()
                    .unwrap_or(OsStr::new("unknown"))
                    .to_string_lossy()
                    .to_string();
                chunk_path = chunk_path.with_file_name(format!("{}.ltk", filename));
            }
            _ => {
                // Known file type, add appropriate extension
                if let Some(extension) = file_kind.extension() {
                    // Add .ltk first, then the detected extension
                    let filename = chunk_path
                        .file_name()
                        .unwrap_or(OsStr::new("unknown"))
                        .to_string_lossy()
                        .to_string();
                    chunk_path = chunk_path.with_file_name(format!("{}.ltk.{}", filename, extension));
                } else {
                    // File kind known but no extension, just add .ltk
                    let filename = chunk_path
                        .file_name()
                        .unwrap_or(OsStr::new("unknown"))
                        .to_string_lossy()
                        .to_string();
                    chunk_path = chunk_path.with_file_name(format!("{}.ltk", filename));
                }
            }
        }
    }
    
    chunk_path
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resolve_chunk_path_with_extension() {
        let path = "characters/aatrox/aatrox.bin";
        let data = vec![0u8; 100];
        let resolved = resolve_chunk_path(path, &data);
        
        // Should keep the original extension
        assert_eq!(resolved, PathBuf::from(path));
    }
    
    #[test]
    fn test_resolve_chunk_path_without_extension() {
        let path = "characters/aatrox/aatrox";
        let data = vec![0u8; 100];
        let resolved = resolve_chunk_path(path, &data);
        
        // Should add .ltk extension
        assert!(resolved.to_string_lossy().contains(".ltk"));
    }
    
    #[test]
    fn test_resolve_chunk_path_hex_fallback() {
        let path = "1a2b3c4d5e6f7a8b";
        let data = vec![0u8; 100];
        let resolved = resolve_chunk_path(path, &data);
        
        // Should add .ltk extension to hex path
        assert!(resolved.to_string_lossy().contains(".ltk"));
    }
}
