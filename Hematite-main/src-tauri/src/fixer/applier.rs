//! Fixer Applier Module
//!
//! Implements transform actions from the config schema.
//! Modifies BIN files in-place based on config-driven rules.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::analyzer::bin_parser::{
    BinTree, BinProperty, PropertyValueEnum,
};
use crate::analyzer::hash_dict::HashDict;
use crate::analyzer::wad_cache::WadCache;
use crate::analyzer::detect_issue;
use crate::config::schema::{BinDataType, FixRule, ParentEmbed, TransformAction};

use ltk_meta::value::*;

/// Result of applying fixes to a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    /// Path to the file that was processed
    pub file_path: String,
    /// List of fixes that were successfully applied
    pub fixes_applied: Vec<AppliedFix>,
    /// List of fixes that failed
    pub fixes_failed: Vec<FailedFix>,
    /// Whether all requested fixes succeeded
    pub success: bool,
}

/// A successfully applied fix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedFix {
    /// ID of the fix that was applied
    pub fix_id: String,
    /// Human-readable description of what was changed
    pub description: String,
    /// Number of changes made (e.g., fields modified)
    pub changes_count: u32,
}

/// A fix that failed to apply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedFix {
    /// ID of the fix that failed
    pub fix_id: String,
    /// Error message
    pub error: String,
}

/// Context for applying fixes to a single file
pub struct FixContext<'a> {
    /// The parsed BIN tree (mutable for in-place modification)
    pub bin_tree: BinTree,
    /// WAD cache for checking file existence
    pub wad_cache: &'a WadCache,
    /// Hash dictionary for type/field name resolution
    pub hash_dict: &'a HashDict,
    /// Files marked for removal from WAD
    pub files_to_remove: Vec<String>,
    /// Reverse lookup: field name -> hash
    field_name_to_hash: HashMap<String, u32>,
    /// Reverse lookup: type name -> hash
    type_name_to_hash: HashMap<String, u32>,
}

impl<'a> FixContext<'a> {
    /// Create a new fix context
    pub fn new(bin_tree: BinTree, wad_cache: &'a WadCache, hash_dict: &'a HashDict) -> Self {
        // Build reverse lookup maps
        let field_name_to_hash: HashMap<String, u32> = hash_dict
            .fields
            .iter()
            .map(|(k, v)| (v.to_lowercase(), *k))
            .collect();
        
        let type_name_to_hash: HashMap<String, u32> = hash_dict
            .types
            .iter()
            .map(|(k, v)| (v.to_lowercase(), *k))
            .collect();

        Self {
            bin_tree,
            wad_cache,
            hash_dict,
            files_to_remove: Vec::new(),
            field_name_to_hash,
            type_name_to_hash,
        }
    }

    /// Get field hash from name (reverse lookup)
    pub fn get_field_hash(&self, name: &str) -> Option<u32> {
        self.field_name_to_hash.get(&name.to_lowercase()).copied()
    }

    /// Get type hash from name (reverse lookup)
    pub fn get_type_hash(&self, name: &str) -> Option<u32> {
        self.type_name_to_hash.get(&name.to_lowercase()).copied()
    }
}

/// Apply a list of fixes to a file
///
/// # Arguments
/// * `file_path` - Path to the file being fixed (for reporting)
/// * `context` - Mutable fix context with BIN tree and resources
/// * `fix_rules` - Map of fix_id -> FixRule to apply
/// * `selected_fix_ids` - Which fixes to apply (by ID)
///
/// # Returns
/// FixResult containing success/failure details
pub fn apply_transforms(
    file_path: &str,
    context: &mut FixContext,
    fix_rules: &HashMap<String, FixRule>,
    selected_fix_ids: &[String],
) -> FixResult {
    let mut fixes_applied = Vec::new();
    let mut fixes_failed = Vec::new();

    for fix_id in selected_fix_ids {
        let Some(rule) = fix_rules.get(fix_id) else {
            fixes_failed.push(FailedFix {
                fix_id: fix_id.clone(),
                error: "Fix rule not found in config".to_string(),
            });
            continue;
        };

        if !rule.enabled {
            continue;
        }

        // Check detection FIRST - only apply if the issue is actually detected
        let is_detected = detect_issue(
            &rule.detect,
            context.wad_cache,
            Some(&context.bin_tree),
            context.hash_dict,
        );

        if !is_detected {
            // Issue not detected in this file - skip silently
            continue;
        }

        match apply_single_transform(context, &rule.apply, fix_id) {
            Ok(changes_count) => {
                if changes_count > 0 {
                    fixes_applied.push(AppliedFix {
                        fix_id: fix_id.clone(),
                        description: rule.description.clone(),
                        changes_count,
                    });
                }
            }
            Err(e) => {
                fixes_failed.push(FailedFix {
                    fix_id: fix_id.clone(),
                    error: format!("{:?}", e),
                });
            }
        }
    }

    let success = fixes_failed.is_empty();
    FixResult {
        file_path: file_path.to_string(),
        fixes_applied,
        fixes_failed,
        success,
    }
}

/// Apply a single transform action
fn apply_single_transform(
    context: &mut FixContext,
    action: &TransformAction,
    fix_id: &str,
) -> Result<u32> {
    match action {
        TransformAction::EnsureField {
            field,
            value,
            data_type,
            create_parent,
            entry_type,
        } => apply_ensure_field(context, field, value, data_type, create_parent.as_ref(), entry_type.as_deref()),

        TransformAction::RenameHash { from_hash, to_hash } => {
            apply_rename_hash(context, from_hash, to_hash)
        }

        TransformAction::ReplaceStringExtension { from, to } => {
            apply_replace_string_extension(context, from, to)
        }

        TransformAction::RemoveFromWad => {
            // Mark this file for removal - actual removal happens at WAD repack stage
            context.files_to_remove.push(fix_id.to_string());
            log::debug!("Marked file for removal from WAD: {}", fix_id);
            Ok(1)
        }

        // Smart transforms
        TransformAction::ChangeFieldType {
            field_name,
            from_type,
            to_type,
            transform_rule,
            append_values,
        } => apply_change_field_type(
            context,
            field_name,
            from_type,
            to_type,
            transform_rule,
            append_values,
        ),

        TransformAction::EnsureFieldWithContext {
            field,
            data_type,
            values,
        } => {
            // Use default value for now (context resolution would need file path parsing)
            apply_ensure_field(context, field, &values.default, data_type, None, None)
        }

        // Regex transforms
        TransformAction::RegexReplace {
            field_pattern,
            find_pattern,
            replace_pattern,
        } => apply_regex_replace(context, field_pattern, find_pattern, replace_pattern),

        TransformAction::RegexRenameField {
            field_pattern,
            new_name_pattern,
        } => apply_regex_rename_field(context, field_pattern, new_name_pattern),
    }
}

// =============================================================================
// TRANSFORM IMPLEMENTATIONS
// =============================================================================

/// Ensure a field exists with the specified value
fn apply_ensure_field(
    context: &mut FixContext,
    field: &str,
    value: &serde_json::Value,
    data_type: &BinDataType,
    create_parent: Option<&ParentEmbed>,
    entry_type: Option<&str>,
) -> Result<u32> {
    // Pre-calculate all hashes before borrowing bin_tree mutably
    let field_hash = context
        .get_field_hash(field)
        .ok_or_else(|| anyhow!("Unknown field name: {}", field))?;

    let parent_info = if let Some(parent) = create_parent {
        let parent_hash = context
            .get_field_hash(&parent.field)
            .ok_or_else(|| anyhow!("Unknown parent field: {}", parent.field))?;
        let parent_type_hash = context
            .get_type_hash(&parent.embed_type)
            .ok_or_else(|| anyhow!("Unknown parent type: {}", parent.embed_type))?;
        Some((parent_hash, parent_type_hash, parent.field.clone()))
    } else {
        None
    };

    // Get entry type hash if filtering
    let entry_type_hash = if let Some(et) = entry_type {
        Some(context.get_type_hash(et).ok_or_else(|| anyhow!("Unknown entry type: {}", et))?)
    } else {
        None
    };

    let mut changes = 0u32;

    // Get mutable access to objects
    let object_keys: Vec<u32> = context.bin_tree.objects.keys().cloned().collect();

    for path_hash in object_keys {
        let obj = context.bin_tree.objects.get_mut(&path_hash).unwrap();

        // Filter by entry type if specified
        if let Some(type_hash) = entry_type_hash {
            if obj.class_hash != type_hash {
                continue;  // Skip objects that don't match the target entry type
            }
        }

        // Check if we need to create parent embed first
        if let Some((parent_hash, parent_type_hash, ref parent_field)) = parent_info {
            if !obj.properties.contains_key(&parent_hash) {
                // Parent embed doesn't exist - create it with the field inside
                let mut embed_props = indexmap::IndexMap::new();

                // Add the target field inside the embed
                if let Some(prop_value) = json_to_property_value(value, data_type)? {
                    embed_props.insert(
                        field_hash,
                        BinProperty {
                            name_hash: field_hash,
                            value: prop_value,
                        },
                    );
                }

                let embed_value = EmbeddedValue(StructValue {
                    class_hash: parent_type_hash,
                    properties: embed_props,
                });

                obj.properties.insert(
                    parent_hash,
                    BinProperty {
                        name_hash: parent_hash,
                        value: PropertyValueEnum::Embedded(embed_value),
                    },
                );

                changes += 1;
                log::debug!("Created parent embed '{}' with field '{}'", parent_field, field);
            } else {
                // Parent embed exists - add or update the field inside it
                if let Some(prop) = obj.properties.get_mut(&parent_hash) {
                    if let PropertyValueEnum::Embedded(ref mut embed_val) = prop.value {
                        // Always insert/update the field (replaces existing value if wrong)
                        if let Some(prop_value) = json_to_property_value(value, data_type)? {
                            let existed = embed_val.0.properties.contains_key(&field_hash);
                            embed_val.0.properties.insert(
                                field_hash,
                                BinProperty {
                                    name_hash: field_hash,
                                    value: prop_value,
                                },
                            );
                            changes += 1;
                            if existed {
                                log::debug!("Updated field '{}' in existing embed '{}'", field, parent_field);
                            } else {
                                log::debug!("Added field '{}' to existing embed '{}'", field, parent_field);
                            }
                        }
                    }
                }
            }
        } else {
            // No parent creation needed, just add/update the field directly
            if let Some(prop_value) = json_to_property_value(value, data_type)? {
                obj.properties.insert(
                    field_hash,
                    BinProperty {
                        name_hash: field_hash,
                        value: prop_value,
                    },
                );
                changes += 1;
                log::debug!("Set field '{}' to {:?}", field, value);
            }
        }
    }

    Ok(changes)
}

/// Rename a field's hash (key name)
fn apply_rename_hash(context: &mut FixContext, from_hash: &str, to_hash: &str) -> Result<u32> {
    let from_hash_val = context
        .get_field_hash(from_hash)
        .ok_or_else(|| anyhow!("Unknown field name: {}", from_hash))?;
    let to_hash_val = context
        .get_field_hash(to_hash)
        .ok_or_else(|| anyhow!("Unknown field name: {}", to_hash))?;

    let mut changes = 0u32;

    // Iterate through all objects and their properties
    for (_path_hash, obj) in context.bin_tree.objects.iter_mut() {
        changes += rename_hash_in_properties(&mut obj.properties, from_hash_val, to_hash_val);
    }

    log::debug!(
        "Renamed {} occurrences of '{}' to '{}'",
        changes,
        from_hash,
        to_hash
    );
    Ok(changes)
}

/// Recursively rename hash in properties (handles nested embeds)
fn rename_hash_in_properties(
    properties: &mut indexmap::IndexMap<u32, BinProperty>,
    from_hash: u32,
    to_hash: u32,
) -> u32 {
    let mut changes = 0u32;

    // Check if we need to rename at this level
    if let Some(prop) = properties.swap_remove(&from_hash) {
        properties.insert(to_hash, prop);
        changes += 1;
    }

    // Recursively check nested structures
    for (_hash, prop) in properties.iter_mut() {
        changes += rename_hash_in_value(&mut prop.value, from_hash, to_hash);
    }

    changes
}

/// Recursively rename hash in a property value
fn rename_hash_in_value(value: &mut PropertyValueEnum, from_hash: u32, to_hash: u32) -> u32 {
    let mut changes = 0u32;

    match value {
        PropertyValueEnum::Embedded(e) => {
            changes += rename_hash_in_properties(&mut e.0.properties, from_hash, to_hash);
        }
        PropertyValueEnum::Struct(s) => {
            changes += rename_hash_in_properties(&mut s.properties, from_hash, to_hash);
        }
        PropertyValueEnum::Container(c) => {
            for item in c.items.iter_mut() {
                changes += rename_hash_in_value(item, from_hash, to_hash);
            }
        }
        PropertyValueEnum::UnorderedContainer(c) => {
            for item in c.0.items.iter_mut() {
                changes += rename_hash_in_value(item, from_hash, to_hash);
            }
        }
        PropertyValueEnum::Optional(o) => {
            if let Some(inner) = o.value.as_mut() {
                changes += rename_hash_in_value(inner, from_hash, to_hash);
            }
        }
        _ => {}
    }

    changes
}

/// Replace file extension in all string values
fn apply_replace_string_extension(
    context: &mut FixContext,
    from: &str,
    to: &str,
) -> Result<u32> {
    let mut changes = 0u32;
    let wad_cache = context.wad_cache;

    for (_path_hash, obj) in context.bin_tree.objects.iter_mut() {
        for (_prop_hash, prop) in obj.properties.iter_mut() {
            changes += replace_extension_in_value(&mut prop.value, from, to, wad_cache);
        }
    }

    log::debug!(
        "Replaced {} extension occurrences from '{}' to '{}'",
        changes,
        from,
        to
    );
    Ok(changes)
}

/// Recursively replace extension in string values
fn replace_extension_in_value(
    value: &mut PropertyValueEnum,
    from: &str,
    to: &str,
    wad_cache: &WadCache,
) -> u32 {
    let mut changes = 0u32;

    match value {
        PropertyValueEnum::String(s) => {
            if s.0.to_lowercase().ends_with(from) {
                // Only replace if the file doesn't exist in WAD
                if !wad_cache.has_file_path(&s.0) {
                    let new_value = format!("{}{}", &s.0[..s.0.len() - from.len()], to);
                    s.0 = new_value;
                    changes += 1;
                }
            }
        }
        PropertyValueEnum::Container(c) => {
            for item in c.items.iter_mut() {
                changes += replace_extension_in_value(item, from, to, wad_cache);
            }
        }
        PropertyValueEnum::UnorderedContainer(c) => {
            for item in c.0.items.iter_mut() {
                changes += replace_extension_in_value(item, from, to, wad_cache);
            }
        }
        PropertyValueEnum::Embedded(e) => {
            for (_h, p) in e.0.properties.iter_mut() {
                changes += replace_extension_in_value(&mut p.value, from, to, wad_cache);
            }
        }
        PropertyValueEnum::Struct(s) => {
            for (_h, p) in s.properties.iter_mut() {
                changes += replace_extension_in_value(&mut p.value, from, to, wad_cache);
            }
        }
        PropertyValueEnum::Optional(o) => {
            if let Some(inner) = o.value.as_mut() {
                changes += replace_extension_in_value(inner, from, to, wad_cache);
            }
        }
        PropertyValueEnum::Map(m) => {
            for (k, v) in m.entries.iter_mut() {
                // Keys use PropertyValueUnsafeEq - skip for now
                let _ = k;
                changes += replace_extension_in_value(v, from, to, wad_cache);
            }
        }
        _ => {}
    }

    changes
}

/// Change a field's data type (e.g., vec3→vec4, link→string)
fn apply_change_field_type(
    context: &mut FixContext,
    field_name: &str,
    from_type: &str,
    to_type: &str,
    transform_rule: &str,
    append_values: &[serde_json::Value],
) -> Result<u32> {
    let mut changes = 0u32;

    // Collect object keys to avoid borrow issues
    let object_keys: Vec<u32> = context.bin_tree.objects.keys().cloned().collect();

    for path_hash in object_keys {
        let obj = context.bin_tree.objects.get_mut(&path_hash).unwrap();
        
        // Collect property keys that match the field pattern
        let matching_props: Vec<u32> = obj
            .properties
            .keys()
            .filter(|&hash| {
                if let Some(name) = context.hash_dict.get_field(*hash) {
                    matches_field_pattern(name, field_name)
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        for prop_hash in matching_props {
            if let Some(prop) = obj.properties.get_mut(&prop_hash) {
                // Check if current type matches from_type
                if !property_matches_type(&prop.value, from_type) {
                    continue;
                }

                // Convert the value
                if let Some(new_value) = convert_property_type(
                    &prop.value,
                    from_type,
                    to_type,
                    transform_rule,
                    append_values,
                )? {
                    let field_name_str = context.hash_dict.get_field(prop_hash).unwrap_or("unknown");
                    log::info!(
                        "Changed field '{}' type from {} to {}",
                        field_name_str,
                        from_type,
                        to_type
                    );
                    prop.value = new_value;
                    changes += 1;
                }
            }
        }
    }

    if changes > 0 {
        log::info!(
            "ChangeFieldType: converted {} fields from {} to {}",
            changes,
            from_type,
            to_type
        );
    }

    Ok(changes)
}

/// Check if a field name matches a pattern (supports wildcards: *pattern*, pattern*, *pattern)
fn matches_field_pattern(field_name: &str, pattern: &str) -> bool {
    let name_lower = field_name.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    if pattern_lower == "*" {
        return true;
    }

    // Handle wildcard patterns
    if pattern_lower.starts_with('*') && pattern_lower.ends_with('*') {
        // *foo* - contains
        let inner = &pattern_lower[1..pattern_lower.len() - 1];
        return name_lower.contains(inner);
    } else if pattern_lower.starts_with('*') {
        // *foo - ends with
        let suffix = &pattern_lower[1..];
        return name_lower.ends_with(suffix);
    } else if pattern_lower.ends_with('*') {
        // foo* - starts with
        let prefix = &pattern_lower[..pattern_lower.len() - 1];
        return name_lower.starts_with(prefix);
    }

    // Exact match
    name_lower == pattern_lower
}

/// Check if a property value matches the expected type string
fn property_matches_type(value: &PropertyValueEnum, type_str: &str) -> bool {
    match (value, type_str.to_lowercase().as_str()) {
        (PropertyValueEnum::U8(_), "u8") => true,
        (PropertyValueEnum::U16(_), "u16") => true,
        (PropertyValueEnum::U32(_), "u32") => true,
        (PropertyValueEnum::U64(_), "u64") => true,
        (PropertyValueEnum::I8(_), "i8") => true,
        (PropertyValueEnum::I16(_), "i16") => true,
        (PropertyValueEnum::I32(_), "i32") => true,
        (PropertyValueEnum::I64(_), "i64") => true,
        (PropertyValueEnum::F32(_), "f32") => true,
        (PropertyValueEnum::Bool(_), "bool") => true,
        (PropertyValueEnum::String(_), "string") => true,
        (PropertyValueEnum::Vector2(_), "vec2") => true,
        (PropertyValueEnum::Vector3(_), "vec3") => true,
        (PropertyValueEnum::Vector4(_), "vec4") => true,
        (PropertyValueEnum::Hash(_), "hash" | "link") => true,
        (PropertyValueEnum::Embedded(_), "embed") => true,
        _ => false,
    }
}

/// Convert a property value from one type to another
fn convert_property_type(
    value: &PropertyValueEnum,
    from_type: &str,
    to_type: &str,
    transform_rule: &str,
    append_values: &[serde_json::Value],
) -> Result<Option<PropertyValueEnum>> {
    let from_lower = from_type.to_lowercase();
    let to_lower = to_type.to_lowercase();

    match (from_lower.as_str(), to_lower.as_str()) {
        // vec3 -> vec4 (common color conversion)
        ("vec3", "vec4") => {
            if let PropertyValueEnum::Vector3(Vector3Value(v)) = value {
                // Get the 4th component from append_values, default to 1.0 (alpha)
                let w = append_values
                    .first()
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(1.0);
                
                Ok(Some(PropertyValueEnum::Vector4(Vector4Value(
                    [v.x, v.y, v.z, w].into(),
                ))))
            } else {
                log::warn!("Expected vec3 value but got different type");
                Ok(None)
            }
        }

        // vec2 -> vec3
        ("vec2", "vec3") => {
            if let PropertyValueEnum::Vector2(Vector2Value(v)) = value {
                let z = append_values
                    .first()
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(0.0);
                
                Ok(Some(PropertyValueEnum::Vector3(Vector3Value(
                    [v.x, v.y, z].into(),
                ))))
            } else {
                Ok(None)
            }
        }

        // link/hash -> string
        ("link" | "hash", "string") => {
            match transform_rule {
                "set_empty_string" => {
                    Ok(Some(PropertyValueEnum::String(StringValue(String::new()))))
                }
                "keep_value" | _ => {
                    // Convert hash to hex string representation
                    if let PropertyValueEnum::Hash(HashValue(h)) = value {
                        Ok(Some(PropertyValueEnum::String(StringValue(format!("{:08x}", h)))))
                    } else {
                        Ok(None)
                    }
                }
            }
        }

        // string -> link/hash  
        ("string", "link" | "hash") => {
            if let PropertyValueEnum::String(StringValue(s)) = value {
                // Try to parse as hex, otherwise compute hash
                let hash = u32::from_str_radix(s.trim_start_matches("0x"), 16)
                    .unwrap_or_else(|_| compute_fnv1a_hash(s));
                Ok(Some(PropertyValueEnum::Hash(HashValue(hash))))
            } else {
                Ok(None)
            }
        }

        // f32 -> string
        ("f32", "string") => {
            if let PropertyValueEnum::F32(F32Value(f)) = value {
                Ok(Some(PropertyValueEnum::String(StringValue(f.to_string()))))
            } else {
                Ok(None)
            }
        }

        // u8 -> u32
        ("u8", "u32") => {
            if let PropertyValueEnum::U8(U8Value(v)) = value {
                Ok(Some(PropertyValueEnum::U32(U32Value(*v as u32))))
            } else {
                Ok(None)
            }
        }

        // u32 -> u8 (with clamping)
        ("u32", "u8") => {
            if let PropertyValueEnum::U32(U32Value(v)) = value {
                let clamped = (*v).min(255) as u8;
                Ok(Some(PropertyValueEnum::U8(U8Value(clamped))))
            } else {
                Ok(None)
            }
        }

        _ => {
            log::warn!(
                "Unsupported type conversion: {} -> {}",
                from_type,
                to_type
            );
            Ok(None)
        }
    }
}

/// Compute FNV-1a hash (used for BIN field names)
fn compute_fnv1a_hash(s: &str) -> u32 {
    const FNV_OFFSET: u32 = 0x811c9dc5;
    const FNV_PRIME: u32 = 0x01000193;
    
    s.to_lowercase()
        .bytes()
        .fold(FNV_OFFSET, |hash, byte| {
            (hash ^ (byte as u32)).wrapping_mul(FNV_PRIME)
        })
}

/// Rename fields matching a regex pattern
/// 
/// Supports capture groups in new_name_pattern (e.g., "new_$1_suffix")
fn apply_regex_rename_field(
    context: &mut FixContext,
    field_pattern: &str,
    new_name_pattern: &str,
) -> Result<u32> {
    let field_regex = regex::Regex::new(field_pattern)
        .with_context(|| format!("Invalid field pattern: {}", field_pattern))?;

    let mut changes = 0u32;

    // Collect object keys to avoid borrow issues
    let object_keys: Vec<u32> = context.bin_tree.objects.keys().cloned().collect();

    for path_hash in object_keys {
        let obj = context.bin_tree.objects.get_mut(&path_hash).unwrap();
        
        // Find properties that match the pattern
        let renames: Vec<(u32, u32, String)> = obj
            .properties
            .keys()
            .filter_map(|&hash| {
                let field_name = context.hash_dict.get_field(hash)?;
                if field_regex.is_match(field_name) {
                    // Generate new field name using regex replacement
                    let new_name = field_regex
                        .replace(field_name, new_name_pattern)
                        .to_string();
                    
                    // Compute new hash for the renamed field
                    let new_hash = compute_fnv1a_hash(&new_name);
                    
                    Some((hash, new_hash, new_name))
                } else {
                    None
                }
            })
            .collect();

        // Apply the renames
        for (old_hash, new_hash, new_name) in renames {
            if let Some(mut prop) = obj.properties.swap_remove(&old_hash) {
                // Update the property's name_hash
                prop.name_hash = new_hash;
                
                // Insert with new hash (avoid collision check - replace if exists)
                obj.properties.insert(new_hash, prop);
                
                log::info!(
                    "Renamed field hash 0x{:08x} to '{}' (0x{:08x})",
                    old_hash,
                    new_name,
                    new_hash
                );
                changes += 1;
            }
        }
    }

    if changes > 0 {
        log::info!(
            "RegexRenameField: renamed {} fields matching '{}'",
            changes,
            field_pattern
        );
    }

    Ok(changes)
}

fn apply_regex_replace(
    context: &mut FixContext,
    field_pattern: &str,
    find_pattern: &str,
    replace_pattern: &str,
) -> Result<u32> {
    let field_regex = regex::Regex::new(field_pattern)
        .with_context(|| format!("Invalid field pattern: {}", field_pattern))?;
    let find_regex = regex::Regex::new(find_pattern)
        .with_context(|| format!("Invalid find pattern: {}", find_pattern))?;

    let mut changes = 0u32;

    for (_path_hash, obj) in context.bin_tree.objects.iter_mut() {
        for (prop_hash, prop) in obj.properties.iter_mut() {
            // Check if field name matches pattern
            if let Some(field_name) = context.hash_dict.get_field(*prop_hash) {
                if field_regex.is_match(field_name) {
                    changes += regex_replace_in_value(&mut prop.value, &find_regex, replace_pattern);
                }
            }
        }
    }

    log::debug!(
        "Regex replaced {} occurrences matching '{}' -> '{}'",
        changes,
        find_pattern,
        replace_pattern
    );
    Ok(changes)
}

/// Recursively apply regex replacement to string values
fn regex_replace_in_value(
    value: &mut PropertyValueEnum,
    find_regex: &regex::Regex,
    replace_pattern: &str,
) -> u32 {
    let mut changes = 0u32;

    match value {
        PropertyValueEnum::String(s) => {
            if find_regex.is_match(&s.0) {
                let new_value = find_regex.replace_all(&s.0, replace_pattern).to_string();
                if new_value != s.0 {
                    s.0 = new_value;
                    changes += 1;
                }
            }
        }
        PropertyValueEnum::Container(c) => {
            for item in c.items.iter_mut() {
                changes += regex_replace_in_value(item, find_regex, replace_pattern);
            }
        }
        PropertyValueEnum::UnorderedContainer(c) => {
            for item in c.0.items.iter_mut() {
                changes += regex_replace_in_value(item, find_regex, replace_pattern);
            }
        }
        PropertyValueEnum::Embedded(e) => {
            for (_h, p) in e.0.properties.iter_mut() {
                changes += regex_replace_in_value(&mut p.value, find_regex, replace_pattern);
            }
        }
        PropertyValueEnum::Struct(s) => {
            for (_h, p) in s.properties.iter_mut() {
                changes += regex_replace_in_value(&mut p.value, find_regex, replace_pattern);
            }
        }
        PropertyValueEnum::Optional(o) => {
            if let Some(inner) = o.value.as_mut() {
                changes += regex_replace_in_value(inner, find_regex, replace_pattern);
            }
        }
        _ => {}
    }

    changes
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Convert JSON value to BIN property value
fn json_to_property_value(
    value: &serde_json::Value,
    data_type: &BinDataType,
) -> Result<Option<PropertyValueEnum>> {
    match data_type {
        BinDataType::U8 => {
            let v = value.as_u64().ok_or_else(|| anyhow!("Expected u8"))? as u8;
            Ok(Some(PropertyValueEnum::U8(U8Value(v))))
        }
        BinDataType::U16 => {
            let v = value.as_u64().ok_or_else(|| anyhow!("Expected u16"))? as u16;
            Ok(Some(PropertyValueEnum::U16(U16Value(v))))
        }
        BinDataType::U32 => {
            let v = value.as_u64().ok_or_else(|| anyhow!("Expected u32"))? as u32;
            Ok(Some(PropertyValueEnum::U32(U32Value(v))))
        }
        BinDataType::U64 => {
            let v = value.as_u64().ok_or_else(|| anyhow!("Expected u64"))?;
            Ok(Some(PropertyValueEnum::U64(U64Value(v))))
        }
        BinDataType::I8 => {
            let v = value.as_i64().ok_or_else(|| anyhow!("Expected i8"))? as i8;
            Ok(Some(PropertyValueEnum::I8(I8Value(v))))
        }
        BinDataType::I16 => {
            let v = value.as_i64().ok_or_else(|| anyhow!("Expected i16"))? as i16;
            Ok(Some(PropertyValueEnum::I16(I16Value(v))))
        }
        BinDataType::I32 => {
            let v = value.as_i64().ok_or_else(|| anyhow!("Expected i32"))? as i32;
            Ok(Some(PropertyValueEnum::I32(I32Value(v))))
        }
        BinDataType::I64 => {
            let v = value.as_i64().ok_or_else(|| anyhow!("Expected i64"))?;
            Ok(Some(PropertyValueEnum::I64(I64Value(v))))
        }
        BinDataType::F32 => {
            let v = value.as_f64().ok_or_else(|| anyhow!("Expected f32"))? as f32;
            Ok(Some(PropertyValueEnum::F32(F32Value(v))))
        }
        BinDataType::F64 => {
            // F64 not directly supported in league-toolkit, use F32
            let v = value.as_f64().ok_or_else(|| anyhow!("Expected f64"))? as f32;
            log::warn!("F64 not supported, using F32");
            Ok(Some(PropertyValueEnum::F32(F32Value(v))))
        }
        BinDataType::Bool => {
            let v = value.as_bool().ok_or_else(|| anyhow!("Expected bool"))?;
            Ok(Some(PropertyValueEnum::Bool(BoolValue(v))))
        }
        BinDataType::String => {
            let v = value.as_str().ok_or_else(|| anyhow!("Expected string"))?;
            Ok(Some(PropertyValueEnum::String(StringValue(v.to_string()))))
        }
        BinDataType::Vec2 => {
            let arr = value.as_array().ok_or_else(|| anyhow!("Expected array"))?;
            if arr.len() != 2 {
                return Err(anyhow!("Vec2 requires 2 elements"));
            }
            let x = arr[0].as_f64().ok_or_else(|| anyhow!("Expected f32"))? as f32;
            let y = arr[1].as_f64().ok_or_else(|| anyhow!("Expected f32"))? as f32;
            Ok(Some(PropertyValueEnum::Vector2(Vector2Value([x, y].into()))))
        }
        BinDataType::Vec3 => {
            let arr = value.as_array().ok_or_else(|| anyhow!("Expected array"))?;
            if arr.len() != 3 {
                return Err(anyhow!("Vec3 requires 3 elements"));
            }
            let x = arr[0].as_f64().ok_or_else(|| anyhow!("Expected f32"))? as f32;
            let y = arr[1].as_f64().ok_or_else(|| anyhow!("Expected f32"))? as f32;
            let z = arr[2].as_f64().ok_or_else(|| anyhow!("Expected f32"))? as f32;
            Ok(Some(PropertyValueEnum::Vector3(Vector3Value([x, y, z].into()))))
        }
        BinDataType::Vec4 => {
            let arr = value.as_array().ok_or_else(|| anyhow!("Expected array"))?;
            if arr.len() != 4 {
                return Err(anyhow!("Vec4 requires 4 elements"));
            }
            let x = arr[0].as_f64().ok_or_else(|| anyhow!("Expected f32"))? as f32;
            let y = arr[1].as_f64().ok_or_else(|| anyhow!("Expected f32"))? as f32;
            let z = arr[2].as_f64().ok_or_else(|| anyhow!("Expected f32"))? as f32;
            let w = arr[3].as_f64().ok_or_else(|| anyhow!("Expected f32"))? as f32;
            Ok(Some(PropertyValueEnum::Vector4(Vector4Value([x, y, z, w].into()))))
        }
        BinDataType::Hash => {
            let v = value.as_u64().ok_or_else(|| anyhow!("Expected hash"))? as u32;
            Ok(Some(PropertyValueEnum::Hash(HashValue(v))))
        }
        BinDataType::Link => {
            // Link is handled as Hash in league-toolkit
            let v = value.as_u64().ok_or_else(|| anyhow!("Expected link"))? as u32;
            Ok(Some(PropertyValueEnum::Hash(HashValue(v))))
        }
        // Complex types not fully supported yet
        BinDataType::Pointer | BinDataType::Embed => {
            log::debug!("Complex type {:?} not fully supported in json_to_property_value", data_type);
            Ok(None)
        }
    }
}



// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_result_serialization() {
        let result = FixResult {
            file_path: "test.bin".to_string(),
            fixes_applied: vec![AppliedFix {
                fix_id: "healthbar_fix".to_string(),
                description: "Added UnitHealthBarStyle field".to_string(),
                changes_count: 1,
            }],
            fixes_failed: vec![],
            success: true,
        };

        let json = serde_json::to_string(&result).expect("Failed to serialize");
        assert!(json.contains("healthbar_fix"));
        assert!(json.contains("test.bin"));

        let parsed: FixResult = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(parsed.file_path, "test.bin");
        assert!(parsed.success);
    }

    #[test]
    fn test_applied_fix_clone() {
        let fix = AppliedFix {
            fix_id: "test".to_string(),
            description: "Test fix".to_string(),
            changes_count: 5,
        };

        let cloned = fix.clone();
        assert_eq!(fix.fix_id, cloned.fix_id);
        assert_eq!(fix.changes_count, cloned.changes_count);
    }

    #[test]
    fn test_json_to_property_value_u8() {
        let value = serde_json::json!(12);
        let result = json_to_property_value(&value, &BinDataType::U8).unwrap();
        assert!(matches!(result, Some(PropertyValueEnum::U8(U8Value(12)))));
    }

    #[test]
    fn test_json_to_property_value_string() {
        let value = serde_json::json!("test.tex");
        let result = json_to_property_value(&value, &BinDataType::String).unwrap();
        match result {
            Some(PropertyValueEnum::String(StringValue(s))) => assert_eq!(s, "test.tex"),
            _ => panic!("Expected String value"),
        }
    }

    #[test]
    fn test_json_to_property_value_vec3() {
        let value = serde_json::json!([1.0, 2.0, 3.0]);
        let result = json_to_property_value(&value, &BinDataType::Vec3).unwrap();
        match result {
            Some(PropertyValueEnum::Vector3(Vector3Value(v))) => {
                assert_eq!(v.x, 1.0);
                assert_eq!(v.y, 2.0);
                assert_eq!(v.z, 3.0);
            }
            _ => panic!("Expected Vec3 value"),
        }
    }

    #[test]
    fn test_matches_field_pattern_exact() {
        assert!(matches_field_pattern("TextureName", "TextureName"));
        assert!(matches_field_pattern("texturename", "TextureName")); // case insensitive
        assert!(!matches_field_pattern("TexturePath", "TextureName"));
    }

    #[test]
    fn test_matches_field_pattern_wildcards() {
        // *pattern* - contains
        assert!(matches_field_pattern("myColorValue", "*Color*"));
        assert!(matches_field_pattern("ColorTest", "*Color*"));
        assert!(!matches_field_pattern("myValue", "*Color*"));

        // pattern* - starts with
        assert!(matches_field_pattern("TextureName", "Texture*"));
        assert!(matches_field_pattern("TexturePath", "Texture*"));
        assert!(!matches_field_pattern("SomeTexture", "Texture*"));

        // *pattern - ends with
        assert!(matches_field_pattern("MyTexture", "*Texture"));
        assert!(matches_field_pattern("SomeTexture", "*Texture"));
        assert!(!matches_field_pattern("TextureName", "*Texture"));

        // * matches all
        assert!(matches_field_pattern("anything", "*"));
    }

    #[test]
    fn test_property_matches_type() {
        assert!(property_matches_type(&PropertyValueEnum::U8(U8Value(12)), "u8"));
        assert!(property_matches_type(&PropertyValueEnum::String(StringValue("test".to_string())), "string"));
        assert!(property_matches_type(&PropertyValueEnum::Vector3(Vector3Value([1.0, 2.0, 3.0].into())), "vec3"));
        assert!(property_matches_type(&PropertyValueEnum::Hash(HashValue(0x12345678)), "link"));
        assert!(property_matches_type(&PropertyValueEnum::Hash(HashValue(0x12345678)), "hash"));
        
        // Negative cases
        assert!(!property_matches_type(&PropertyValueEnum::U8(U8Value(12)), "u32"));
        assert!(!property_matches_type(&PropertyValueEnum::String(StringValue("test".to_string())), "u8"));
    }

    #[test]
    fn test_convert_vec3_to_vec4() {
        let vec3 = PropertyValueEnum::Vector3(Vector3Value([1.0, 2.0, 3.0].into()));
        let append = vec![serde_json::json!(1.0)];
        
        let result = convert_property_type(&vec3, "vec3", "vec4", "convert", &append).unwrap();
        
        match result {
            Some(PropertyValueEnum::Vector4(Vector4Value(v))) => {
                assert_eq!(v.x, 1.0);
                assert_eq!(v.y, 2.0);
                assert_eq!(v.z, 3.0);
                assert_eq!(v.w, 1.0);
            }
            _ => panic!("Expected Vec4 value"),
        }
    }

    #[test]
    fn test_convert_link_to_string() {
        let link = PropertyValueEnum::Hash(HashValue(0xDEADBEEF));
        
        let result = convert_property_type(&link, "link", "string", "keep_value", &[]).unwrap();
        
        match result {
            Some(PropertyValueEnum::String(StringValue(s))) => {
                assert_eq!(s, "deadbeef");
            }
            _ => panic!("Expected String value"),
        }
    }

    #[test]
    fn test_compute_fnv1a_hash() {
        // Known FNV-1a hashes for common field names
        let hash = compute_fnv1a_hash("texturename");
        assert!(hash != 0); // Just verify it produces a non-zero hash
        
        // Same input should give same output
        assert_eq!(compute_fnv1a_hash("test"), compute_fnv1a_hash("TEST")); // lowercase normalization
        
        // Different inputs should give different outputs
        assert_ne!(compute_fnv1a_hash("foo"), compute_fnv1a_hash("bar"));
    }
}
