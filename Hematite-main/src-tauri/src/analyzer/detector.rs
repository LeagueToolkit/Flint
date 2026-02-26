//! Detector Module
//!
//! Maps config rules to actual detection logic.
//! Analyzes files to detect which fixes are needed.
//!
//! Uses the shared RitoShark hash dictionary for class_hash → entry_type
//! and name_hash → field name resolution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::schema::{DetectionRule, FixRule};
use super::wad_cache::WadCache;
use super::bin_parser::{parse_bin_file, extract_all_strings, BinTree, PropertyValueEnum, BinProperty};
use super::hash_dict::HashDict;

/// Result of analyzing a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// Path to the analyzed file
    pub file_path: String,
    /// List of detected issues that can be fixed
    pub detected_issues: Vec<DetectedIssue>,
}

/// A single detected issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedIssue {
    /// Unique ID of the fix that can resolve this (e.g., "healthbar_fix")
    pub fix_id: String,
    /// Human-readable name of the fix
    pub fix_name: String,
    /// Severity level for UI display
    pub severity: String,
    /// Description of what this fix does
    pub description: String,
}

/// Analyze a BIN file against all enabled fix rules
///
/// # Arguments
/// * `file_path` - Path to the BIN file to analyze
/// * `wad_cache` - Cache of file paths in the WAD (for checking if files exist)
/// * `fix_rules` - Map of fix ID -> FixRule from config
/// * `hash_dict` - Hash dictionary for type/field name resolution
///
/// # Returns
/// A ScanResult containing all detected issues
pub fn analyze_file(
    file_path: &str,
    wad_cache: &WadCache,
    fix_rules: &HashMap<String, FixRule>,
    hash_dict: &HashDict,
) -> Result<ScanResult> {
    let mut detected_issues = Vec::new();

    // Try to parse the file as BIN
    let bin_tree = match parse_bin_file(file_path) {
        Ok(tree) => Some(tree),
        Err(e) => {
            log::debug!("Could not parse {} as BIN: {:?}", file_path, e);
            None
        }
    };

    // Check each enabled fix rule
    for (fix_id, rule) in fix_rules {
        if !rule.enabled {
            continue;
        }

        let is_detected = detect_issue(&rule.detect, wad_cache, bin_tree.as_ref(), hash_dict);

        if is_detected {
            detected_issues.push(DetectedIssue {
                fix_id: fix_id.clone(),
                fix_name: rule.name.clone(),
                severity: format!("{:?}", rule.severity).to_lowercase(),
                description: rule.description.clone(),
            });
        }
    }

    Ok(ScanResult {
        file_path: file_path.to_string(),
        detected_issues,
    })
}

/// Check if a specific detection rule matches
///
/// # Arguments
/// * `rule` - The detection rule to check
/// * `wad_cache` - Cache for checking file existence
/// * `bin_tree` - Optional parsed BIN tree
/// * `hash_dict` - Hash dictionary for type/field resolution
///
/// # Returns
/// `true` if the issue is detected, `false` otherwise
pub fn detect_issue(
    rule: &DetectionRule,
    wad_cache: &WadCache,
    bin_tree: Option<&BinTree>,
    hash_dict: &HashDict,
) -> bool {
    match rule {
        DetectionRule::MissingOrWrongField {
            entry_type,
            embed_path,
            embed_type,
            field,
            expected_value,
        } => {
            if !hash_dict.is_loaded() {
                log::debug!("MissingOrWrongField skipped: hash dictionary not loaded");
                return false;
            }
            
            let Some(tree) = bin_tree else {
                return false;
            };

            // Find objects matching entry_type
            for (_path_hash, obj) in &tree.objects {
                // Check if this object's class matches entry_type
                let obj_type = hash_dict.get_type(obj.class_hash);
                if obj_type != Some(entry_type.as_str()) {
                    continue;
                }

                // If embed_type is specified, we need to find an embed with that CLASS TYPE
                // This matches Python's behavior: any(i.hash_type == BIN_HASH["CharacterHealthBarDataRecord"] for i in entry.data)
                if let Some(expected_embed_type) = embed_type {
                    // Search ALL properties for an embed with matching type
                    let mut found_embed = false;
                    let mut missing_field_in_embed = false;
                    
                    for (_prop_hash, prop) in &obj.properties {
                        if let PropertyValueEnum::Embedded(embed_val) = &prop.value {
                            // Check if this embed's class type matches
                            let actual_type = hash_dict.get_type(embed_val.0.class_hash);
                            if actual_type == Some(expected_embed_type.as_str()) {
                                found_embed = true;
                                
                                // Check if the target field exists inside this embed
                                let field_prop = embed_val.0.properties.iter().find(|(name_hash, _)| {
                                    hash_dict.get_field(**name_hash) == Some(field.as_str())
                                });
                                
                                match field_prop {
                                    None => {
                                        // Field is missing - needs fix
                                        log::debug!("Missing field '{}' inside embed of type '{}' in entry type '{}'", 
                                            field, expected_embed_type, entry_type);
                                        missing_field_in_embed = true;
                                    }
                                    Some((_, existing_prop)) => {
                                        // Field exists - check if value matches expected
                                        if let Some(expected) = expected_value {
                                            if !value_matches(&existing_prop.value, expected) {
                                                log::debug!("Field '{}' has wrong value (expected {:?}) in entry type '{}'", 
                                                    field, expected, entry_type);
                                                missing_field_in_embed = true; // Wrong value = needs fix
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    if missing_field_in_embed {
                        // Field is missing inside an existing embed - needs fix
                        return true;
                    }
                    
                    if !found_embed {
                        // No embed of the expected type found at all
                        // Python creates the entire embed in this case, so we need to detect this
                        log::debug!("No embed of type '{}' found in entry type '{}' - will create", 
                            expected_embed_type, entry_type);
                        return true;
                    }
                    
                } else if let Some(embed_field_name) = embed_path {
                    // Legacy path: search by field NAME (not type)
                    // Find the embed field in this object
                    let embed_prop = obj.properties.iter().find(|(name_hash, _)| {
                        hash_dict.get_field(**name_hash) == Some(embed_field_name.as_str())
                    });
                    
                    if let Some((_, prop)) = embed_prop {
                        // Found the embed - now check for the field inside it
                        if let PropertyValueEnum::Embedded(embed_val) = &prop.value {
                            // Check if the field exists inside the embed
                            let field_exists = embed_val.0.properties.iter().any(|(name_hash, _)| {
                                hash_dict.get_field(*name_hash) == Some(field.as_str())
                            });
                            
                            if !field_exists {
                                log::debug!("Missing field '{}' inside embed '{}' in entry type '{}'", 
                                    field, embed_field_name, entry_type);
                                return true;
                            }
                            
                            let _ = expected_value;
                        }
                    }
                    // If embed field doesn't exist, don't report - we only fix existing embeds
                    
                } else {
                    // No embed_path or embed_type - check field at top level
                    let field_matches = obj.properties.iter().any(|(name_hash, _prop)| {
                        hash_dict.get_field(*name_hash) == Some(field.as_str())
                    });

                    if !field_matches {
                        log::debug!("Missing field '{}' in entry type '{}'", field, entry_type);
                        return true;
                    }

                    let _ = expected_value;
                }
            }
            
            false
        }

        DetectionRule::FieldHashExists { entry_type, path } => {
            if !hash_dict.is_loaded() {
                log::debug!("FieldHashExists skipped: hash dictionary not loaded");
                return false;
            }
            
            let Some(tree) = bin_tree else {
                return false;
            };

            // Parse the path (e.g., "SamplerValues.*.TextureName")
            let path_parts: Vec<&str> = path.split('.').collect();
            if path_parts.is_empty() {
                return false;
            }

            // Find objects matching entry_type
            for (_path_hash, obj) in &tree.objects {
                let obj_type = hash_dict.get_type(obj.class_hash);
                if obj_type != Some(entry_type.as_str()) {
                    continue;
                }

                // Search for the field along the path
                if search_field_path(&obj.properties, &path_parts, hash_dict) {
                    log::debug!("Found field path '{}' in entry type '{}'", path, entry_type);
                    return true;
                }
            }
            
            false
        }

        DetectionRule::StringExtensionNotInWad {
            entry_type,
            fields,
            extension,
        } => {
            // Note: entry_type and fields filtering requires hash dict
            // For now, we check ALL strings in the BIN for the extension
            if let Some(tree) = bin_tree {
                let strings = extract_all_strings(tree);
                for s in strings {
                    if s.to_lowercase().ends_with(extension) && !wad_cache.has_file_path(&s) {
                        log::debug!("Found string '{}' with extension '{}' not in WAD", s, extension);
                        return true;
                    }
                }
            }
            let _ = (entry_type, fields); // Will use when hash dict available
            false
        }

        DetectionRule::RecursiveStringExtensionNotInWad {
            extension,
            path_prefixes,
        } => {
            // This can work without hash dict - just scan all strings
            if let Some(tree) = bin_tree {
                let strings = extract_all_strings(tree);
                for s in strings {
                    let lower = s.to_lowercase();
                    // Check extension
                    if !lower.ends_with(extension) {
                        continue;
                    }
                    // Check path prefix (if specified)
                    if !path_prefixes.is_empty() {
                        let matches_prefix = path_prefixes.iter().any(|prefix| {
                            lower.starts_with(&prefix.to_lowercase())
                        });
                        if !matches_prefix {
                            continue;
                        }
                    }
                    // Check if file exists in WAD
                    if !wad_cache.has_file_path(&s) {
                        log::debug!("Found string '{}' with extension '{}' not in WAD", s, extension);
                        return true;
                    }
                }
            }
            false
        }

        DetectionRule::EntryTypeExistsAny { entry_types } => {
            if !hash_dict.is_loaded() {
                log::debug!("EntryTypeExistsAny skipped: hash dictionary not loaded");
                return false;
            }
            
            let Some(tree) = bin_tree else {
                return false;
            };

            // Check if any object's class matches one of the entry_types
            for (_path_hash, obj) in &tree.objects {
                if let Some(type_name) = hash_dict.get_type(obj.class_hash) {
                    if entry_types.iter().any(|et| et.eq_ignore_ascii_case(type_name)) {
                        log::debug!("Found champion data type '{}' in BIN", type_name);
                        return true;
                    }
                }
            }
            
            false
        }

        DetectionRule::BnkVersionNotIn { allowed_versions } => {
            // BNK detection is handled separately - this flag just marks the rule
            // The actual BNK version check happens at the file scanning stage
            // For now, return false as BIN files are not BNK files
            log::debug!(
                "BnkVersionNotIn detection: this rule applies to BNK files, not BIN files. allowed={:?}",
                allowed_versions
            );
            // Note: BNK detection should use parse_bnk_version() with raw file bytes
            // This is handled in the scanner/processor level, not here
            false
        }

        // =====================================================================
        // SMART/PATTERN DETECTION (Stage 3.5)
        // =====================================================================

        DetectionRule::PatternMatch {
            entry_type,
            field_name,
            expected_data_type,
            wrong_data_type,
            context,
            champion_filter,
        } => {
            // Requires hash dictionary
            log::debug!(
                "PatternMatch detection skipped (requires hash dict): entry_type={}, field={}",
                entry_type, field_name
            );
            let _ = (expected_data_type, wrong_data_type, context, champion_filter);
            false
        }

        // =====================================================================
        // LAZY/REGEX DETECTION (Stage 3.5)
        // =====================================================================

        DetectionRule::RegexMatch {
            entry_type_pattern,
            field_name_pattern,
            value_pattern,
            data_type_pattern,
            path_pattern,
            context,
        } => {
            // Requires hash dictionary for entry_type matching
            log::debug!(
                "RegexMatch detection skipped (requires hash dict): field_pattern={}",
                field_name_pattern
            );
            let _ = (entry_type_pattern, value_pattern, data_type_pattern, path_pattern, context);
            false
        }

        DetectionRule::RegexMultiMatch { conditions, context } => {
            // Requires hash dictionary
            log::debug!(
                "RegexMultiMatch detection skipped (requires hash dict): {} conditions",
                conditions.len()
            );
            let _ = context;
            false
        }
    }
}

/// Search for a field along a dot-separated path (e.g., "SamplerValues.*.TextureName")
/// Supports wildcards (*) to iterate through containers/lists
fn search_field_path(
    properties: &indexmap::IndexMap<u32, BinProperty>,
    path_parts: &[&str],
    hash_dict: &HashDict,
) -> bool {
    if path_parts.is_empty() {
        return false;
    }

    let current_part = path_parts[0];
    let remaining = &path_parts[1..];

    // If this is the last part, check if the field exists
    if remaining.is_empty() {
        return properties.iter().any(|(name_hash, _)| {
            hash_dict.get_field(*name_hash)
                .map(|n| n.eq_ignore_ascii_case(current_part))
                .unwrap_or(false)
        });
    }

    // Find the current field and recurse into it
    for (name_hash, prop) in properties {
        let field_name = match hash_dict.get_field(*name_hash) {
            Some(n) => n,
            None => continue,
        };

        if !field_name.eq_ignore_ascii_case(current_part) {
            continue;
        }

        // Found the field, now check if remaining path exists inside it
        if search_field_in_value(&prop.value, remaining, hash_dict) {
            return true;
        }
    }

    false
}

/// Recursively search for a field path inside a property value
fn search_field_in_value(
    value: &PropertyValueEnum,
    path_parts: &[&str],
    hash_dict: &HashDict,
) -> bool {
    if path_parts.is_empty() {
        return false;
    }

    let current_part = path_parts[0];
    let remaining = &path_parts[1..];

    match value {
        // For containers, * means iterate through all items
        PropertyValueEnum::Container(c) => {
            if current_part == "*" {
                // Wildcard: check each item in the container
                for item in &c.items {
                    if remaining.is_empty() {
                        // If no more parts after *, we're looking for existence of any item
                        return true;
                    }
                    if search_field_in_value(item, remaining, hash_dict) {
                        return true;
                    }
                }
            }
            false
        }
        PropertyValueEnum::UnorderedContainer(c) => {
            if current_part == "*" {
                for item in &c.0.items {
                    if remaining.is_empty() {
                        return true;
                    }
                    if search_field_in_value(item, remaining, hash_dict) {
                        return true;
                    }
                }
            }
            false
        }
        PropertyValueEnum::Embedded(e) => {
            // Search inside embedded struct properties
            if current_part == "*" {
                // For embeds, * matches the embed itself - search its properties
                search_field_path(&e.0.properties, remaining, hash_dict)
            } else {
                // Check if the field name matches current_part
                // Then search remaining in that field's value
                search_field_path(&e.0.properties, path_parts, hash_dict)
            }
        }
        PropertyValueEnum::Struct(s) => {
            if current_part == "*" {
                search_field_path(&s.properties, remaining, hash_dict)
            } else {
                search_field_path(&s.properties, path_parts, hash_dict)
            }
        }
        PropertyValueEnum::Optional(o) => {
            if let Some(inner) = &o.value {
                search_field_in_value(inner, path_parts, hash_dict)
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Check if a property value matches an expected JSON value
fn value_matches(value: &PropertyValueEnum, expected: &serde_json::Value) -> bool {
    match (value, expected) {
        (PropertyValueEnum::U8(v), serde_json::Value::Number(n)) => {
            n.as_u64().map(|e| v.0 as u64 == e).unwrap_or(false)
        }
        (PropertyValueEnum::I8(v), serde_json::Value::Number(n)) => {
            n.as_i64().map(|e| v.0 as i64 == e).unwrap_or(false)
        }
        (PropertyValueEnum::U16(v), serde_json::Value::Number(n)) => {
            n.as_u64().map(|e| v.0 as u64 == e).unwrap_or(false)
        }
        (PropertyValueEnum::I16(v), serde_json::Value::Number(n)) => {
            n.as_i64().map(|e| v.0 as i64 == e).unwrap_or(false)
        }
        (PropertyValueEnum::U32(v), serde_json::Value::Number(n)) => {
            n.as_u64().map(|e| v.0 as u64 == e).unwrap_or(false)
        }
        (PropertyValueEnum::I32(v), serde_json::Value::Number(n)) => {
            n.as_i64().map(|e| v.0 as i64 == e).unwrap_or(false)
        }
        (PropertyValueEnum::Bool(v), serde_json::Value::Bool(b)) => v.0 == *b,
        _ => false, // Other types not supported for comparison yet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_result_serialization() {
        let result = ScanResult {
            file_path: "test.fantome".to_string(),
            detected_issues: vec![DetectedIssue {
                fix_id: "healthbar_fix".to_string(),
                fix_name: "Missing HP Bar".to_string(),
                severity: "high".to_string(),
                description: "Adds UnitHealthBarStyle field to fix missing HP bar".to_string(),
            }],
        };

        let json = serde_json::to_string(&result).expect("Failed to serialize");
        assert!(json.contains("healthbar_fix"));
        assert!(json.contains("Missing HP Bar"));
        assert!(json.contains("test.fantome"));

        // Verify we can deserialize back
        let parsed: ScanResult = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(parsed.file_path, "test.fantome");
        assert_eq!(parsed.detected_issues.len(), 1);
        assert_eq!(parsed.detected_issues[0].fix_id, "healthbar_fix");
    }

    #[test]
    fn test_detected_issue_clone() {
        let issue = DetectedIssue {
            fix_id: "test".to_string(),
            fix_name: "Test Fix".to_string(),
            severity: "low".to_string(),
            description: "Test description".to_string(),
        };

        let cloned = issue.clone();
        assert_eq!(issue.fix_id, cloned.fix_id);
    }
}
