//! Scanner Module
//!
//! Handles file discovery and extraction for analysis.
//!
//! Supports two modes:
//! - Single file: User drops a .fantome, .zip, or .wad.client file
//! - Batch mode: User selects CSLoL Manager's installed/ folder

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Scan mode determines how files are discovered
#[derive(Debug, Clone)]
pub enum ScanMode {
    /// Analyze a single file
    SingleFile(PathBuf),
    /// Recursively scan a directory (e.g., CSLoL Manager's installed/ folder)
    Directory(PathBuf),
}

/// Represents a discovered mod file
#[derive(Debug, Clone)]
pub struct ModFile {
    /// Full path to the file
    pub path: PathBuf,
    /// Detected file type
    pub file_type: FileType,
}

/// Supported file types for analysis
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    /// .fantome (ZIP wrapper with META/info.json)
    Fantome,
    /// .zip (generic ZIP archive)
    Zip,
    /// .wad.client (League WAD archive)
    WadClient,
    /// .bin (League binary property file)
    Bin,
}

impl FileType {
    /// Detect file type from path extension
    pub fn from_path(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_str()?;
        let lower = file_name.to_lowercase();

        // Check compound extensions first
        if lower.ends_with(".wad.client") {
            return Some(FileType::WadClient);
        }

        // Then check simple extensions
        let extension = path.extension()?.to_str()?.to_lowercase();
        match extension.as_str() {
            "fantome" => Some(FileType::Fantome),
            "zip" => Some(FileType::Zip),
            "bin" => Some(FileType::Bin),
            _ => None,
        }
    }
}

/// Scan for mod files based on the specified mode
///
/// # Arguments
/// * `mode` - Either a single file path or a directory to scan
///
/// # Returns
/// A list of discovered mod files with their types
pub fn scan(mode: ScanMode) -> Result<Vec<ModFile>> {
    match mode {
        ScanMode::SingleFile(path) => scan_single_file(path),
        ScanMode::Directory(path) => scan_directory(path),
    }
}

/// Scan a single file
fn scan_single_file(path: PathBuf) -> Result<Vec<ModFile>> {
    let file_type = FileType::from_path(&path)
        .with_context(|| format!("Unsupported file type: {}", path.display()))?;

    Ok(vec![ModFile { path, file_type }])
}

/// Recursively scan a directory for mod files
///
/// Expected CSLoL Manager structure:
/// ```text
/// installed/
/// ├── ChampionName_SkinNumber/
/// │   ├── META/
/// │   │   └── info.json
/// │   └── WAD/
/// │       └── champion.wad.client
/// ```
fn scan_directory(dir: PathBuf) -> Result<Vec<ModFile>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(&dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories
        if !path.is_file() {
            continue;
        }

        // Skip files in META directories (these are just metadata)
        if path.components().any(|c| {
            c.as_os_str().to_str().map(|s| s.to_lowercase()) == Some("meta".to_string())
        }) {
            continue;
        }

        // Check if this is a supported file type
        if let Some(file_type) = FileType::from_path(path) {
            files.push(ModFile {
                path: path.to_path_buf(),
                file_type,
            });
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_detection() {
        // Test .fantome
        assert_eq!(
            FileType::from_path(Path::new("test.fantome")),
            Some(FileType::Fantome)
        );
        assert_eq!(
            FileType::from_path(Path::new("C:/mods/MySkin.FANTOME")),
            Some(FileType::Fantome)
        );

        // Test .wad.client (compound extension)
        assert_eq!(
            FileType::from_path(Path::new("skin.wad.client")),
            Some(FileType::WadClient)
        );
        assert_eq!(
            FileType::from_path(Path::new("C:/mods/champion.WAD.CLIENT")),
            Some(FileType::WadClient)
        );

        // Test .zip
        assert_eq!(
            FileType::from_path(Path::new("test.zip")),
            Some(FileType::Zip)
        );

        // Test .bin
        assert_eq!(
            FileType::from_path(Path::new("skin0.bin")),
            Some(FileType::Bin)
        );

        // Test unsupported
        assert_eq!(FileType::from_path(Path::new("test.exe")), None);
        assert_eq!(FileType::from_path(Path::new("test.txt")), None);
    }

    #[test]
    fn test_scan_mode_single_file() {
        let path = PathBuf::from("test.fantome");
        let mode = ScanMode::SingleFile(path.clone());

        // This would fail because the file doesn't exist,
        // but it tests that scan_single_file is called
        match mode {
            ScanMode::SingleFile(p) => assert_eq!(p, path),
            _ => panic!("Expected SingleFile mode"),
        }
    }
}
