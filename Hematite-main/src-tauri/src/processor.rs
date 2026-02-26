//! File Processor Module
//!
//! Handles the end-to-end workflow for processing skin files:
//! - Fantome extraction and repacking
//! - WAD chunk processing
//! - BIN file analysis and fixing

use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{BufReader, Cursor, Read, Write};
use std::path::Path;
use tempfile::TempDir;
use walkdir::WalkDir;

use crate::analyzer::{
    parse_bin_bytes, write_bin_bytes, HashDict, WadCache,
    WadModifier, WadModification,
};
use crate::config::fetcher::get_config;
use crate::config::schema::FixRule;
use crate::fixer::{apply_transforms, FixContext, FixResult};
use crate::logging;

/// Result of processing a file
#[derive(Debug)]
pub struct ProcessResult {
    pub files_processed: u32,
    pub fixes_applied: u32,
    pub fixes_failed: u32,
    pub errors: Vec<String>,
}

impl Default for ProcessResult {
    fn default() -> Self {
        Self {
            files_processed: 0,
            fixes_applied: 0,
            fixes_failed: 0,
            errors: Vec::new(),
        }
    }
}

impl ProcessResult {
    pub fn merge(&mut self, other: ProcessResult) {
        self.files_processed += other.files_processed;
        self.fixes_applied += other.fixes_applied;
        self.fixes_failed += other.fixes_failed;
        self.errors.extend(other.errors);
    }
}

/// Process a fantome/ZIP file
pub fn process_fantome(
    fantome_path: &Path,
    output_path: Option<&Path>,
    selected_fixes: &[String],
    dry_run: bool,
) -> Result<ProcessResult> {
    use zip::ZipArchive;
    
    logging::log_debug(&format!("Processing fantome: {}", fantome_path.display()));
    
    let mut result = ProcessResult::default();
    
    // Open the fantome (ZIP) file
    let file = File::open(fantome_path)
        .with_context(|| format!("Failed to open fantome: {}", fantome_path.display()))?;
    let mut archive = ZipArchive::new(BufReader::new(file))
        .context("Failed to read fantome as ZIP")?;
    
    // Create temp directory for extraction
    let temp_dir = TempDir::new().context("Failed to create temp directory")?;
    let temp_path = temp_dir.path();
    
    logging::log_debug(&format!("Extracting to temp: {}", temp_path.display()));
    
    // Extract all files
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_path = temp_path.join(file.name());
        
        if file.is_dir() {
            fs::create_dir_all(&file_path)?;
        } else {
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out_file = File::create(&file_path)?;
            std::io::copy(&mut file, &mut out_file)?;
        }
    }
    
    // Find and process WAD files
    for entry in WalkDir::new(temp_path) {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.to_lowercase())
                .unwrap_or_default();
            
            if file_name.ends_with(".wad.client") {
                logging::log_analyzing(&file_name);
                
                match process_wad_file(path, selected_fixes, dry_run) {
                    Ok(wad_result) => {
                        result.merge(wad_result);
                    }
                    Err(e) => {
                        let error = format!("Error processing WAD {}: {:?}", file_name, e);
                        logging::log_warning(&error);
                        result.errors.push(error);
                    }
                }
            }
        }
    }
    
    result.files_processed += 1;
    
    // If not dry run and we made changes, repack the fantome
    if !dry_run && result.fixes_applied > 0 {
        let output = output_path.unwrap_or(fantome_path);
        
        logging::log_debug(&format!("Repacking fantome to: {}", output.display()));
        
        // Create new ZIP file
        let out_file = File::create(output)
            .with_context(|| format!("Failed to create output file: {}", output.display()))?;
        let mut zip_writer = zip::ZipWriter::new(out_file);
        
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        
        // Add all files from temp directory
        for entry in WalkDir::new(temp_path) {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let relative_path = path.strip_prefix(temp_path)?;
                let name = relative_path.to_string_lossy().replace('\\', "/");
                
                zip_writer.start_file(&name, options)?;
                let mut file = File::open(path)?;
                std::io::copy(&mut file, &mut zip_writer)?;
            }
        }
        
        zip_writer.finish()?;
        
        logging::log_fix_success("fantome_repack", 1);
    }
    
    Ok(result)
}

/// Process a WAD file
pub fn process_wad_file(
    wad_path: &Path,
    selected_fixes: &[String],
    dry_run: bool,
) -> Result<ProcessResult> {
    logging::log_debug(&format!("Processing WAD: {}", wad_path.display()));
    
    let mut result = ProcessResult::default();
    result.files_processed = 1;
    
    // Open WAD file
    let file = File::open(wad_path)
        .with_context(|| format!("Failed to open WAD: {}", wad_path.display()))?;
    let reader = BufReader::new(file);
    
    // Create WAD modifier
    let mut modifier = WadModifier::new(reader)
        .with_context(|| format!("Failed to mount WAD: {}", wad_path.display()))?;
    
    // Build WAD cache for lookups (from the same file)
    let wad_data = fs::read(wad_path)?;
    let wad_cache = WadCache::from_bytes(&wad_data)
        .context("Failed to build WAD cache")?;
    
    // Load fix config
    let config = get_config().context("Failed to load fix configuration")?;
    
    // Load hash dictionary (try to load, use empty if not available)
    let hash_dict = HashDict::load().unwrap_or_else(|_| HashDict::new());
    
    // Collect modifications
    let mut modifications: Vec<WadModification> = Vec::new();
    
    // Get all chunk hashes
    let chunk_hashes = modifier.chunk_hashes();
    
    for path_hash in chunk_hashes {
        // Get chunk data
        let chunk_data = match modifier.get_chunk_data(path_hash) {
            Ok(data) => data,
            Err(e) => {
                logging::log_debug(&format!("Failed to read chunk {:016x}: {}", path_hash, e));
                continue;
            }
        };
        
        // Check if this is a BIN file (PROP magic bytes)
        let is_bin = chunk_data.len() >= 4 && &chunk_data[0..4] == b"PROP";
        
        if !is_bin {
            continue;
        }
        
        // Get chunk path from hash dict if available
        let chunk_path = hash_dict.game.get(&path_hash)
            .cloned()
            .unwrap_or_else(|| format!("{:016x}.bin", path_hash));
        
        // Process BIN
        match process_bin_data(&chunk_data, &chunk_path, &wad_cache, &hash_dict, &config.fixes, selected_fixes, dry_run) {
            Ok((modified_data, fix_result)) => {
                result.fixes_applied += fix_result.fixes_applied.len() as u32;
                result.fixes_failed += fix_result.fixes_failed.len() as u32;
                
                if let Some(data) = modified_data {
                    modifications.push(WadModification {
                        path_hash,
                        new_data: data,
                    });
                }
            }
            Err(e) => {
                logging::log_debug(&format!("Skipping chunk {:016x}: {}", path_hash, e));
            }
        }
    }
    
    // If not dry run and we have modifications, rebuild WAD
    if !dry_run && !modifications.is_empty() {
        logging::log_debug(&format!("Applying {} modifications to WAD", modifications.len()));
        
        // Re-open WAD for modification
        let file = File::open(wad_path)?;
        let reader = BufReader::new(file);
        let mut modifier = WadModifier::new(reader)?;
        
        // Apply modifications
        for modification in modifications {
            modifier.replace_chunk(modification.path_hash, modification.new_data)?;
        }
        
        // Write to temp file then rename
        let temp_path = wad_path.with_extension("wad.client.tmp");
        {
            let output_file = File::create(&temp_path)?;
            let mut writer = std::io::BufWriter::new(output_file);
            modifier.build(&mut writer)?;
        }
        
        // Replace original with modified
        fs::rename(&temp_path, wad_path)?;
    }
    
    Ok(result)
}

/// Process BIN data and return modified data if changes were made
fn process_bin_data(
    data: &[u8],
    file_path: &str,
    wad_cache: &WadCache,
    hash_dict: &HashDict,
    fix_rules: &std::collections::HashMap<String, FixRule>,
    selected_fixes: &[String],
    dry_run: bool,
) -> Result<(Option<Vec<u8>>, FixResult)> {
    // Parse BIN
    let bin_tree = parse_bin_bytes(data)
        .with_context(|| format!("Failed to parse BIN: {}", file_path))?;
    
    // Create fix context
    let mut context = FixContext::new(bin_tree, wad_cache, hash_dict);
    
    // Determine which fixes to apply
    let fixes_to_apply: Vec<String> = if selected_fixes.is_empty() {
        // Auto-detect: apply all enabled fixes
        fix_rules.iter()
            .filter(|(_, rule)| rule.enabled)
            .map(|(id, _)| id.clone())
            .collect()
    } else {
        selected_fixes.to_vec()
    };
    
    // Apply transforms
    let fix_result = apply_transforms(file_path, &mut context, fix_rules, &fixes_to_apply);
    
    // Log results
    for applied in &fix_result.fixes_applied {
        logging::log_fix_success_with_path(&applied.fix_id, applied.changes_count, file_path);
    }
    
    for failed in &fix_result.fixes_failed {
        logging::log_fix_failed(&failed.fix_id, &failed.error);
    }
    
    // If changes were made and not dry run, serialize back
    if !dry_run && !fix_result.fixes_applied.is_empty() {
        let modified_data = write_bin_bytes(&context.bin_tree)
            .with_context(|| format!("Failed to serialize BIN: {}", file_path))?;
        return Ok((Some(modified_data), fix_result));
    }
    
    Ok((None, fix_result))
}

/// Process a standalone BIN file
pub fn process_bin_file(
    bin_path: &Path,
    output_path: Option<&Path>,
    selected_fixes: &[String],
    dry_run: bool,
) -> Result<ProcessResult> {
    logging::log_debug(&format!("Processing BIN: {}", bin_path.display()));
    
    let mut result = ProcessResult::default();
    result.files_processed = 1;
    
    // Read BIN file
    let data = fs::read(bin_path)
        .with_context(|| format!("Failed to read BIN: {}", bin_path.display()))?;
    
    // Load config and hash dict
    let config = get_config().context("Failed to load fix configuration")?;
    let hash_dict = HashDict::load().unwrap_or_else(|_| HashDict::new());
    let wad_cache = WadCache::new(); // Empty cache for standalone BIN
    
    // Process
    let file_path = bin_path.to_string_lossy().to_string();
    match process_bin_data(&data, &file_path, &wad_cache, &hash_dict, &config.fixes, selected_fixes, dry_run) {
        Ok((modified_data, fix_result)) => {
            result.fixes_applied = fix_result.fixes_applied.len() as u32;
            result.fixes_failed = fix_result.fixes_failed.len() as u32;
            
            // Write output if not dry run
            if !dry_run {
                if let Some(data) = modified_data {
                    let output = output_path.unwrap_or(bin_path);
                    fs::write(output, data)
                        .with_context(|| format!("Failed to write BIN: {}", output.display()))?;
                }
            }
        }
        Err(e) => {
            result.errors.push(format!("Error processing BIN: {}", e));
            result.fixes_failed = 1;
        }
    }
    
    Ok(result)
}
