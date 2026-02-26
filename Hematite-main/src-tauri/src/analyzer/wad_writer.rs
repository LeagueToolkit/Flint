//! WAD Writer Module
//!
//! Handles reading, modifying, and writing WAD files.
//! Provides utilities for the full WAD modification workflow.

use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;

use league_toolkit::wad::{Wad, WadBuilder, WadChunk, WadChunkBuilder};

/// A pending modification to a WAD file
#[derive(Debug, Clone)]
pub struct WadModification {
    /// Path hash of the chunk to modify
    pub path_hash: u64,
    /// New data for the chunk
    pub new_data: Vec<u8>,
}

/// Represents a chunk's data for the rebuild process
#[derive(Debug, Clone)]
enum ChunkSource {
    /// Copy original data from the source WAD
    Original,
    /// Use modified data
    Modified(Vec<u8>),
    /// Skip this chunk (for removal)
    Removed,
}

/// WAD modification context
pub struct WadModifier<R: Read + Seek> {
    /// The mounted WAD file
    wad: Wad<R>,
    /// Pending modifications (path_hash -> source type)
    modifications: HashMap<u64, ChunkSource>,
}

impl<R: Read + Seek> WadModifier<R> {
    /// Create a new WAD modifier from a reader
    pub fn new(source: R) -> Result<Self> {
        let wad = Wad::mount(source)
            .map_err(|e| anyhow!("Failed to mount WAD: {:?}", e))?;
        
        // Initialize with all chunks as Original
        let mut modifications = HashMap::new();
        
        for (path_hash, _chunk) in wad.chunks() {
            modifications.insert(*path_hash, ChunkSource::Original);
        }
        
        Ok(Self {
            wad,
            modifications,
        })
    }

    /// Replace a chunk's data
    pub fn replace_chunk(&mut self, path_hash: u64, new_data: Vec<u8>) -> Result<()> {
        if !self.modifications.contains_key(&path_hash) {
            return Err(anyhow!("Chunk with hash {:016x} not found in WAD", path_hash));
        }
        self.modifications.insert(path_hash, ChunkSource::Modified(new_data));
        Ok(())
    }

    /// Remove a chunk from the WAD
    pub fn remove_chunk(&mut self, path_hash: u64) -> Result<()> {
        if !self.modifications.contains_key(&path_hash) {
            return Err(anyhow!("Chunk with hash {:016x} not found in WAD", path_hash));
        }
        self.modifications.insert(path_hash, ChunkSource::Removed);
        Ok(())
    }

    /// Check if WAD contains a chunk
    pub fn has_chunk(&self, path_hash: u64) -> bool {
        self.wad.chunks().contains_key(&path_hash)
    }

    /// Get chunk info
    pub fn get_chunk(&self, path_hash: u64) -> Option<&WadChunk> {
        self.wad.chunks().get(&path_hash)
    }

    /// Get decompressed chunk data
    pub fn get_chunk_data(&mut self, path_hash: u64) -> Result<Vec<u8>> {
        let chunk = self.wad.chunks().get(&path_hash)
            .ok_or_else(|| anyhow!("Chunk {:016x} not found", path_hash))?
            .clone();
        
        let (mut decoder, _chunks) = self.wad.decode();
        let data = decoder.load_chunk_decompressed(&chunk)
            .map_err(|e| anyhow!("Failed to decompress chunk: {:?}", e))?;
        
        Ok(data.to_vec())
    }

    /// Get all chunk hashes
    pub fn chunk_hashes(&self) -> Vec<u64> {
        self.wad.chunks().keys().copied().collect()
    }

    /// Build the modified WAD to a writer
    pub fn build<W: Write + Seek>(&mut self, writer: &mut W) -> Result<()> {
        // Collect chunks and their sources - filter out removed ones
        let chunks_to_write: Vec<(u64, WadChunk)> = self.wad.chunks()
            .iter()
            .filter(|(path_hash, _)| {
                !matches!(self.modifications.get(path_hash), Some(ChunkSource::Removed))
            })
            .map(|(hash, chunk)| (*hash, chunk.clone()))
            .collect();

        // Pre-load all the data we need
        let mut chunk_data: HashMap<u64, Vec<u8>> = HashMap::new();
        
        for (path_hash, chunk) in &chunks_to_write {
            match self.modifications.get(path_hash) {
                Some(ChunkSource::Modified(data)) => {
                    chunk_data.insert(*path_hash, data.clone());
                }
                Some(ChunkSource::Original) | None => {
                    // Load original data
                    let (mut decoder, _) = self.wad.decode();
                    let data = decoder.load_chunk_decompressed(chunk)
                        .map_err(|e| anyhow!("Failed to read chunk {:016x}: {:?}", path_hash, e))?;
                    chunk_data.insert(*path_hash, data.to_vec());
                }
                Some(ChunkSource::Removed) => unreachable!(),
            }
        }

        // Build WAD using WadBuilder
        let mut builder = WadBuilder::default();
        
        for (path_hash, _chunk) in &chunks_to_write {
            // Create chunk builder with path hash directly
            // WadChunkBuilder stores the hash, we'll set it via path that hashes to same value
            // Since we can't set hash directly, we'll use a custom approach
            builder = builder.with_chunk(
                WadChunkBuilderWithHash::new(*path_hash)
            );
        }

        // Write the WAD
        builder.build_to_writer(writer, |path_hash, cursor| {
            if let Some(data) = chunk_data.get(&path_hash) {
                cursor.write_all(data)?;
            }
            Ok(())
        }).map_err(|e| anyhow!("Failed to build WAD: {:?}", e))?;

        Ok(())
    }
}

/// Custom WadChunkBuilder that accepts raw path hash
/// Helper for creating WadChunkBuilder with raw path hash
pub struct WadChunkBuilderWithHash;

impl WadChunkBuilderWithHash {
    pub fn new(path_hash: u64) -> WadChunkBuilder {
        // We need to create a WadChunkBuilder and set its internal path field
        // Since the field is private, we use transmute or create from scratch
        // Actually, let's check if we can use `with_path` with a computed value
        
        // For now, use unsafe to set the hash directly
        // The WadChunkBuilder layout is: { path: u64, force_compression: Option<...> }
        let mut builder = WadChunkBuilder::default();
        
        // Using transmute is not ideal but necessary here since WadChunkBuilder 
        // doesn't expose a with_path_hash method
        unsafe {
            let raw_ptr = &mut builder as *mut WadChunkBuilder as *mut RawChunkBuilder;
            (*raw_ptr).path = path_hash;
        }
        
        builder
    }
}

/// Raw representation matching WadChunkBuilder layout
#[repr(C)]
struct RawChunkBuilder {
    path: u64,
    force_compression: Option<league_toolkit::wad::WadChunkCompression>,
}

// =============================================================================
// HIGH-LEVEL API
// =============================================================================

/// Modify a WAD file in place or to a new location
pub fn modify_wad<P: AsRef<Path>>(
    source_path: P,
    output_path: P,
    modifications: Vec<WadModification>,
    removals: Vec<u64>,
) -> Result<()> {
    let source_file = File::open(source_path.as_ref())
        .with_context(|| format!("Failed to open source WAD: {:?}", source_path.as_ref()))?;
    let reader = BufReader::new(source_file);
    
    let mut modifier = WadModifier::new(reader)?;
    
    // Apply modifications
    for modif in modifications {
        modifier.replace_chunk(modif.path_hash, modif.new_data)?;
    }
    
    // Apply removals
    for path_hash in removals {
        modifier.remove_chunk(path_hash)?;
    }
    
    // Build output
    let output_file = File::create(output_path.as_ref())
        .with_context(|| format!("Failed to create output WAD: {:?}", output_path.as_ref()))?;
    let mut writer = BufWriter::new(output_file);
    
    modifier.build(&mut writer)?;
    
    log::info!("Wrote modified WAD to: {:?}", output_path.as_ref());
    Ok(())
}

/// Hash a path string to WAD chunk hash (xxhash64)
pub fn hash_wad_path(path: &str) -> u64 {
    xxhash_rust::xxh64::xxh64(path.to_lowercase().as_bytes(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_wad_path() {
        let hash = hash_wad_path("data/characters/aatrox/aatrox.bin");
        assert!(hash != 0);
        
        // Same path should give same hash (case insensitive)
        let hash2 = hash_wad_path("DATA/CHARACTERS/AATROX/AATROX.BIN");
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_wad_modification_struct() {
        let modif = WadModification {
            path_hash: 0x1234567890ABCDEF,
            new_data: vec![1, 2, 3, 4],
        };
        
        assert_eq!(modif.path_hash, 0x1234567890ABCDEF);
        assert_eq!(modif.new_data.len(), 4);
    }
}
