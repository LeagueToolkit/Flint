# Hematite 🔴

> A high-performance League of Legends custom skin fixer built with Tauri 2.0 + Rust

![Status](https://img.shields.io/badge/status-backend_complete-green)
![Tauri](https://img.shields.io/badge/tauri-2.0-blue)
![Rust](https://img.shields.io/badge/rust-1.70+-orange)
![Tests](https://img.shields.io/badge/tests-60%20passing-brightgreen)

## 🎯 Project Vision

Hematite is a **remote-config-driven** skin fixer that:
- ✅ Analyzes League of Legends custom skins for common issues
- ✅ Fixes broken health bars, white models, black icons, and VFX issues
- ✅ Supports both **single file** and **batch processing** (CSLoL Manager folders)
- ✅ Updates fix logic via **GitHub-hosted JSON** (no app recompilation needed)

## ✨ Features

| Feature | Status |
|---------|--------|
| BIN file parsing/writing | ✅ Complete |
| WAD file reading/writing | ✅ Complete |
| Config-driven fix detection | ✅ Complete |
| Hash dictionary loading | ✅ Complete |
| 8 transform actions | ✅ Complete |
| Tauri commands | ✅ Complete |
| Frontend UI | � In Progress |

### Supported Fixes

1. **Healthbar Fix** - Adds/updates `UnitHealthBarStyle` field (detects by embed class type)
2. **Map Geometry** - Fixes white/broken environment visuals
3. **DDS→TEX Conversion** - Updates deprecated texture references
4. **Icon Path Fix** - Corrects black/missing item/ability icons
5. **Hash Rename** - Updates deprecated field names
6. **Remove Deprecated** - Removes obsolete files from WAD

## �📁 Project Structure

```
hematite/
├── docs/                    # Documentation
│   ├── ARCHITECTURE.md      # System design decisions
│   ├── CONFIG_SCHEMA.md     # JSON config documentation
│   └── REFERENCE_ANALYSIS.md # Python fix analysis
├── config/                  # Remote config files
│   ├── fix_config.json      # Fix rule definitions
│   └── champion_list.json   # Champion metadata
├── src/                     # Frontend (Vanilla JS)
│   ├── index.html
│   ├── main.js              # Tauri command calls
│   └── styles.css           # Dark glassmorphism theme
├── src-tauri/               # Rust backend
│   ├── src/
│   │   ├── config/          # Remote config system
│   │   │   ├── schema.rs    # Config type definitions
│   │   │   ├── fetcher.rs   # HTTP + cache fallback
│   │   │   └── cache.rs     # Local cache with TTL
│   │   ├── analyzer/        # File scanning & detection
│   │   │   ├── bin_parser.rs    # BIN read/write
│   │   │   ├── wad_cache.rs     # WAD path indexing
│   │   │   ├── wad_writer.rs    # WAD modification
│   │   │   ├── detector.rs      # Issue detection
│   │   │   ├── scanner.rs       # File discovery
│   │   │   ├── hash_dict.rs     # Hash resolution
│   │   │   └── hash_downloader.rs # CD hash fetching
│   │   ├── fixer/           # Fix application logic
│   │   │   └── applier.rs   # Transform implementations
│   │   └── commands/        # Tauri IPC commands
│   │       ├── analyze.rs   # analyze_path, get_fix_config
│   │       └── fix.rs       # apply_fixes
│   └── Cargo.toml
└── Reference-Code/          # Python reference implementations
```

## 🚀 Current Status

**Stage:** 6 - Fixer Engine ✅

**Completed:**
- [x] Tauri 2.0 project initialization
- [x] Config system (schema, fetcher, cache)
- [x] Analyzer engine (scanner, detector, WAD cache)
- [x] Hash dictionary + downloader
- [x] BIN parsing & writing (ltk_meta)
- [x] WAD reading & writing (ltk_wad)
- [x] Fixer with 8 transform actions
- [x] Tauri commands (analyze_path, apply_fixes, get_fix_config)
- [x] 60 unit tests passing

**Next Steps:**
- [ ] Stage 7: Frontend UI with drag & drop
- [ ] Wire up Tauri commands to UI
- [ ] Add batch processing UI
- [ ] Progress reporting

## 🛠️ Tech Stack

- **Frontend:** Vanilla HTML/CSS/JS (Dark Glassmorphism UI)
- **Backend:** Rust (Tauri 2.0)
- **Parsing:** [league-toolkit](../league-toolkit-main) (local dependency)
- **Config:** Remote JSON from GitHub

## 🔧 Development

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build

# Run tests
cd src-tauri && cargo test --lib
```

## 🤝 Dependencies

**Rust Libraries:**
- `tauri` (2.0) - Desktop framework
- `serde` + `serde_json` - JSON serialization
- `reqwest` - HTTP client for config fetching
- `regex` - Pattern matching
- `anyhow` - Error handling
- `indexmap` - Ordered hash maps
- `league-toolkit` - League file parsing (local)
  - `ltk_meta` - BIN file handling
  - `ltk_wad` - WAD file handling

## 📚 Documentation

- [Architecture Overview](docs/ARCHITECTURE.md)
- [Config Schema Design](docs/CONFIG_SCHEMA.md)
- [Reference Analysis](docs/REFERENCE_ANALYSIS.md)

## 💎 Why "Hematite"?

Hematite is the primary ore of iron. When iron oxidizes, it becomes *rust*. Since this tool is built in Rust and "cleans up" broken skins, the name is a fitting metaphor.

---

**Last Updated:** 2026-01-18
**Current Version:** 0.1.0-dev
