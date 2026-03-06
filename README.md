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
- **Preview** 3D models, textures, BIN files, and animations in real-time
- **Edit** game property files (BIN) with syntax highlighting
- **Recolor** textures with smart filtering and multiple blending modes
- **Export** custom mods compatible with popular mod managers (Fantome, Modpkg)
- **Browse** the entire game archive without extracting anything

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

### 🌐 WAD Explorer
Browse the entire game's WAD archive library in a virtual filesystem. Lazy-load chunks on demand, search with regex, and preview files instantly—all without extracting to disk.

### 🖼️ Asset Preview
- **3D Models**: SKN/SKL meshes with textures, animations (ANM), and skeleton visualization
- **Textures**: DDS/TEX decoding with BC1, BC3, ETC format support
- **BIN Editor**: VS Code-style syntax highlighting for game property files
- **Hex Viewer**: Binary file inspection with offset display

### 🎨 Texture Recoloring
Batch recolor textures with **Hue Shift**, **Colorize**, or **Grayscale + Tint** modes. Smart filtering automatically skips distortion maps and preserves transparency.

### 💾 Checkpoint System
Create named snapshots of your project. Restore to any checkpoint instantly or compare changes between versions.

### 📤 Mod Export
Export to `.fantome` (cslol-manager) or `.modpkg` (League Mod Tools) with:
- **Refathering**: Custom asset paths to prevent mod conflicts
- **BIN Concatenation**: Merge linked BINs for better compatibility
- Auto-update support with secure signature verification

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

| Layer | Technology |
|-------|------------|
| **Frontend** | React 18, TypeScript, Vite 5 |
| **State Management** | Zustand 4 ([docs](docs/STATE_MANAGEMENT.md)) |
| **Backend** | Rust, Tauri 2.0 |
| **3D Rendering** | Three.js, React Three Fiber |
| **Hash Resolution** | LMDB (via `heed`), `memmap2` |
| **Parallel Processing** | `rayon`, `tokio` |
| **BIN Parsing** | `ltk_ritobin`, `ltk_meta` |
| **WAD Handling** | `league-toolkit`, `memmap2` |
| **Texture Decoding** | `ltk_texture` (DDS/TEX/BC1/BC3) |
| **Mesh Parsing** | `ltk_mesh`, `ltk_anim` |
| **Mod Export** | `ltk_fantome`, `ltk_modpkg` |
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
| **BIN Editing** | ✅ Working (Full read/write) |
| **Refathering** | ✅ Working (Asset path rewriting) |
| **BIN Concatenation** | ✅ Working (Linked BIN merging) |
| **WAD Explorer** | ✅ Working (VFS browser) |
| **LMDB Hash Cache** | ✅ Working (Memory-mapped DB) |
| **Parallel BIN Conversion** | ✅ Working (Multi-threaded) |
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
