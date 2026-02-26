//! Commands Module
//!
//! Tauri IPC layer between frontend and backend.
//! Exposes analyze and fix commands.

pub mod analyze;
pub mod fix;

pub use analyze::{analyze_path, get_fix_config};
pub use fix::{apply_fixes, ApplyFixesRequest};
