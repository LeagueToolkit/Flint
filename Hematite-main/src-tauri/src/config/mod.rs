//! Configuration management module.
//!
//! This module handles the fix configuration system including:
//! - Schema definitions for fix rules
//! - Fetching configuration from GitHub
//! - Local caching with TTL
//! - Champion/subchamp lists for context-aware detection

pub mod cache;
pub mod champion_list;
pub mod fetcher;
pub mod schema;

// Re-export commonly used types
pub use schema::{
    BinDataType, ContextualValues, DetectionRule, FixConfig, FixRule, ParentEmbed,
    RegexCondition, Severity, TransformAction,
};

pub use champion_list::ChampionList;
pub use fetcher::get_config;
