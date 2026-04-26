//! Tauri commands for the "split BIN" right-click action.
//!
//! Two endpoints:
//! - `analyze_bin_for_split` — read a BIN and return its objects grouped by
//!   class, plus the default VFX selection. Drives the modal preview.
//! - `split_bin_entries` — perform the split (writes new sibling BIN, updates
//!   parent dependencies, removes moved objects from parent).

use flint_ltk::bin::{classify_vfx_objects, group_by_class, read_bin, split_bin};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// One class group surfaced to the modal: a class hash, an optional
/// resolved class name (looked up via the BIN hash table when available),
/// the per-object path hashes in this group, and whether the class is in
/// the default VFX set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinSplitClassGroup {
    pub class_hash: String,
    pub class_name: Option<String>,
    pub path_hashes: Vec<String>,
    pub is_vfx_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinSplitAnalysis {
    /// Total objects in the parent BIN.
    pub total_objects: usize,
    /// Class groups for the modal table. Sorted by class hash so the order
    /// is deterministic between calls.
    pub groups: Vec<BinSplitClassGroup>,
}

/// Resolve the WAD-folder root for a given BIN path. Convention is
/// `<project>/content/base/<champion>.wad.client/data/...`. Walk parents
/// until we find a directory whose name ends in `.wad.client`; if none
/// exists fall back to the immediate parent (best-effort).
fn find_wad_root(bin_path: &Path) -> PathBuf {
    let mut cur = bin_path.parent();
    while let Some(p) = cur {
        if p.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_lowercase().ends_with(".wad.client"))
            .unwrap_or(false)
        {
            return p.to_path_buf();
        }
        cur = p.parent();
    }
    bin_path.parent().unwrap_or(Path::new(".")).to_path_buf()
}

/// Read a BIN and return its class breakdown for the split modal.
#[tauri::command]
pub async fn analyze_bin_for_split(bin_path: String) -> Result<BinSplitAnalysis, String> {
    let path = PathBuf::from(&bin_path);
    let data = std::fs::read(&path).map_err(|e| format!("Failed to read BIN: {}", e))?;
    let bin = read_bin(&data).map_err(|e| format!("Failed to parse BIN: {}", e))?;

    let total = bin.objects.len();
    let vfx_set: HashSet<u32> = classify_vfx_objects(&bin).into_iter().collect();

    let class_cache = flint_ltk::bin::get_cached_bin_hashes();
    let cache = class_cache.read();

    let groups: Vec<BinSplitClassGroup> = group_by_class(&bin)
        .into_iter()
        .map(|(class_hash, hashes)| {
            // Resolve class name via BIN hash table (HashMapProvider). The
            // provider exposes `get_type` for class-name lookups in v0.4.
            let class_name = lookup_bin_hash_name(&cache, class_hash);
            let is_vfx_default = hashes.iter().any(|h| vfx_set.contains(h));
            BinSplitClassGroup {
                class_hash: format!("{:08x}", class_hash),
                class_name,
                path_hashes: hashes.iter().map(|h| format!("{:08x}", h)).collect(),
                is_vfx_default,
            }
        })
        .collect();

    Ok(BinSplitAnalysis {
        total_objects: total,
        groups,
    })
}

/// Look up a BIN class hash in the cached HashMapProvider. Returns the
/// resolved name when present; falls back to `None` so the frontend renders
/// the raw hex.
fn lookup_bin_hash_name(
    provider: &flint_ltk::bin::ltk_bridge::HashMapProvider,
    hash: u32,
) -> Option<String> {
    use flint_ltk::bin::ltk_bridge::HashProvider;
    // Class hashes resolve through the `types` table on the provider.
    provider.lookup_type(hash).map(|s| s.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinSplitResult {
    pub moved: usize,
    pub link_added: String,
}

/// Move the listed objects out of `bin_path` into a new sibling BIN at
/// `output_filename` (just the filename — written next to the parent). The
/// parent's dependency list is updated to reference the new file.
#[tauri::command]
pub async fn split_bin_entries(
    bin_path: String,
    output_filename: String,
    path_hashes: Vec<String>,
) -> Result<BinSplitResult, String> {
    if output_filename.contains('/') || output_filename.contains('\\') {
        return Err("output_filename must be a bare filename, no slashes".to_string());
    }
    let parsed: Result<HashSet<u32>, _> = path_hashes
        .iter()
        .map(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16))
        .collect();
    let move_hashes = parsed.map_err(|e| format!("Invalid hex hash: {}", e))?;

    let parent = PathBuf::from(&bin_path);
    let project_root = find_wad_root(&parent);

    let result = tokio::task::spawn_blocking(move || {
        split_bin(&parent, &project_root, &output_filename, &move_hashes)
    })
    .await
    .map_err(|e| format!("Task panicked: {}", e))?
    .map_err(|e| e.to_string())?;

    Ok(BinSplitResult {
        moved: result.moved,
        link_added: result.link_added,
    })
}
