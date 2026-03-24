//! Compatibility bridge to ltk_meta and ltk_ritobin for BIN file handling.
//!
//! This module provides a simplified interface to the League Toolkit libraries,
//! wrapping their APIs for use throughout the application.

use std::io::Cursor;
use std::sync::OnceLock;
use parking_lot::RwLock;
use ltk_meta::{BinTree};

/// Maximum allowed BIN file size (50MB - no legitimate BIN should be larger)
pub const MAX_BIN_SIZE: usize = 50 * 1024 * 1024;

/// Error type for BIN operations
#[derive(Debug)]
pub struct BinError(pub String);

impl std::fmt::Display for BinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BinError {}

/// Result type for BIN operations
pub type Result<T> = std::result::Result<T, BinError>;

/// Read a binary BIN file from bytes.
///
/// # Arguments
/// * `data` - The binary data to parse
///
/// # Returns
/// A `BinTree` structure containing the parsed data
///
/// # Safety
/// This function validates file size and magic bytes to prevent memory issues
/// from corrupt files. Files larger than 50MB are rejected.
pub fn read_bin(data: &[u8]) -> Result<BinTree> {
    // DEFENSIVE: Log file info before parsing
    tracing::debug!(
        "read_bin: size={} bytes, magic={:02x?}",
        data.len(),
        &data[..std::cmp::min(8, data.len())]
    );

    // Reject obviously corrupt files (too large)
    if data.len() > MAX_BIN_SIZE {
        tracing::error!(
            "BIN file rejected: {} bytes exceeds max size of {} bytes",
            data.len(),
            MAX_BIN_SIZE
        );
        return Err(BinError(format!(
            "BIN file too large ({} bytes, max {} bytes) - likely corrupt",
            data.len(),
            MAX_BIN_SIZE
        )));
    }

    // Validate BIN magic bytes (PROP or PTCH)
    if data.len() >= 4 {
        let magic = &data[0..4];
        if magic != b"PROP" && magic != b"PTCH" {
            tracing::error!(
                "Invalid BIN magic bytes: {:02x?} (expected PROP or PTCH)",
                magic
            );
            return Err(BinError(format!(
                "Invalid BIN magic bytes: {:02x?} (expected PROP or PTCH)",
                magic
            )));
        }
    } else {
        tracing::error!("BIN file too small: {} bytes (minimum 4 bytes for magic)", data.len());
        return Err(BinError(format!(
            "BIN file too small ({} bytes, minimum 4 bytes for magic)",
            data.len()
        )));
    }

    // catch_unwind to handle OOM panics from ltk_meta
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // CRITICAL: Print right before the dangerous call - flush to ensure visibility before crash
        use std::io::Write;
        println!("[ltk_bridge] Calling BinTree::from_reader ({} bytes)...", data.len());
        let _ = std::io::stdout().flush();
        
        let mut cursor = Cursor::new(data);
        BinTree::from_reader(&mut cursor)
    }));

    match result {
        Ok(Ok(tree)) => {
            tracing::debug!(
                "Successfully parsed BIN: {} objects, {} dependencies",
                tree.objects.len(),
                tree.dependencies.len()
            );
            Ok(tree)
        }
        Ok(Err(e)) => {
            tracing::error!("BIN parse failed: {} (file was {} bytes)", e, data.len());
            Err(BinError(format!("Failed to parse bin: {}", e)))
        }
        Err(panic_info) => {
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            tracing::error!(
                "CRITICAL: Parser panicked on {} byte file: {}",
                data.len(),
                panic_msg
            );
            Err(BinError(format!(
                "Parser panicked (likely OOM or stack overflow): {}",
                panic_msg
            )))
        }
    }
}

/// Write a BinTree to binary format.
///
/// # Arguments
/// * `tree` - The BinTree to serialize
///
/// # Returns
/// A Vec<u8> containing the binary data
pub fn write_bin(tree: &BinTree) -> Result<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());
    tree.to_writer(&mut buffer)
        .map_err(|e| BinError(format!("Failed to write bin: {}", e)))?;
    Ok(buffer.into_inner())
}

/// Convert a BinTree to ritobin text format with hash name lookup.
///
/// # Arguments
/// * `tree` - The BinTree to convert
/// * `hashes` - Hash provider for name lookup
///
/// # Returns
/// A String containing the ritobin text format with resolved names
pub fn tree_to_text_with_hashes<H: ltk_ritobin::HashProvider>(
    tree: &BinTree,
    hashes: &H,
) -> Result<String> {
    ltk_ritobin::write_with_hashes(tree, hashes)
        .map_err(|e| BinError(format!("Failed to convert to text: {}", e)))
}

/// Load BIN-specific hash files into a HashMapProvider
///
/// Loads hashes from the RitoShark hash directory:
/// - hashes.bintypes.txt (type names)
/// - hashes.binfields.txt (field/property names)
/// - hashes.binentries.txt (entry/object names)
/// - hashes.binhashes.txt (generic hashes)
///
/// Uses the built-in load_from_directory method which properly maps
/// each file to its category (entries, fields, hashes, types).
///
/// # Returns
/// A HashMapProvider populated with all loaded hashes
pub fn load_bin_hashes() -> HashMapProvider {
    use crate::hash::{get_or_open_env, downloader::get_ritoshark_hash_dir};

    let mut hashes = HashMapProvider::new();

    // Get the RitoShark hash directory
    let hash_dir = match get_ritoshark_hash_dir() {
        Ok(dir) => dir.to_string_lossy().into_owned(),
        Err(e) => {
            tracing::warn!("Failed to get hash directory: {}", e);
            return hashes;
        }
    };

    // Get LMDB environment
    let env = match get_or_open_env(&hash_dir) {
        Some(e) => e,
        None => {
            tracing::warn!("Failed to open LMDB, cannot load BIN hashes");
            return hashes;
        }
    };

    // Read all hashes from LMDB and populate HashMapProvider
    let rtxn = match env.read_txn() {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Failed to open LMDB read transaction: {}", e);
            return hashes;
        }
    };

    let db = match env.open_database::<heed::types::Bytes, heed::types::Str>(&rtxn, None) {
        Ok(Some(d)) => d,
        _ => {
            tracing::warn!("Failed to open LMDB database");
            return hashes;
        }
    };

    // Iterate all LMDB entries and add to HashMapProvider
    let iter = match db.iter(&rtxn) {
        Ok(i) => i,
        Err(e) => {
            tracing::warn!("Failed to create LMDB iterator: {}", e);
            return hashes;
        }
    };

    let mut count = 0;
    for result in iter {
        match result {
            Ok((key_bytes, path_str)) => {
                if key_bytes.len() == 8 {
                    let hash = u64::from_be_bytes([
                        key_bytes[0], key_bytes[1], key_bytes[2], key_bytes[3],
                        key_bytes[4], key_bytes[5], key_bytes[6], key_bytes[7],
                    ]);

                    // Only load 32-bit hashes into HashMapProvider (BIN files use 32-bit hashes)
                    if hash < 0x1_0000_0000 {
                        let hash_32 = hash as u32;
                        hashes.insert_type(hash_32, path_str.to_string());
                        hashes.insert_field(hash_32, path_str.to_string());
                        hashes.insert_entry(hash_32, path_str.to_string());
                        hashes.insert_hash(hash_32, path_str.to_string());
                        count += 1;
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Error reading LMDB entry: {}", e);
            }
        }
    }

    tracing::info!("Loaded {} hashes from LMDB for BIN resolution", count);

    hashes
}

/// Global cache for BIN hash provider - loaded once, reused for all conversions
/// This eliminates the massive overhead of loading hash files for every BIN conversion
static BIN_HASHES_CACHE: OnceLock<RwLock<HashMapProvider>> = OnceLock::new();

/// Get or initialize the cached BIN hash provider
/// 
/// This is thread-safe and will only load hashes from disk once.
/// All subsequent calls return the cached version.
pub fn get_cached_bin_hashes() -> &'static RwLock<HashMapProvider> {
    BIN_HASHES_CACHE.get_or_init(|| {
        tracing::info!("Initializing global BIN hash cache...");
        let hashes = load_bin_hashes();
        tracing::info!("Global BIN hash cache initialized with {} hashes", hashes.total_count());
        RwLock::new(hashes)
    })
}

/// Reload the BIN hash cache from disk
///
/// Call this after updating hash files to refresh the cache
pub fn reload_bin_hash_cache() {
    if let Some(cache) = BIN_HASHES_CACHE.get() {
        tracing::info!("Reloading BIN hash cache from disk...");
        let new_hashes = load_bin_hashes();
        let total = new_hashes.total_count();
        *cache.write() = new_hashes;
        tracing::info!("BIN hash cache reloaded with {} hashes", total);
    }
}

/// Convert a BinTree to ritobin text format using the cached hash provider
///
/// This is the preferred method for BIN conversion as it reuses the globally
/// cached hash provider instead of loading from disk each time.
pub fn tree_to_text_cached(tree: &BinTree) -> Result<String> {
    let hashes = get_cached_bin_hashes().read();
    tree_to_text_with_hashes(tree, &*hashes)
}

/// Parse ritobin text format to BinTree.
///
/// # Arguments
/// * `text` - The ritobin text to parse
///
/// # Returns
/// A BinTree structure
pub fn text_to_tree(text: &str) -> Result<BinTree> {
    ltk_ritobin::parse_to_bin_tree(text)
        .map_err(|e| BinError(format!("Failed to parse text: {}", e)))
}

// Re-export ltk_ritobin types for hash provider support
pub use ltk_ritobin::HashMapProvider;
