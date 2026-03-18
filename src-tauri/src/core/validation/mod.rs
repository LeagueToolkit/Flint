// Validation module exports
pub mod engine;

pub use engine::{validate_assets, extract_asset_references, ValidationReport, AssetReference};
