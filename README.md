<p align="center">
  <img src="https://img.shields.io/badge/League%20of%20Legends-Modding-C89B3C?style=for-the-badge&logo=riotgames&logoColor=white" alt="League Modding">
  <img src="https://img.shields.io/badge/Built%20with-Tauri%202.0-24C8D8?style=for-the-badge&logo=tauri&logoColor=white" alt="Tauri">
  <img src="https://img.shields.io/badge/Rust-Backend-DEA584?style=for-the-badge&logo=rust&logoColor=black" alt="Rust">
  <img src="https://img.shields.io/badge/React-TypeScript-61DAFB?style=for-the-badge&logo=react&logoColor=black" alt="React">
</p>

<h1 align="center">🔥 FLINT</h1>
<h3 align="center">League of Legends Asset Extractor & Modding IDE</h3>

<p align="center">
  <em>A powerful, modern tool for extracting and modifying League of Legends champion skins and assets.</em>
</p>

---

## What is Flint?

Flint is a desktop application that lets you:
- **Extract** champion skins and assets from League of Legends game files
- **Preview** 3D models, textures, BIN files, animations, and particle effects in real-time
- **Edit** game property files (BIN, LuaBin, TroyBin) with syntax highlighting
- **Recolor** textures with smart filtering and multiple blending modes
- **Export** custom mods compatible with popular mod managers (Fantome, ModPkg, LTK Manager)
- **Browse** the entire game archive with instant lazy loading and background indexing
- **Validate & Fix** mods with Hematite v2 rule engine integration

Built with **Rust** (Tauri 2) backend for performance and **React** + **TypeScript** frontend for a modern UI.

---

## ✨ Key Features

### 🎬 Animated Loading Screen Creator
Create custom animated loading screens from video files:
- **Video-to-Spritesheet**: Converts MP4/WebM to optimized spritesheets
- **16k Texture Budget**: Automatic grid optimization for WebGL limits
- **Frame Control**: Adjustable FPS, trimming, and scaling
- **Live Preview**: See your animation before exporting
- **Auto-injection**: Automatically patches game UI BIN files

### 🌐 WAD Explorer (Enhanced)
Browse the entire game's WAD archive library in a virtual filesystem with:
- **Instant Loading**: Lazy chunk loading on folder expand (no upfront indexing)
- **Background Indexing**: Optional full-game search with multi-threaded bulk processing
- **Smart Detection**: Magic-byte recognition for LuaBin, TroyBin, BIN, and image formats
- **Preview Anywhere**: Open any file directly from WAD without extracting

### 🖼️ Asset Preview
- **3D Models**: SKN/SKL meshes with textures, animations (ANM), and skeleton visualization
- **Textures**: DDS/TEX decoding with BC1, BC3, BC5, ETC format support
- **BIN Editor**: VS Code-style Monaco editor with syntax highlighting and bracket matching
- **LuaBin Preview**: Decompiled Lua scripts with syntax highlighting
- **TroyBin Preview**: Particle effect definitions with INI syntax highlighting
- **Hex Viewer**: Binary file inspection with offset display

### 🎨 Texture Recoloring
Batch recolor textures with **Hue Shift**, **Colorize**, or **Grayscale + Tint** modes. Smart filtering automatically skips distortion maps and preserves transparency.

### 💾 Checkpoint System
Create named snapshots of your project. Restore to any checkpoint instantly or compare changes between versions.

### 🔧 Hematite v2 Integration
Built-in mod validation and fixing powered by [Hematite](https://github.com/LeagueToolkit/Hematite):
- **Rule Engine**: Detects common mod issues (missing shaders, broken references, etc.)
- **Auto-Fix**: One-click fixes for shader paths, material properties, and more
- **Remote Config**: Auto-updates validation rules from GitHub
- **Trait-Based API**: Modular architecture with BinProvider, HashProvider, WadProvider

### 🎨 Jade & Quartz Integration
Optional integration with external League modding tools:
- **Jade**: Alternative BIN parser with advanced editing features
- **Quartz**: Visual paint mode for texture editing via interop messaging
- **Dual Engine**: Choose between Flint's LTK or Jade for BIN parsing
- **Seamless Handoff**: JSON-based interop for cross-tool workflows

### 📤 Mod Export
Export to `.fantome` (cslol-manager), `.modpkg` (League Mod Tools), or sync to **LTK Manager** with:
- **Refathering**: Custom asset paths (`ASSETS/{creator}/{project}/`) to prevent mod conflicts
- **BIN Concatenation**: Merge linked BINs for better compatibility
- **Thumbnail Embedding**: Auto-include custom 256x256 WebP thumbnails
- **LTK Manager Sync**: One-click install to mod launcher library + auto-profile addition

### 🗂️ Multi-Tab Workspace
Work on multiple projects simultaneously. Open project tabs, individual WAD sessions, and the WAD Explorer side-by-side without losing state.

### ⚡ High-Performance Hash Resolution
- **LMDB Cache**: Lightning Memory-Mapped Database stores 4M+ hash mappings
- **1 GB Virtual Address Space**: Memory-mapped for instant lookups (~5-20 MB real RAM)
- **Parallel Processing**: Multi-threaded BIN conversion with rayon
- **DataDragon API**: Auto-fetches champion/skin metadata from Riot's CDN

**[📖 See all features in detail](docs/FEATURES.md)**

---

## 🚀 Quick Start

### Prerequisites
- **Rust** 1.75+ ([Install](https://rustup.rs/))
- **Node.js** v20+ ([Install](https://nodejs.org/))
- **Windows 10+**

### Installation

```bash
# Clone the repository
git clone https://github.com/LeagueToolkit/Flint
cd "Flint - Asset Extractor"

# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

### Building for Production

```bash
# Create optimized build with NSIS installer
npm run tauri build
```

Installer output: `src-tauri/target/release/bundle/nsis/Flint_{version}_x64-setup.exe`

---

## 🛠️ Tech Stack

### Architecture
- **Workspace Crates**: Binary crate (`src-tauri/`) + library crate (`flint-ltk`)
- **Separation**: Tauri commands in binary, all LTK business logic in `flint-ltk`
- **Modules**: 14 domain modules (bin, wad, hash, mesh, audio, repath, export, etc.)

| Layer | Technology |
|-------|------------|
| **Frontend** | React 18, TypeScript, Vite 5 |
| **State Management** | Zustand 4 ([docs](docs/STATE_MANAGEMENT.md)) |
| **Backend** | Rust, Tauri 2.0 |
| **3D Rendering** | Three.js, React Three Fiber |
| **Hash Resolution** | LMDB (via `heed`), `memmap2` |
| **Parallel Processing** | `rayon`, `tokio` |
| **LTK Core** | `league-toolkit` (git rev 6137083, v0.4 API) |
| **BIN Parsing** | `ltk_ritobin`, `ltk_meta` |
| **WAD Handling** | `ltk_file`, `memmap2` |
| **Texture Decoding** | `ltk_texture` (DDS/TEX with BC1/BC3/BC5) |
| **Mesh Parsing** | `ltk_mesh`, `ltk_anim` |
| **Mod Export** | `ltk_fantome`, `ltk_modpkg`, `ltk_mod_core` |
| **Validation** | `hematite-core`, `hematite-ltk` |
| **Champion Data** | DataDragon/CommunityDragon API |

**[🏗️ See full architecture](docs/ARCHITECTURE.md)**

---

## 📚 Documentation

- **[Features](docs/FEATURES.md)** - Comprehensive feature documentation
- **[Architecture](docs/ARCHITECTURE.md)** - Project structure and data flow
- **[State Management](docs/STATE_MANAGEMENT.md)** - Zustand store architecture
- **[BNK Editor & Texture Parsing](docs/BNK_EDITOR_AND_TEXTURE_PARSING.md)** - Audio and texture technical details

---

## ✅ Status

| Feature | Status |
|---------|--------|
| **3D Model Preview** | ✅ Working (SKN/SKL/SCB/SCO) |
| **Animation Preview** | ✅ Working (ANM) |
| **Animated Loading Screens** | ✅ Working (Video→spritesheet) |
| **BIN Editing** | ✅ Working (Full read/write with bracket validation) |
| **LuaBin Preview** | ✅ Working (Decompiled Lua with syntax highlighting) |
| **TroyBin Preview** | ✅ Working (Particle effects with INI syntax) |
| **Refathering** | ✅ Working (Asset path rewriting) |
| **BIN Concatenation** | ✅ Working (Linked BIN merging) |
| **WAD Explorer** | ✅ Working (Lazy loading + background indexing) |
| **LMDB Hash Cache** | ✅ Working (Memory-mapped DB) |
| **Hematite v2** | ✅ Working (Validation & auto-fix) |
| **Jade/Quartz Integration** | ✅ Working (Dual BIN engine + interop) |
| **LTK Manager Sync** | ✅ Working (One-click mod install) |
| **Sound Bank Editing** | 🔜 Planned (BNK/WPK) |

---

## 🎨 Theming

Flint supports custom color themes! Copy [src/themes/default.css](src/themes/default.css) and modify the CSS variables:

```css
:root {
  --accent-primary: #your-color;
  --accent-secondary: #your-secondary-color;
  /* ... */
}
```

---

## 🤝 Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

---

## 📜 License

This project is for educational purposes. League of Legends and all related assets are property of Riot Games.

---

<p align="center">
  <strong>Made with ❤️ for the League modding community</strong>
</p>
