# Map Project Type — Plan

A new Flint project type for editing **League maps** (Howling Abyss, Summoner's Rift, etc.) ported from the workflow used by [MapgeoAddon](https://github.com/TheKillerey/MapgeoAddon). Lets a user pick a map, optionally pick a variant, extract its WAD(s) directly into a project's `content/base/`, and start working in Flint with everything (mapgeo, materials, textures, particles, lightmaps) already on disk.

---

## 1. Where the data lives in a League install

```
<League>/Game/DATA/FINAL/Maps/Shipping/
    Map11.wad.client          ← geometry, materials, models, textures
    Map11LEVELS.wad.client    ← lightmaps, grass tints, level data
    Map12.wad.client
    …
    Common.wad.client         ← cross-map shared assets
```

Inside any `Map*.wad.client`, after extraction:

```
data/maps/mapgeometry/<map_id>/
    <variant>.mapgeo            ← geometry binary
    <variant>.materials.bin     ← material binary
    <variant>.materials.json    ← (older maps) JSON sidecar
assets/maps/<map_id>/
    materials/…  textures/…  models/…
```

A "variant" is a sub-skin of the map, e.g. `room`, `room_revival`, `srx_baseworld`. Each variant pairs a `.mapgeo` and a `.materials.bin`.

---

## 2. What MapgeoAddon does (relevant parts only)

`project_manager.py` walks this flow:

1. **`get_maps_wad_dir(league_path)`** — find `Game/DATA/FINAL/Maps/Shipping/`.
2. **`_ensure_riot_wad_cache(league_path, map_id)`** — find `Map<id>.wad.client` (preferred) or `Map<id>.wad`, extract the whole thing into a per-map cache dir keyed by the map id, write a `.extraction_done` marker so it never re-extracts.
3. **`_ensure_riot_levels_wad_cache(...)`** — same but for `Map<id>LEVELS.wad.client` (lightmap WAD). Pre-registers known LEVELS path hashes so they resolve to readable names.
4. **`get_riot_wad_variants(league_path, map_id)`** — scan the cache for `data/maps/mapgeometry/<id>/*.mapgeo` + `*.materials.bin`, group by variant base name, return a sorted list.
5. The Blender side then loads the selected `.mapgeo` and `.materials.bin` from the cache.

**Flint adapts this:** instead of caching the WAD outside the project and pointing tools at it, Flint **extracts the WAD directly into `<project>/content/base/Map<id>.wad.client/`**, exactly the same way skin projects are structured (see [project.rs:215](src-tauri/crates/flint-ltk/src/project/project.rs#L215)). After that the file watcher, BIN editor, preview panel, and exporters all "just work" because the layout matches what Flint already knows.

---

## 3. New Flint project type: `map`

### Project layout on disk

```
<output>/<project name>/
    mod.config.json
    flint.json                   ← stores { mapId, variant, includeLevels }
    content/base/
        Map11.wad.client/
            data/maps/mapgeometry/map11/<variant>.mapgeo
            data/maps/mapgeometry/map11/<variant>.materials.bin
            assets/maps/map11/...
        Map11LEVELS.wad.client/  ← only if "include LEVELS" is checked
            data/maps/<...>/lightmaps/...
    output/
```

### Repathing

Skin projects rewrite asset paths under `ASSETS/<creator>/<project>/`. Maps **do not get repathed** — the in-game map system loads files by their canonical path. The repathing pipeline is skipped entirely for map projects. (We can revisit this later if creators want a multi-variant workflow.)

---

## 4. Backend changes

### `crates/flint-ltk/src/map/mod.rs` (new)

Pure logic, no Tauri deps. Mirrors the patterns in [project/project.rs](src-tauri/crates/flint-ltk/src/project/project.rs) and [wad/extractor.rs](src-tauri/crates/flint-ltk/src/wad/extractor.rs).

```rust
pub struct MapEntry { pub id: String, pub display_name: String, pub wad_path: PathBuf, pub levels_wad_path: Option<PathBuf> }
pub struct MapVariant { pub name: String, pub mapgeo_path: String, pub materials_path: String }

pub fn maps_shipping_dir(league_path: &Path) -> Option<PathBuf>;
pub fn list_available_maps(league_path: &Path) -> Result<Vec<MapEntry>>;
pub fn list_map_variants(league_path: &Path, map_id: &str, resolve: impl Fn(&[u64]) -> HashMap<u64, String>) -> Result<Vec<MapVariant>>;
pub fn create_map_project(name: &str, map_id: &str, league_path: &Path, output_dir: &Path, author: Option<String>) -> Result<Project>;
```

`list_available_maps` regex-matches `^Map(\d+)(LEVELS)?\.wad(\.client)?$` and groups main/levels by id. Display name is hardcoded for known IDs (`map11 → Summoner's Rift`, `map12 → Howling Abyss`, …) with a fallback to `Map <id>`.

`list_map_variants` mounts the WAD without fully extracting (uses `Wad::mount` + `wad_toc.chunks()` like `extract_skin_assets`), bulk-resolves all path hashes via the existing LMDB helper, then filters paths that match `data/maps/mapgeometry/<map_id>/<variant>.{mapgeo,materials.bin}` and groups them.

### `commands/map_project.rs` (new) — Tauri wrappers

```rust
#[tauri::command] pub async fn list_available_maps(league_path: String) -> Result<Vec<MapEntry>, String>;
#[tauri::command] pub async fn list_map_variants(league_path: String, map_id: String, lmdb: State<LmdbCacheState>) -> Result<Vec<MapVariant>, String>;
#[tauri::command] pub async fn create_map_project(
    name: String,
    map_id: String,
    variant: Option<String>,        // future: filter to a single variant
    include_levels: bool,
    league_path: String,
    output_path: String,
    creator_name: Option<String>,
    lmdb: State<LmdbCacheState>,
    app: AppHandle,
) -> Result<Project, String>;
```

The create flow mirrors [project.rs:create_project](src-tauri/src/commands/project.rs):

1. Prime LMDB (`lmdb.prime(...)`).
2. Locate `Map<id>.wad.client` (and optionally the LEVELS WAD).
3. Call `core_create_project(name, "map-<id>", 0, league_path, output_dir, author)` — reuse the existing project struct, store the map id in the `champion` slot (treated as a project-type tag).
4. For each WAD: call `extract_full_wad(wad_path, content_base.join(<wad_name>.wad.client), resolve)` — a thin function that reuses the same mmap+rayon plan as `extract_skin_assets` but **without** the skin filter (no `skinN`-only logic; we want the whole WAD).
5. Emit `project-create-progress` events with the same phase names (`init`, `create`, `extract`, `complete`) so the existing UI progress overlay works unchanged.
6. **Skip repathing.**

### Reusing extraction code

`extract_skin_assets` does almost what we need — it just filters out chunks that don't start with `assets/` or `data/`. For maps that filter is fine (everything we want is under those two prefixes). But the function carries `champion`/`skin_id` in its signature for naming the output dir.

**Plan:** factor the inner extraction loop into `extract_wad_to_dir(wad_path, output_dir, resolve)` and have both `extract_skin_assets` and the new map command call it. This is a small refactor and keeps the single fast path. If that's too invasive in v1, we wrap a new `extract_map_assets` that calls the same logic with a synthesized "wad_folder_name" and no skin filter.

### `main.rs` registration

Append the three commands to the `invoke_handler!` macro alongside the existing `commands::project::*` entries.

---

## 5. Frontend changes

### `src/lib/api.ts`

```ts
export interface MapEntry { id: string; displayName: string; hasLevels: boolean }
export interface MapVariant { name: string; mapgeo: string; materials: string }

export async function listAvailableMaps(leaguePath: string): Promise<MapEntry[]>;
export async function listMapVariants(leaguePath: string, mapId: string): Promise<MapVariant[]>;
export async function createMapProject(p: {
    name: string; mapId: string; variant?: string; includeLevels: boolean;
    projectPath: string; leaguePath: string; creatorName?: string;
}): Promise<Project>;
```

### `NewProjectModal.tsx`

Add `'map'` to the `ProjectType` union, a new type card (terrain icon), and a form:

```
┌──────────────────────────────────────┐
│ Project Name  [______________]       │
│ Location      [______________] [...] │
│                                      │
│ Map           [▼ Summoner's Rift]    │  ← from listAvailableMaps()
│ Variant       [▼ <auto>]             │  ← from listMapVariants() (optional UI)
│ ☑ Include LEVELS WAD (lightmaps)     │
└──────────────────────────────────────┘
```

V1 keeps the form intentionally minimal (Map + checkbox). The variant picker is a stretch goal — the backend returns variants but if the user doesn't pick one, all variants get extracted, which is the safest default.

`canCreateMap` mirrors `canCreateSkin`. `handleCreateMap` calls `api.createMapProject(...)` and routes through `finishProjectCreation`.

### `finishProjectCreation` already opens any project regardless of type — no changes needed there. The recent-projects entry uses `championName` as a label; for maps we pass the map's display name (e.g. `"Summoner's Rift"`) and `0` for skin.

---

## 6. Open questions / follow-ups

- **Variant filtering on extract**: do we want to extract _only_ the chosen variant's `.mapgeo` and `.materials.bin` plus any assets they reference? That's a meaningful disk-space win but requires walking the materials BIN to collect referenced texture/model paths first. V1 extracts the whole map WAD; smart filtering can come later.
- **Common.wad.client**: some map assets live there. V1 ignores it; if missing references show up in previews we add an "Include Common.wad" checkbox and another extraction pass.
- **Map metadata**: the hardcoded id → display-name map should eventually pull from CommunityDragon (`maps.json`) the same way champions/skins do.
- **Detecting map projects in `NewProjectModal` recents / `ProjectListModal`**: the existing list keys off `champion`. Storing map id in that slot as `"map-11"` is the pragmatic shortcut; a cleaner fix is adding a `kind: "skin" | "loading-screen" | "hud" | "map"` field to `flint.json`.

---

## 7. File-by-file summary

| File | Change |
|---|---|
| `src-tauri/crates/flint-ltk/src/map/mod.rs` | **new** — map discovery + variant scanning |
| `src-tauri/crates/flint-ltk/src/lib.rs` | export `pub mod map;` |
| `src-tauri/crates/flint-ltk/src/wad/extractor.rs` | factor out `extract_wad_to_dir` (or add `extract_map_assets` wrapper) |
| `src-tauri/src/commands/map_project.rs` | **new** — three Tauri commands |
| `src-tauri/src/commands/mod.rs` | `pub mod map_project;` |
| `src-tauri/src/main.rs` | register the three new commands |
| `src/lib/api.ts` | three new wrapper functions + types |
| `src/components/modals/NewProjectModal.tsx` | new type card + form + handler |
