//! BIN Parser Module
//!
//! Provides wrappers and helpers for parsing League BIN files using ltk_meta.
//! This module bridges league-toolkit's BinTree API with our detection needs.

use anyhow::{Context, Result};
use std::fs::File;
use std::io::Cursor;
use std::path::Path;

// Re-export ltk_meta types for convenience
pub use ltk_meta::{
    BinTree, BinTreeObject, BinProperty, PropertyValueEnum, BinPropertyKind,
};

/// Load and parse a BIN file from disk
pub fn parse_bin_file<P: AsRef<Path>>(path: P) -> Result<BinTree> {
    let mut file = File::open(path.as_ref())
        .with_context(|| format!("Failed to open BIN: {:?}", path.as_ref()))?;
    
    BinTree::from_reader(&mut file)
        .map_err(|e| anyhow::anyhow!("Failed to parse BIN: {:?}", e))
}

/// Parse BIN from bytes (for files extracted from WAD)
pub fn parse_bin_bytes(data: &[u8]) -> Result<BinTree> {
    let mut cursor = Cursor::new(data);
    BinTree::from_reader(&mut cursor)
        .map_err(|e| anyhow::anyhow!("Failed to parse BIN from bytes: {:?}", e))
}

/// Write a BIN tree back to a file
pub fn write_bin_file<P: AsRef<Path>>(tree: &BinTree, path: P) -> Result<()> {
    use std::io::BufWriter;
    
    let file = File::create(path.as_ref())
        .with_context(|| format!("Failed to create BIN file: {:?}", path.as_ref()))?;
    let mut writer = BufWriter::new(file);
    
    tree.to_writer(&mut writer)
        .with_context(|| format!("Failed to write BIN: {:?}", path.as_ref()))?;
    
    Ok(())
}

/// Write a BIN tree to bytes (for embedding in WAD)
pub fn write_bin_bytes(tree: &BinTree) -> Result<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    tree.to_writer(&mut cursor)
        .map_err(|e| anyhow::anyhow!("Failed to write BIN to bytes: {:?}", e))?;
    Ok(cursor.into_inner())
}

/// Extract all string values from a BIN tree recursively.
/// Useful for dds/tex path detection.
pub fn extract_all_strings(tree: &BinTree) -> Vec<String> {
    let mut strings = Vec::new();
    for (_hash, obj) in &tree.objects {
        extract_strings_from_object(obj, &mut strings);
    }
    strings
}

/// Extract strings from a single BIN object
fn extract_strings_from_object(obj: &BinTreeObject, strings: &mut Vec<String>) {
    for (_hash, prop) in &obj.properties {
        extract_strings_from_value(&prop.value, strings);
    }
}

/// Recursively extract strings from a property value
fn extract_strings_from_value(value: &PropertyValueEnum, strings: &mut Vec<String>) {
    use ltk_meta::value::*;
    
    match value {
        PropertyValueEnum::String(s) => {
            strings.push(s.0.clone());
        }
        PropertyValueEnum::Container(c) => {
            for item in &c.items {
                extract_strings_from_value(item, strings);
            }
        }
        PropertyValueEnum::UnorderedContainer(c) => {
            // UnorderedContainerValue wraps ContainerValue via .0
            for item in &c.0.items {
                extract_strings_from_value(item, strings);
            }
        }
        PropertyValueEnum::Embedded(e) => {
            // EmbeddedValue wraps StructValue via .0
            for (_h, p) in &e.0.properties {
                extract_strings_from_value(&p.value, strings);
            }
        }
        PropertyValueEnum::Struct(s) => {
            for (_h, p) in &s.properties {
                extract_strings_from_value(&p.value, strings);
            }
        }
        PropertyValueEnum::Optional(o) => {
            if let Some(inner) = &o.value {
                extract_strings_from_value(inner, strings);
            }
        }
        PropertyValueEnum::Map(m) => {
            // MapValue uses PropertyValueUnsafeEq for keys
            for (k, v) in &m.entries {
                // k is PropertyValueUnsafeEq, access inner value via .0
                extract_strings_from_value(&k.0, strings);
                extract_strings_from_value(v, strings);
            }
        }
        // Primitive types that aren't strings - skip
        _ => {}
    }
}

/// Check if a BIN tree contains any object with a specific class hash
pub fn has_class_hash(tree: &BinTree, class_hash: u32) -> bool {
    tree.objects.values().any(|obj| obj.class_hash == class_hash)
}

/// Find all objects with a specific class hash
pub fn find_objects_by_class(tree: &BinTree, class_hash: u32) -> Vec<&BinTreeObject> {
    tree.objects
        .values()
        .filter(|obj| obj.class_hash == class_hash)
        .collect()
}

/// Check if an object has a property with a specific name hash
pub fn has_property(obj: &BinTreeObject, name_hash: u32) -> bool {
    obj.properties.contains_key(&name_hash)
}

/// Get a property value as a u8 (for things like UnitHealthBarStyle)
pub fn get_u8_property(obj: &BinTreeObject, name_hash: u32) -> Option<u8> {
    use ltk_meta::value::U8Value;
    
    obj.properties.get(&name_hash).and_then(|prop| {
        match &prop.value {
            PropertyValueEnum::U8(U8Value(v)) => Some(*v),
            _ => None,
        }
    })
}

/// Get a property value as a string
pub fn get_string_property(obj: &BinTreeObject, name_hash: u32) -> Option<String> {
    use ltk_meta::value::StringValue;
    
    obj.properties.get(&name_hash).and_then(|prop| {
        match &prop.value {
            PropertyValueEnum::String(StringValue(s)) => Some(s.clone()),
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bin_tree_creation() {
        // Test that we can create an empty BinTree
        let tree = BinTree::default();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
    }

    #[test]
    fn test_find_objects_empty() {
        let tree = BinTree::default();
        let objects = find_objects_by_class(&tree, 0x12345678);
        assert!(objects.is_empty());
    }

    #[test]
    fn test_has_class_hash_false() {
        let tree = BinTree::default();
        assert!(!has_class_hash(&tree, 0x12345678));
    }
    
    #[test]
    fn test_extract_strings_empty() {
        let tree = BinTree::default();
        let strings = extract_all_strings(&tree);
        assert!(strings.is_empty());
    }
}
