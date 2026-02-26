# Hematite TODO List

## ✅ Stage 1: Project Skeleton

- [x] Initialize Tauri 2.0 project
- [x] Create folder structure
- [x] Configure Cargo.toml dependencies
- [x] Set up module scaffolding

## ✅ Stage 1.5: Documentation System

- [x] Create README.md with project overview
- [x] Create docs/ARCHITECTURE.md
- [x] Create docs/CONFIG_SCHEMA.md
- [x] Create docs/TODO.md
- [x] Create docs/CHANGELOG.md
- [x] Create docs/CONTRIBUTING.md

## ✅ Stage 2: Reference Analysis

- [x] Read all Python files in Reference-Code/
- [x] Document each fix type (7 fixes identified)
- [x] Identify BIN fields and data types
- [x] Design unified JSON schema
- [x] Create REFERENCE_ANALYSIS.md

## ✅ Stage 3: Config System

- [x] Implement `config/schema.rs` (Rust structs for all fix types)
- [x] Implement `config/fetcher.rs` (GitHub HTTP client + cache fallback)
- [x] Implement `config/cache.rs` (Local storage with 1-hour TTL)
- [x] Create `fix_config.json` with 7 active fixes
- [x] Add error handling for network failures
- [x] `cargo check` passes

**Files Created:**
- `src/config/schema.rs` - FixConfig, FixRule, DetectionRule, TransformAction enums
- `src/config/fetcher.rs` - HTTP fetch with embedded fallback
- `src/config/cache.rs` - TTL-based local caching
- `fix_config.json` - Complete config with 7 fixes

## ✅ Stage 3.5: Smart Pattern Matching

- [x] Add `regex = 1.10` dependency
- [x] Add `PatternMatch` detection type (smart context-aware)
- [x] Add `RegexMatch` detection type (lazy/unknown fixes)
- [x] Add `RegexMultiMatch` detection type (complex multi-condition)
- [x] Add new transform actions:
  - [x] `ChangeFieldType` - vec3→vec4, link→string, etc.
  - [x] `EnsureFieldWithContext` - champion/subchamp-specific values
  - [x] `RegexReplace` - regex-based value replacement with capture groups
  - [x] `RegexRenameField` - regex-based field renaming
- [x] Create `champion_list.rs`:
  - [x] GitHub fetch + 1-week TTL cache
  - [x] Embedded fallback with 170+ champions
  - [x] Subchamp mapping (ZedShadow→Zed, etc.)
  - [x] Path parsing for context detection
- [x] 50 tests passing

**Files Created/Modified:**
- `src/config/schema.rs` - 3 new detection types, 4 new transforms
- `src/config/champion_list.rs` - Champion/subchamp context system

## ✅ Stage 4: Analyzer Engine

- [x] Implement `analyzer/wad_cache.rs`
  - [x] WadCache struct with xxhash64-based file index
  - [x] `from_wad()` using ltk_wad `Wad::mount()`
  - [x] `from_bytes()` for ZIP-embedded WADs
  - [x] Case-insensitive path lookups via xxhash64
- [x] Implement `analyzer/scanner.rs`
  - [x] Single file detection (.fantome, .zip, .wad.client)
  - [x] Directory recursion (CSLoL Manager structure)
  - [ ] Archive extraction to temp directory (TODO)
- [x] Implement `analyzer/bin_parser.rs`
  - [x] `parse_bin_file()` using ltk_meta `BinTree::from_reader()`
  - [x] `extract_all_strings()` for recursive string extraction
  - [x] Helper functions for property access
- [x] Implement `analyzer/detector.rs`
  - [x] ScanResult / DetectedIssue structs
  - [x] Working detection for `StringExtensionNotInWad`
  - [x] Working detection for `RecursiveStringExtensionNotInWad`
  - [x] Other rules stubbed (require hash dictionary for class_hash mapping)
- [ ] Write integration tests with real WAD files

## 🔄 Stage 5: League Toolkit Integration (PARTIAL)

**Note:** Full detection requires a hash dictionary mapping class_hash → entry_type name.
Currently working:
- [x] WAD file parsing with `ltk_wad::Wad::mount()`
- [x] BIN file parsing with `ltk_meta::BinTree::from_reader()`
- [x] String extension detection (dds→tex conversion)
- [ ] Entry type matching (requires hash dictionary)
- [ ] Pattern/regex matching (requires hash dictionary)

## ✅ Stage 6: Fixer Engine

- [x] Implement `fixer/applier.rs`
  - [x] `EnsureField` - Add/set field with optional parent embed creation
  - [x] `RenameHash` - Rename field hashes recursively
  - [x] `ReplaceStringExtension` - .dds→.tex conversion with WAD check
  - [x] `RemoveFromWad` - Mark files for WAD removal
  - [x] `ChangeFieldType` - Convert types (vec3→vec4, link→string)
  - [x] `RegexReplace` - Regex value replacement with capture groups
  - [x] `RegexRenameField` - Regex field renaming
  - [x] `EnsureFieldWithContext` - Champion-specific values
- [x] Helper functions:
  - [x] `matches_field_pattern` - Wildcard pattern matching
  - [x] `property_matches_type` - Type validation
  - [x] `convert_property_type` - 8 type conversions
  - [x] `compute_fnv1a_hash` - BIN field hash computation
- [x] 50 unit tests passing

**Files Updated:**
- `src/fixer/applier.rs` - Complete with 8 transform actions

## ⚪ Stage 7: Frontend UI

- [ ] Design dark glassmorphism theme
- [ ] Implement drag & drop zone
- [ ] Create analysis results table
- [ ] Add fix selection checkboxes
- [ ] Wire up Tauri command calls
- [ ] Add loading states and error toasts

## ⚪ Stage 8: Polish

- [ ] Add app icons
- [ ] Write user documentation
- [ ] Create GitHub Actions CI/CD
- [ ] Performance profiling
- [ ] Release v1.0.0

---

**Last Updated:** 2026-01-17
