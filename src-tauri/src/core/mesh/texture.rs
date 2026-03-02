//! Texture resolution from BIN files
//! 
//! Parses skin0.bin to extract texture mappings for SKN materials.
//! Supports SkinMeshDataProperties with texture and materialOverride fields.

// Imports from original file
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::bin::ltk_bridge;
use serde::Serialize;
use regex::Regex;

/// Extended material properties including UV transformations
#[derive(Debug, Clone, Serialize, Default)]
pub struct MaterialProperties {
    /// Diffuse texture path
    pub texture_path: String,
    
    /// UV scale (tiling) - [scaleU, scaleV]
    /// From paramValue "UVScaleAndOffset" vec4[0,1]
    pub uv_scale: Option<[f32; 2]>,
    
    /// UV offset (shift) - [offsetU, offsetV]  
    /// From paramValue "UVScaleAndOffset" vec4[2,3]
    pub uv_offset: Option<[f32; 2]>,
    
    /// Flipbook texture atlas size - [columns, rows]
    /// From paramValue "FlipbookSize" vec4[0,1]
    pub flipbook_size: Option<[u32; 2]>,
    
    /// Current flipbook frame index
    /// From paramValue "FrameIndex" vec4[0]
    pub flipbook_frame: Option<f32>,
}

/// Texture mapping extracted from BIN file with UV transform parameters
///
/// DEPRECATED: Use bin_texture_discovery::discover_material_textures instead
#[deprecated(note = "Use bin_texture_discovery::discover_material_textures instead")]
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Default)]
pub struct TextureMapping {
    /// Default texture path for meshes without specific override
    pub default_texture: Option<String>,
    
    /// Per-material properties including texture and UV transforms
    /// Key = submesh/material name, Value = material properties
    pub material_properties: HashMap<String, MaterialProperties>,
    
    /// Static material references that couldn't be resolved (for debugging)
    pub static_materials: Vec<String>,
    
    /// Raw ritobin content for late lookups
    #[serde(skip)]
    pub ritobin_content: String,
}

/// Find skin BIN (or skinN.bin) relative to an SKN file
///
/// Uses smart root detection: walks up from the mesh path to find a directory
/// containing `data/`, then searches `data/characters/{champion}/skins/`.
pub fn find_skin_bin(skn_path: &Path) -> Option<PathBuf> {
    tracing::debug!("Looking for skin BIN relative to: {}", skn_path.display());

    let champion_name = extract_champion_name(skn_path);
    let skin_folder = extract_skin_folder_from_path(skn_path);

    tracing::debug!("Extracted: champion={:?}, skin_folder={:?}", champion_name, skin_folder);

    let champion_name = champion_name?;

    // Find project root by walking up to find a directory with `data/` child
    if let Some(root) = find_project_root_from_path(skn_path) {
        tracing::debug!("Project root: {}", root.display());

        let skins_dir = root
            .join("data")
            .join("characters")
            .join(&champion_name)
            .join("skins");

        if let Some(found) = search_skins_dir(&skins_dir, skin_folder.as_deref()) {
            return Some(found);
        }
    }

    tracing::warn!("skin BIN not found for: {}", skn_path.display());
    None
}

/// Extract champion name from a file path.
/// Tries multiple strategies: characters/ pattern, WAD folder name, filename.
fn extract_champion_name(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy().to_lowercase();
    let components: Vec<&str> = path_str.split(&['/', '\\'][..]).collect();

    // Strategy 1: "characters/{champion}" pattern
    for (i, part) in components.iter().enumerate() {
        if *part == "characters" && i + 1 < components.len() {
            return Some(components[i + 1].to_string());
        }
    }

    // Strategy 2: Extract from WAD folder name (e.g., "aurora.wad.client" → "aurora")
    for part in &components {
        if let Some(name) = part.strip_suffix(".wad.client")
            .or_else(|| part.strip_suffix(".wad"))
        {
            if !name.is_empty() {
                tracing::debug!("Extracted champion from WAD folder: {}", name);
                return Some(name.to_string());
            }
        }
    }

    // Strategy 3: Parent of skins/base folder
    for (i, part) in components.iter().enumerate() {
        if (*part == "base" || *part == "skins") && i > 0 {
            let potential = components[i - 1];
            if !potential.is_empty()
                && potential != "assets"
                && potential != "data"
                && !potential.contains("wad")
            {
                return Some(potential.to_string());
            }
        }
    }

    // Strategy 4: Filename (skip generic names) — last resort
    if let Some(file_name) = path.file_stem() {
        let name = file_name.to_string_lossy().to_lowercase();
        // Only use if it looks like a simple champion name (no dots/underscores suggesting compound names)
        if !name.starts_with("skin") && name != "base" && !name.is_empty() && !name.contains('.') {
            tracing::debug!("Using filename as champion name: {}", name);
            return Some(name);
        }
    }

    None
}

/// Extract skin folder name from a path (e.g., "skin0", "skin20", "base" → "skin0")
fn extract_skin_folder_from_path(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy().to_lowercase();
    let components: Vec<&str> = path_str.split(&['/', '\\'][..]).collect();

    // Strategy 1: Look for skins/{skinN} pattern
    for (i, part) in components.iter().enumerate() {
        if *part == "skins" && i + 1 < components.len() {
            let next = components[i + 1];
            if next.starts_with("skin") {
                return Some(next.to_string());
            } else if next == "base" {
                return Some("skin0".to_string());
            }
        }
    }

    // Strategy 2: Look for any directory component matching "skinN" pattern
    // (handles WAD-extracted paths like .../skin11/champion.skn)
    for part in components.iter().rev() {
        if part.starts_with("skin") && part.len() > 4 && part[4..].chars().all(|c| c.is_ascii_digit()) {
            return Some(part.to_string());
        }
    }

    None
}

/// Find the project root by walking up the directory tree until we find
/// a directory that contains a `data/` subdirectory.
///
/// Skips `.wad.client` / `.wad` folders — those are extracted WAD content,
/// not the actual project root.
fn find_project_root_from_path(file_path: &Path) -> Option<PathBuf> {
    let mut current = file_path.parent()?;
    let mut best: Option<PathBuf> = None;

    for _ in 0..15 {
        let data_dir = current.join("data");
        if data_dir.exists() && data_dir.is_dir() {
            let dir_name = current.file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            // Skip WAD folders — keep searching upward for the real project root
            if dir_name.ends_with(".wad.client") || dir_name.ends_with(".wad") {
                tracing::debug!("Skipping WAD folder as project root: {}", current.display());
                if best.is_none() {
                    best = Some(current.to_path_buf());
                }
            } else {
                tracing::debug!("Found project root (has data/): {}", current.display());
                return Some(current.to_path_buf());
            }
        }
        current = match current.parent() {
            Some(p) => p,
            None => break,
        };
    }

    // Fall back to WAD folder if no better root found
    if let Some(ref fallback) = best {
        tracing::debug!("Using WAD folder as fallback project root: {}", fallback.display());
    }
    best
}

/// Search a skins directory for BIN files, trying multiple strategies.
fn search_skins_dir(skins_dir: &Path, skin_folder: Option<&str>) -> Option<PathBuf> {
    if !skins_dir.exists() {
        return None;
    }

    // Strategy 1: *Concat.bin (pre-merged, highest priority)
    if let Ok(entries) = std::fs::read_dir(skins_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains("concat") && name.ends_with(".bin") {
                tracing::debug!("Found concat BIN: {}", entry.path().display());
                return Some(entry.path());
            }
        }
    }

    // Strategy 2: Specific skin folder
    if let Some(skin) = skin_folder {
        let nested = skins_dir.join(skin).join(format!("{}.bin", skin));
        if nested.exists() {
            tracing::debug!("Found nested skin BIN: {}", nested.display());
            return Some(nested);
        }
        let flat = skins_dir.join(format!("{}.bin", skin));
        if flat.exists() {
            tracing::debug!("Found flat skin BIN: {}", flat.display());
            return Some(flat);
        }
    }

    // Strategy 3: Fallback to skin0.bin
    let skin0 = skins_dir.join("skin0.bin");
    if skin0.exists() {
        tracing::debug!("Found fallback skin0.bin");
        return Some(skin0);
    }

    // Strategy 4: Any .bin file
    if let Ok(entries) = std::fs::read_dir(skins_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("bin") {
                tracing::debug!("Found fallback BIN: {}", path.display());
                return Some(path);
            }
        }
    }

    None
}

/// Extract texture mappings from a skin0.bin file
///
/// Parses the BIN file by converting it to Ritobin text format and using regex
/// to find skinMeshProperties and material overrides.
///
/// DEPRECATED: Use bin_texture_discovery::discover_material_textures instead
#[deprecated(note = "Use bin_texture_discovery::discover_material_textures instead")]
#[allow(dead_code)]
pub fn extract_texture_mapping(bin_path: &Path) -> anyhow::Result<TextureMapping> {
    let data = fs::read(bin_path)?;
    let tree = ltk_bridge::read_bin(&data)
        .map_err(|e| anyhow::anyhow!("Failed to parse BIN: {}", e))?;
    
    // Convert to text using cached hashes for better readability/matching
    let textual_content = ltk_bridge::tree_to_text_cached(&tree)
        .map_err(|e| anyhow::anyhow!("Failed to convert BIN to text: {}", e))?;
        
    extract_texture_mapping_from_text(&textual_content)
}

/// Parse Ritobin text to extract texture mappings
/// 
/// Uses regex to find:
/// 1. valid skinMeshProperties block (with default texture)
/// 2. materialOverride blocks (with submesh -> texture/material mappings)
/// 3. StaticMaterialDef blocks (to resolve material links)
#[allow(clippy::regex_creation_in_loops)]
pub fn extract_texture_mapping_from_text(content: &str) -> anyhow::Result<TextureMapping> {
    let mut mapping = TextureMapping {
        ritobin_content: content.to_string(),
        ..Default::default()
    };
    
    // 1. Find skinMeshProperties block header
    // We look for: skinMeshProperties: embed = SkinMeshDataProperties { ... }
    let skin_mesh_header_regex = Regex::new(r"skinMeshProperties:\s*embed\s*=\s*(?:SkinMeshDataProperties\s*)?").unwrap();
    
    if let Some(header_match) = skin_mesh_header_regex.find(content) {
        // Use brace counting to extract the full properties block
        if let Some(properties_block) = extract_braced_block(content, header_match.end() - 1) {
            tracing::debug!("Found skinMeshProperties block ({} chars)", properties_block.len());
            
            // Extract default texture
            // texture: string = "ASSETS/..."
            let texture_regex = Regex::new(r#"texture:\s*string\s*=\s*"([^"]+)""#).unwrap();
            if let Some(tex_captures) = texture_regex.captures(&properties_block) {
                let tex_path = tex_captures.get(1).unwrap().as_str().to_string();
                if !tex_path.is_empty() {
                    tracing::debug!("Default texture: {}", tex_path);
                    mapping.default_texture = Some(tex_path);
                }
            }
            
            // Find materialOverride list header
            let override_header_regex = Regex::new(r"materialOverride:\s*list\[embed\]\s*=\s*").unwrap();
            
            if let Some(override_match) = override_header_regex.find(&properties_block) {
                // Use brace counting to extract the full list
                if let Some(list_content) = extract_braced_block(&properties_block, override_match.end() - 1) {
                    tracing::debug!("Found materialOverride list ({} chars)", list_content.len());
                    
                    // Split by "SkinMeshDataProperties_MaterialOverride" 
                    let parts: Vec<&str> = list_content.split("SkinMeshDataProperties_MaterialOverride").collect();
                    
                    for (idx, part) in parts.iter().enumerate() {
                        // Check if this part has a submesh definition
                        let submesh_regex = Regex::new(r#"submesh:\s*string\s*=\s*"([^"]+)""#).unwrap();
                        if let Some(sub_captures) = submesh_regex.captures(part) {
                            let submesh_name = sub_captures.get(1).unwrap().as_str().to_string();
                            tracing::debug!("Found materialOverride[{}]: submesh='{}'", idx, submesh_name);
                            
                            // Check for direct texture
                            let tex_regex = Regex::new(r#"texture:\s*string\s*=\s*"([^"]+)""#).unwrap();
                            if let Some(tex_match) = tex_regex.captures(part) {
                                let tex_path = tex_match.get(1).unwrap().as_str().to_string();
                                tracing::debug!("  -> Direct texture: {}", tex_path);
                                // Direct textures have no UV transforms
                                let props = MaterialProperties {
                                    texture_path: tex_path,
                                    ..Default::default()
                                };
                                mapping.material_properties.insert(submesh_name.clone(), props);
                                continue;
                            }
                            
                            // Check for material link (string) - CASE INSENSITIVE
                            // Material: link = "Characters/..." or material: link = "..."
                            let mat_link_regex = Regex::new(r#"(?i)material:\s*link\s*=\s*"([^"]+)""#).unwrap();
                            if let Some(mat_match) = mat_link_regex.captures(part) {
                                let mat_path = mat_match.get(1).unwrap().as_str().to_string();
                                tracing::debug!("  -> Material link (string): {}", mat_path);
                                
                                // Resolve material link - now returns MaterialProperties with UV transforms
                                if let Some(props) = resolve_material_texture(content, &mat_path) {
                                    tracing::debug!("  -> RESOLVED to: {}", props.texture_path);
                                    mapping.material_properties.insert(submesh_name.clone(), props);
                                } else {
                                    tracing::warn!("  -> FAILED to resolve material link!");
                                    mapping.static_materials.push(format!("Link: {} -> {}", submesh_name, mat_path));
                                }
                                continue;
                            }

                            // Check for material link (hash)
                            // material: link = 0x12345678
                            let mat_hash_regex = Regex::new(r#"material:\s*link\s*=\s*(0x[0-9a-fA-F]+)"#).unwrap();
                            if let Some(hash_match) = mat_hash_regex.captures(part) {
                                let mat_hash = hash_match.get(1).unwrap().as_str();
                                tracing::debug!("  -> Material link (hash): {}", mat_hash);
                                
                                // Try to resolve hex hash to MaterialProperties
                                if let Some(props) = resolve_material_texture_by_hash(content, mat_hash) {
                                    tracing::debug!("  -> RESOLVED to: {}", props.texture_path);
                                    mapping.material_properties.insert(submesh_name.clone(), props);
                                } else {
                                    tracing::warn!("  -> FAILED to resolve material hash!");
                                    mapping.static_materials.push(format!("Hash: {} -> {}", submesh_name, mat_hash));
                                }
                                continue;
                            }
                            
                            tracing::warn!("  -> No texture or material link found for submesh");
                        }
                    }
                }
            }
        }
    }
    
    tracing::info!("Final material_properties count: {}", mapping.material_properties.len());
    Ok(mapping)
}

/// Look up MaterialProperties for a material by searching for StaticMaterialDef with matching name
/// 
/// This is used for materials that aren't in the materialOverride list but have their 
/// own StaticMaterialDef block in the BIN file.
pub fn lookup_material_texture_by_name(ritobin_content: &str, material_name: &str) -> Option<MaterialProperties> {
    tracing::debug!("Looking up StaticMaterialDef for material: {}", material_name);
    
    // Helper to extract MaterialProperties from a block
    let extract_props = |block: &str| -> Option<MaterialProperties> {
        if let Some(texture_path) = extract_diffuse_texture_from_block(block) {
            let (uv_scale, uv_offset, flipbook_size, flipbook_frame) = extract_param_values(block);
            Some(MaterialProperties {
                texture_path,
                uv_scale,
                uv_offset,
                flipbook_size,
                flipbook_frame,
            })
        } else {
            None
        }
    };
    
    // Strategy 1: Exact path match
    // Pattern: "ExactMaterialName" = StaticMaterialDef
    let exact_pattern = format!(r#""{}"\s*=\s*StaticMaterialDef\s*"#, regex::escape(material_name));
    if let Ok(regex) = Regex::new(&exact_pattern) {
        if let Some(mat) = regex.find(ritobin_content) {
            tracing::debug!("Found exact StaticMaterialDef match at position {}", mat.start());
            if let Some(block) = extract_braced_block(ritobin_content, mat.end() - 1) {
                if let Some(props) = extract_props(&block) {
                    tracing::debug!("Resolved '{}' to texture: {}", material_name, props.texture_path);
                    return Some(props);
                }
            }
        }
    }
    
    // Strategy 2: Path ends with material name
    // Pattern: ".../{material_name}" = StaticMaterialDef
    let ends_with_pattern = format!(r#""[^"]*/{}"[^=]*=\s*StaticMaterialDef\s*"#, regex::escape(material_name));
    if let Ok(regex) = Regex::new(&ends_with_pattern) {
        if let Some(mat) = regex.find(ritobin_content) {
            tracing::debug!("Found path-ending StaticMaterialDef match at position {}", mat.start());
            if let Some(block) = extract_braced_block(ritobin_content, mat.end() - 1) {
                if let Some(props) = extract_props(&block) {
                    tracing::debug!("Resolved '{}' to texture: {}", material_name, props.texture_path);
                    return Some(props);
                }
            }
        }
    }
    
    // Strategy 3: Contains material name anywhere in path (partial match)
    // Pattern: "...{material_name}..." = StaticMaterialDef
    let contains_pattern = format!(r#""[^"]*{}[^"]*"\s*=\s*StaticMaterialDef\s*"#, regex::escape(material_name));
    if let Ok(regex) = Regex::new(&contains_pattern) {
        if let Some(mat) = regex.find(ritobin_content) {
            tracing::debug!("Found partial StaticMaterialDef match at position {}", mat.start());
            if let Some(block) = extract_braced_block(ritobin_content, mat.end() - 1) {
                if let Some(props) = extract_props(&block) {
                    tracing::debug!("Resolved '{}' to texture: {}", material_name, props.texture_path);
                    return Some(props);
                }
            }
        }
    }
    
    // Strategy 4: Case-insensitive search
    let lower_name = material_name.to_lowercase();
    let case_insensitive_pattern = format!(r#"(?i)"[^"]*{}[^"]*"\s*=\s*StaticMaterialDef\s*"#, regex::escape(&lower_name));
    if let Ok(regex) = Regex::new(&case_insensitive_pattern) {
        if let Some(mat) = regex.find(ritobin_content) {
            tracing::debug!("Found case-insensitive StaticMaterialDef match at position {}", mat.start());
            if let Some(block) = extract_braced_block(ritobin_content, mat.end() - 1) {
                if let Some(props) = extract_props(&block) {
                    tracing::debug!("Resolved '{}' to texture: {}", material_name, props.texture_path);
                    return Some(props);
                }
            }
        }
    }
    tracing::debug!("No StaticMaterialDef found for material: {}", material_name);
    None
}

/// Extract UV transform parameters from a StaticMaterialDef block's paramValues
/// 
/// Parses:
/// - UVScaleAndOffset: vec4 = { scaleU, scaleV, offsetU, offsetV }
/// - FlipbookSize: vec4 = { cols, rows, 0, 0 }
/// - FrameIndex: vec4 = { index, 0, 0, 0 }
#[allow(clippy::type_complexity, clippy::regex_creation_in_loops)]
fn extract_param_values(material_block: &str) -> (Option<[f32; 2]>, Option<[f32; 2]>, Option<[u32; 2]>, Option<f32>) {
    let mut uv_scale: Option<[f32; 2]> = None;
    let mut uv_offset: Option<[f32; 2]> = None;
    let mut flipbook_size: Option<[u32; 2]> = None;
    let mut flipbook_frame: Option<f32> = None;
    
    // Find paramValues block - can be list[embed] or list2[embed]
    let param_regex = match Regex::new(r"(?i)paramValues:\s*list2?\[embed\]\s*=\s*") {
        Ok(r) => r,
        Err(_) => return (None, None, None, None),
    };
    
    let param_match = match param_regex.find(material_block) {
        Some(m) => m,
        None => return (None, None, None, None),
    };
    
    // Extract paramValues block using brace counting
    if let Some(param_block) = extract_braced_block(material_block, param_match.end() - 1) {
        // Split by StaticMaterialShaderParamDef
        let params: Vec<&str> = param_block.split("StaticMaterialShaderParamDef").collect();
        
        for param in params {
            // Extract parameter name
            let name_regex = match Regex::new(r#"name:\s*string\s*=\s*"([^"]+)""#) {
                Ok(r) => r,
                Err(_) => continue,
            };
            
            if let Some(name_match) = name_regex.captures(param) {
                let param_name = match name_match.get(1) {
                    Some(m) => m.as_str(),
                    None => continue,
                };
                
                // Extract vec4 value: value: vec4 = { x, y, z, w }
                let value_regex = match Regex::new(r"value:\s*vec4\s*=\s*\{\s*([^}]+)\s*\}") {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                
                if let Some(value_match) = value_regex.captures(param) {
                    let values_str = match value_match.get(1) {
                        Some(m) => m.as_str(),
                        None => continue,
                    };
                    
                    let values: Vec<f32> = values_str
                        .split(',')
                        .filter_map(|s| s.trim().parse::<f32>().ok())
                        .collect();
                    
                    match param_name {
                        "UVScaleAndOffset" if values.len() >= 4 => {
                            uv_scale = Some([values[0], values[1]]);
                            uv_offset = Some([values[2], values[3]]);
                            tracing::debug!("Found UVScaleAndOffset: scale=[{}, {}], offset=[{}, {}]", 
                                values[0], values[1], values[2], values[3]);
                        }
                        "FlipbookSize" if values.len() >= 2 => {
                            flipbook_size = Some([values[0] as u32, values[1] as u32]);
                            tracing::debug!("Found FlipbookSize: [{}, {}]", values[0], values[1]);
                        }
                        "FrameIndex" if !values.is_empty() => {
                            flipbook_frame = Some(values[0]);
                            tracing::debug!("Found FrameIndex: {}", values[0]);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    
    (uv_scale, uv_offset, flipbook_size, flipbook_frame)
}

/// Resolve a material path to MaterialProperties by searching the BIN content
/// 
/// Returns texture path AND UV transform parameters
fn resolve_material_texture(content: &str, material_path: &str) -> Option<MaterialProperties> {
    tracing::debug!("Resolving material link: '{}'", material_path);
    
    // Escape special characters in material path for regex
    let escaped_path = regex::escape(material_path);
    
    // Find the definition header: "MaterialPath" = StaticMaterialDef {
    let def_pattern = format!(r#""{}"\s*=\s*StaticMaterialDef\s*"#, escaped_path);
    tracing::debug!("Searching with pattern: {}", def_pattern);
    
    let def_regex = match Regex::new(&def_pattern) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Invalid regex pattern: {}", e);
            return None;
        }
    };
    
    if let Some(def_match) = def_regex.find(content) {
        tracing::debug!("Found StaticMaterialDef for '{}' at position {}", material_path, def_match.start());
        
        // Use brace counting to extract the full block
        if let Some(block) = extract_braced_block(content, def_match.end() - 1) {
            tracing::debug!("Extracted block ({} chars)", block.len());
            
            // Extract texture path
            if let Some(texture_path) = extract_diffuse_texture_from_block(&block) {
                tracing::debug!("Found texture: {}", texture_path);
                
                // Extract UV transform parameters
                let (uv_scale, uv_offset, flipbook_size, flipbook_frame) = extract_param_values(&block);
                
                let props = MaterialProperties {
                    texture_path,
                    uv_scale,
                    uv_offset,
                    flipbook_size,
                    flipbook_frame,
                };
                
                tracing::debug!("SUCCESS: '{}' resolved with transforms", material_path);
                return Some(props);
            } else {
                tracing::warn!("FAILED: Could not find diffuse texture in StaticMaterialDef block for '{}'", material_path);
                let preview: String = block.chars().take(500).collect();
                tracing::debug!("Block preview: {}", preview);
            }
        } else {
            tracing::warn!("Failed to extract braced block after StaticMaterialDef header");
        }
    } else {
        tracing::warn!("Could not find StaticMaterialDef for material path: '{}'", material_path);
    }
    
    None
}

/// Resolve a hex hash material reference to MaterialProperties
fn resolve_material_texture_by_hash(content: &str, hash: &str) -> Option<MaterialProperties> {
    tracing::debug!("Resolving material link (hash): {}", hash);
    
    // Find the definition header: 0xABCDEF = StaticMaterialDef {
    // Hash matching is case-insensitive
    let pattern = format!(r"(?i){}\s*=\s*StaticMaterialDef\s*", regex::escape(hash));
    let regex = Regex::new(&pattern).ok()?;
    
    if let Some(mat) = regex.find(content) {
        tracing::debug!("Found StaticMaterialDef for hash {} at position {}", hash, mat.start());
        
        // Use brace counting to extract the full block
        if let Some(block) = extract_braced_block(content, mat.end() - 1) {
            if let Some(texture_path) = extract_diffuse_texture_from_block(&block) {
                let (uv_scale, uv_offset, flipbook_size, flipbook_frame) = extract_param_values(&block);
                return Some(MaterialProperties {
                    texture_path,
                    uv_scale,
                    uv_offset,
                    flipbook_size,
                    flipbook_frame,
                });
            }
        }
    }
    
    tracing::debug!("Failed to resolve material hash: {}", hash);
    None
}

/// Extract content between matched braces starting at the given position
/// The position should point to (or before) the opening '{'
fn extract_braced_block(content: &str, start_after: usize) -> Option<String> {
    let bytes = content.as_bytes();
    let mut brace_count = 0;
    let mut block_start = None;
    
    // Search from start_after position
    for (i, &ch) in bytes[start_after..].iter().enumerate() {
        let actual_idx = start_after + i;
        if ch == b'{' {
            if block_start.is_none() {
                block_start = Some(actual_idx + 1); // Start after opening brace
            }
            brace_count += 1;
        } else if ch == b'}' {
            brace_count -= 1;
            if brace_count == 0 {
                if let Some(start) = block_start {
                    let block = content[start..actual_idx].to_string();
                    tracing::trace!("Extracted block ({} chars)", block.len());
                    return Some(block);
                }
            }
        }
    }
    
    tracing::warn!("Failed to find matching closing brace");
    None
}

/// Extract Diffuse/Color texture path from a StaticMaterialDef block
/// 
/// Looks for common diffuse texture names in samplerValues, with fallback to first sampler
#[allow(clippy::regex_creation_in_loops)]
fn extract_diffuse_texture_from_block(block: &str) -> Option<String> {
    // Find samplerValues list inside the block
    // Can be list[embed] or list2[embed]
    let sampler_regex = Regex::new(r"(?i)samplerValues:\s*list2?\[embed\]\s*=\s*").ok()?;
    let sampler_match = sampler_regex.find(block)?;
    
    tracing::trace!("Found samplerValues at position {}", sampler_match.start());
    
    // Extract the samplerValues block using brace counting
    if let Some(sampler_block) = extract_braced_block(block, sampler_match.end() - 1) {
        // Split by StaticMaterialShaderSamplerDef to process each sampler
        let samplers: Vec<&str> = sampler_block.split("StaticMaterialShaderSamplerDef").collect();
        
        // First pass: look for known diffuse texture names
        let diffuse_names = [
            "diffuse_color",
            "diffuse_texture", 
            "diffuse",
            "base_color",
            "basecolor",
            "albedo",
            "color",
            "_cm",  // Common suffix for color maps
        ];
        
        for sampler in &samplers {
            let lower_sampler = sampler.to_lowercase();
            
            // Check if this sampler has a known diffuse-like name
            let is_diffuse = diffuse_names.iter().any(|name| lower_sampler.contains(name));
            
            if is_diffuse {
                // Extract texturePath
                let path_regex = Regex::new(r#"texturePath:\s*string\s*=\s*"([^"]+)""#).ok()?;
                if let Some(path_match) = path_regex.captures(sampler) {
                    let texture_path = path_match.get(1).unwrap().as_str().to_string();
                    tracing::debug!("Found diffuse texture: {}", texture_path);
                    return Some(texture_path);
                }
            }
        }
        
        // Fallback: Use the first sampler with a texturePath (often the diffuse)
        tracing::debug!("No named diffuse found, trying first sampler as fallback");
        for sampler in &samplers {
            let path_regex = Regex::new(r#"texturePath:\s*string\s*=\s*"([^"]+)""#).ok()?;
            if let Some(path_match) = path_regex.captures(sampler) {
                let texture_path = path_match.get(1).unwrap().as_str().to_string();
                // Skip obvious non-diffuse textures
                let lower_path = texture_path.to_lowercase();
                if !lower_path.contains("normal") && 
                   !lower_path.contains("_nm") && 
                   !lower_path.contains("mask") &&
                   !lower_path.contains("noise") &&
                   !lower_path.contains("ramp") {
                    tracing::debug!("Using first valid texture as fallback: {}", texture_path);
                    return Some(texture_path);
                }
            }
        }
    }
    
    tracing::debug!("No diffuse texture found in block");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_texture_mapping_complex() {
        let ritobin_content = r#"
        skinMeshProperties: embed = SkinMeshDataProperties {
            skeleton: string = "ASSETS/Characters/Test/Skins/Skin0/Test.skl"
            simpleSkin: string = "ASSETS/Characters/Test/Skins/Skin0/Test.skn"
            texture: string = "ASSETS/Characters/Test/Skins/Skin0/Test_Base_TX_CM.tex"
            materialOverride: list[embed] = {
                SkinMeshDataProperties_MaterialOverride {
                    texture: string = "ASSETS/Characters/Test/Skins/Skin0/Direct_Override.tex"
                    submesh: string = "DirectMesh"
                }
                SkinMeshDataProperties_MaterialOverride {
                    material: link = "Characters/Test/Skins/Skin0/Materials/LinkedMat"
                    submesh: string = "LinkedMesh"
                }
            }
        }
        
        "Characters/Test/Skins/Skin0/Materials/LinkedMat" = StaticMaterialDef {
            name: string = "Characters/Test/Skins/Skin0/Materials/LinkedMat"
            samplerValues: list2[embed] = {
                StaticMaterialShaderSamplerDef {
                    textureName: string = "Diffuse_Color"
                    texturePath: string = "ASSETS/Characters/Test/Skins/Skin0/Resolved_Linked.tex"
                    addressU: u32 = 1
                    addressV: u32 = 1
                }
                StaticMaterialShaderSamplerDef {
                    textureName: string = "Normal_Map"
                    texturePath: string = "ASSETS/Characters/Test/Skins/Skin0/Resolved_Normal.tex"
                }
            }
        }
        "#;
        
        let mapping = extract_texture_mapping_from_text(ritobin_content).unwrap();
        
        // Check default texture
        assert_eq!(mapping.default_texture, Some("ASSETS/Characters/Test/Skins/Skin0/Test_Base_TX_CM.tex".to_string()));
        
        // Check overrides - now using material_properties
        assert_eq!(mapping.material_properties.get("DirectMesh").map(|p| &p.texture_path), Some(&"ASSETS/Characters/Test/Skins/Skin0/Direct_Override.tex".to_string()));
        assert_eq!(mapping.material_properties.get("LinkedMesh").map(|p| &p.texture_path), Some(&"ASSETS/Characters/Test/Skins/Skin0/Resolved_Linked.tex".to_string()));
    }

    #[test]
    fn test_extract_texture_mapping_simple() {
        let ritobin_content = r#"
        skinMeshProperties: embed = SkinMeshDataProperties {
            texture: string = "ASSETS/Simple.tex"
        }
        "#;
        
        let mapping = extract_texture_mapping_from_text(ritobin_content).unwrap();
        assert_eq!(mapping.default_texture, Some("ASSETS/Simple.tex".to_string()));
        assert!(mapping.material_properties.is_empty());
    }

    #[test]
    fn test_extract_texture_mapping_hex_hash() {
        let ritobin_content = r#"
        skinMeshProperties: embed = SkinMeshDataProperties {
            texture: string = "ASSETS/Characters/Test/Skins/Skin0/Default.tex"
            materialOverride: list[embed] = {
                SkinMeshDataProperties_MaterialOverride {
                    material: link = 0xABCDEF12
                    submesh: string = "HashedMesh"
                }
            }
        }
        
        0xABCDEF12 = StaticMaterialDef {
            name: string = "HashedMaterial"
            samplerValues: list2[embed] = {
                StaticMaterialShaderSamplerDef {
                    textureName: string = "Diffuse_Color"
                    texturePath: string = "ASSETS/Characters/Test/Skins/Skin0/Hashed_Resolved.tex"
                    addressU: u32 = 1
                    addressV: u32 = 1
                }
            }
        }
        "#;
        
        let mapping = extract_texture_mapping_from_text(ritobin_content).unwrap();
        
        // Check that hex hash was resolved
        assert_eq!(
            mapping.material_properties.get("HashedMesh").map(|p| &p.texture_path), 
            Some(&"ASSETS/Characters/Test/Skins/Skin0/Hashed_Resolved.tex".to_string())
        );
        // Should not appear in static_materials since it was resolved
        assert!(mapping.static_materials.is_empty());
    }

    #[test]
    fn test_extract_braced_block() {
        let content = r#"outer { inner { nested } more } end"#;
        let block = extract_braced_block(content, 5).unwrap();
        assert_eq!(block.trim(), "inner { nested } more");
    }
}

