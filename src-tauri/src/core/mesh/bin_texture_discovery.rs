//! BIN Texture Discovery - Quartz-style implementation
//!
//! Recursively scans BIN files to find texture mappings for SKN and SCB meshes.
//! Based on Quartz's discoverMaterialTextureHints approach.

use std::collections::HashMap;
use std::path::Path;
use serde_json::Value;

use super::texture::MaterialProperties;

/// Result of texture discovery from BIN files
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct TextureHints {
    /// Default texture for meshes without specific material
    pub default_texture: Option<String>,

    /// Per-material texture mappings (material_name -> MaterialProperties)
    pub material_hints: HashMap<String, MaterialProperties>,

    /// Per-SKN default textures (skn_path -> texture_path)
    pub default_texture_by_skn: HashMap<String, String>,

    /// All discovered texture references
    pub discovered_texture_refs: Vec<String>,
}

/// Discover material→texture mappings from BIN file content
///
/// Scans through all entries in the BIN to find:
/// - SkinMeshDataProperties (with simpleSkin, material, texture, materialOverride)
/// - StaticMaterialDef (with samplerValues containing Diffuse_Texture)
/// - MaterialOverride entries (submesh→texture/material mappings)
pub fn discover_material_textures(bin_content: &Value, character_folder: Option<&str>) -> TextureHints {
    let mut hints = TextureHints::default();
    let mut submesh_to_material: HashMap<String, String> = HashMap::new();
    let mut submesh_to_texture: HashMap<String, String> = HashMap::new();
    let mut material_to_texture: HashMap<String, MaterialProperties> = HashMap::new();
    let mut skin_mesh_defaults: Vec<SkinMeshDefault> = Vec::new();

    tracing::info!("🔍 Starting BIN texture discovery (character_folder={:?})", character_folder);

    // Parse the BIN tree
    if let Some(entries) = bin_content.get("entries").and_then(|e| e.as_array()) {
        for (idx, entry) in entries.iter().enumerate() {
            let entry_type = get_string_field(entry, "type");
            let entry_hash = get_string_field(entry, "hash");

            tracing::trace!("Entry {}: type={}, hash={}", idx, entry_type, entry_hash);

            // Check if this is a StaticMaterialDef
            if entry_type.to_lowercase().contains("staticmaterialdef") ||
               entry_hash.to_lowercase().contains("/materials/") {
                scan_static_material_def(entry, &mut material_to_texture);
            }

            // Scan all fields for material overrides and skin mesh properties
            if let Some(data) = entry.get("data").and_then(|d| d.as_array()) {
                for field in data {
                    // Scan for materialOverride fields
                    scan_for_material_overrides(
                        field,
                        &mut submesh_to_material,
                        &mut submesh_to_texture,
                        &mut hints,
                        character_folder,
                    );

                    // Scan for SkinMeshDataProperties
                    scan_for_skin_mesh_defaults(
                        field,
                        &mut skin_mesh_defaults,
                    );
                }
            }
        }
    }

    tracing::info!("📊 Discovery results:");
    tracing::info!("  - material_to_texture: {} entries", material_to_texture.len());
    tracing::info!("  - submesh_to_material: {} entries", submesh_to_material.len());
    tracing::info!("  - submesh_to_texture: {} direct entries", submesh_to_texture.len());
    tracing::info!("  - skin_mesh_defaults: {} entries", skin_mesh_defaults.len());

    // Join: submesh → material → texture
    for (submesh_key, material_ref) in &submesh_to_material {
        // Check if there's already a direct texture override
        if submesh_to_texture.contains_key(submesh_key) {
            continue;
        }

        // Try to resolve via material reference
        if let Some(props) = resolve_texture_by_material_ref(&material_to_texture, material_ref) {
            tracing::debug!("✓ Resolved '{}' via material '{}' → {}", submesh_key, material_ref, props.texture_path);
            hints.material_hints.insert(submesh_key.clone(), props.clone());

            if !hints.discovered_texture_refs.contains(&props.texture_path.to_lowercase()) {
                hints.discovered_texture_refs.push(props.texture_path.to_lowercase());
            }
        } else {
            tracing::warn!("✗ Failed to resolve material '{}' for submesh '{}'", material_ref, submesh_key);
        }
    }

    // Process SkinMeshDataProperties defaults
    let mut default_texture_hint = None;
    for default in &skin_mesh_defaults {
        // Resolve texture from material reference first, then direct texture
        let resolved = if !default.material_ref.is_empty() {
            resolve_texture_by_material_ref(&material_to_texture, &default.material_ref)
                .map(|p| p.texture_path.clone())
        } else {
            None
        }.or_else(|| {
            if !default.texture_path.is_empty() {
                Some(default.texture_path.clone())
            } else {
                None
            }
        });

        if let Some(tex) = resolved {
            if looks_like_texture_path(&tex) {
                // Store by simpleSkin path
                if !default.simple_skin_path.is_empty() {
                    let skin_key = normalize_key(&default.simple_skin_path);
                    hints.default_texture_by_skn.insert(skin_key, tex.clone());
                }

                // First valid texture becomes the fallback default
                if default_texture_hint.is_none() {
                    default_texture_hint = Some(tex.clone());
                }

                if !hints.discovered_texture_refs.contains(&tex.to_lowercase()) {
                    hints.discovered_texture_refs.push(tex.to_lowercase());
                }
            }
        }
    }

    if let Some(default_tex) = default_texture_hint {
        tracing::info!("🎨 Default texture: {}", default_tex);
        hints.default_texture = Some(default_tex.clone());
        hints.material_hints.insert("__default__".to_string(), MaterialProperties {
            texture_path: default_tex,
            ..Default::default()
        });
    }

    tracing::info!("✅ Final material_hints: {} entries", hints.material_hints.len());
    hints
}

#[derive(Debug, Clone, Default)]
struct SkinMeshDefault {
    simple_skin_path: String,
    material_ref: String,
    texture_path: String,
}

/// Scan a StaticMaterialDef entry for Diffuse_Texture samplerValue
fn scan_static_material_def(entry: &Value, material_to_texture: &mut HashMap<String, MaterialProperties>) {
    let entry_hash = get_string_field(entry, "hash");
    let entry_type = get_string_field(entry, "type");

    tracing::debug!("📦 Scanning StaticMaterialDef: hash={}, type={}", entry_hash, entry_type);

    // Get material reference name (prefer "name" field, fallback to hash)
    let mut material_ref_name = entry_hash.clone();
    if let Some(data) = entry.get("data").and_then(|d| d.as_array()) {
        if let Some(name_field) = find_field_by_name(data, "name") {
            if let Some(name_value) = read_string_like(name_field) {
                if !name_value.is_empty() {
                    material_ref_name = name_value;
                }
            }
        }

        // Find samplerValues field
        if let Some(sampler_values) = find_field_by_name(data, "samplervalues") {
            if let Some(samplers) = get_field_embed_list(sampler_values) {
                // Look for Diffuse_Texture sampler
                for sampler_item in samplers {
                    if let Some(sampler_fields) = get_field_data(sampler_item) {
                        let texture_name = find_field_by_name(sampler_fields, "texturename")
                            .and_then(read_string_like)
                            .unwrap_or_default();

                        // Check if this is a diffuse texture
                        if normalize_key(&texture_name).contains("diffuse_texture") {
                            if let Some(texture_path_field) = find_field_by_name(sampler_fields, "texturepath") {
                                if let Some(texture_path) = read_string_like(texture_path_field) {
                                    if looks_like_texture_path(&texture_path) {
                                        tracing::debug!("  ✓ Found Diffuse_Texture: {}", texture_path);

                                        // Extract UV parameters
                                        let (uv_scale, uv_offset, flipbook_size, flipbook_frame) =
                                            extract_param_values_from_fields(data);

                                        let props = MaterialProperties {
                                            texture_path: texture_path.replace('\\', "/"),
                                            uv_scale,
                                            uv_offset,
                                            flipbook_size,
                                            flipbook_frame,
                                        };

                                        add_material_alias(material_to_texture, &material_ref_name, props);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Recursively scan for materialOverride fields
fn scan_for_material_overrides(
    field: &Value,
    submesh_to_material: &mut HashMap<String, String>,
    submesh_to_texture: &mut HashMap<String, String>,
    hints: &mut TextureHints,
    character_folder: Option<&str>,
) {
    let field_type = field.get("type").and_then(|t| t.as_u64()).unwrap_or(0);

    // Container types: LIST(128), LIST2(129), EMBED(131), OPTION(133), MAP(134)
    let is_container = matches!(field_type, 128 | 129 | 131 | 133 | 134);

    // Check if this is a materialOverride EMBED
    if field_type == 131 {  // EMBED
        if let Some(fields) = get_field_data(field) {
            try_extract_override_from_embed(
                fields,
                submesh_to_material,
                submesh_to_texture,
                hints,
                character_folder,
            );

            // Recurse into children
            for child in fields {
                scan_for_material_overrides(
                    child,
                    submesh_to_material,
                    submesh_to_texture,
                    hints,
                    character_folder,
                );
            }
        }
    }

    // LIST/LIST2
    if field_type == 128 || field_type == 129 {
        if let Some(items) = field.get("data").and_then(|d| d.as_array()) {
            for item in items {
                if is_container {
                    scan_for_material_overrides(
                        item,
                        submesh_to_material,
                        submesh_to_texture,
                        hints,
                        character_folder,
                    );
                }
            }
        }
    }

    // OPTION
    if field_type == 133 {
        if let Some(payload) = field.get("data").and_then(|d| d.as_object()) {
            if let Some(data_val) = payload.values().next() {
                scan_for_material_overrides(
                    data_val,
                    submesh_to_material,
                    submesh_to_texture,
                    hints,
                    character_folder,
                );
            }
        }
    }

    // MAP
    if field_type == 134 {
        if let Some(map) = field.get("data").and_then(|d| d.as_object()) {
            for value in map.values() {
                if is_container {
                    scan_for_material_overrides(
                        value,
                        submesh_to_material,
                        submesh_to_texture,
                        hints,
                        character_folder,
                    );
                }
            }
        }
    }
}

/// Try to extract materialOverride from an EMBED field
fn try_extract_override_from_embed(
    embed_fields: &[Value],
    submesh_to_material: &mut HashMap<String, String>,
    submesh_to_texture: &mut HashMap<String, String>,
    hints: &mut TextureHints,
    character_folder: Option<&str>,
) {
    let submesh_field = find_field_by_name(embed_fields, "submesh");
    let material_field = find_field_by_name(embed_fields, "material");
    let texture_field = find_field_by_name(embed_fields, "texture");

    let submesh_name = submesh_field.and_then(read_string_like);

    if let Some(submesh) = submesh_name {
        let submesh_key = normalize_simple(&submesh);

        // Check for material link
        if let Some(material_ref) = material_field.and_then(read_string_like) {
            if !material_ref.is_empty() {
                submesh_to_material.insert(submesh_key.clone(), material_ref);
            }
        }

        // Check for direct texture
        if let Some(texture_path) = texture_field.and_then(read_string_like) {
            let normalized_path = texture_path.replace('\\', "/");
            if looks_like_texture_path(&normalized_path) &&
               texture_matches_character(character_folder, &normalized_path) {
                submesh_to_texture.insert(submesh_key.clone(), normalized_path.clone());
                hints.material_hints.insert(submesh_key, MaterialProperties {
                    texture_path: normalized_path.clone(),
                    ..Default::default()
                });

                if !hints.discovered_texture_refs.contains(&normalized_path.to_lowercase()) {
                    hints.discovered_texture_refs.push(normalized_path.to_lowercase());
                }
            }
        }
    }
}

/// Recursively scan for SkinMeshDataProperties
fn scan_for_skin_mesh_defaults(
    field: &Value,
    defaults: &mut Vec<SkinMeshDefault>,
) {
    let field_type = field.get("type").and_then(|t| t.as_u64()).unwrap_or(0);

    // EMBED
    if field_type == 131 {
        if let Some(fields) = get_field_data(field) {
            let hash_type = get_string_field(field, "hashType");
            let field_names: Vec<String> = fields.iter()
                .map(|f| {
                    let hash = get_string_field(f, "hash");
                    normalize_key(&hash)
                })
                .collect();

            // Check if this looks like a SkinMeshDataProperties
            let is_skin_mesh = hash_type.to_lowercase().contains("skinmeshdataproperties") ||
                field_names.iter().any(|n| n == "simpleskin") ||
                (field_names.iter().any(|n| n == "materialoverride") &&
                 field_names.iter().any(|n| n == "texture"));

            if is_skin_mesh {
                let simple_skin = find_field_by_name(fields, "simpleskin")
                    .and_then(read_string_like)
                    .unwrap_or_default()
                    .replace('\\', "/");

                let material_ref = find_field_by_name(fields, "material")
                    .and_then(read_string_like)
                    .unwrap_or_default();

                let texture_path = find_field_by_name(fields, "texture")
                    .and_then(read_string_like)
                    .unwrap_or_default()
                    .replace('\\', "/");

                if !simple_skin.is_empty() || !material_ref.is_empty() || !texture_path.is_empty() {
                    tracing::debug!("📝 Found SkinMeshDataProperties: skn={}, mat={}, tex={}",
                        simple_skin, material_ref, texture_path);
                    defaults.push(SkinMeshDefault {
                        simple_skin_path: simple_skin,
                        material_ref,
                        texture_path,
                    });
                }
            }

            // Recurse into children
            for child in fields {
                scan_for_skin_mesh_defaults(child, defaults);
            }
        }
    }

    // LIST/LIST2
    if field_type == 128 || field_type == 129 {
        if let Some(items) = field.get("data").and_then(|d| d.as_array()) {
            for item in items {
                scan_for_skin_mesh_defaults(item, defaults);
            }
        }
    }

    // OPTION
    if field_type == 133 {
        if let Some(payload) = field.get("data").and_then(|d| d.as_object()) {
            if let Some(data_val) = payload.values().next() {
                scan_for_skin_mesh_defaults(data_val, defaults);
            }
        }
    }

    // MAP
    if field_type == 134 {
        if let Some(map) = field.get("data").and_then(|d| d.as_object()) {
            for value in map.values() {
                scan_for_skin_mesh_defaults(value, defaults);
            }
        }
    }
}

/// Extract UV parameters from StaticMaterialDef paramValues
#[allow(clippy::type_complexity)]
fn extract_param_values_from_fields(fields: &[Value]) -> (Option<[f32; 2]>, Option<[f32; 2]>, Option<[u32; 2]>, Option<f32>) {
    let mut uv_scale = None;
    let mut uv_offset = None;
    let mut flipbook_size = None;
    let mut flipbook_frame = None;

    if let Some(param_values) = find_field_by_name(fields, "paramvalues") {
        if let Some(params) = get_field_embed_list(param_values) {
            for param in params {
                if let Some(param_fields) = get_field_data(param) {
                    let name = find_field_by_name(param_fields, "name")
                        .and_then(read_string_like)
                        .unwrap_or_default();

                    // Extract vec4 value
                    if let Some(value_field) = find_field_by_name(param_fields, "value") {
                        if value_field.get("type").and_then(|t| t.as_u64()) == Some(14) {  // VEC4
                            if let Some(vec) = value_field.get("data").and_then(|d| d.as_array()) {
                                let values: Vec<f32> = vec.iter()
                                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                                    .collect();

                                match name.as_str() {
                                    "UVScaleAndOffset" if values.len() >= 4 => {
                                        uv_scale = Some([values[0], values[1]]);
                                        uv_offset = Some([values[2], values[3]]);
                                    }
                                    "FlipbookSize" if values.len() >= 2 => {
                                        flipbook_size = Some([values[0] as u32, values[1] as u32]);
                                    }
                                    "FrameIndex" if !values.is_empty() => {
                                        flipbook_frame = Some(values[0]);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (uv_scale, uv_offset, flipbook_size, flipbook_frame)
}

// ============================================================================
// Helper Functions (matching Quartz's normalization logic)
// ============================================================================

fn normalize_key(s: &str) -> String {
    s.to_lowercase().replace('\\', "/")
}

fn normalize_simple(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn looks_like_texture_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    (lower.ends_with(".dds") || lower.ends_with(".tex") ||
     lower.ends_with(".png") || lower.ends_with(".jpg") ||
     lower.ends_with(".jpeg") || lower.ends_with(".tga")) &&
    (lower.contains('/') || lower.contains("assets"))
}

fn texture_matches_character(character_folder: Option<&str>, texture_path: &str) -> bool {
    if let Some(folder) = character_folder {
        let lower = normalize_key(texture_path);
        // Allow global textures
        if lower.contains("/shared/") || lower.contains("/global/") || lower.contains("/common/") {
            return true;
        }
        // Check if it's from the right character folder
        let expected = format!("/characters/{}/", folder.to_lowercase());
        lower.contains(&expected)
    } else {
        true
    }
}

fn add_material_alias(
    map: &mut HashMap<String, MaterialProperties>,
    material_ref: &str,
    props: MaterialProperties,
) {
    let ref_path = normalize_key(material_ref);
    let ref_base = Path::new(&ref_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let ref_no_inst = ref_base.trim_end_matches("_inst");

    map.insert(ref_path.clone(), props.clone());
    map.insert(normalize_simple(material_ref), props.clone());
    map.insert(ref_base.clone(), props.clone());
    map.insert(normalize_simple(&ref_base), props.clone());
    map.insert(ref_no_inst.to_string(), props.clone());
    map.insert(normalize_simple(ref_no_inst), props);
}

fn resolve_texture_by_material_ref(
    material_map: &HashMap<String, MaterialProperties>,
    material_ref: &str,
) -> Option<MaterialProperties> {
    let ref_path = normalize_key(material_ref);
    let ref_base = Path::new(&ref_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let ref_no_inst = ref_base.trim_end_matches("_inst");

    material_map.get(&ref_path)
        .or_else(|| material_map.get(&normalize_simple(material_ref)))
        .or_else(|| material_map.get(&ref_base))
        .or_else(|| material_map.get(&normalize_simple(&ref_base)))
        .or_else(|| material_map.get(ref_no_inst))
        .or_else(|| material_map.get(&normalize_simple(ref_no_inst)))
        .cloned()
}

// ============================================================================
// JSON Helper Functions
// ============================================================================

fn get_string_field(obj: &Value, field: &str) -> String {
    obj.get(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn get_field_data(field: &Value) -> Option<&Vec<Value>> {
    field.get("data").and_then(|d| d.as_array())
}

fn get_field_embed_list(field: &Value) -> Option<Vec<&Value>> {
    field.get("data")
        .and_then(|d| d.as_array())
        .map(|arr| arr.iter().collect())
}

fn find_field_by_name<'a>(fields: &'a [Value], target: &str) -> Option<&'a Value> {
    let target_lower = target.to_lowercase();
    fields.iter().find(|f| {
        let field_name = get_string_field(f, "hash");
        let normalized = normalize_key(&field_name);
        normalized == target_lower || normalized.ends_with(&format!("/{}", target_lower))
    })
}

fn read_string_like(field: &Value) -> Option<String> {
    let field_type = field.get("type").and_then(|t| t.as_u64())?;

    match field_type {
        16 => {  // STRING
            field.get("data").and_then(|d| d.as_str()).map(|s| s.to_string())
        }
        17 | 132 => {  // LINK / POINTER
            field.get("data").and_then(|d| d.as_str()).map(|s| s.to_string())
        }
        18 => {  // HASH
            field.get("data").and_then(|d| d.as_str()).map(|s| s.to_string())
        }
        _ => None
    }
}
