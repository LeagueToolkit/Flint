//! Map project discovery & creation.
//!
//! Adapted from MapgeoAddon's `project_manager.py` — the relevant flow is:
//! find `Map<id>.wad.client` (and optionally `Map<id>LEVELS.wad.client`) in
//! `<League>/Game/DATA/FINAL/Maps/Shipping/`, scan it for variants
//! (`data/maps/mapgeometry/<id>/<variant>.{mapgeo,materials.bin}`), and pull
//! the WAD into the project.

use crate::error::{Error, Result};
use crate::project::{create_project as core_create_project, Project};
use crate::wad::extractor::{extract_full_wad_filtered, resolve_wad_paths, ExtractionResult};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A map discovered in a League install.
#[derive(Debug, Clone, Serialize)]
pub struct MapEntry {
    /// Lowercase id, e.g. `"map11"`.
    pub id: String,
    /// Human-readable name, e.g. `"Summoner's Rift"`. Falls back to `"Map 11"`.
    pub display_name: String,
    /// Absolute path to `Map<id>.wad.client`.
    pub wad_path: PathBuf,
    /// Absolute path to `Map<id>LEVELS.wad.client`, if present.
    pub levels_wad_path: Option<PathBuf>,
}

/// A single mapgeo + materials.bin pair living inside a map WAD.
#[derive(Debug, Clone, Serialize)]
pub struct MapVariant {
    /// Variant base name, e.g. `"room"`, `"srx_baseworld"`.
    pub name: String,
    /// Resolved WAD-relative path to the .mapgeo file.
    pub mapgeo: String,
    /// Resolved WAD-relative path to the .materials.bin file.
    pub materials: String,
}

/// True if a WAD filename (already lowercased) carries a locale segment like
/// `.en_us.` / `.de_de.` between the map id and the WAD extension. League ships
/// one of these per supported language carrying audio/text bundles only — they
/// are useless for asset modding.
fn is_locale_wad_name(lower_name: &str) -> bool {
    // Strip the WAD suffix first so we look at just the stem.
    let stem = lower_name
        .strip_suffix(".wad.client")
        .or_else(|| lower_name.strip_suffix(".wad"))
        .unwrap_or(lower_name);
    // After the map id, a `.xx_yy` locale dot-segment is a clear signal.
    // Example stems: "map11.en_us", "map11.de_de", "common.fr_fr"
    if let Some(dot_idx) = stem.rfind('.') {
        let suffix = &stem[dot_idx + 1..];
        if is_locale_token(suffix) { return true; }
    }
    false
}

/// True for tokens like `en_us`, `de_de`, `pt_br` (already lowercase).
fn is_locale_token(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 5
        && bytes[2] == b'_'
        && bytes[0..2].iter().all(|b| b.is_ascii_lowercase())
        && bytes[3..5].iter().all(|b| b.is_ascii_lowercase())
}

/// True if a resolved chunk path lives under a localized subtree (`.../<locale>/...`
/// or filenames containing a locale token). We skip these during map extraction
/// because the modding workflow doesn't need per-language voiceover/text bundles.
pub fn is_locale_path(path_lower: &str) -> bool {
    for segment in path_lower.split(['/', '\\']) {
        if is_locale_token(segment) { return true; }
        // Filenames with a `.<locale>.` infix, e.g. `voices.en_us.bnk`.
        for part in segment.split('.') {
            if is_locale_token(part) { return true; }
        }
    }
    false
}

/// Locate `<league>/Game/DATA/FINAL/Maps/Shipping/`.
pub fn maps_shipping_dir(league_path: &Path) -> Option<PathBuf> {
    let candidate = league_path
        .join("Game")
        .join("DATA")
        .join("FINAL")
        .join("Maps")
        .join("Shipping");
    if candidate.is_dir() { Some(candidate) } else { None }
}

/// Hardcoded display names for the well-known map IDs. Anything not listed
/// falls back to `Map <N>` (or the raw id if it doesn't parse as a number).
fn display_name_for(id: &str) -> String {
    let lower = id.to_lowercase();
    let known: &[(&str, &str)] = &[
        ("map11", "Summoner's Rift"),
        ("map12", "Howling Abyss"),
        ("map21", "Nexus Blitz"),
        ("map22", "TFT"),
        ("map30", "Arena"),
        ("map33", "Swarm"),
        ("common", "Common (shared assets)"),
    ];
    for (k, v) in known {
        if &lower == k { return (*v).to_string(); }
    }
    if let Some(rest) = lower.strip_prefix("map") {
        if rest.chars().all(|c| c.is_ascii_digit()) {
            return format!("Map {}", rest);
        }
    }
    id.to_string()
}

/// Scan the maps shipping dir for `Map*.wad.client` files (and matching
/// `Map*LEVELS.wad.client` siblings). Common.wad.client is included too.
pub fn list_available_maps(league_path: &Path) -> Result<Vec<MapEntry>> {
    let dir = maps_shipping_dir(league_path)
        .ok_or_else(|| Error::InvalidInput(format!(
            "Maps directory not found under '{}'. Expected Game/DATA/FINAL/Maps/Shipping/.",
            league_path.display()
        )))?;

    // First pass: index every .wad / .wad.client by lowercase filename so we
    // can pair main + LEVELS without doing a second readdir.
    let mut by_name: HashMap<String, PathBuf> = HashMap::new();
    for entry in fs::read_dir(&dir).map_err(|e| Error::io_with_path(e, &dir))? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if !path.is_file() { continue; }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
        if !name.ends_with(".wad.client") && !name.ends_with(".wad") { continue; }
        // Skip language-specific WADs (Map11.en_US.wad.client, Map11.de_DE.wad.client, …).
        // Their stem after stripping the WAD suffix matches `<id>.<locale>` where the
        // locale is two lowercase letters + underscore + two uppercase letters in the
        // original filename. Working in lowercase so we just match `_xx`.
        if is_locale_wad_name(&name) { continue; }
        by_name.insert(name, path);
    }

    let mut maps: HashMap<String, MapEntry> = HashMap::new();

    for (name, path) in &by_name {
        // Strip .wad.client / .wad
        let stem = name
            .strip_suffix(".wad.client")
            .or_else(|| name.strip_suffix(".wad"))
            .unwrap_or(name.as_str());

        // Skip LEVELS WADs in the main pass — they're paired below.
        if stem.ends_with("levels") { continue; }

        // Accept "common" or anything that starts with "map".
        let id = if stem == "common" { "common".to_string() }
                 else if stem.starts_with("map") { stem.to_string() }
                 else { continue };

        // Look for a paired LEVELS WAD using the same suffix style.
        let levels_name_client = format!("{}levels.wad.client", id);
        let levels_name_plain = format!("{}levels.wad", id);
        let levels_wad_path = by_name
            .get(&levels_name_client)
            .or_else(|| by_name.get(&levels_name_plain))
            .cloned();

        maps.insert(id.clone(), MapEntry {
            display_name: display_name_for(&id),
            id,
            wad_path: path.clone(),
            levels_wad_path,
        });
    }

    let mut result: Vec<MapEntry> = maps.into_values().collect();
    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
}

/// Mount the map's WAD, resolve every chunk's path hash, and group the
/// resulting paths by variant (the base name shared between a `.mapgeo`
/// and a `.materials.bin` under `data/maps/mapgeometry/<map_id>/`).
pub fn list_map_variants(
    league_path: &Path,
    map_id: &str,
    resolve_paths: impl Fn(&[u64]) -> HashMap<u64, String>,
) -> Result<Vec<MapVariant>> {
    let maps = list_available_maps(league_path)?;
    let entry = maps.into_iter().find(|m| m.id.eq_ignore_ascii_case(map_id))
        .ok_or_else(|| Error::InvalidInput(format!(
            "Map id '{}' not found in League install at '{}'", map_id, league_path.display()
        )))?;

    let resolved = resolve_wad_paths(&entry.wad_path, resolve_paths)?;

    // Group resolved paths under data/maps/mapgeometry/<map_id>/ by variant base.
    let prefix = format!("data/maps/mapgeometry/{}/", map_id.to_lowercase());
    let mut variants: HashMap<String, MapVariant> = HashMap::new();

    for (_hash, path) in resolved {
        let lower = path.to_lowercase();
        if !lower.starts_with(&prefix) { continue; }
        let rel = &lower[prefix.len()..];
        // We only care about the file name; sub-folders (lightmaps/, etc.)
        // get filtered out here.
        if rel.contains('/') { continue; }

        if let Some(base) = rel.strip_suffix(".mapgeo") {
            variants.entry(base.to_string())
                .and_modify(|v| v.mapgeo = path.clone())
                .or_insert_with(|| MapVariant {
                    name: base.to_string(),
                    mapgeo: path.clone(),
                    materials: String::new(),
                });
        } else if let Some(base) = rel.strip_suffix(".materials.bin") {
            variants.entry(base.to_string())
                .and_modify(|v| v.materials = path.clone())
                .or_insert_with(|| MapVariant {
                    name: base.to_string(),
                    mapgeo: String::new(),
                    materials: path.clone(),
                });
        }
    }

    // Only keep variants that have both halves — a lone mapgeo or lone
    // materials.bin is not loadable.
    let mut result: Vec<MapVariant> = variants
        .into_values()
        .filter(|v| !v.mapgeo.is_empty() && !v.materials.is_empty())
        .collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// Output of `create_map_project`.
#[derive(Debug, Clone, Serialize)]
pub struct MapProjectResult {
    pub project: Project,
    pub main_extracted: usize,
    pub levels_extracted: usize,
}

/// Create a new map project: makes the project structure, then extracts
/// `Map<id>.wad.client` (and optionally the LEVELS WAD) into
/// `<project>/content/base/<wad_filename>/`.
///
/// The `champion` slot of the Project struct is set to `"map-<id>"` so the
/// recent-projects list and existing UI code can identify it without a
/// schema change. (See plan §6 for the longer-term cleanup.)
#[allow(clippy::too_many_arguments)]
pub fn create_map_project(
    name: &str,
    map_id: &str,
    include_levels: bool,
    league_path: &Path,
    output_dir: &Path,
    author: Option<String>,
    resolve_paths: impl Fn(&[u64]) -> HashMap<u64, String> + Send + Sync,
    progress: impl Fn(&str, &str),
) -> Result<MapProjectResult> {
    progress("init", "Locating map WADs...");
    let maps = list_available_maps(league_path)?;
    let entry = maps.into_iter().find(|m| m.id.eq_ignore_ascii_case(map_id))
        .ok_or_else(|| Error::InvalidInput(format!(
            "Map id '{}' not found in League install at '{}'", map_id, league_path.display()
        )))?;

    progress("create", "Creating project structure...");
    let champion_tag = format!("map-{}", entry.id);
    let project = core_create_project(name, &champion_tag, 0, league_path, output_dir, author)?;

    let assets_root = project.assets_path();

    let main_wad_name = entry.wad_path
        .file_name().and_then(|n| n.to_str())
        .unwrap_or("map.wad.client")
        .to_string();
    let main_out = assets_root.join(&main_wad_name);

    progress("extract", &format!("Extracting {}...", main_wad_name));
    let main_result: ExtractionResult = match extract_full_wad_filtered(
        &entry.wad_path,
        &main_out,
        &resolve_paths,
        is_locale_path,
    ) {
        Ok(r) => r,
        Err(e) => {
            // Best-effort cleanup so a half-extracted dir doesn't linger.
            let _ = fs::remove_dir_all(&project.project_path);
            return Err(e);
        }
    };

    let mut levels_count = 0usize;
    if include_levels {
        if let Some(levels_path) = entry.levels_wad_path.as_ref() {
            let levels_name = levels_path
                .file_name().and_then(|n| n.to_str())
                .unwrap_or("levels.wad.client")
                .to_string();
            let levels_out = assets_root.join(&levels_name);
            progress("extract", &format!("Extracting {}...", levels_name));
            match extract_full_wad_filtered(levels_path, &levels_out, &resolve_paths, is_locale_path) {
                Ok(r) => { levels_count = r.extracted_count; }
                Err(e) => {
                    tracing::warn!("LEVELS WAD extraction failed (project still usable): {}", e);
                }
            }
        } else {
            tracing::info!("No LEVELS WAD found alongside {}; skipping.", main_wad_name);
        }
    }

    progress("complete", "Map project created.");
    Ok(MapProjectResult {
        project,
        main_extracted: main_result.extracted_count,
        levels_extracted: levels_count,
    })
}
