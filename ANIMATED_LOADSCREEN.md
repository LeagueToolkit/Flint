# Animated Loading Screen Generator — Technical Documentation

## Overview

This feature allows users to create animated loading screen mods for League of Legends by converting a video file into a spritesheet-based animation. The mod consists of two files injected into the game:

1. **`spritesheet.tex`** — A TEX-encoded atlas containing all video frames arranged in a grid
2. **Modified `uibase`** — The game's UI base BIN file with an injected animation configuration block

## Architecture

```
Frontend (Browser)                    Rust Backend (Tauri)
───────────────────                   ────────────────────
1. Select video file
2. Trim duration
3. Adjust resolution
4. 16k budget validation
5. Extract frames (<video>+<canvas>)
6. Composite spritesheet
7. Export PNG → temp file
                    ─── IPC ───►      8. Read PNG → decode to RGBA
                                      9. Encode RGBA → TEX (BC1)
                                      10. Write spritesheet.tex
                                      11. Find UI.wad.client
                                      12. Extract uibase (hash 667b27d63a614c36)
                                      13. BIN → text → inject block → text → BIN
                                      14. Validate brackets
                                      15. Write modified uibase to project
```

## Files Involved

### Frontend
| File | Purpose |
|------|---------|
| `src/lib/spritesheet.ts` | Video processing utilities: grid calculation, budget validation, frame extraction, spritesheet assembly |
| `src/components/modals/NewProjectModal.tsx` | UI for video selection, trimming, resolution, budget display |
| `src/lib/api.ts` | `createLoadingScreenProject()` API bridge function |
| `src/styles/index.css` | CSS for video picker, trim controls, budget indicator |

### Backend (Rust)
| File | Purpose |
|------|---------|
| `src-tauri/src/commands/project.rs` | `create_loading_screen_project` command + helper functions |
| `src-tauri/src/main.rs` | Command registration |
| `src-tauri/Cargo.toml` | Added `"png"` feature to `image` crate |

## Grid Calculation Algorithm

The algorithm finds the optimal grid (cols × rows) arrangement that:
1. Uses **exactly** `totalFrames` cells (`cols × rows = totalFrames`)
2. Keeps both dimensions **under 16,384 pixels**
3. Makes the spritesheet **as close to square** as possible

```
function calculateGrid(totalFrames, frameW, frameH):
    bestResult = null
    minGap = Infinity

    for x = 1 to sqrt(totalFrames):
        if totalFrames % x != 0: continue
        y = totalFrames / x

        for (cols, rows) in [(x, y), (y, x)]:
            sheetW = cols * frameW
            sheetH = rows * frameH
            if sheetW > 16384 or sheetH > 16384: continue

            gap = |sheetW - sheetH|
            if gap < minGap:
                minGap = gap
                bestResult = { cols, rows, sheetW, sheetH }

    return bestResult  // null if no valid arrangement exists
```

This is O(√n) — optimized from the original Python's O(n²) brute force.

## Calculated BIN Values

Given video dimensions `W×H`, scale factor `S`, FPS `F`, trim `[start, end]`:

| Variable | Formula | Example |
|----------|---------|---------|
| `frameWidth` | `floor(W × S)` | `floor(1920 × 0.5) = 960` |
| `frameHeight` | `floor(H × S)` | `floor(1080 × 0.5) = 540` |
| `totalFrames` | `floor((end - start) × F)` | `floor(3.0 × 30) = 90` |
| `cols` | Best column count from grid algorithm | `10` |
| `rows` | `totalFrames / cols` | `9` |
| `sheetWidth` | `cols × frameWidth` | `9600` |
| `sheetHeight` | `rows × frameHeight` | `4860` |
| `mTextureUV` | `{ 0, 0, frameW/sheetW, frameH/sheetH }` | `{ 0, 0, 0.1, 0.111 }` |
| `FramesPerSecond` | User's chosen FPS | `30` |
| `NumberOfFramesPerRowInAtlas` | `cols` | `10` |

## BIN Injection Block

The following block is appended at the **root level** (end of file) of the uibase BIN text:

```
0x93e61733 = UiElementEffectAnimationData {
    name: string = "ClientStates/LoadingScreen/UX/LoadingScreenClassic/UIBase/LoadingScreen/{CREATOR_NAME}"
    Scene: link = "ClientStates/LoadingScreen/UX/LoadingScreenClassic/UIBase/LoadingScreen"
    Enabled: bool = true
    Layer: u32 = 0
    Position: pointer = UiPositionRect {
        UIRect: embed = UiElementRect {
            Position: vec2 = { 0, 0 }
            Size: vec2 = { 1920, 1080 }
            SourceResolutionWidth: u16 = 1920
            SourceResolutionHeight: u16 = 1080
        }
        IgnoreGlobalScale: bool = true
    }
    TextureData: pointer = AtlasData {
        mTextureName: string = "ASSETS/Animatedloadscreen/spritesheet.tex"
        mTextureSourceResolutionWidth: u32 = {SHEET_WIDTH}
        mTextureSourceResolutionHeight: u32 = {SHEET_HEIGHT}
        mTextureUV: vec4 = { 0, 0, {UV_W}, {UV_H} }
    }
    FramesPerSecond: f32 = {FPS}
    TotalNumberOfFrames: f32 = {TOTAL_FRAMES}
    NumberOfFramesPerRowInAtlas: f32 = {COLS}
    mFinishBehavior: u8 = 1
}
```

### Variable Substitutions
- `{CREATOR_NAME}` — From Flint settings (`state.creatorName`), defaults to `"SirDexal"`
- `{SHEET_WIDTH}` / `{SHEET_HEIGHT}` — Full spritesheet pixel dimensions
- `{UV_W}` — `frameWidth / sheetWidth` (single frame's UV width)
- `{UV_H}` — `frameHeight / sheetHeight` (single frame's UV height)
- `{FPS}` — Frames per second
- `{TOTAL_FRAMES}` — Total frame count
- `{COLS}` — Number of columns in the atlas grid

## uibase File Details

- **WAD**: `UI.wad.client` (located in `{leaguePath}/Game/DATA/FINAL/`)
- **Chunk hash**: `0x667b27d63a614c36`
- **Format**: Binary BIN file (no file extension in the WAD)
- **Processing**: Extract as bytes → `read_bin_ltk()` → `tree_to_text_cached()` → inject block → `text_to_tree()` → `write_bin_ltk()` → write to project

### Bracket Validation

Before and after injection, the BIN text is validated:
- Count `{` and `}` characters across all lines
- Ensure depth never goes negative (no unmatched `}`)
- Ensure final depth is zero (all `{` are closed)

## TEX Encoding

- **Input**: PNG file (from browser canvas export)
- **Decode**: `image::ImageReader::open()` → `decode()` → `into_rgba8()`
- **Encode**: `ltk_texture::Tex::encode_rgba_image(img, EncodeOptions::new(Format::Bc1))`
- **Format**: BC1 (DXT1) — block-compressed, 4:1 ratio, no alpha
- **No mipmaps** — UI textures don't need mip levels
- **Output**: Written to `content/base/UI.wad.client/assets/Animatedloadscreen/spritesheet.tex`

## Project Structure

```
{project-name}/
├── mod.config.json          (league-mod compatible, layer: "base")
├── flint.json               (Flint metadata: champion="loading-screen", skin_id=0)
├── content/
│   └── base/
│       └── UI.wad.client/
│           ├── assets/
│           │   └── animatedloadscreen/
│           │       └── spritesheet.tex      ← encoded spritesheet
│           └── clientstates/
│               └── loadingscreen/
│                   └── ux/
│                       └── loadingscreenclassic/
│                           └── uibase       ← modified BIN (no extension)
└── output/                  (for exports)
```

## Frame Extraction Flow (Browser)

1. Load video into `<video>` element via `URL.createObjectURL(file)`
2. Create `<canvas>` at spritesheet dimensions (`sheetWidth × sheetHeight`)
3. Create small temporary `<canvas>` at frame dimensions (`frameW × frameH`)
4. For each frame `i` (0 to totalFrames-1):
   - Calculate time: `trimStart + i / fps`
   - Seek video: set `video.currentTime`, await `seeked` event
   - Draw video to small canvas (downscales automatically)
   - Copy small canvas to spritesheet canvas at position `(col * frameW, row * frameH)`
   - Where `col = i % cols`, `row = floor(i / cols)`
5. Export: `canvas.toBlob('image/png')` → write to temp file via Tauri FS

## Troubleshooting

### "Exceeds 16,384 pixel limit"
- Reduce resolution scale (try 50% or 25%)
- Shorten the video duration (trim start/end)
- Lower the FPS (24 or 15 instead of 30)

### "No valid grid found"
- The total frame count has no factor pairs that fit within 16k
- The budget calculator suggests alternative frame counts
- Adjust trim or FPS to reach a suggested count

### TEX encoding fails
- Ensure the video isn't corrupt
- Very large spritesheets (near 16k) may fail due to memory — try reducing dimensions
- Check that `ltk_texture` supports the BC1 format (it should via `intel-tex` feature)

### uibase extraction fails
- Ensure League path is set correctly in Settings
- Verify `UI.wad.client` exists in `{leaguePath}/Game/DATA/FINAL/`
- The hash `0x667b27d63a614c36` must exist in the WAD (game version dependent)

### Animation doesn't play in-game
- Verify the spritesheet path matches: `ASSETS/Animatedloadscreen/spritesheet.tex`
- Check that `{CREATOR_NAME}` is properly set (no spaces or special characters)
- Ensure `mFinishBehavior: u8 = 1` is present (loop once)
- Verify UV values: `frameW/sheetW` and `frameH/sheetH` should be < 1.0
