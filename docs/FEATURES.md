# Flint Features

Comprehensive documentation of all features in Flint.

## Animated Loading Screen Creator

Create custom animated loading screens for League of Legends from video files.

### Overview
Convert any video (MP4, WebM) into a League-compatible animated loading screen with automatic spritesheet optimization and BIN injection.

### Features
- **Video Upload**: Drag-and-drop or select video files
- **16k Texture Budget Validation**: Real-time check against WebGL 16384px limits
- **Grid Optimization**: Automatic calculation of optimal cols×rows layout
- **Trimming**: Adjust start/end points to clip unwanted footage
- **Scaling**: Scale down to fit budget (100%, 75%, 50%, 25%)
- **FPS Control**: Adjust animation frame rate (15, 24, 30, 60 fps)
- **Live Preview**: See trimmed video preview before exporting
- **Progress Tracking**: Real-time progress during spritesheet generation

### Technical Details
- **Spritesheet Generation**: HTML5 `<video>` + `<canvas>` frame extraction
- **TEX Encoding**: Converts PNG to BC1/DXT1 compressed texture
- **UI BIN Patching**: Automatically injects animation config into `uibase`
- **Output**: `.tex` spritesheet + patched BIN → export as `.fantome`

### Workflow
1. Click "New Project" → "Animated Loading Screen"
2. Upload video file (MP4/WebM)
3. Adjust trim start/end, scale, and FPS
4. Wait for budget validation (green = fits in 16k limit)
5. Generate spritesheet (shows progress bar)
6. Project created with spritesheet.tex + patched uibase
7. Export as Fantome and load in mod manager

### Budget Logic
- **Frame dimensions** = `video_width × scale` × `video_height × scale`
- **Total frames** = `(trimEnd - trimStart) × fps`
- **Grid search**: Finds optimal cols×rows that minimizes `|sheetWidth - sheetHeight|`
- **Limit**: Both `sheetWidth` and `sheetHeight` must be ≤ 16384px
- If over budget, suggests lower frame counts that would fit

---

## Smart Game Detection

Flint automatically detects your League of Legends installation through multiple methods:

- **Windows Registry**: Reads Riot Client and Steam registry keys
- **Common Paths**: Checks standard installation directories
- **Custom Paths**: Allows manual path selection
- **Real-time Validation**: Verifies game files before proceeding

The detection runs on first launch and guides you through setup if League isn't found automatically.

---

## Multi-Tab Workspace

Work on multiple projects and browse WAD files simultaneously with a unified tab system:

- **Project Tabs**: Open multiple extraction projects side-by-side
- **WAD Sessions**: Browse individual `.wad.client` files in dedicated tabs
- **WAD Explorer**: Singleton tab for browsing the entire game archive
- **Seamless Switching**: Jump between any tab without losing state
- **Independent Closing**: Close individual tabs; fallback logic ensures you never lose context

### Tab Types
1. **Project Tab** - Full extraction project with file tree
2. **Extract Session** - Single WAD file browser
3. **WAD Explorer** - Game-wide WAD virtual filesystem

---

## WAD Explorer

Browse the entire game's WAD archive library without extracting anything to disk.

### Features
- **Virtual File System**: Tree view organized by category (Champions, Maps, Audio, etc.)
- **Lazy Loading**: Only loads WAD chunks when you expand them in the tree
- **Instant Preview**: View textures, BIN files, audio, and hex data inline
- **Search**: Debounced search with regex toggle across all loaded WADs
- **Quick Filters**: One-click filter cards for asset types (Textures, BIN, Audio, Models)
- **Context Menu**: Right-click to copy path, copy hash, or extract files

### Workflow
1. Click "WAD Explorer" in welcome screen
2. Game WADs are scanned and categorized
3. Expand categories → WADs → folders to browse
4. Click files to preview inline
5. Use search (Ctrl+F) to find specific assets
6. Right-click to extract or copy information

---

## WAD Archive Operations

High-performance WAD file handling powered by `league-toolkit`.

### Capabilities
- **Fast Reading**: Efficient WAD chunk parsing
- **Hash Resolution**: Automatic filename resolution via CommunityDragon hashtables
- **Compression Support**: Handles ZSTD and Deflate compressed chunks
- **Selective Extraction**: Extract only the assets you need
- **Individual WAD Browsing**: Open any `.wad.client` file in a dedicated tab

### Supported Operations
- Extract champion skins (all assets)
- Extract individual files
- Browse WAD contents without extraction
- Preview files directly from WAD (in-memory)

---

## BIN File Editor

Full-featured editor for League's property files (`.bin`).

### Features
- **Syntax Highlighting**: VS Code-style editor with bracket pair colorization
- **Python-like Format**: Human-readable ritobin text representation
- **Auto-conversion**: BIN files pre-converted to `.ritobin` on extraction for instant loading
- **Save Support**: Edit and save back to binary `.bin` format
- **Type Support**: All BIN data types supported:
  - Primitives: `bool`, `i8/u8`, `i16/u16`, `i32/u32`, `i64/u64`, `f32`
  - Vectors: `Vec2`, `Vec3`, `Vec4`
  - Matrices: `Mtx44`
  - Colors: `RGBA`
  - Strings and Hashes
  - Links, Pointers, Embeds, Options
  - Containers: `List`, `Map`, `Optional`

### Workflow
1. Click a `.bin` file in the file tree
2. Editor loads with syntax-highlighted ritobin format
3. Edit values as needed
4. Save to convert back to binary `.bin`

---

## Asset Preview

### 3D Models

Real-time WebGL preview for champion meshes and static objects.

**Supported Formats**:
- **SKN** (Skinned Mesh) - Champion models with texture support
- **SKL** (Skeleton) - Bone structure visualization
- **SCB/SCO** (Static Mesh) - Particle geometry and props

**Features**:
- Texture mapping with automatic discovery
- Material visibility toggles
- Wireframe mode
- Skeleton bone visualization
- Animation playback (ANM files)
- Throttled animation updates (~30fps for performance)
- WebGL resource cleanup

**Controls**:
- Left-click drag: Rotate camera
- Right-click drag: Pan camera
- Scroll: Zoom in/out
- Toggle materials: Show/hide submeshes
- Wireframe: Enable mesh wireframe overlay

### Textures

DDS and TEX file decoding via `ltk_texture`.

**Supported Formats**:
- BC1 (DXT1) - RGB with 1-bit alpha
- BC3 (DXT5) - RGBA with interpolated alpha
- ETC (Ericsson Texture Compression)
- Uncompressed RGBA

**Features**:
- Automatic format detection
- Mipmap support
- Transparent background handling
- Before/after comparison (recolor mode)

### Hex Viewer

Binary file inspection with clean formatting.

**Features**:
- Hexadecimal and ASCII side-by-side
- Offset display
- Syntax highlighting for printable characters
- Copy hex values

### Text Files

Syntax-highlighted text viewer for common formats:
- JSON
- XML
- Plain text
- Shader code (GLSL, HLSL)

### Images

Standard image preview:
- PNG
- JPG
- Base64-encoded images

---

## Texture Recoloring

Batch recolor textures with multiple blending modes.

### Recoloring Modes

1. **Hue Shift**
   - Rotates all colors on the hue wheel
   - Preserves saturation and brightness
   - Great for subtle color variations

2. **Colorize**
   - Converts entire texture to a single hue
   - Preserves original shading and depth
   - Best for strong color changes

3. **Grayscale + Tint**
   - Removes all color, then applies tint overlay
   - Creates monochrome effect with accent color
   - Ideal for metallic or dark themes

### Smart Filtering

- **Auto-skip Distortion Maps**: Automatically ignores `distort`/`distortion` textures (UV effect maps)
- **Preserve Transparency**: Alpha channels remain unchanged
- **Black Background Protection**: Prevents color bleeding into pure black areas
- **Optional Override**: Checkbox to include distortion textures if needed

### Workflow
1. Select folder in file tree
2. Click "Recolor Textures" button
3. Choose recolor mode
4. Pick color from presets or custom picker
5. Preview before/after
6. Apply to all textures in folder

---

## Checkpoint System

Version control for your project assets.

### Features
- **Named Snapshots**: Create checkpoints with descriptive names
- **Instant Restoration**: Restore entire project to any checkpoint
- **Comparison View**: See exactly what changed between checkpoints
- **Auto-checkpoint**: Optional automatic snapshots before destructive operations

### Use Cases
- Before bulk texture recoloring
- Before exporting to test changes
- Before refathering/repathing operations
- Creating milestone backups

### Storage
- Checkpoints stored in `.flint/checkpoints/` within project
- Full directory snapshots (not diffs)
- Fast creation and restoration

---

## Mod Export

Export projects to mod formats compatible with popular mod managers.

### Fantome Format (`.fantome`)

Compatible with **cslol-manager**.

**Features**:
- Champion and skin metadata embedding
- Automatic path normalization
- ZIP-based archive format
- Supports custom skins and asset overrides

**Export Options**:
- Refathering (custom asset paths)
- BIN concatenation
- Version metadata

### Modpkg Format (`.modpkg`)

Compatible with **League Mod Tools**.

**Features**:
- JSON manifest generation
- Asset path mapping
- Supports multiple champions/skins
- Mod metadata (author, version, description)

**Export Options**:
- Same as Fantome
- Additional metadata fields
- Custom install paths

### Workflow
1. Open export modal (Ctrl+E or menu)
2. Select format (Fantome or Modpkg)
3. Configure options (refathering, concatenation)
4. Set metadata (name, version, author)
5. Choose output location
6. Export and install with mod manager

---

## LMDB Hash System

High-performance hash resolution using Lightning Memory-Mapped Database (LMDB).

### Overview
Replaces the old in-memory hashtable (420 MB → 264 MB arena) with a memory-mapped B-tree database for instant, zero-copy hash lookups.

### Features
- **Memory-Mapped**: 1 GB virtual address space, only 5-20 MB physically loaded
- **MVCC**: Lock-free concurrent reads (unlimited parallel readers)
- **Lazy Loading**: Only accessed B-tree pages are paged in from disk
- **Auto-rebuild**: Detects stale `.txt` files and rebuilds LMDB automatically
- **Process-wide cache**: Single `Arc<heed::Env>` shared across all operations
- **Build lock**: Prevents concurrent builds from corrupting the database

### Performance
- **~4 million hash entries** stored in B-tree format
- **Typical resolve**: 4000 hashes warms ~5-20 MB RAM (vs 264 MB for full arena)
- **Single read transaction**: All lookups in one LMDB read (microseconds after warm-up)
- **Bulk resolution**: Deduplicates hashes before lookup to avoid redundant B-tree traversals

### Implementation
- **Location**: [`src-tauri/src/core/hash/lmdb_cache.rs`](../src-tauri/src/core/hash/lmdb_cache.rs)
- **Dependency**: `heed = "0.20"` (safe Rust wrapper for LMDB)
- **Format**: Key = `u64` big-endian bytes, Value = UTF-8 path string
- **Storage**: `%appdata%/RitoShark/hashes/hashes.lmdb/data.mdb`

### Functions
- `get_or_open_env(hash_dir)` - Open or reuse cached LMDB env
- `resolve_hashes_lmdb(hashes, env)` - Resolve slice of hashes to paths
- `resolve_hashes_lmdb_bulk(hashes, env)` - Deduplicated bulk resolution
- `build_hash_db(hash_dir)` - Rebuild LMDB from `.txt` files (thread-safe)
- `drop_lmdb_cache()` - Close env (needed before deleting LMDB files on Windows)

### Sources
- `hashes.game.txt` - Game asset paths (~1.9M entries)
- `hashes.lcu.txt` - League client paths (~60K entries)
- `hashes.extracted.txt` - Community-extracted paths (~40K entries)

---

## DataDragon API Integration

Fetches champion and skin metadata from Riot's official CDN.

### Overview
Integrates with Riot's DataDragon and CommunityDragon APIs to provide champion/skin selection in the New Project modal.

### Features
- **Champion List**: Fetches all champions with ID, name, and alias
- **Skin Data**: Fetches all skins for a champion with splash/tile art URLs
- **Caching**: In-memory cache for API responses (per-session)
- **Retry Logic**: Automatic retry with exponential backoff on failure
- **Fallback**: Defaults to patch 14.23.1 if version fetch fails

### Endpoints
- **Patch Version**: `https://ddragon.leagueoflegends.com/api/versions.json`
- **Champion Summary**: `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champion-summary.json`
- **Champion Details**: `https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/champions/{id}.json`

### Implementation
- **Location**: [`src/lib/datadragon.ts`](../src/lib/datadragon.ts)
- **Functions**:
  - `getLatestPatch()` - Get current League patch version
  - `fetchChampions()` - Get all champions
  - `fetchChampionSkins(championId)` - Get skins for a champion

### Usage in UI
- New Project modal → Champion dropdown (searchable)
- Skin selection with splash art preview
- Automatic ID → internal name conversion for extraction

---

## Parallel Processing

Multi-threaded operations for maximum performance.

### Parallel BIN Conversion
Pre-converts all BIN files in a project to `.ritobin` format using `rayon` for parallel processing.

**Features**:
- **Concurrent conversion**: Processes 50 BINs at a time across thread pool
- **Cache detection**: Skips files with up-to-date `.ritobin` files (mtime check)
- **Progress events**: Emits `bin-convert-progress` events to frontend
- **Safe parallelism**: Pre-warms hash cache on main thread before spawning workers
- **Error handling**: Continues on individual file failures, logs warnings

**Performance**:
- Typical project: ~200 BIN files converted in 5-10 seconds
- Cache hits: Skips already-converted files (instant)

### Parallel Asset Extraction
Loads 3D model assets (mesh, skeleton, animations) concurrently.

**Features**:
- **Three concurrent IPC calls**: Mesh, skeleton, and animations load simultaneously
- **Non-blocking**: UI remains responsive during loads
- **Error isolation**: One asset failing doesn't block others

### Concurrent Hash Resolution
LMDB's MVCC allows unlimited concurrent read transactions.

**Features**:
- **Lock-free reads**: Multiple threads/tasks can resolve hashes in parallel
- **Single read txn per batch**: Groups lookups to minimize transaction overhead

---

## Real-time Progress Tracking

Live progress updates during long-running operations.

### Project Creation Progress
Emits `project-create-progress` events with phase information:

**Phases**:
- `init` - Initializing (LMDB hash env)
- `create` - Creating project structure
- `extract` - Extracting assets from WAD
- `repath` - Repathing assets (if enabled)
- `complete` - Project created successfully

**Event payload**:
```json
{
  "phase": "extract",
  "message": "Extracting Aurora skin 11 assets..."
}
```

### BIN Conversion Progress
Emits `bin-convert-progress` events during parallel BIN conversion:

**Event payload**:
```json
{
  "current": 42,
  "total": 200,
  "file": "skin11.bin",
  "status": "converting"
}
```

### Loading Screen Generation Progress
Spritesheet generation calls `onProgress` callback:

**Callback signature**:
```typescript
onProgress?: (current: number, total: number) => void
```

**Usage**: Updates modal progress bar frame-by-frame

---

## Startup Loading Screen

Animated loading UI displayed during app initialization.

### Features
- **Inline SVG animation**: Sliding gradient bar (pure CSS)
- **Non-blocking**: Uses `font-display: swap` for fonts
- **Automatic removal**: Removed when React mounts and paints
- **Double RAF**: Waits for two `requestAnimationFrame` cycles before showing window
- **Smooth transition**: Window only shows after DOM is fully rendered

### Implementation
- **Location**: [`index.html`](../index.html) `#loading-screen`
- **Styling**: Inline styles + CSS keyframe animation
- **Removal**: [`src/main.tsx`](../src/main.tsx) line 28-31

### Animation
```css
@keyframes loading {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(400%); }
}
```

Gradient bar slides from left to right infinitely with ease-in-out timing.

---

## Advanced Features

### Refathering System

**Purpose**: Prevent asset path conflicts between multiple mods.

**How it Works**:
1. Scans all BIN files for asset path references
2. Rewrites paths to use custom `ASSETS/{Creator}/{Project}/` prefix
3. Renames actual files on disk to match new paths
4. Updates all BIN links to point to new locations

**When to Use**:
- Publishing mods to the community
- Running multiple custom skins simultaneously
- Avoiding conflicts with other mods

**Example**:
```
Original:  assets/characters/aurora/skins/skin11/aurora.skn
Refathered: ASSETS/YourName/AuroraNeon/aurora.skn
```

### BIN Concatenation

**Purpose**: Improve mod manager compatibility by merging linked BIN files.

**How it Works**:
1. Detects linked BIN files (e.g., `skin0.bin` → `skin0_skins_*.bin`)
2. Merges all linked BINs into `__Concat.bin`
3. Updates main BIN to point to concatenated file
4. Removes individual linked BIN references

**When to Use**:
- Exporting for mod managers that don't support linked BINs
- Simplifying mod structure
- Reducing file count

**Benefits**:
- Better compatibility
- Faster loading in-game
- Cleaner mod structure

### Hash Resolution

**Purpose**: Convert 64-bit XXHash values to human-readable file paths.

**How it Works**:
1. Downloads CommunityDragon hashtables on first run
2. Loads hashtables into memory (optimized arena structure)
3. Resolves hashes during WAD extraction and browsing
4. Shows both hash and resolved path in UI

**Hashtable Stats**:
- ~4 million entries
- ~264 MB memory usage (optimized)
- Updated periodically from CommunityDragon

---

## Auto-Updater

Built-in update system for seamless version management.

### Features
- **Startup Check**: Checks for updates when app launches (if enabled)
- **Settings Toggle**: Enable/disable automatic updates in settings
- **Skip Version**: Dismiss updates you don't want to install
- **Secure Updates**: Cryptographic signature verification
- **Background Download**: Updates download without blocking work

### Update Process
1. App checks GitHub releases for new version
2. Notification appears if update available
3. Click "Update" to download installer
4. Installer runs after download completes
5. App restarts with new version

### Settings
- **Auto-update Enabled**: Toggle in Settings modal
- **Skipped Version**: Stored in localStorage, cleared on new releases

---

## Theming System

Customize Flint's appearance with CSS-based themes.

### Default Theme
- Gray-red palette with accent colors
- Dark mode optimized
- High contrast for readability

### Custom Themes
1. Copy `src/themes/default.css`
2. Modify CSS variables:
   ```css
   :root {
     --accent-primary: #your-color;
     --accent-secondary: #your-secondary-color;
     --bg-primary: #background-color;
     /* ... */
   }
   ```
3. Import in `main.tsx`

### Theme Variables
- Background colors (primary, secondary, tertiary)
- Text colors (primary, secondary, muted)
- Accent colors (primary, secondary, hover)
- Border colors
- Syntax highlighting colors (BIN editor)

---

## Roadmap

### In Progress
- **Sound Bank Editing** - BNK/WPK audio file preview and editing

### Planned
- **Particle System Preview** - BIN-based particle effect visualization
- **Shader Editor** - GLSL shader editing with live preview
- **Material Editor** - Visual material property editing
- **Bulk Asset Operations** - Batch rename, move, delete
- **Mod Conflict Detection** - Detect overlapping asset paths
- **Community Integration** - Share and browse community mods
- **Loading Screen Templates** - Pre-made templates for common animation styles

### Completed
- ✅ SKN/SKL 3D Preview
- ✅ SCB/SCO Static Mesh Preview
- ✅ Animation Preview (ANM)
- ✅ Animated Loading Screens (Video→spritesheet)
- ✅ LMDB Hash Cache (Memory-mapped database)
- ✅ Parallel Asset Loading
- ✅ Parallel BIN Conversion (Multi-threaded)
- ✅ DataDragon API Integration
- ✅ WAD Explorer VFS
- ✅ In-memory WAD Preview
- ✅ Auto-updater
- ✅ Real-time Progress Tracking
- ✅ Startup Loading Animation

---

## Performance Features

### Frontend Optimizations
- **LRU Image Cache**: Limits memory usage for texture previews
- **Virtual Scrolling**: Only renders visible file tree nodes
- **Code Splitting**: Lazy-loads heavy components (Monaco, Three.js)
- **Memoization**: React.memo on expensive components

### Backend Optimizations
- **Hashtable Arena**: Custom memory layout (420MB → 264MB)
- **Lazy WAD Loading**: Only reads chunks on demand
- **Parallel Asset Loading**: Concurrent mesh/skeleton/animation loading
- **Zero-Copy Decoding**: Direct byte passing to frontend

### Resource Cleanup
- **WebGL Cleanup**: Disposes geometry and textures on unmount
- **Context Loss Handlers**: Gracefully handles GPU resets
- **Animation Throttling**: Limits animation updates to ~30fps
- **RAF Cleanup**: Cancels animation frames on unmount
