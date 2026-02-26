//! Fixer Module
//!
//! Handles applying fixes to BIN files and WAD archives.
//! Uses config-driven transform actions to modify files.

pub mod applier;

pub use applier::{apply_transforms, AppliedFix, FailedFix, FixContext, FixResult};
