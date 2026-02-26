//! Analyzer Module
//!
//! This module handles file analysis including:
//! - WAD file path caching for lookups
//! - BIN file parsing and property access
//! - BNK audio file version parsing
//! - Hash dictionary for resolving hashes to names
//! - Hash file downloading from CommunityDragon
//! - Scanning for files (single and batch)
//! - Detecting issues in files against config rules
//! - WAD modification and writing

pub mod wad_cache;
pub mod bin_parser;
pub mod bnk_version;
pub mod hash_dict;
pub mod hash_downloader;
pub mod scanner;
pub mod detector;
#[cfg(feature = "wad-writer")]
pub mod wad_writer;

// Re-export public types
pub use wad_cache::WadCache;
pub use bin_parser::{parse_bin_file, parse_bin_bytes, write_bin_file, write_bin_bytes, extract_all_strings, BinTree, BinTreeObject};
pub use bnk_version::{parse_bnk_version, is_bnk_extension, is_events_bnk_path, filter_removable_bnks, BnkInfo, ALLOWED_BNK_VERSIONS};
pub use hash_dict::HashDict;
pub use hash_downloader::{download_hashes, needs_download, DownloadProgress, DownloadStats};
pub use scanner::{scan, ScanMode, ModFile, FileType};
pub use detector::{analyze_file, detect_issue, ScanResult, DetectedIssue};
#[cfg(feature = "wad-writer")]
pub use wad_writer::{WadModifier, WadModification, modify_wad, hash_wad_path};


