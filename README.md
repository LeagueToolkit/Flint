<!-- ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ -->
<!--                          F  L  I  N  T                                    -->
<!-- ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ -->

<p align="center">
  <img src="https://capsule-render.vercel.app/api?type=waving&color=0:991B1B,50:EF4444,100:F87171&height=220&section=header&text=FLINT&fontSize=100&fontColor=ffffff&fontAlignY=38&animation=twinkling&desc=League%20of%20Legends%20Modding%20IDE&descAlignY=62&descSize=18" alt="Flint banner"/>
</p>

<p align="center">
  <a href="https://github.com/RitoShark/Flint/releases/latest">
    <img src="https://img.shields.io/github/v/release/RitoShark/Flint?style=for-the-badge&color=EF4444&labelColor=0d1117&logo=github" alt="Release"/>
  </a>
  <img src="https://img.shields.io/badge/Tauri-2.0-24C8D8?style=for-the-badge&logo=tauri&logoColor=white&labelColor=0d1117" alt="Tauri"/>
  <img src="https://img.shields.io/badge/Rust-DEA584?style=for-the-badge&logo=rust&logoColor=black&labelColor=0d1117" alt="Rust"/>
  <img src="https://img.shields.io/badge/React-61DAFB?style=for-the-badge&logo=react&logoColor=black&labelColor=0d1117" alt="React"/>
  <img src="https://img.shields.io/badge/License-MIT-22c55e?style=for-the-badge&labelColor=0d1117" alt="MIT"/>
</p>

<p align="center">
  <a href="https://git.io/typing-svg">
    <img src="https://readme-typing-svg.demolab.com?font=JetBrains+Mono&weight=700&size=22&duration=2800&pause=800&color=EF4444&center=true&vCenter=true&width=620&lines=Extract.+Preview.+Edit.+Ship.;One+IDE+for+every+League+mod.;Built+on+Rust+%E2%9A%99+React+%E2%9A%9B+Tauri+2." alt="Typing banner"/>
  </a>
</p>

<p align="center">
  <sub>⚡ instant WAD browsing &nbsp;•&nbsp; 🎮 live 3D previews &nbsp;•&nbsp; 🎞 video-to-loadscreen &nbsp;•&nbsp; 🧠 Hematite auto-fix</sub>
</p>

<br/>

---

## 🎯 What is Flint?

**Flint** is a desktop IDE for building League of Legends mods — from pulling raw assets out of the game, to previewing them, editing them, and shipping a finished `.fantome` / `.modpkg`. Everything lives in one window. No CLI chains, no folder juggling.

```
  WAD  ─▶  Extract  ─▶  Preview  ─▶  Edit  ─▶  Validate  ─▶  Export
  (game)   (project)   (3D/2D/BIN)  (Monaco)  (Hematite)    (ship it)
```

<br/>

## ✨ Features

<table>
<tr>
<td width="50%" valign="top">

### 🗂 **WAD Explorer**
Lazy-loaded virtual filesystem over the entire game archive. Browse 4M+ files with instant folder expand and optional background indexing.

### 🖼 **Live Previews**
- **3D** — SKN/SKL/SCB/SCO meshes with animations & skeletons
- **Textures** — DDS/TEX (BC1/BC3/BC5/ETC)
- **BIN / LuaBin / TroyBin** — syntax-highlighted
- **Audio** — BNK/WPK with waveform + zoom

### 🎞 **Animated Loading Screens**
Drop a video in, get a working loadscreen out. Auto spritesheet packing, 16k texture budget, FPS trim, live preview, UI BIN auto-patch.

### 🎨 **Texture Recolor**
Batch hue-shift, colorize, or tint. Skips distortion maps, preserves alpha.

</td>
<td width="50%" valign="top">

### 🧠 **Hematite v2**
Plugged into [Hematite](https://github.com/LeagueToolkit/Hematite)'s rule engine — detects broken references, missing shaders, bad materials, and fixes them in one click. Rules hot-update from GitHub.

### 💾 **Checkpoints**
Git-lite for your project. Snapshot, diff, restore. Survives dev restarts.

### 📤 **Export Everywhere**
Ship to `.fantome`, `.modpkg`, or one-click sync into **LTK Manager**. Refathering, BIN concat, and thumbnail embedding are all built in.

### 🔌 **Jade & Quartz Interop**
Swap the BIN engine to Jade, or hand a texture off to Quartz paint mode. JSON interop, no file juggling.

### ⚡ **LMDB Hash Cache**
Memory-mapped 4M+ hash DB. ~5-20 MB RAM, instant lookups, bulk resolve for fast WAD extraction.

</td>
</tr>
</table>

<br/>

## 🚀 Quick Start

```bash
# Clone
git clone https://github.com/RitoShark/Flint
cd "Flint/Flint - Asset Extractor"

# Install + run
npm install
npm run tauri dev
```

<details>
<summary><strong>📦 Prerequisites</strong></summary>

| Tool | Version |
|------|---------|
| Rust | 1.75+ ([rustup](https://rustup.rs)) |
| Node | v20+ ([nodejs.org](https://nodejs.org)) |
| OS   | Windows 10 / 11 |

</details>

<details>
<summary><strong>📀 Building a release installer</strong></summary>

```bash
npm run tauri build
```
Output: `src-tauri/target/release/bundle/nsis/Flint_{version}_x64-setup.exe`

</details>

<br/>

## 🏗 Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  React 18 + TypeScript + Vite  ←→  Zustand stores           │
└─────────────────────────────────────────────────────────────┘
                         ▲
                         │  Tauri 2 IPC
                         ▼
┌─────────────────────────────────────────────────────────────┐
│  src-tauri/  (binary crate)  ─  Tauri commands + state      │
│  crates/flint-ltk/  (library) ─  14 domain modules:         │
│     bin · wad · hash · mesh · audio · repath · export       │
│     champion · league · validation · project · checkpoint   │
│     hud · error                                             │
└─────────────────────────────────────────────────────────────┘
                         ▲
                         │
┌─────────────────────────────────────────────────────────────┐
│  league-toolkit (Rust) · Hematite v2 · LMDB · rayon         │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Stack |
|-------|-------|
| Frontend | React 18 · TypeScript · Vite 5 · Zustand 4 |
| 3D | Three.js · React Three Fiber |
| Editor | Monaco (custom Ritobin language) |
| Backend | Rust · Tauri 2 · rayon · tokio |
| LTK core | `league-toolkit` v0.4 (rev `6137083`) |
| Hashing | LMDB via `heed` + `memmap2` |
| Validation | `hematite-core` · `hematite-ltk` |
| Export | `ltk_fantome` · `ltk_modpkg` · `ltk_mod_core` |

<br/>

## 🎨 Theming

Flint ships with full CSS-variable theming. Copy [src/themes/default.css](src/themes/default.css) and override:

```css
:root {
  --accent-primary:   #EF4444;
  --accent-hover:     #DC2626;
  --accent-secondary: #F87171;
  --accent-muted:     #991B1B;
}
```

<br/>

## 🤝 Contributing

PRs welcome. Keep commits conventional — `feat:`, `fix:`, `perf:`, `refactor:` — they feed the changelog via [git-cliff](cliff.toml).

```bash
git checkout -b feat/your-feature
# hack hack hack
git commit -m "feat(scope): short imperative message"
```

<br/>

## 📜 License

[MIT](LICENSE) — do whatever, just don't sue.

> League of Legends, all champion art, and all referenced game assets are property of **Riot Games, Inc.** Flint is an unofficial community tool and is not endorsed by Riot Games.

<br/>

<p align="center">
  <img src="https://capsule-render.vercel.app/api?type=waving&color=0:F87171,50:EF4444,100:991B1B&height=100&section=footer&animation=twinkling" alt="footer"/>
</p>

<p align="center">
  <sub>🔥 Made for the League modding community · Not affiliated with Riot Games</sub>
</p>
