//! Schema definitions for the fix configuration system.
//!
//! This module defines the Rust types that represent the JSON configuration
//! fetched from GitHub. Supports three levels of detection:
//! - **Simple/Rigid**: Exact field matching (fast, clear)
//! - **Smart/Pattern**: Pattern matching with champion context
//! - **Lazy/Regex**: Full regex for maximum flexibility

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root configuration containing all fix rules.
/// This is fetched from GitHub and cached locally.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixConfig {
    /// Schema version (semver)
    pub version: String,
    /// ISO date of last config update
    pub last_updated: String,
    /// Map of fix_id -> FixRule
    pub fixes: HashMap<String, FixRule>,
}

/// A single fix rule with detection and transformation logic.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixRule {
    /// Human-readable name for display
    pub name: String,
    /// Description of what this fix does
    pub description: String,
    /// Whether this fix is enabled
    pub enabled: bool,
    /// Severity level for UI display
    pub severity: Severity,
    /// How to detect if this fix is needed
    pub detect: DetectionRule,
    /// What transformation to apply
    pub apply: TransformAction,
}

/// Severity levels for display purposes.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

// =============================================================================
// DETECTION RULES
// =============================================================================

/// Detection rules to check if a file needs fixing.
/// Uses serde's internally tagged enum representation.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum DetectionRule {
    // =========================================================================
    // SIMPLE/RIGID DETECTION (Original types - fast and exact)
    // =========================================================================

    /// Check if field is missing or has wrong value in an embedded structure.
    /// Used by: healthbar_fix
    #[serde(rename = "missing_or_wrong_field")]
    MissingOrWrongField {
        /// Entry type to search for (e.g., "SkinCharacterDataProperties")
        entry_type: String,
        /// Optional path to embedded field (e.g., "HealthBarData")
        #[serde(skip_serializing_if = "Option::is_none")]
        embed_path: Option<String>,
        /// Optional type of embedded structure
        #[serde(skip_serializing_if = "Option::is_none")]
        embed_type: Option<String>,
        /// Field name to check
        field: String,
        /// Expected value (if present and different, needs fix)
        #[serde(skip_serializing_if = "Option::is_none")]
        expected_value: Option<serde_json::Value>,
    },

    /// Check if a field hash exists (when it shouldn't).
    /// Used by: staticmat_texturepath, staticmat_samplername
    #[serde(rename = "field_hash_exists")]
    FieldHashExists {
        /// Entry type to search for (e.g., "StaticMaterialDef")
        entry_type: String,
        /// Dot-separated path with wildcards (e.g., "SamplerValues.*.TextureName")
        path: String,
    },

    /// Check if string field ends with extension and file NOT in WAD.
    /// Used by: black_icons
    #[serde(rename = "string_extension_not_in_wad")]
    StringExtensionNotInWad {
        /// Entry type to search for
        entry_type: String,
        /// Field names to check
        fields: Vec<String>,
        /// Extension to look for (e.g., ".dds")
        extension: String,
    },

    /// Recursively search all strings for extension not in WAD.
    /// Used by: dds_to_tex
    #[serde(rename = "recursive_string_extension_not_in_wad")]
    RecursiveStringExtensionNotInWad {
        /// Extension to search for
        extension: String,
        /// Only convert paths starting with these prefixes
        path_prefixes: Vec<String>,
    },

    /// Check if BIN contains any of these entry types.
    /// Used by: champion_bin_remover
    #[serde(rename = "entry_type_exists_any")]
    EntryTypeExistsAny {
        /// List of entry type names that indicate champion data
        entry_types: Vec<String>,
    },

    /// Check BNK file version against allowed list.
    /// Used by: bnk_remover
    #[serde(rename = "bnk_version_not_in")]
    BnkVersionNotIn {
        /// Versions that are allowed (others will be removed)
        allowed_versions: Vec<u32>,
    },

    // =========================================================================
    // SMART/PATTERN DETECTION (Context-aware with champion filtering)
    // =========================================================================

    /// Smart pattern-based detection with champion context.
    /// Use for fixes that depend on whether it's a champion, subchamp, or specific character.
    #[serde(rename = "pattern_match")]
    PatternMatch {
        /// Entry type to search (exact match or wildcard with *)
        entry_type: String,

        /// Field name to check (exact match or wildcard: "*ResourceResolver*")
        field_name: String,

        /// Expected data type (e.g., "link", "string", "u32")
        #[serde(skip_serializing_if = "Option::is_none")]
        expected_data_type: Option<String>,

        /// Wrong data type (what we're detecting as broken)
        #[serde(skip_serializing_if = "Option::is_none")]
        wrong_data_type: Option<String>,

        /// Context filter: "champion", "subchamp", or "all"
        #[serde(default = "default_context")]
        context: String,

        /// Specific champions/subchamps to target (empty = all matching context)
        #[serde(default)]
        champion_filter: Vec<String>,
    },

    // =========================================================================
    // LAZY/REGEX DETECTION (Maximum flexibility for unknown fixes)
    // =========================================================================

    /// Regex-based field detection for maximum flexibility.
    /// Use when you need to match patterns you don't fully know yet.
    #[serde(rename = "regex_match")]
    RegexMatch {
        /// Entry type pattern (regex, e.g., "Skin.*Properties")
        #[serde(skip_serializing_if = "Option::is_none")]
        entry_type_pattern: Option<String>,

        /// Field name pattern (regex, e.g., ".*Resolver.*")
        field_name_pattern: String,

        /// Field value pattern (regex, optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        value_pattern: Option<String>,

        /// Data type pattern (regex, e.g., "link|pointer")
        #[serde(skip_serializing_if = "Option::is_none")]
        data_type_pattern: Option<String>,

        /// Entry path pattern (regex, e.g., "Characters/.*/Skins/.*")
        #[serde(skip_serializing_if = "Option::is_none")]
        path_pattern: Option<String>,

        /// Champion context filter (optional, uses champion_list)
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },

    /// Multi-condition regex for complex detection scenarios.
    /// All conditions must match (AND logic).
    #[serde(rename = "regex_multi_match")]
    RegexMultiMatch {
        /// All conditions must match (AND logic)
        conditions: Vec<RegexCondition>,

        /// Champion context filter
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },
}

/// A single regex condition for multi-match detection.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegexCondition {
    /// What to check: "field_name", "field_value", "data_type", "entry_type", "path"
    pub target: String,

    /// Regex pattern to match
    pub pattern: String,

    /// Invert match (NOT logic) - true means "must NOT match"
    #[serde(default)]
    pub invert: bool,
}

fn default_context() -> String {
    "all".to_string()
}

// =============================================================================
// TRANSFORM ACTIONS
// =============================================================================

/// Transformation actions to apply when fixing files.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum TransformAction {
    // =========================================================================
    // SIMPLE TRANSFORMS (Original types)
    // =========================================================================

    /// Add field if missing, or set value if exists.
    /// Used by: healthbar_fix
    #[serde(rename = "ensure_field")]
    EnsureField {
        /// Field name to add/set
        field: String,
        /// Value to set
        value: serde_json::Value,
        /// Data type for the field
        data_type: BinDataType,
        /// If field's parent doesn't exist, create it
        #[serde(skip_serializing_if = "Option::is_none")]
        create_parent: Option<ParentEmbed>,
        /// Only apply to objects of this entry type
        #[serde(skip_serializing_if = "Option::is_none")]
        entry_type: Option<String>,
    },

    /// Rename a field's hash (key name).
    /// Used by: staticmat_texturepath, staticmat_samplername
    #[serde(rename = "rename_hash")]
    RenameHash {
        /// Source field name
        from_hash: String,
        /// Target field name
        to_hash: String,
    },

    /// Replace file extension in string values.
    /// Used by: black_icons, dds_to_tex
    #[serde(rename = "replace_string_extension")]
    ReplaceStringExtension {
        /// Extension to find (e.g., ".dds")
        from: String,
        /// Extension to replace with (e.g., ".tex")
        to: String,
    },

    /// Remove the entire file from WAD archive.
    /// Used by: champion_bin_remover, bnk_remover
    #[serde(rename = "remove_from_wad")]
    RemoveFromWad,

    // =========================================================================
    // SMART TRANSFORMS (Context-aware)
    // =========================================================================

    /// Change a field's data type (e.g., link -> string, vec3 -> vec4).
    #[serde(rename = "change_field_type")]
    ChangeFieldType {
        /// Field name to change (exact or pattern with *)
        field_name: String,
        /// Source type being changed from
        from_type: String,
        /// Target type to change to
        to_type: String,
        /// How to handle the value: "keep_value", "set_empty_string", "convert"
        #[serde(default = "default_transform_rule")]
        transform_rule: String,
        /// For vector expansion (vec3->vec4): values to append
        #[serde(default)]
        append_values: Vec<serde_json::Value>,
    },

    /// Add/set field with champion-specific values.
    /// Different champions/subchamps can have different values.
    #[serde(rename = "ensure_field_with_context")]
    EnsureFieldWithContext {
        /// Field name to add/set
        field: String,
        /// Data type for the field
        data_type: BinDataType,
        /// Contextual values (default + champion/subchamp overrides)
        values: ContextualValues,
    },

    // =========================================================================
    // REGEX TRANSFORMS (Maximum flexibility)
    // =========================================================================

    /// Regex-based field value replacement.
    /// Supports capture groups ($1, $2, etc.) in replacement.
    #[serde(rename = "regex_replace")]
    RegexReplace {
        /// Field name pattern to target (regex)
        field_pattern: String,
        /// Value pattern to find (regex)
        find_pattern: String,
        /// Replacement pattern (supports capture groups: $1, $2)
        replace_pattern: String,
    },

    /// Regex-based field rename.
    /// Supports capture groups for dynamic renaming.
    #[serde(rename = "regex_rename_field")]
    RegexRenameField {
        /// Field name pattern to match (regex)
        field_pattern: String,
        /// New name pattern (supports capture groups)
        new_name_pattern: String,
    },
}

/// Contextual values for champion/subchamp-specific fixes.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextualValues {
    /// Default value used when no specific override exists
    pub default: serde_json::Value,

    /// Champion-specific overrides (champion name -> value)
    #[serde(default)]
    pub champions: HashMap<String, serde_json::Value>,

    /// Subchamp-specific overrides (subchamp name -> value)
    #[serde(default)]
    pub subchamps: HashMap<String, serde_json::Value>,
}

fn default_transform_rule() -> String {
    "keep_value".to_string()
}

// =============================================================================
// SUPPORTING TYPES
// =============================================================================

/// Parent embed configuration for creating nested structures.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParentEmbed {
    /// Field name for the parent embed
    pub field: String,
    /// Type name for the parent embed
    #[serde(rename = "type")]
    pub embed_type: String,
}

/// BIN file data types supported by the config system.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BinDataType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    String,
    Bool,
    Vec2,
    Vec3,
    Vec4,
    Hash,
    Link,
    Pointer,
    Embed,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_healthbar_fix() {
        let json = r#"{
            "name": "Missing HP Bar",
            "description": "Adds UnitHealthBarStyle field",
            "enabled": true,
            "severity": "high",
            "detect": {
                "type": "missing_or_wrong_field",
                "entry_type": "SkinCharacterDataProperties",
                "field": "UnitHealthBarStyle",
                "expected_value": 12
            },
            "apply": {
                "type": "ensure_field",
                "field": "UnitHealthBarStyle",
                "value": 12,
                "data_type": "u8"
            }
        }"#;

        let rule: FixRule = serde_json::from_str(json).expect("Failed to parse FixRule");
        assert_eq!(rule.name, "Missing HP Bar");
        assert!(rule.enabled);
        assert_eq!(rule.severity, Severity::High);
    }

    #[test]
    fn test_deserialize_rename_hash() {
        let json = r#"{
            "name": "White Model",
            "description": "Renames TextureName to TexturePath",
            "enabled": true,
            "severity": "critical",
            "detect": {
                "type": "field_hash_exists",
                "entry_type": "StaticMaterialDef",
                "path": "SamplerValues.*.TextureName"
            },
            "apply": {
                "type": "rename_hash",
                "from_hash": "TextureName",
                "to_hash": "TexturePath"
            }
        }"#;

        let rule: FixRule = serde_json::from_str(json).expect("Failed to parse FixRule");
        assert_eq!(rule.severity, Severity::Critical);
    }

    #[test]
    fn test_deserialize_pattern_match() {
        let json = r#"{
            "name": "Resource Resolver Type Fix",
            "description": "Changes mResourceResolver from link to string in subchamp skins",
            "enabled": true,
            "severity": "medium",
            "detect": {
                "type": "pattern_match",
                "entry_type": "SkinCharacterDataProperties",
                "field_name": "mResourceResolver",
                "wrong_data_type": "link",
                "expected_data_type": "string",
                "context": "subchamp",
                "champion_filter": []
            },
            "apply": {
                "type": "change_field_type",
                "field_name": "mResourceResolver",
                "from_type": "link",
                "to_type": "string",
                "transform_rule": "keep_value"
            }
        }"#;

        let rule: FixRule = serde_json::from_str(json).expect("Failed to parse pattern_match");
        assert_eq!(rule.name, "Resource Resolver Type Fix");
        
        match rule.detect {
            DetectionRule::PatternMatch { context, .. } => {
                assert_eq!(context, "subchamp");
            }
            _ => panic!("Expected PatternMatch detection"),
        }
    }

    #[test]
    fn test_deserialize_regex_match() {
        let json = r#"{
            "name": "Texture Path Format Fix",
            "description": "Fixes any texture path with wrong format",
            "enabled": true,
            "severity": "low",
            "detect": {
                "type": "regex_match",
                "field_name_pattern": ".*[Tt]exture.*",
                "value_pattern": ".*\\.dds$",
                "data_type_pattern": "string"
            },
            "apply": {
                "type": "regex_replace",
                "field_pattern": ".*[Tt]exture.*",
                "find_pattern": "\\.dds$",
                "replace_pattern": ".tex"
            }
        }"#;

        let rule: FixRule = serde_json::from_str(json).expect("Failed to parse regex_match");
        
        match rule.detect {
            DetectionRule::RegexMatch { field_name_pattern, .. } => {
                assert_eq!(field_name_pattern, ".*[Tt]exture.*");
            }
            _ => panic!("Expected RegexMatch detection"),
        }
    }

    #[test]
    fn test_deserialize_regex_multi_match() {
        let json = r#"{
            "name": "Complex VFX Fix",
            "description": "Fixes VFX paths in particle entries",
            "enabled": true,
            "severity": "medium",
            "detect": {
                "type": "regex_multi_match",
                "conditions": [
                    {"target": "entry_type", "pattern": ".*VfxSystem.*", "invert": false},
                    {"target": "field_name", "pattern": ".*[Pp]article.*", "invert": false}
                ],
                "context": "champion"
            },
            "apply": {
                "type": "regex_replace",
                "field_pattern": ".*[Pp]article.*",
                "find_pattern": "^(.*)$",
                "replace_pattern": "assets/particles/$1"
            }
        }"#;

        let rule: FixRule = serde_json::from_str(json).expect("Failed to parse regex_multi_match");
        
        match rule.detect {
            DetectionRule::RegexMultiMatch { conditions, context } => {
                assert_eq!(conditions.len(), 2);
                assert_eq!(context, Some("champion".to_string()));
            }
            _ => panic!("Expected RegexMultiMatch detection"),
        }
    }

    #[test]
    fn test_deserialize_contextual_values() {
        let json = r#"{
            "name": "Champion-Specific HP Bar",
            "description": "Adds HP bar with champion/subchamp-specific values",
            "enabled": true,
            "severity": "high",
            "detect": {
                "type": "pattern_match",
                "entry_type": "SkinCharacterDataProperties",
                "field_name": "UnitHealthBarStyle",
                "expected_data_type": "u8",
                "context": "all"
            },
            "apply": {
                "type": "ensure_field_with_context",
                "field": "UnitHealthBarStyle",
                "data_type": "u8",
                "values": {
                    "default": 12,
                    "champions": {"Jhin": 12, "Zed": 12},
                    "subchamps": {"ZedShadow": 5, "NaafiriPackmate": 3}
                }
            }
        }"#;

        let rule: FixRule = serde_json::from_str(json).expect("Failed to parse contextual values");
        
        match rule.apply {
            TransformAction::EnsureFieldWithContext { values, .. } => {
                assert_eq!(values.default, serde_json::json!(12));
                assert_eq!(values.subchamps.get("ZedShadow"), Some(&serde_json::json!(5)));
            }
            _ => panic!("Expected EnsureFieldWithContext transform"),
        }
    }

    #[test]
    fn test_deserialize_change_field_type() {
        let json = r#"{
            "name": "Vec3 to Vec4 Color",
            "description": "Converts color fields from vec3 to vec4",
            "enabled": true,
            "severity": "medium",
            "detect": {
                "type": "regex_match",
                "field_name_pattern": ".*[Cc]olor.*",
                "data_type_pattern": "vec3"
            },
            "apply": {
                "type": "change_field_type",
                "field_name": "*color*",
                "from_type": "vec3",
                "to_type": "vec4",
                "transform_rule": "convert",
                "append_values": [1.0]
            }
        }"#;

        let rule: FixRule = serde_json::from_str(json).expect("Failed to parse change_field_type");
        
        match rule.apply {
            TransformAction::ChangeFieldType { from_type, to_type, append_values, .. } => {
                assert_eq!(from_type, "vec3");
                assert_eq!(to_type, "vec4");
                assert_eq!(append_values.len(), 1);
            }
            _ => panic!("Expected ChangeFieldType transform"),
        }
    }
}
