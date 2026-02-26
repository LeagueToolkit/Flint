//! Logging System for Hematite CLI
//!
//! Provides structured logging with:
//! - Session ID tracking for each fix operation
//! - Colored console output
//! - Summary statistics collection
//! - Optional JSON output for automation

use chrono::Local;
use colored::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use uuid::Uuid;

/// Session ID for the current fix operation
static SESSION_ID: Mutex<Option<String>> = Mutex::new(None);

/// Statistics counters
static FILES_PROCESSED: AtomicU32 = AtomicU32::new(0);
static FIXES_APPLIED: AtomicU32 = AtomicU32::new(0);
static FIXES_FAILED: AtomicU32 = AtomicU32::new(0);
static WARNINGS_COUNT: AtomicU32 = AtomicU32::new(0);

/// Log level for the CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Quiet,   // Errors only
    Normal,  // Info + Warnings + Errors
    Verbose, // Debug + Info + Warnings + Errors
    Trace,   // Everything
}

impl LogLevel {
    pub fn to_filter(&self) -> EnvFilter {
        match self {
            LogLevel::Quiet => EnvFilter::new("error"),
            LogLevel::Normal => EnvFilter::new("info"),
            LogLevel::Verbose => EnvFilter::new("debug"),
            LogLevel::Trace => EnvFilter::new("trace"),
        }
    }
}

/// Initialize the logging system
pub fn init_logging(level: LogLevel, json_output: bool) {
    let filter = level.to_filter();

    if json_output {
        // JSON output for automation
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .json()
                    .with_timer(fmt::time::ChronoLocal::rfc_3339())
                    .with_target(true)
                    .with_thread_ids(true),
            )
            .init();
    } else {
        // Colored console output
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .with_target(false)
                    .with_timer(fmt::time::ChronoLocal::new("%H:%M:%S".to_string()))
                    .with_ansi(true),
            )
            .init();
    }
}

/// Create a new session ID for tracking
pub fn create_session() -> String {
    let id = Uuid::new_v4().to_string();
    let short_id = &id[..8]; // First 8 chars for readability
    
    if let Ok(mut session) = SESSION_ID.lock() {
        *session = Some(short_id.to_string());
    }
    
    short_id.to_string()
}

/// Get the current session ID
pub fn get_session_id() -> Option<String> {
    SESSION_ID.lock().ok().and_then(|s| s.clone())
}

/// Log the start of a fix session
pub fn log_session_start(input_path: &str, selected_fixes: &[String]) {
    let session_id = create_session();
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    
    println!();
    println!("{}", "═".repeat(60).cyan());
    println!("{} {} {}", 
        "HEMATITE SKIN FIXER".bold().cyan(),
        "v0.1.0".dimmed(),
        format!("[{}]", session_id).yellow()
    );
    println!("{}", "═".repeat(60).cyan());
    println!();
    println!("{}: {}", "Session".bold(), session_id.yellow());
    println!("{}: {}", "Started".bold(), timestamp.to_string().dimmed());
    println!("{}: {}", "Input".bold(), input_path.green());
    
    if !selected_fixes.is_empty() {
        println!("{}: {}", "Fixes".bold(), selected_fixes.join(", ").cyan());
    } else {
        println!("{}: {}", "Fixes".bold(), "Auto-detect".cyan());
    }
    println!();
}

/// Log detecting issues in a file
pub fn log_analyzing(file_path: &str) {
    tracing::info!(target: "hematite", "🔍 Analyzing: {}", file_path);
}

/// Log a detected issue
pub fn log_issue_detected(fix_id: &str, description: &str, severity: &str) {
    let severity_colored = match severity.to_lowercase().as_str() {
        "critical" => severity.red().bold(),
        "high" => severity.red(),
        "medium" => severity.yellow(),
        "low" => severity.dimmed(),
        _ => severity.normal(),
    };
    
    tracing::info!(
        target: "hematite",
        "⚠️  {} [{}]: {}",
        fix_id.cyan(),
        severity_colored,
        description
    );
}

/// Log applying a fix
pub fn log_applying_fix(fix_id: &str, target: &str) {
    tracing::info!(target: "hematite", "🔧 Applying {} to {}", fix_id.cyan(), target);
}

/// Log a successful fix
pub fn log_fix_success(fix_id: &str, changes_count: u32) {
    FIXES_APPLIED.fetch_add(1, Ordering::SeqCst);
    tracing::info!(
        target: "hematite",
        "{}",
        format!("✅ {} applied ({} changes)", fix_id, changes_count).green()
    );
}

/// Log a successful fix with file path context
pub fn log_fix_success_with_path(fix_id: &str, changes_count: u32, file_path: &str) {
    FIXES_APPLIED.fetch_add(1, Ordering::SeqCst);
    // Extract just the filename from the path for cleaner output
    let file_name = std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);
    tracing::info!(
        target: "hematite",
        "{}",
        format!("✅ {} applied ({} changes) → {}", fix_id, changes_count, file_name).green()
    );
}

/// Log a failed fix
pub fn log_fix_failed(fix_id: &str, error: &str) {
    FIXES_FAILED.fetch_add(1, Ordering::SeqCst);
    tracing::error!(
        target: "hematite",
        "{}",
        format!("❌ {} failed: {}", fix_id, error).red()
    );
}

/// Log a warning
pub fn log_warning(message: &str) {
    WARNINGS_COUNT.fetch_add(1, Ordering::SeqCst);
    tracing::warn!(target: "hematite", "⚠️  {}", message.yellow());
}

/// Log debug information
pub fn log_debug(message: &str) {
    tracing::debug!(target: "hematite", "🔍 {}", message);
}

/// Increment files processed counter
pub fn increment_files_processed() {
    FILES_PROCESSED.fetch_add(1, Ordering::SeqCst);
}

/// Log the session summary
pub fn log_session_summary(duration_secs: f64) {
    let files = FILES_PROCESSED.load(Ordering::SeqCst);
    let applied = FIXES_APPLIED.load(Ordering::SeqCst);
    let failed = FIXES_FAILED.load(Ordering::SeqCst);
    let warnings = WARNINGS_COUNT.load(Ordering::SeqCst);
    
    println!();
    println!("{}", "═".repeat(60).cyan());
    println!("{}", "SESSION SUMMARY".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
    println!();
    
    println!("{}: {}", "Files Processed".bold(), files.to_string().white());
    
    if applied > 0 {
        println!("{}: {}", "Fixes Applied".bold(), applied.to_string().green());
    } else {
        println!("{}: {}", "Fixes Applied".bold(), "0".dimmed());
    }
    
    if failed > 0 {
        println!("{}: {}", "Fixes Failed".bold(), failed.to_string().red());
    }
    
    if warnings > 0 {
        println!("{}: {}", "Warnings".bold(), warnings.to_string().yellow());
    }
    
    println!("{}: {:.2}s", "Duration".bold(), duration_secs);
    
    if failed == 0 && applied > 0 {
        println!();
        println!("{}", "✅ All fixes applied successfully!".green().bold());
    } else if failed > 0 {
        println!();
        println!("{}", "⚠️  Some fixes failed. Check the log for details.".yellow());
    } else if applied == 0 {
        println!();
        println!("{}", "ℹ️  No issues detected that require fixing.".dimmed());
    }
    
    println!();
}

/// Reset all counters (useful for testing)
pub fn reset_counters() {
    FILES_PROCESSED.store(0, Ordering::SeqCst);
    FIXES_APPLIED.store(0, Ordering::SeqCst);
    FIXES_FAILED.store(0, Ordering::SeqCst);
    WARNINGS_COUNT.store(0, Ordering::SeqCst);
}

/// Get statistics as a struct (for JSON output)
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionStats {
    pub session_id: Option<String>,
    pub files_processed: u32,
    pub fixes_applied: u32,
    pub fixes_failed: u32,
    pub warnings: u32,
    pub duration_secs: f64,
}

impl SessionStats {
    pub fn collect(duration_secs: f64) -> Self {
        Self {
            session_id: get_session_id(),
            files_processed: FILES_PROCESSED.load(Ordering::SeqCst),
            fixes_applied: FIXES_APPLIED.load(Ordering::SeqCst),
            fixes_failed: FIXES_FAILED.load(Ordering::SeqCst),
            warnings: WARNINGS_COUNT.load(Ordering::SeqCst),
            duration_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let session_id = create_session();
        assert_eq!(session_id.len(), 8);
    }

    #[test]
    fn test_log_level_filter() {
        let quiet = LogLevel::Quiet.to_filter();
        let normal = LogLevel::Normal.to_filter();
        let verbose = LogLevel::Verbose.to_filter();
        // Just verify they don't panic
    }

    #[test]
    fn test_counters() {
        reset_counters();
        assert_eq!(FILES_PROCESSED.load(Ordering::SeqCst), 0);
        
        increment_files_processed();
        assert_eq!(FILES_PROCESSED.load(Ordering::SeqCst), 1);
        
        reset_counters();
        assert_eq!(FILES_PROCESSED.load(Ordering::SeqCst), 0);
    }
}
