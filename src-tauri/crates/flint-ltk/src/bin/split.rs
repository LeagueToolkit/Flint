//! BIN split — extract a subset of objects from a parent BIN into a sibling
//! "linked" BIN file, leaving an entry in the parent's dependency list so the
//! engine merges them back at load time.
//!
//! ## Wire format reference (Quartz parity)
//!
//! Quartz's `bin:combineLinkedBins` action does the inverse of this — it
//! reads `parent.entries` + `parent.links`, walks each linked path in
//! `links`, appends the linked file's `entries` into the parent, deletes the
//! linked file from disk, and prunes the merged path from `links`. See
//! `Quartz/src/main/ipc/channels/binTools.js`.
//!
//! Split is the inverse of that pipeline. The wire format we emit needs to
//! be byte-compatible: a sibling `.bin` with the moved entries in
//! `bin.objects`, an empty `dependencies`, and the parent's `dependencies`
//! gets the new file's project-relative path appended.
//!
//! ## VFX classification
//!
//! `classify_vfx_objects` returns the path hashes whose class hash matches
//! a known VFX class. The list is the conservative initial set; growing it
//! is safe (more entries get split out), shrinking is safe too (fewer get
//! split — the BIN still works because skipped entries stay in the parent).
//!
//! BIN class hashes are 32-bit FNV1a of the lowercase class name string.
//! See `event_mapper.rs::fnv1_hash` for FNV-1 vs FNV-1a; we use 1a here.

use crate::error::{Error, Result};
use ltk_meta::{Bin, BinObject};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Result of a successful split operation.
#[derive(Debug, Clone)]
pub struct SplitResult {
    /// Number of objects moved out of the parent into the new file.
    pub moved: usize,
    /// Project-relative path appended to parent's `dependencies` (forward
    /// slashes, lowercase — engine convention).
    pub link_added: String,
}

/// Compute the FNV-1a 32-bit hash of a string. Used to derive BIN class
/// hashes from their textual class names. Always lowercases the input,
/// matching the convention `ltk_ritobin` and Riot's tooling use.
fn fnv1a_lower(s: &str) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for b in s.to_lowercase().bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

/// Class names that we treat as "VFX" by default for the right-click split
/// action. Conservative — anything ambiguous is left in the parent BIN.
pub const VFX_CLASS_NAMES: &[&str] = &[
    "VfxSystemDefinitionData",
    "VfxParticleEmitterDefinitionData",
    "VfxRendererDefinitionData",
    "VfxComplexEmitterDefinitionData",
    "TrailDef",
    "BeamDef",
    "ParticleParameterDef",
    "ParticleSystemDef",
];

/// Returns the path hashes of every object in `bin` whose class hash matches
/// one of [`VFX_CLASS_NAMES`].
pub fn classify_vfx_objects(bin: &Bin) -> Vec<u32> {
    let vfx_hashes: HashSet<u32> = VFX_CLASS_NAMES.iter().map(|n| fnv1a_lower(n)).collect();
    bin.objects
        .iter()
        .filter_map(|(path_hash, obj)| {
            if vfx_hashes.contains(&obj.class_hash) {
                Some(*path_hash)
            } else {
                None
            }
        })
        .collect()
}

/// Group objects by class hash for the modal preview. Returns a Vec of
/// `(class_hash, path_hashes)` so the frontend can show "VfxSystemDefinitionData
/// (×42)" with checkboxes.
pub fn group_by_class(bin: &Bin) -> Vec<(u32, Vec<u32>)> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (path_hash, obj) in bin.objects.iter() {
        map.entry(obj.class_hash).or_default().push(*path_hash);
    }
    map.into_iter().collect()
}

/// Split a subset of objects out of `parent_bin_path` into a new sibling
/// file at `parent_dir / output_filename`. Updates the parent's
/// `dependencies` to reference the new file (project-relative form).
///
/// `project_root` is the WAD-folder root used as the link path's base — for
/// a project laid out as `content/base/<champion>.wad.client/data/...`, the
/// engine expects link paths like `data/SomeFile.bin`, so we strip
/// `<project_root>` from the absolute path of the new file.
///
/// On success returns the move count and the link string we appended.
pub fn split_bin(
    parent_bin_path: &Path,
    project_root: &Path,
    output_filename: &str,
    move_hashes: &HashSet<u32>,
) -> Result<SplitResult> {
    if move_hashes.is_empty() {
        return Err(Error::InvalidInput(
            "split_bin: no objects selected to move".to_string(),
        ));
    }

    // 1. Read parent.
    let parent_data = std::fs::read(parent_bin_path)
        .map_err(|e| Error::io_with_path(e, parent_bin_path))?;
    let mut parent = crate::bin::read_bin(&parent_data)
        .map_err(|e| Error::InvalidInput(format!("Failed to parse parent BIN: {}", e)))?;

    // 2. Pull moved objects out of parent.
    let mut moved_objects: Vec<BinObject> = Vec::with_capacity(move_hashes.len());
    parent.objects.retain(|hash, obj| {
        if move_hashes.contains(hash) {
            moved_objects.push(obj.clone());
            false
        } else {
            true
        }
    });

    if moved_objects.is_empty() {
        return Err(Error::InvalidInput(
            "split_bin: none of the requested hashes existed in the parent BIN".to_string(),
        ));
    }

    let moved_count = moved_objects.len();

    // 3. Build the new sibling BIN (objects only, empty dependencies).
    let new_bin = Bin::builder().objects(moved_objects).build();

    // 4. Decide the on-disk location of the new file (sibling of parent).
    let parent_dir = parent_bin_path
        .parent()
        .ok_or_else(|| Error::InvalidInput("Parent BIN has no parent directory".to_string()))?;
    let new_full_path: PathBuf = parent_dir.join(output_filename);

    if new_full_path.exists() {
        return Err(Error::InvalidInput(format!(
            "Output file already exists: {}",
            new_full_path.display()
        )));
    }

    // 5. Compute the engine-relative link string. Engine reads
    //    `bin.dependencies` as paths relative to the WAD root (the directory
    //    containing `data/`, `assets/`, etc).
    let link_rel = new_full_path
        .strip_prefix(project_root)
        .map_err(|_| {
            Error::InvalidInput(format!(
                "Output path {} is not inside project root {}",
                new_full_path.display(),
                project_root.display()
            ))
        })?
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();

    // 6. Write the new BIN to disk first — only mutate the parent if this
    //    succeeds, so a failure leaves the project in its original state.
    let new_bytes = crate::bin::write_bin(&new_bin)
        .map_err(|e| Error::InvalidInput(format!("Failed to serialize new BIN: {}", e)))?;
    std::fs::write(&new_full_path, &new_bytes)
        .map_err(|e| Error::io_with_path(e, &new_full_path))?;

    // 7. Append link to parent.dependencies (skip duplicates).
    if !parent.dependencies.iter().any(|d| d.eq_ignore_ascii_case(&link_rel)) {
        parent.dependencies.push(link_rel.clone());
    }

    // 8. Write parent back.
    let parent_bytes = crate::bin::write_bin(&parent)
        .map_err(|e| Error::InvalidInput(format!("Failed to serialize parent BIN: {}", e)))?;
    std::fs::write(parent_bin_path, &parent_bytes)
        .map_err(|e| Error::io_with_path(e, parent_bin_path))?;

    Ok(SplitResult {
        moved: moved_count,
        link_added: link_rel,
    })
}
