//! Hematite - League of Legends Skin Fixer Library
//!
//! This library provides functionality for detecting and fixing common issues
//! in League of Legends skin files (.fantome, .wad.client, .bin).
//!
//! # Modules
//! - `config` - Configuration schema and fetching
//! - `analyzer` - File analysis (WAD, BIN parsing, detection)
//! - `fixer` - Fix application logic
//! - `processor` - End-to-end file processing (fantome, WAD, BIN)
//! - `logging` - Structured logging system
//!
//! # Usage
//! This library can be used in two modes:
//! 1. **CLI Mode** - Standalone command-line tool (`hematite-cli`)
//! 2. **UI Mode** - Tauri-based GUI application (requires `tauri-ui` feature)

pub mod config;
pub mod analyzer;
pub mod fixer;
#[cfg(feature = "wad-writer")]
pub mod processor;
pub mod logging;

#[cfg(feature = "tauri-ui")]
pub mod commands;

#[cfg(feature = "tauri-ui")]
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Run the Tauri application (UI mode)
#[cfg(feature = "tauri-ui")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Placeholder run function when Tauri is not available
#[cfg(not(feature = "tauri-ui"))]
pub fn run() {
    eprintln!("Tauri UI is not enabled. Use hematite-cli instead.");
    eprintln!("To enable UI: cargo build --features tauri-ui");
}

