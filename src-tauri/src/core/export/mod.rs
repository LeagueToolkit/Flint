//! Export module for creating distributable mod packages

/// Generate a default filename for the fantome package
/// (Convenience wrapper around ltk_fantome)
pub fn generate_fantome_filename(name: &str, version: &str) -> String {
    let slug = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    format!("{}_{}.fantome", slug, version)
}
