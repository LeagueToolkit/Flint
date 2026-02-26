//! Hash Dictionary Module
//!
//! Loads shared RitoShark hash files from `%APPDATA%\RitoShark\Requirements\Hashes\`.
//! These are downloaded by `ritoshark-hash-service.exe` and shared across all RitoShark tools.
//!
//! Hash files map numeric hashes to human-readable names:
//! - `hashes.bintypes.txt` - class_hash → type name (e.g., "SkinCharacterDataProperties")
//! - `hashes.binfields.txt` - name_hash → field name (e.g., "UnitHealthBarStyle")
//! - `hashes.binentries.txt` - path_hash → entry path
//! - `hashes.game.txt` - xxhash64 → asset path (for WAD file lookups)

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use anyhow::{Context, Result};

/// Hash dictionary for resolving League file hashes to names.
///
/// Loaded from shared RitoShark hash files in `%APPDATA%\RitoShark\Requirements\Hashes\`.
#[derive(Debug, Default)]
pub struct HashDict {
    /// class_hash (u32) -> type name (e.g., "SkinCharacterDataProperties")
    pub types: HashMap<u32, String>,
    
    /// name_hash (u32) -> field name (e.g., "UnitHealthBarStyle")
    pub fields: HashMap<u32, String>,
    
    /// path_hash (u32) -> entry path
    pub entries: HashMap<u32, String>,
    
    /// xxhash64 (u64) -> asset path (for WAD lookups)
    pub game: HashMap<u64, String>,
}

impl HashDict {
    /// Create an empty hash dictionary
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the RitoShark hash directory path
    pub fn get_hash_dir() -> Result<PathBuf> {
        let appdata = std::env::var("APPDATA")
            .context("APPDATA environment variable not set")?;
        Ok(PathBuf::from(appdata)
            .join("RitoShark")
            .join("Requirements")
            .join("Hashes"))
    }

    /// Load hash dictionary from RitoShark shared directory.
    /// Returns Ok with empty dicts if files don't exist (graceful fallback).
    pub fn load() -> Result<Self> {
        let hash_dir = Self::get_hash_dir()?;
        
        if !hash_dir.exists() {
            log::warn!(
                "Hash directory does not exist: {}. Run ritoshark-hash-service.exe to download.",
                hash_dir.display()
            );
            return Ok(Self::new());
        }

        let mut dict = Self::new();

        // Load bin types (class_hash -> type name)
        let types_file = hash_dir.join("hashes.bintypes.txt");
        if types_file.exists() {
            dict.types = load_u32_hash_file(&types_file)
                .context("Failed to load bintypes")?;
            log::info!("Loaded {} bin type hashes", dict.types.len());
        }

        // Load bin fields (name_hash -> field name)
        let fields_file = hash_dir.join("hashes.binfields.txt");
        if fields_file.exists() {
            dict.fields = load_u32_hash_file(&fields_file)
                .context("Failed to load binfields")?;
            log::info!("Loaded {} bin field hashes", dict.fields.len());
        }

        // Load bin entries (path_hash -> entry path)
        let entries_file = hash_dir.join("hashes.binentries.txt");
        if entries_file.exists() {
            dict.entries = load_u32_hash_file(&entries_file)
                .context("Failed to load binentries")?;
            log::info!("Loaded {} bin entry hashes", dict.entries.len());
        }

        // Load game hashes (xxhash64 -> asset path)
        let game_file = hash_dir.join("hashes.game.txt");
        if game_file.exists() {
            dict.game = load_u64_hash_file(&game_file)
                .context("Failed to load game hashes")?;
            log::info!("Loaded {} game asset hashes", dict.game.len());
        }

        Ok(dict)
    }

    /// Check if the hash dictionary is loaded (has any hashes)
    pub fn is_loaded(&self) -> bool {
        !self.types.is_empty() || !self.fields.is_empty()
    }

    /// Lookup type name from class_hash
    pub fn get_type(&self, hash: u32) -> Option<&str> {
        self.types.get(&hash).map(|s| s.as_str())
    }

    /// Lookup field name from name_hash
    pub fn get_field(&self, hash: u32) -> Option<&str> {
        self.fields.get(&hash).map(|s| s.as_str())
    }

    /// Lookup entry path from path_hash
    pub fn get_entry(&self, hash: u32) -> Option<&str> {
        self.entries.get(&hash).map(|s| s.as_str())
    }

    /// Lookup asset path from xxhash64
    pub fn get_game_path(&self, hash: u64) -> Option<&str> {
        self.game.get(&hash).map(|s| s.as_str())
    }

    /// Get total number of loaded hashes
    pub fn total_count(&self) -> usize {
        self.types.len() + self.fields.len() + self.entries.len() + self.game.len()
    }
}

/// Load a hash file with u32 hashes (format: "<hex_hash> <name>")
fn load_u32_hash_file(path: &PathBuf) -> Result<HashMap<u32, String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut map = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Format: "<hex_hash> <name>" or "<hex_hash>\t<name>"
        if let Some((hash_str, name)) = line.split_once(|c| c == ' ' || c == '\t') {
            let hash_str = hash_str.trim_start_matches("0x");
            if let Ok(hash) = u32::from_str_radix(hash_str, 16) {
                map.insert(hash, name.to_string());
            }
        }
    }

    Ok(map)
}

/// Load a hash file with u64 hashes (format: "<hex_hash> <name>")
fn load_u64_hash_file(path: &PathBuf) -> Result<HashMap<u64, String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut map = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Format: "<hex_hash> <name>" or "<hex_hash>\t<name>"
        if let Some((hash_str, name)) = line.split_once(|c| c == ' ' || c == '\t') {
            let hash_str = hash_str.trim_start_matches("0x");
            if let Ok(hash) = u64::from_str_radix(hash_str, 16) {
                map.insert(hash, name.to_string());
            }
        }
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_dict_new() {
        let dict = HashDict::new();
        assert!(!dict.is_loaded());
        assert_eq!(dict.total_count(), 0);
    }

    #[test]
    fn test_get_hash_dir() {
        // Should not panic if APPDATA exists
        if std::env::var("APPDATA").is_ok() {
            let dir = HashDict::get_hash_dir();
            assert!(dir.is_ok());
            let path = dir.unwrap();
            assert!(path.ends_with("RitoShark\\Requirements\\Hashes"));
        }
    }

    #[test]
    fn test_hash_lookup() {
        let mut dict = HashDict::new();
        dict.types.insert(0x12345678, "TestType".to_string());
        dict.fields.insert(0xABCDEF00, "TestField".to_string());

        assert_eq!(dict.get_type(0x12345678), Some("TestType"));
        assert_eq!(dict.get_field(0xABCDEF00), Some("TestField"));
        assert_eq!(dict.get_type(0x00000000), None);
        assert!(dict.is_loaded());
    }
}
