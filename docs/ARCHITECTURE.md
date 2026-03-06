# Flint Architecture

## Project Structure

```
flint/
├── src/                        # React TypeScript Frontend
│   ├── main.tsx                # Application entry point
│   ├── components/             # UI Components
│   │   ├── App.tsx             # Root component & layout
│   │   ├── TopBar.tsx          # Navigation, tab bar & export actions
│   │   ├── FileTree.tsx        # Project asset file browser
│   │   ├── WadExplorer.tsx     # Unified game WAD browser (VFS)
│   │   ├── WadBrowser.tsx      # Single WAD session browser panel
│   │   ├── WadPreviewPanel.tsx # In-memory WAD chunk preview
│   │   ├── CenterPanel.tsx     # Dynamic content area
│   │   ├── PreviewPanel.tsx    # Asset preview container
│   │   ├── StatusBar.tsx       # Status & hash info
│   │   ├── WelcomeScreen.tsx   # Landing page
│   │   ├── ContextMenu.tsx     # Right-click menus
│   │   ├── CheckpointTimeline.tsx # Checkpoint UI
│   │   ├── Toast.tsx           # Notification toasts
│   │   ├── modals/             # Modal dialogs
│   │   │   ├── NewProjectModal.tsx
│   │   │   ├── ExportModal.tsx
│   │   │   ├── SettingsModal.tsx
│   │   │   ├── FirstTimeSetupModal.tsx
│   │   │   ├── RecolorModal.tsx
│   │   │   └── UpdateModal.tsx
│   │   └── preview/            # Asset preview panels
│   │       ├── BinEditor.tsx / LazyBinEditor.tsx
│   │       ├── BinPropertyTree.tsx
│   │       ├── ModelPreview.tsx / LazyModelPreview.tsx
│   │       ├── ImagePreview.tsx
│   │       ├── TextPreview.tsx
│   │       ├── HexViewer.tsx
│   │       └── AssetPreviewTooltip.tsx
│   ├── lib/                    # Utilities & API bridge
│   │   ├── api.ts              # Tauri command wrappers
│   │   ├── stores/             # Zustand state management
│   │   │   ├── index.ts        # Root store (useAppState hook)
│   │   │   ├── appMetadataStore.ts
│   │   │   ├── configStore.ts
│   │   │   ├── projectTabStore.ts
│   │   │   ├── navigationStore.ts
│   │   │   ├── wadExtractStore.ts
│   │   │   ├── wadExplorerStore.ts
│   │   │   ├── championStore.ts
│   │   │   ├── modalStore.ts
│   │   │   ├── notificationStore.ts
│   │   │   └── navigationCoordinator.ts
│   │   ├── imageCache.ts       # LRU image cache
│   │   ├── types.ts            # TypeScript type definitions
│   │   ├── utils.ts            # Helper functions
│   │   ├── logger.ts           # Frontend logging
│   │   ├── fileIcons.tsx       # File type icon mapping
│   │   ├── ritobinLanguage.ts  # Monaco BIN syntax definition
│   │   └── datadragon.ts       # Champion data integration
│   ├── styles/                 # Global CSS styles
│   └── themes/                 # Customizable CSS themes
│
├── src-tauri/                  # Rust Backend
│   ├── src/
│   │   ├── main.rs             # Application entry point
│   │   ├── lib.rs              # Library exports
│   │   ├── error.rs            # Error types & handling
│   │   ├── state.rs            # Managed application state
│   │   ├── commands/           # Tauri IPC handlers
│   │   │   ├── project.rs      # Project CRUD operations
│   │   │   ├── export.rs       # Mod export commands
│   │   │   ├── bin.rs          # BIN file operations
│   │   │   ├── file.rs         # File I/O & preview
│   │   │   ├── wad.rs          # WAD archive commands
│   │   │   ├── hash.rs         # Hash resolution
│   │   │   ├── champion.rs     # Champion & skin commands
│   │   │   ├── checkpoint.rs   # Checkpoint commands
│   │   │   ├── mesh.rs         # 3D mesh commands
│   │   │   ├── league.rs       # League detection commands
│   │   │   ├── updater.rs      # App update commands
│   │   │   └── validation.rs   # Asset validation commands
│   │   └── core/               # Core functionality
│   │       ├── bin/            # BIN parsing & conversion
│   │       ├── wad/            # WAD extraction
│   │       ├── hash/           # CommunityDragon hashtables
│   │       ├── repath/         # Asset repathing & refathering
│   │       ├── export/         # Fantome/Modpkg export
│   │       ├── mesh/           # SKN/SKL/SCB mesh parsing
│   │       ├── league/         # Game detection
│   │       ├── project/        # Project management
│   │       ├── champion/       # Champion & skin discovery
│   │       ├── validation/     # Asset validation
│   │       ├── checkpoint.rs   # Checkpoint system
│   │       └── frontend_log.rs # Frontend log forwarding
│   └── Cargo.toml              # Rust dependencies
│
├── .github/workflows/          # CI/CD
│   ├── build.yml               # Build + auto-release on push to main
│   └── release.yml             # Tag-based release
│
└── docs/                       # Documentation
```

## Tech Stack

### Frontend
- **Framework**: React 18 with TypeScript
- **Build Tool**: Vite 5
- **State Management**: Zustand 4 (domain-sliced stores)
- **3D Rendering**: Three.js with React Three Fiber
- **Code Editor**: Monaco Editor (VS Code engine)
- **Styling**: CSS Modules with custom theming system

### Backend
- **Framework**: Tauri 2.0
- **Language**: Rust
- **Async Runtime**: Tokio
- **File Parsing**: LeagueToolkit crates
  - `ltk_ritobin` / `ltk_meta` - BIN file parsing
  - `ltk_mesh` - SKN/SKL/SCB/SCO mesh parsing
  - `ltk_anim` - ANM animation parsing
  - `ltk_texture` - DDS/TEX texture decoding
  - `ltk_fantome` / `ltk_modpkg` - Mod format export
- **WAD Operations**: `league-toolkit`
- **Hash Resolution**: CommunityDragon hashtables with `xxhash-rust`

## Key Dependencies

### Rust Backend
- `tauri` 2.0 - Cross-platform desktop framework
- `league-toolkit` - WAD archive operations
- `ltk_mesh` - SKN/SKL/SCB/SCO mesh parsing
- `ltk_anim` - ANM animation parsing
- `ltk_ritobin` / `ltk_meta` - BIN file parsing
- `ltk_fantome` / `ltk_modpkg` - Mod format export
- `ltk_texture` - DDS/TEX texture decoding
- `heed` 0.20 - LMDB wrapper (Lightning Memory-Mapped Database)
- `memmap2` - Memory-mapped file I/O for WAD extraction
- `rayon` - Data parallelism for BIN conversion
- `reqwest` - HTTP client for hash downloading
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `walkdir` - Recursive directory traversal
- `image` - PNG/JPEG encoding/decoding
- `glam` - Vector math for BIN injection

### Frontend
- `@tauri-apps/api` 2.0 - Tauri JavaScript bindings
- `@tauri-apps/plugin-dialog` - Native file dialogs
- `react` 18.3 - UI framework
- `zustand` 4.5+ - State management
- `typescript` 5.6 - Type safety
- `@react-three/fiber` - Three.js React renderer
- `@react-three/drei` - Three.js helpers
- `@monaco-editor/react` - Code editor component
- `react-window` - Virtual scrolling
- HTML5 APIs:
  - `<video>` - Video frame extraction for spritesheets
  - `<canvas>` - Frame compositing and export
  - `fetch` - DataDragon/CommunityDragon API calls

## Data Flow

### Hash Resolution Architecture

Flint uses a two-tier hash resolution system:

1. **LMDB Cache (Primary)** - Memory-mapped B-tree database
   - 1 GB virtual address space (only 5-20 MB physically loaded)
   - Lock-free concurrent reads (MVCC)
   - Auto-rebuilds from `.txt` sources when stale
   - Process-wide singleton `Arc<heed::Env>`

2. **In-memory Arena (Fallback)** - Sorted Vec + string arena
   - 264 MB for 4M entries (down from 420 MB HashMap)
   - Used when LMDB is unavailable
   - Zero-copy `Cow::Borrowed` on hit

**Resolution Flow**:
```
WAD chunk hash (u64)
  ↓
LMDB lookup (memory-mapped B-tree)
  ↓
Path string (or fallback to hex)
```

### DataDragon Integration

Champion and skin data fetched from Riot CDN:

```
New Project Modal
  ↓
fetchChampions() → CommunityDragon API
  ↓
User selects champion
  ↓
fetchChampionSkins(id) → CommunityDragon API
  ↓
Display skins with splash art
  ↓
User selects skin → extract from WAD
```

**Caching**: API responses cached in-memory for session duration

### Project Creation Flow
1. User selects champion/skin from DataDragon API
2. Frontend calls `create_project()` Tauri command with parameters
3. Backend **Phase 1 - Init**: Prime LMDB hash env (build from `.txt` if stale)
4. Backend **Phase 2 - Create**: Create project directory structure
5. Backend **Phase 3 - Extract**: Extract skin assets from WAD using LMDB resolver
6. Backend **Phase 4 - Repath** (optional): Apply refathering and BIN concatenation
7. Backend **Phase 5 - Complete**: Emit completion event
8. Frontend opens project tab and triggers parallel BIN conversion

### Loading Screen Project Flow
1. User uploads video file (MP4/WebM)
2. Frontend extracts video metadata (width, height, duration, fps)
3. User adjusts trim, scale, FPS → real-time budget validation
4. Frontend generates spritesheet:
   - Seeks video to each frame time
   - Draws frame to temp canvas (downscales)
   - Composites onto main spritesheet canvas
   - Exports as PNG Blob
5. Frontend calls `create_loading_screen_project()` with PNG bytes
6. Backend:
   - Writes PNG to temp file (frees IPC buffer)
   - Decodes PNG with `image` crate (no memory limits)
   - Encodes to BC1/DXT1 TEX format
   - Extracts `uibase` chunk from `UI.wad.client`
   - Injects animation config into BIN tree
   - Writes TEX + patched BIN to project
7. Frontend opens project tab → ready to export as Fantome

### Parallel BIN Conversion Flow
1. User creates/opens project
2. Frontend calls `preconvert_project_bins()`
3. Backend:
   - Pre-warms hash cache on main thread
   - Scans project for `.bin` files (excludes `.ritobin` cache)
   - Filters to files needing conversion (mtime check)
   - Processes in batches of 50 using `rayon` thread pool
   - Each worker: reads BIN → parses → converts to text → writes `.ritobin`
   - Emits progress events per batch
4. Frontend displays progress bar
5. On completion, BIN editor loads instantly (reads `.ritobin` cache)

### Asset Preview Flow
1. User clicks file in tree
2. Frontend determines file type
3. Frontend calls appropriate Tauri command:
   - `read_file_info()` - Text/hex preview
   - `read_skn_mesh()` / `read_skl_skeleton()` - 3D models
   - `decode_bytes_to_png()` - DDS/TEX textures
4. Backend processes file and returns data
5. Frontend renders preview in appropriate panel

### Mod Export Flow
1. User selects export format (Fantome or Modpkg)
2. Frontend calls `export_fantome()` or `export_modpkg()`
3. Backend:
   - Gathers all project assets
   - Applies refathering/repathing if configured
   - Concatenates linked BIN files if enabled
   - Packages into mod format
   - Writes output file
4. Frontend shows success notification with file location

## Advanced Features

### Refathering System
- **Purpose**: Prevent asset path conflicts between mods
- **Implementation**: `src-tauri/src/core/repath/refather.rs`
- **How it works**:
  1. Scans all BIN files for asset path references
  2. Rewrites paths to use custom `ASSETS/{Creator}/{Project}/` prefix
  3. Renames actual files on disk to match new paths
  4. Updates all BIN links to point to new locations

### BIN Concatenation
- **Purpose**: Improve mod manager compatibility
- **Implementation**: `src-tauri/src/core/bin/concat.rs`
- **How it works**:
  1. Detects linked BIN files (e.g., `skin0.bin` → `skin0_skins_*.bin`)
  2. Merges all linked BINs into `__Concat.bin`
  3. Updates main BIN to point to concatenated file
  4. Removes individual linked BIN references

### Checkpoint System
- **Purpose**: Version control for project assets
- **Implementation**: `src-tauri/src/core/checkpoint.rs`
- **How it works**:
  1. Creates snapshots of entire project directory
  2. Stores checkpoints in `.flint/checkpoints/`
  3. Supports restoration and comparison operations
  4. Auto-checkpoint option before risky operations

## Performance Optimizations

### Frontend
- **LRU Image Cache**: Limits memory usage for texture previews
- **Virtual Scrolling**: Only renders visible file tree nodes (react-window)
- **Code Splitting**: Lazy-loads Monaco editor and Three.js on demand
- **Memoization**: React.memo on heavy components (FileTree, ModelPreview)
- **Double RAF**: Waits for two `requestAnimationFrame` cycles before showing window (smooth startup)
- **Font Display Swap**: Non-blocking font loads with `font-display: swap`

### Backend - Hash Resolution
- **LMDB Memory Mapping**: 1 GB virtual address space, only 5-20 MB physically loaded
  - OS pages in B-tree nodes on-demand
  - Typical resolve: 4000 hashes warms ~5-20 MB (vs 264 MB for full arena)
- **Lock-free Reads**: MVCC allows unlimited concurrent hash lookups
- **Sorted Inserts**: 2× faster LMDB writes by sorting keys before inserting
- **Deduplication**: Bulk resolver deduplicates hashes to avoid redundant B-tree traversals
- **Build Lock**: Prevents concurrent LMDB builds from corrupting database

### Backend - Parallel Processing
- **Rayon Thread Pool**: Multi-threaded BIN conversion (50 files per batch)
  - Pre-warms hash cache on main thread before spawning workers
  - Atomic counters for thread-safe progress tracking
  - Continues on individual file failures
- **Concurrent Asset Loading**: Mesh, skeleton, animations load in parallel (3 simultaneous IPC calls)
- **Memory-mapped WAD Reading**: `memmap2` for zero-copy WAD chunk access
- **Async Runtime**: Tokio for non-blocking I/O and task spawning

### Backend - Caching
- **LMDB Auto-rebuild**: Only rebuilds when `.txt` files are newer than LMDB
- **Ritobin Cache**: Skips BIN conversion if `.ritobin` is up-to-date (mtime check)
- **Process-wide LMDB Env**: Single `Arc<heed::Env>` shared across all operations
- **Lazy WAD Loading**: Only reads chunks when expanded in UI
- **Hashtable Arena Fallback**: Custom memory layout reduces hash table from 420MB → 264MB (when LMDB unavailable)
- **Zero-Copy Texture Decoding**: Passes decoded bytes directly to frontend

## Security Considerations

- File paths validated to prevent directory traversal
- User input sanitized before filesystem operations
- No shell command execution from user input
- Tauri's security context isolation between frontend/backend
