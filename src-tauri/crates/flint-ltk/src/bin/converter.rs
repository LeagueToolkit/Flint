//! Bin converter for converting between binary, text, and JSON formats
//!
//! This module provides functionality to convert League of Legends .bin files
//! between different formats using ltk_meta and ltk_ritobin.

use crate::bin::ltk_bridge::{tree_to_text_cached, text_to_tree};
use crate::error::{Error, Result};
use ltk_meta::Bin;

// Helper function to create BinConversion errors
fn bin_error(message: impl Into<String>) -> Error {
    Error::BinConversion {
        message: message.into(),
        path: None,
    }
}

/// Convert a Bin to Python-like text format
///
/// Uses cached hash provider for resolving hashes to readable names
pub fn bin_to_text(tree: &Bin) -> Result<String> {
    tree_to_text_cached(tree)
        .map_err(|e| bin_error(format!("Failed to convert to text: {}", e)))
}

/// Convert Python-like text format to Bin
pub fn text_to_bin(text: &str) -> Result<Bin> {
    text_to_tree(text)
        .map_err(|e| bin_error(format!("Failed to parse text: {}", e)))
}

/// Convert a Bin to JSON format
pub fn bin_to_json(tree: &Bin) -> Result<String> {
    serde_json::to_string_pretty(tree)
        .map_err(|e| bin_error(format!("JSON serialization failed: {}", e)))
}

/// Convert JSON format to a Bin
pub fn json_to_bin(json: &str) -> Result<Bin> {
    serde_json::from_str(json)
        .map_err(|e| bin_error(format!("JSON parse error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_roundtrip() {
        // Create a simple Bin
        let tree = Bin::new(std::iter::empty::<ltk_meta::BinObject>(), std::iter::empty::<String>());
        
        // Convert to JSON
        let json = bin_to_json(&tree).unwrap();
        
        // Convert back
        let tree2 = json_to_bin(&json).unwrap();
        
        assert_eq!(tree.objects.len(), tree2.objects.len());
    }
}
