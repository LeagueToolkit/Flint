# Config Schema Design

> **STATUS:** ✅ Implemented - Pure JSON-driven (no custom handlers)

## Purpose

The config schema defines how Hematite detects and fixes skin issues. The config is:
1. Fetched from GitHub on startup
2. Cached locally with 1-hour TTL
3. Falls back to embedded config if network fails

---

## Schema Structure

```json
{
  "version": "1.0.0",
  "last_updated": "2026-01-11",
  "fixes": {
    "[fix_id]": {
      "name": "Human-readable name",
      "description": "What this fix does",
      "enabled": true,
      "severity": "low|medium|high|critical",
      "detect": { /* detection rule */ },
      "apply": { /* fix action */ }
    }
  }
}
```

---

## Detection Types (6)

| Type | Description | Used By |
|------|-------------|---------|
| `missing_or_wrong_field` | Field missing or wrong value | healthbar_fix |
| `field_hash_exists` | Field hash exists (shouldn't) | staticmat x2 |
| `string_extension_not_in_wad` | String ends with ext, not in WAD | black_icons |
| `recursive_string_extension_not_in_wad` | Deep search for ext not in WAD | dds_to_tex |
| `entry_type_exists_any` | BIN contains entry types | champion_bin_remover |
| `bnk_version_not_in` | BNK version not in allowed list | bnk_remover |

---

## Action Types (4)

| Type | Description | Used By |
|------|-------------|---------|
| `ensure_field` | Add/set field value | healthbar_fix |
| `rename_hash` | Rename field's hash | staticmat x2 |
| `replace_string_extension` | Replace file extension in string | black_icons, dds_to_tex |
| `remove_from_wad` | Remove file from WAD | champion_bin, bnk_remover |

---

## BIN Data Types

| Type | JSON Value | Example |
|------|------------|---------|
| `u8` | `12` | UnitHealthBarStyle |
| `f32` | `1.5` | Radius |
| `string` | `"path.tex"` | TexturePath |
| `vec3` | `[1.0, 2.0, 3.0]` | ConstantValue |
| `embed` | nested object | HealthBarData |

---

## Active Fixes (7)

1. **healthbar_fix** - Adds UnitHealthBarStyle field (value: 12)
2. **staticmat_texturepath** - Renames TextureName → TexturePath
3. **staticmat_samplername** - Renames SamplerName → TextureName
4. **black_icons** - Converts .dds icon refs → .tex
5. **dds_to_tex** - Converts all .dds texture refs → .tex
6. **champion_bin_remover** - Removes champion data files
7. **bnk_remover** - Removes BNK files with version ≠ 134/145

---

## Deprecated

**VFX Shape Fix** - Not implemented. Patch 14.1 was 2 years ago; any mods still affected are rare. Future complex fixes requiring custom handlers will be added in app updates.

---

## Rust Implementation

See `src-tauri/src/config/`:
- `schema.rs` - Type definitions
- `fetcher.rs` - HTTP fetch + fallback
- `cache.rs` - Local caching

**Entry point:** `config::get_config()` returns `Result<FixConfig>`

---

**Last Updated:** 2026-01-11
