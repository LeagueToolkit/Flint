use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudData {
    #[serde(rename = "type")]
    pub hud_type: String,
    pub version: u32,
    pub linked: Vec<String>,
    pub entries: HashMap<String, HudEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub enabled: bool,
    #[serde(rename = "Layer")]
    pub layer: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<HudPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "TextureData")]
    pub texture_data: Option<TextureData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Scene")]
    pub scene: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudPosition {
    #[serde(rename = "UIRect")]
    pub ui_rect: UiRect,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Anchors")]
    pub anchors: Option<Anchors>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiRect {
    pub position: Vec2,
    #[serde(rename = "Size")]
    pub size: Vec2,
    #[serde(rename = "SourceResolutionWidth")]
    pub source_resolution_width: u16,
    #[serde(rename = "SourceResolutionHeight")]
    pub source_resolution_height: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anchors {
    #[serde(rename = "Anchor")]
    pub anchor: Vec2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureData {
    #[serde(rename = "mTextureName")]
    pub texture_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mTextureUV")]
    pub texture_uv: Option<Vec4>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vec4 {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

/// Parse a .py HUD file into structured data
pub fn parse_hud_file(content: &str) -> Result<HudData> {
    let lines: Vec<&str> = content.lines().collect();
    let mut data = HudData {
        hud_type: String::new(),
        version: 0,
        linked: Vec::new(),
        entries: HashMap::new(),
    };

    let mut current_entry: Option<(String, HudEntry)> = None;
    let mut in_entries_block = false;
    let mut entry_indent = 0;
    let mut in_position = false;
    let mut in_ui_rect = false;
    let mut in_anchors = false;
    let mut in_texture_data = false;

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let indent = line.len() - line.trim_start().len();

        // Parse top-level properties
        if trimmed.contains("type: string = \"PROP\"") {
            data.hud_type = "PROP".to_string();
            continue;
        }

        if let Some(version_str) = trimmed.strip_prefix("version: u32 = ") {
            data.version = version_str.trim().parse().unwrap_or(0);
            continue;
        }

        if trimmed.contains("linked: list[string] = {}") {
            continue;
        }

        if trimmed.contains("entries: map[hash,embed] = {") {
            in_entries_block = true;
            continue;
        }

        // Parse entry definitions
        if in_entries_block {
            // Check for new entry
            if let Some(captures) = parse_entry_header(trimmed) {
                // Save previous entry
                if let Some((key, entry)) = current_entry.take() {
                    data.entries.insert(key, entry);
                }

                let (entry_key, entry_type) = captures;
                current_entry = Some((
                    entry_key.to_string(),
                    HudEntry {
                        name: entry_key.to_string(),
                        entry_type: entry_type.to_string(),
                        enabled: true,
                        layer: 0,
                        position: None,
                        texture_data: None,
                        scene: None,
                        extra: HashMap::new(),
                    },
                ));
                entry_indent = indent;
                in_position = false;
                in_ui_rect = false;
                in_anchors = false;
                in_texture_data = false;
                continue;
            }

            // Parse entry properties
            if let Some((_, ref mut entry)) = current_entry {
                if indent > entry_indent {
                    parse_entry_property(trimmed, entry, &mut in_position, &mut in_ui_rect, &mut in_anchors, &mut in_texture_data);
                }
            }
        }
    }

    // Save last entry
    if let Some((key, entry)) = current_entry {
        data.entries.insert(key, entry);
    }

    Ok(data)
}

fn parse_entry_header(line: &str) -> Option<(&str, &str)> {
    // Match: "ClientStates/..." = UiElementXXX {
    if !line.contains(" = ") || !line.contains("ClientStates/") {
        return None;
    }

    let parts: Vec<&str> = line.split(" = ").collect();
    if parts.len() != 2 {
        return None;
    }

    let key = parts[0].trim().trim_matches('"');
    let type_part = parts[1].trim();
    let entry_type = type_part.split_whitespace().next()?;

    Some((key, entry_type))
}

fn parse_entry_property(
    line: &str,
    entry: &mut HudEntry,
    in_position: &mut bool,
    in_ui_rect: &mut bool,
    in_anchors: &mut bool,
    in_texture_data: &mut bool,
) {
    // Name
    if let Some(name) = parse_string_value(line, "name: string = ") {
        entry.name = name;
        return;
    }

    // Enabled
    if line.contains("enabled: bool = ") {
        entry.enabled = line.contains("true");
        return;
    }

    // Layer
    if let Some(layer) = parse_u32_value(line, "Layer: u32 = ") {
        entry.layer = layer;
        return;
    }

    // Scene
    if let Some(scene) = parse_string_value(line, "Scene: link = ") {
        entry.scene = Some(scene);
        return;
    }

    // Position block
    if line.contains("position: pointer = UiPositionRect {") {
        entry.position = Some(HudPosition {
            ui_rect: UiRect {
                position: Vec2 { x: 0.0, y: 0.0 },
                size: Vec2 { x: 0.0, y: 0.0 },
                source_resolution_width: 1600,
                source_resolution_height: 1200,
            },
            anchors: None,
        });
        *in_position = true;
        return;
    }

    if *in_position {
        // UIRect block
        if line.contains("UIRect: embed = UiElementRect {") {
            *in_ui_rect = true;
            return;
        }

        if *in_ui_rect {
            // Position vec2
            if let Some((x, y)) = parse_vec2(line, "position: vec2 = ") {
                if let Some(ref mut pos) = entry.position {
                    pos.ui_rect.position = Vec2 { x, y };
                }
                return;
            }

            // Size vec2
            if let Some((x, y)) = parse_vec2(line, "Size: vec2 = ") {
                if let Some(ref mut pos) = entry.position {
                    pos.ui_rect.size = Vec2 { x, y };
                }
                return;
            }

            // Source resolution
            if let Some(width) = parse_u16_value(line, "SourceResolutionWidth: u16 = ") {
                if let Some(ref mut pos) = entry.position {
                    pos.ui_rect.source_resolution_width = width;
                }
                return;
            }

            if let Some(height) = parse_u16_value(line, "SourceResolutionHeight: u16 = ") {
                if let Some(ref mut pos) = entry.position {
                    pos.ui_rect.source_resolution_height = height;
                }
                return;
            }
        }

        // Anchors block
        if line.contains("Anchors: pointer = AnchorSingle {") {
            *in_anchors = true;
            if entry.position.is_some() {
                entry.position.as_mut().unwrap().anchors = Some(Anchors {
                    anchor: Vec2 { x: 0.5, y: 1.0 },
                });
            }
            return;
        }

        if *in_anchors {
            if let Some((x, y)) = parse_vec2(line, "Anchor: vec2 = ") {
                if let Some(ref mut pos) = entry.position {
                    if let Some(ref mut anchors) = pos.anchors {
                        anchors.anchor = Vec2 { x, y };
                    }
                }
                return;
            }
        }
    }

    // TextureData block
    if line.contains("TextureData: pointer = AtlasData {") {
        entry.texture_data = Some(TextureData {
            texture_name: String::new(),
            texture_uv: None,
        });
        *in_texture_data = true;
        return;
    }

    if *in_texture_data {
        if let Some(tex_name) = parse_string_value(line, "mTextureName: string = ") {
            if let Some(ref mut tex_data) = entry.texture_data {
                tex_data.texture_name = tex_name;
            }
            return;
        }

        if let Some((x1, y1, x2, y2)) = parse_vec4(line, "mTextureUV: vec4 = ") {
            if let Some(ref mut tex_data) = entry.texture_data {
                tex_data.texture_uv = Some(Vec4 { x1, y1, x2, y2 });
            }
            return;
        }
    }
}

fn parse_string_value(line: &str, prefix: &str) -> Option<String> {
    if !line.contains(prefix) {
        return None;
    }
    let after_prefix = line.split(prefix).nth(1)?;
    let value = after_prefix.trim().trim_matches('"');
    Some(value.to_string())
}

fn parse_u32_value(line: &str, prefix: &str) -> Option<u32> {
    if !line.contains(prefix) {
        return None;
    }
    let after_prefix = line.split(prefix).nth(1)?;
    after_prefix.trim().parse().ok()
}

fn parse_u16_value(line: &str, prefix: &str) -> Option<u16> {
    if !line.contains(prefix) {
        return None;
    }
    let after_prefix = line.split(prefix).nth(1)?;
    after_prefix.trim().parse().ok()
}

fn parse_vec2(line: &str, prefix: &str) -> Option<(f32, f32)> {
    if !line.contains(prefix) {
        return None;
    }
    // Format: "position: vec2 = { 100, 200 }"
    let after_prefix = line.split(prefix).nth(1)?;
    let coords_str = after_prefix.trim().trim_matches('{').trim_matches('}').trim();
    let parts: Vec<&str> = coords_str.split(',').collect();
    if parts.len() != 2 {
        return None;
    }
    let x = parts[0].trim().parse().ok()?;
    let y = parts[1].trim().parse().ok()?;
    Some((x, y))
}

fn parse_vec4(line: &str, prefix: &str) -> Option<(f32, f32, f32, f32)> {
    if !line.contains(prefix) {
        return None;
    }
    let after_prefix = line.split(prefix).nth(1)?;
    let coords_str = after_prefix.trim().trim_matches('{').trim_matches('}').trim();
    let parts: Vec<&str> = coords_str.split(',').collect();
    if parts.len() != 4 {
        return None;
    }
    let x1 = parts[0].trim().parse().ok()?;
    let y1 = parts[1].trim().parse().ok()?;
    let x2 = parts[2].trim().parse().ok()?;
    let y2 = parts[3].trim().parse().ok()?;
    Some((x1, y1, x2, y2))
}

/// Serialize HUD data back to .py format
/// Uses find-and-replace strategy to preserve original structure
pub fn serialize_hud_file(data: &HudData, original_content: &str) -> Result<String> {
    let mut modified = original_content.to_string();

    for (key, entry) in &data.entries {
        if let Some(ref pos) = entry.position {
            // Find the entry definition
            let search_key = format!("\"{}\"", key);

            // Find all occurrences and locate the actual element definition
            let mut entry_start = None;
            let mut search_idx = 0;

            while let Some(found_idx) = modified[search_idx..].find(&search_key) {
                let absolute_idx = search_idx + found_idx;

                // Check if this is an element definition (not a reference)
                let after_key = &modified[absolute_idx + search_key.len()..];
                if after_key.trim_start().starts_with("= UiElement") {
                    entry_start = Some(absolute_idx);
                    break;
                }

                search_idx = absolute_idx + 1;
            }

            if let Some(start_idx) = entry_start {
                let after_entry = &modified[start_idx..];

                // Find position line using regex-like pattern
                if let Some(pos_idx) = find_position_line(after_entry) {
                    let absolute_pos_idx = start_idx + pos_idx;

                    // Extract old position values and indentation
                    if let Some((indent, old_x, old_y, line_len)) = extract_position_values(&modified[absolute_pos_idx..]) {
                        // Build new position line
                        let new_x = pos.ui_rect.position.x.round() as i32;
                        let new_y = pos.ui_rect.position.y.round() as i32;
                        let new_line = format!("{}position: vec2 = {{ {}, {} }}", indent, new_x, new_y);

                        // Replace
                        let before = &modified[..absolute_pos_idx];
                        let after = &modified[absolute_pos_idx + line_len..];
                        modified = format!("{}{}{}", before, new_line, after);

                        tracing::debug!("Updated {} position: ({}, {}) -> ({}, {})", key, old_x, old_y, new_x, new_y);
                    }
                }
            }
        }
    }

    Ok(modified)
}

fn find_position_line(content: &str) -> Option<usize> {
    // Look for "position: vec2 = {" pattern
    content.find("position: vec2 = {")
}

fn extract_position_values(content: &str) -> Option<(String, i32, i32, usize)> {
    // Extract indent, x, y values and total line length
    let line_end = content.find('\n').unwrap_or(content.len());
    let line = &content[..line_end];

    // Get indentation
    let indent_len = line.len() - line.trim_start().len();
    let indent = " ".repeat(indent_len);

    // Extract coordinates
    let coords_start = line.find('{')?;
    let coords_end = line.find('}')?;
    let coords_str = &line[coords_start + 1..coords_end];
    let parts: Vec<&str> = coords_str.split(',').collect();

    if parts.len() != 2 {
        return None;
    }

    let x: i32 = parts[0].trim().parse().ok()?;
    let y: i32 = parts[1].trim().parse().ok()?;

    Some((indent, x, y, line.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_position_line() {
        let content = r#"        position: vec2 = { 100, 200 }
        Size: vec2 = { 50, 50 }"#;

        if let Some(idx) = find_position_line(content) {
            if let Some((indent, x, y, len)) = extract_position_values(&content[idx..]) {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
                assert_eq!(indent, "        ");
            }
        }
    }
}
