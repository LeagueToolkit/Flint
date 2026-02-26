//! WAD Cache Module
//!
//! Provides an in-memory index of file paths/hashes from WAD files.
//! Uses league-toolkit's ltk_wad for parsing.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use anyhow::{Context, Result};
use league_toolkit::wad::Wad;

/// In-memory index of all file paths in a WAD.
/// 
/// WAD files store paths as xxhash64 hashes. Without a hash dictionary,
/// we can only store and compare hashes directly. For path-based lookups,
/// we hash the input path and check against stored hashes.
#[derive(Debug, Clone)]
pub struct WadCache {
    /// Set of all path hashes (as u64) in the WAD
    path_hashes: HashSet<u64>,

    /// Set of known file paths (lowercase, normalized) - populated when paths are known
    known_paths: HashSet<String>,
}

impl WadCache {
    /// Create empty cache
    pub fn new() -> Self {
        Self {
            path_hashes: HashSet::new(),
            known_paths: HashSet::new(),
        }
    }

    /// Build cache from a WAD file using league-toolkit
    pub fn from_wad<P: AsRef<Path>>(wad_path: P) -> Result<Self> {
        let file = File::open(wad_path.as_ref())
            .with_context(|| format!("Failed to open WAD: {:?}", wad_path.as_ref()))?;
        let reader = BufReader::new(file);
        
        Self::from_reader(reader)
    }

    /// Build cache from bytes (for WAD embedded in ZIP/Fantome)
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let cursor = Cursor::new(data);
        Self::from_reader(cursor)
    }

    /// Internal: Build cache from any Read+Seek source
    fn from_reader<R: Read + std::io::Seek>(reader: R) -> Result<Self> {
        let wad = Wad::mount(reader)
            .map_err(|e| anyhow::anyhow!("Failed to parse WAD: {:?}", e))?;
        
        let mut cache = Self::new();
        
        // Store all path hashes from the WAD
        for chunk in wad.chunks() {
            cache.path_hashes.insert(chunk.path_hash());
        }
        
        log::debug!("Loaded {} chunks from WAD", cache.path_hashes.len());
        Ok(cache)
    }

    /// Check if a file with this path hash (u64) exists
    pub fn has_hash(&self, hash: u64) -> bool {
        self.path_hashes.contains(&hash)
    }

    /// Check if a file with this path hash (hex string) exists
    pub fn has_file_hash(&self, hash_str: &str) -> bool {
        if let Ok(hash) = u64::from_str_radix(hash_str.trim_start_matches("0x"), 16) {
            self.path_hashes.contains(&hash)
        } else {
            false
        }
    }

    /// Check if a file with this path exists
    /// 
    /// This hashes the path using xxhash64 and checks against stored hashes.
    /// Note: Requires the xxhash-rust crate with xxh64 feature.
    pub fn has_file_path(&self, path: &str) -> bool {
        // First check known paths (if we have a hash dictionary)
        let normalized = path.to_lowercase().replace('\\', "/");
        if self.known_paths.contains(&normalized) {
            return true;
        }
        
        // Hash the path and check against WAD hashes
        let hash = xxhash_path(&normalized);
        self.path_hashes.contains(&hash)
    }

    /// Check if a .dds file exists (used by black_icons fix)
    pub fn has_dds_file(&self, path: &str) -> bool {
        if !path.to_lowercase().ends_with(".dds") {
            return false;
        }
        self.has_file_path(path)
    }

    /// Add a known path (when we have path resolution)
    pub fn add_known_path(&mut self, path: String) {
        let normalized = path.to_lowercase().replace('\\', "/");
        let hash = xxhash_path(&normalized);
        self.path_hashes.insert(hash);
        self.known_paths.insert(normalized);
    }

    /// Add a file hash directly
    pub fn add_hash(&mut self, hash: u64) {
        self.path_hashes.insert(hash);
    }

    /// Get total hash count
    pub fn hash_count(&self) -> usize {
        self.path_hashes.len()
    }

    /// Get known paths count
    pub fn known_path_count(&self) -> usize {
        self.known_paths.len()
    }

    /// Merge another cache into this one
    pub fn merge(&mut self, other: &WadCache) {
        self.path_hashes.extend(&other.path_hashes);
        self.known_paths.extend(other.known_paths.iter().cloned());
    }
}

impl Default for WadCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a file path using xxhash64 (League's path hashing algorithm)
fn xxhash_path(path: &str) -> u64 {
    use xxhash_rust::xxh64::xxh64;
    // League uses lowercase paths with forward slashes
    let normalized = path.to_lowercase().replace('\\', "/");
    xxh64(normalized.as_bytes(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wad_cache_basic() {
        let mut cache = WadCache::new();
        cache.add_known_path("assets/test.dds".to_string());

        assert!(cache.has_file_path("assets/test.dds"));
        assert!(cache.has_file_path("ASSETS/TEST.DDS")); // Case-insensitive
        assert!(cache.has_dds_file("assets/test.dds"));
        assert_eq!(cache.hash_count(), 1);
        assert_eq!(cache.known_path_count(), 1);
    }

    #[test]
    fn test_wad_cache_path_normalization() {
        let mut cache = WadCache::new();
        cache.add_known_path("assets\\textures\\test.dds".to_string());

        // Should normalize backslashes to forward slashes
        assert!(cache.has_file_path("assets/textures/test.dds"));
        assert!(cache.has_file_path("ASSETS/TEXTURES/TEST.DDS"));
    }

    #[test]
    fn test_wad_cache_dds_check() {
        let mut cache = WadCache::new();
        cache.add_known_path("test.dds".to_string());
        cache.add_known_path("test.tex".to_string());

        assert!(cache.has_dds_file("test.dds"));
        assert!(!cache.has_dds_file("test.tex")); // Not a .dds file
        assert!(!cache.has_dds_file("missing.dds")); // Doesn't exist
    }

    #[test]
    fn test_wad_cache_hash_lookup() {
        let mut cache = WadCache::new();
        cache.add_hash(0x1234567890abcdef);

        assert!(cache.has_hash(0x1234567890abcdef));
        assert!(cache.has_file_hash("1234567890abcdef"));
        assert!(cache.has_file_hash("0x1234567890abcdef"));
        assert!(!cache.has_hash(0xdeadbeef));
    }

    #[test]
    fn test_wad_cache_merge() {
        let mut cache1 = WadCache::new();
        cache1.add_known_path("path1.dds".to_string());
        
        let mut cache2 = WadCache::new();
        cache2.add_known_path("path2.dds".to_string());
        
        cache1.merge(&cache2);
        
        assert!(cache1.has_file_path("path1.dds"));
        assert!(cache1.has_file_path("path2.dds"));
        assert_eq!(cache1.hash_count(), 2);
    }

    #[test]
    fn test_xxhash_path_consistency() {
        // Verify that path hashing is consistent
        let hash1 = xxhash_path("assets/test.dds");
        let hash2 = xxhash_path("ASSETS/TEST.DDS");
        let hash3 = xxhash_path("assets\\test.dds");
        
        assert_eq!(hash1, hash2); // Case insensitive
        assert_eq!(hash1, hash3); // Backslash normalized
    }
}

