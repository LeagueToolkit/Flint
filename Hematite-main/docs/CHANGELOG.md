# Changelog

All notable changes to Hematite will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added
- Initial project structure with Tauri 2.0
- Documentation system (ARCHITECTURE.md, CONFIG_SCHEMA.md, TODO.md, CONTRIBUTING.md)
- Module scaffolding for config, analyzer, fixer, commands
- Dark glassmorphism CSS template
- GitHub Actions workflow skeleton
- **REFERENCE_ANALYSIS.md** - Comprehensive analysis of Python reference code
- **Config System** - Complete JSON-driven fix configuration:
  - `schema.rs` with FixConfig, FixRule, DetectionRule, TransformAction enums
  - `fetcher.rs` with GitHub fetch + embedded fallback
  - `cache.rs` with 1-hour TTL local caching
  - `fix_config.json` with 7 active fixes
- 6 detection types: missing_or_wrong_field, field_hash_exists, string_extension_not_in_wad, recursive_string_extension_not_in_wad, entry_type_exists_any, bnk_version_not_in
- 8 action types: ensure_field, rename_hash, replace_string_extension, remove_from_wad, change_field_type, ensure_field_with_context, regex_replace, regex_rename_field
- **Fixer Engine** - Complete transform implementation (Stage 6):
  - `ChangeFieldType` - 8 type conversions (vec3→vec4, link→string, etc.)
  - `RegexRenameField` - Regex-based field renaming with capture groups
  - Helper functions: `matches_field_pattern`, `property_matches_type`, `convert_property_type`, `compute_fnv1a_hash`
  - 6 new unit tests for helper functions
- **Analyzer Engine** - File scanning and issue detection (Stage 4):
  - `wad_cache.rs` - WAD file parsing with `ltk_wad::Wad::mount()`, xxhash64 path hashing
  - `bin_parser.rs` - BIN file parsing with `ltk_meta::BinTree::from_reader()`, recursive string extraction
  - `scanner.rs` - File type detection (.fantome, .zip, .wad.client) and directory scanning
  - `detector.rs` - Working detection for `StringExtensionNotInWad` and `RecursiveStringExtensionNotInWad`
- **League Toolkit Integration** (Stage 5 partial):
  - `league-toolkit` path dependency with `wad` and `meta` features
  - `xxhash-rust` for path hashing (League's path hash algorithm)
  - `indexmap` for BinTree compatibility
- **Smart Pattern Matching** - Future-proof detection system (Stage 3.5):
  - 3 new detection types: `pattern_match`, `regex_match`, `regex_multi_match`
  - 4 new transform actions: `change_field_type`, `ensure_field_with_context`, `regex_replace`, `regex_rename_field`
  - `champion_list.rs` - Champion/subchamp context for filtering (e.g., ZedShadow → subchamp)
  - `regex = 1.10` dependency for flexible pattern matching
- **Detection System Improvements** (2026-01-18):
  - `value_matches()` helper - Compare PropertyValueEnum against expected JSON values
  - `entry_type` filter for `EnsureField` transform - Only apply to matching object types
  - `FieldHashExists` detection - Recursive path matching with wildcard support (`SamplerValues.*.TextureName`)
  - `EntryTypeExistsAny` detection - Champion data type detection for bin remover

### Changed
- **CONFIG_SCHEMA.md** - Finalized with complete detection/action type definitions
- **TODO.md** - Stages 2, 3, 3.5, 4, 5, 6 complete; Stage 7 in progress
- **schema.rs** - Expanded with regex and context-aware detection types
- **applier.rs** - Fully implemented with 8 transform actions and helper functions
- Test count: 44 → 50 → 60 tests passing
- VFX Shape fix deprecated (Patch 14.1 is 2 years old) - no custom handlers needed

### Fixed
- **Healthbar Detection** (2026-01-18):
  - Detection now searches by embed CLASS TYPE (`CharacterHealthBarDataRecord`) instead of field NAME
  - Added `expected_value` comparison - triggers when value is wrong (9 vs expected 12)
  - Applier now updates existing fields instead of only adding missing ones
  - Added `entry_type` filter to prevent applying fix to all objects (was 43 changes, now 1 per entry)

## [0.1.0] - 2026-01-11

### Added
- Project initialization
- Cargo dependency setup with league-toolkit
- Tauri 2.0 vanilla template
- Frontend and backend folder structure

---

**Last Updated:** 2026-01-18
