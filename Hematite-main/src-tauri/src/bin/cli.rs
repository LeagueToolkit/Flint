//! Hematite CLI - Standalone Skin Fixer
//!
//! Command-line interface for fixing League of Legends skin files.
//!
//! # Usage
//! ```bash
//! hematite-cli "path/to/skin.fantome"                    # Auto-detect and fix all issues
//! hematite-cli "skin.fantome" --healthbar --white-model  # Fix specific issues
//! hematite-cli "skin.fantome" --dry-run                  # Show what would be fixed
//! hematite-cli "skin.fantome" --json > results.json      # JSON output for automation
//! ```

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::time::Instant;

// Import from the library
use hematite_lib::logging::{self, LogLevel, SessionStats};
use hematite_lib::processor;

/// Hematite - League of Legends Skin Fixer
#[derive(Parser, Debug)]
#[command(name = "hematite-cli")]
#[command(author = "RitoShark Team")]
#[command(version = "0.1.0")]
#[command(about = "Fix common issues in League of Legends skin files", long_about = None)]
struct Cli {
    /// Input file or folder path (.fantome, .wad.client, .bin, or directory)
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output path (default: modifies input file in-place)
    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,

    // === Fix Selection Flags ===
    
    /// Fix missing or incorrect health bar style
    #[arg(long, help = "Fix missing HP bar (UnitHealthBarStyle)")]
    healthbar: bool,

    /// Fix white model issues (TextureName -> TexturePath, SamplerName -> TextureName)
    #[arg(long, help = "Fix white model texture references")]
    white_model: bool,

    /// Fix black/missing icons (.dds -> .tex conversion)
    #[arg(long, help = "Fix black icons by converting .dds to .tex")]
    black_icons: bool,

    /// Fix broken particle effects
    #[arg(long, help = "Fix broken particle texture references")]
    particles: bool,

    /// Remove champion bin files that break after patches
    #[arg(long, help = "Remove problematic champion bin files")]
    remove_champion_bins: bool,

    /// Remove BNK audio files with incompatible versions
    #[arg(long, help = "Remove incompatible BNK audio files")]
    remove_bnk: bool,

    /// Fix VFX shape issues (patch 14.1+)
    #[arg(long, help = "Fix VFX particle shape issues")]
    vfx_shape: bool,

    /// Apply all available fixes
    #[arg(long, short = 'a', help = "Apply all available fixes")]
    all: bool,

    // === Output Control ===

    /// Output results as JSON (for automation)
    #[arg(long, help = "Output results as JSON")]
    json: bool,

    /// Dry run - show what would be fixed without making changes
    #[arg(long, help = "Show fixes without applying them")]
    dry_run: bool,

    /// Verbosity level
    #[arg(short, long, value_enum, default_value = "normal")]
    verbosity: Verbosity,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Verbosity {
    /// Only show errors
    Quiet,
    /// Show info, warnings, and errors (default)
    Normal,
    /// Show debug information
    Verbose,
    /// Show all trace information
    Trace,
}

impl From<Verbosity> for LogLevel {
    fn from(v: Verbosity) -> Self {
        match v {
            Verbosity::Quiet => LogLevel::Quiet,
            Verbosity::Normal => LogLevel::Normal,
            Verbosity::Verbose => LogLevel::Verbose,
            Verbosity::Trace => LogLevel::Trace,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    logging::init_logging(cli.verbosity.into(), cli.json);
    
    // Start timer
    let start_time = Instant::now();
    
    // Collect selected fixes
    let selected_fixes = collect_selected_fixes(&cli);
    
    // Validate input
    validate_input(&cli.input)?;
    
    // Log session start (suppressed in JSON mode)
    if !cli.json {
        logging::log_session_start(
            cli.input.to_string_lossy().as_ref(),
            &selected_fixes,
        );
    }
    
    // Run the fixer
    let result = run_fixer(&cli, &selected_fixes);
    
    // Calculate duration
    let duration = start_time.elapsed().as_secs_f64();
    
    // Output results
    if cli.json {
        output_json_results(duration, &result)?;
    } else {
        logging::log_session_summary(duration);
    }
    
    // Return appropriate exit code
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Collect the list of selected fixes based on CLI flags
fn collect_selected_fixes(cli: &Cli) -> Vec<String> {
    let mut fixes = Vec::new();
    
    // If --all is specified, enable everything
    if cli.all {
        fixes.push("healthbar".to_string());
        fixes.push("white_model".to_string());
        fixes.push("black_icons".to_string());
        fixes.push("particles".to_string());
        fixes.push("remove_champion_bins".to_string());
        fixes.push("remove_bnk".to_string());
        fixes.push("vfx_shape".to_string());
        return fixes;
    }
    
    // Otherwise, collect individual flags
    if cli.healthbar {
        fixes.push("healthbar".to_string());
    }
    if cli.white_model {
        fixes.push("white_model".to_string());
    }
    if cli.black_icons {
        fixes.push("black_icons".to_string());
    }
    if cli.particles {
        fixes.push("particles".to_string());
    }
    if cli.remove_champion_bins {
        fixes.push("remove_champion_bins".to_string());
    }
    if cli.remove_bnk {
        fixes.push("remove_bnk".to_string());
    }
    if cli.vfx_shape {
        fixes.push("vfx_shape".to_string());
    }
    
    // If no fixes specified, we'll auto-detect
    fixes
}

/// Validate the input path
fn validate_input(input: &PathBuf) -> Result<()> {
    if !input.exists() {
        anyhow::bail!("Input path does not exist: {}", input.display());
    }
    
    if input.is_file() {
        let ext = input.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        
        let file_name = input.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_lowercase())
            .unwrap_or_default();
        
        // Check for supported file types
        let is_supported = ext == "fantome" 
            || ext == "zip" 
            || ext == "bin"
            || file_name.ends_with(".wad.client");
        
        if !is_supported {
            anyhow::bail!(
                "Unsupported file type: {}. Supported: .fantome, .zip, .wad.client, .bin",
                input.display()
            );
        }
    }
    
    Ok(())
}

/// Run the fixer with the selected options
fn run_fixer(cli: &Cli, selected_fixes: &[String]) -> Result<()> {
    let input_path = cli.input.to_string_lossy();
    
    logging::log_analyzing(&input_path);
    logging::increment_files_processed();
    
    if cli.dry_run {
        tracing::info!("DRY RUN MODE - No changes will be made");
    }
    
    // Determine what to process based on file type
    let input = &cli.input;
    
    if input.is_dir() {
        process_directory(input, selected_fixes, cli)?;
    } else {
        process_file(input, selected_fixes, cli)?;
    }
    
    Ok(())
}

/// Process a directory
fn process_directory(dir: &PathBuf, selected_fixes: &[String], cli: &Cli) -> Result<()> {
    use walkdir::WalkDir;
    
    tracing::info!("Processing directory: {}", dir.display());
    
    for entry in WalkDir::new(dir) {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        
        if path.is_file() {
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();
            
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.to_lowercase())
                .unwrap_or_default();
            
            // Process supported file types
            if ext == "fantome" || ext == "zip" || ext == "bin" || file_name.ends_with(".wad.client") {
                process_file(&path.to_path_buf(), selected_fixes, cli)?;
            }
        }
    }
    
    Ok(())
}

/// Process a single file
fn process_file(file: &PathBuf, selected_fixes: &[String], cli: &Cli) -> Result<()> {
    let file_name = file.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    
    tracing::info!("Processing file: {}", file_name);
    logging::increment_files_processed();
    
    let ext = file.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    
    let file_name_lower = file.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_lowercase())
        .unwrap_or_default();
    
    if ext == "fantome" || ext == "zip" {
        process_fantome(file, selected_fixes, cli)?;
    } else if file_name_lower.ends_with(".wad.client") {
        process_wad(file, selected_fixes, cli)?;
    } else if ext == "bin" {
        process_bin(file, selected_fixes, cli)?;
    }
    
    Ok(())
}

/// Process a .fantome or .zip file
fn process_fantome(file: &PathBuf, selected_fixes: &[String], cli: &Cli) -> Result<()> {
    tracing::debug!("Processing fantome: {}", file.display());
    
    let output = cli.output.as_ref().map(|p| p.as_path());
    
    match processor::process_fantome(file, output, selected_fixes, cli.dry_run) {
        Ok(result) => {
            if result.fixes_applied > 0 {
                tracing::info!("Applied {} fixes to fantome", result.fixes_applied);
            }
            if !result.errors.is_empty() {
                for error in result.errors {
                    logging::log_warning(&error);
                }
            }
            Ok(())
        }
        Err(e) => {
            logging::log_fix_failed("fantome", &e.to_string());
            Err(e)
        }
    }
}

/// Process a .wad.client file
fn process_wad(file: &PathBuf, selected_fixes: &[String], cli: &Cli) -> Result<()> {
    tracing::debug!("Processing WAD: {}", file.display());
    
    match processor::process_wad_file(file, selected_fixes, cli.dry_run) {
        Ok(result) => {
            if result.fixes_applied > 0 {
                tracing::info!("Applied {} fixes to WAD", result.fixes_applied);
            }
            if !result.errors.is_empty() {
                for error in result.errors {
                    logging::log_warning(&error);
                }
            }
            Ok(())
        }
        Err(e) => {
            logging::log_fix_failed("wad", &e.to_string());
            Err(e)
        }
    }
}

/// Process a .bin file
fn process_bin(file: &PathBuf, selected_fixes: &[String], cli: &Cli) -> Result<()> {
    tracing::debug!("Processing BIN: {}", file.display());
    
    let output = cli.output.as_ref().map(|p| p.as_path());
    
    match processor::process_bin_file(file, output, selected_fixes, cli.dry_run) {
        Ok(result) => {
            if result.fixes_applied > 0 {
                tracing::info!("Applied {} fixes to BIN", result.fixes_applied);
            }
            if !result.errors.is_empty() {
                for error in result.errors {
                    logging::log_warning(&error);
                }
            }
            Ok(())
        }
        Err(e) => {
            logging::log_fix_failed("bin", &e.to_string());
            Err(e)
        }
    }
}

/// Output results as JSON
fn output_json_results(duration: f64, result: &Result<()>) -> Result<()> {
    let stats = SessionStats::collect(duration);
    
    #[derive(serde::Serialize)]
    struct JsonOutput {
        success: bool,
        error: Option<String>,
        stats: SessionStats,
    }
    
    let output = JsonOutput {
        success: result.is_ok(),
        error: result.as_ref().err().map(|e| e.to_string()),
        stats,
    };
    
    let json = serde_json::to_string_pretty(&output)?;
    println!("{}", json);
    
    Ok(())
}
