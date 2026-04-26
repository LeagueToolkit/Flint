# Flint TODO

Working backlog. Bullets under each item are concrete sub-tasks; question
blocks are notes to me when I (Claude) need clarification before implementing.

---

## Drag-and-drop import — DONE
Drop OS files/folders onto a project-tree folder → copied via
`import_external_files` Rust command. Hover-to-expand at 1.2s, blue dashed
drop-target outline, ` (n)` filename uniquification on collision.

Files: `commands/file.rs` (`import_external_files`, `copy_dir_recursive`),
`api.ts` (`importExternalFiles`), `FileTree.tsx` (Tauri
`onDragDropEvent` listener with physical→CSS pixel conversion).

---

## Bug — "modified" tag triggered by sidecar files — DONE
`isSidecarFile()` helper in `src/lib/sidecarFiles.ts` consulted in
`App.tsx` before setting `fileStatuses`. Hot reload still fires.

---

## Bug — BIN save reload churn — DONE
Write-echo suppression: `core/write_echo.rs` exposes `mark()` /
`consume()`, backed by a 1.5s window keyed on canonicalized paths.
`save_ritobin_to_bin` and the `.ritobin` cache write in the read path
mark their target paths before writing; the project watcher consumes the
mark and skips emitting `file-changed` for matched events.

---

## BIN split — right-click → "Extract VFX to separate BIN"
This mirrors Quartz's `bin:combineLinkedBins` action in reverse. Split is
the natural inverse of combine, and Quartz already proves the wire format
works in production.

### Quartz wire format (read from `Quartz/src/main/ipc/channels/binTools.js:110-189`)
A BIN file has two collections at the top level:
- `entries` — array of objects, each with a `hash` (entry path hash, hex
  string) and the property tree
- `links` — array of strings; each string is a project-relative path to
  another `.bin` file that the engine should also load alongside this one

Combine = read each path in `links`, append its `entries` to the parent's
`entries` (skipping duplicate hashes), `fs.unlinkSync` the linked file,
remove that link from `parent.links`, write parent.

Split is the inverse:
1. Pick a subset of `parent.entries`.
2. Write a new BIN to disk with `entries: <subset>`, `links: []`.
3. Remove those entries from `parent.entries`.
4. Push the new file's project-relative path onto `parent.links`.
5. Write parent.

### Picking the VFX subset — the actual hard part
Naive approach: filter entries by class name (`VfxSystemDefinitionData`,
`StaticMaterialDef` whose name contains "Vfx", `Particle*`, etc.). This is
wrong because a VFX entry can be referenced by hash from a non-VFX entry
(e.g. a `SkinCharacterDataProperties` field linking a particle by hash).
If the VFX entry moves to another file, the hash reference still resolves
because the engine merges all linked BINs at runtime — but only if the new
file is in `links`. Step 4 ensures that.

So the class-name filter is actually fine, IF we always update `links`.
The reference graph isn't needed for correctness, only for sanity-checking
that we're not orphaning anything.

**VFX class set (initial — verify against actual skins):**
- `VfxSystemDefinitionData`
- `StaticMaterialDef` (only when path contains `/particles/` or referenced
  only by VFX entries — needs a reference pass to filter precisely;
  conservative default = leave in parent BIN)
- `ParticleParameterDef`, `Particle*`
- Hash any class with `Particle`, `Vfx`, `TrailDef`, `BeamDef` in its name

Anything we're unsure about: leave in parent BIN. Splitting too little is
recoverable (user can split again); splitting too much may break the skin.

### UX
Right-click `Skin{N}.bin` in tree → "Split VFX to separate BIN":
- Shows a modal listing every entry that matched the filter, grouped by
  class name, with checkboxes (default all checked).
- "Output filename" field, default `Skin{N}_VFX.bin`.
- Confirm → run split, refresh tree, show toast `Split N entries to
  Skin{N}_VFX.bin`.

### Implementation
- Rust command `split_bin_entries(bin_path, entry_hashes: Vec<String>,
  output_filename: String) -> Result<String, String>` — does steps 1-5
  above using `ltk_meta::Bin` and `flint-ltk/src/bin/`.
- Frontend modal `BinSplitModal.tsx` driven from `FileTree` context menu.
- Class-name filter lives in Rust (`flint-ltk/src/bin/split.rs` —
  `classify_vfx_entries(bin: &Bin) -> Vec<EntryHash>`).

Files (new): `src-tauri/src/commands/bin_split.rs`, `flint-ltk/src/bin/split.rs`,
`src/components/BinSplitModal.tsx`. (Modified): `FileTree.tsx`, `main.rs`,
`api.ts`.

---

## WAD explorer right side — base/empty state — partially DONE
Right-side panel previously only showed quick-filter cards on empty
state. Now also shows a "Recent WADs" list above the cards (last 8
expanded WADs, name + category, click to scroll-to-WAD with the same
mechanic the cheat-sheet uses).

Files: `wadExplorerStore.ts` (new `recentWads: string[]` +
`pushRecentWad` action, session-only), `WadExplorer.tsx` (push on
expand, render recent list in `QuickActionPanel`).

**Still TODO:** folder preview when the user selects a folder node in
the WAD tree (grid of immediate children with sizes + count). That's
shared infrastructure with the custom file explorer item below — defer
until that lands.

---

## Custom file explorer (re-scoped)
Don't build a second app inside the app. Extend `PreviewPanel`:
- When the selection is a folder, switch the preview to a thumbnail-grid
  view: each immediate child rendered with the existing image/model/text
  preview at small size (use the LRU cache).
- Double-click a thumbnail in the grid → enters that file's full preview.
- Double-click a texture → opens a full-resolution modal (just the
  existing image preview at native resolution + pan/zoom, no transform).

This way we reuse: `imageCache.ts`, `ModelPreview.tsx`, `TextPreview.tsx`,
`ImagePreview.tsx`. New code is the grid container + the full-res modal,
both small.

Files: `PreviewPanel.tsx` (folder branch), new `FolderGridView.tsx`, new
`FullResImageModal.tsx`.

---

## General slowness — partially addressed (clarified by user 2026-04-26)

User clarified the actual pain: **project creation and WAD explorer
indexing are ~50% slower than the old version of Flint**. The frontend
re-render fix is unrelated to this — those are heavy Rust-side ops, not
React renders.

**Cut 1 (DONE — adjacent):** `useAppState()` was the biggest known
frontend offender. It
called every store's bare hook with no selector, subscribing all 22
consumers to the full snapshot of nine stores. A new log line, a toast,
or any field change re-rendered every component using it (incl.
TitleBar/StatusBar which are always mounted). Fixed by switching to
`useShallow` selectors that pick only the fields the legacy `state`
object surfaces. Action methods are read once per render off
`getState()` since their refs are stable.

**Cut 2 (DONE — instrumentation):** added phase-timing logs to the two
paths the user called out as slow:
- `create_project` logs every phase (LMDB prime, find_champion_wad,
  wad_contains_skin_bin, core_create_project mkdir, extract_skin_assets,
  organize_project) plus a total summary with per-phase percentages.
- `get_wad_chunks` logs open/parse, hash collect, LMDB resolve, and
  build-response timings, plus cache hit/miss state and chunk count.

User to run the slow ops once and copy the `[TIMING]` lines from the
console / log panel. Next cut targets whichever phase actually dominates.

**Suspects worth confirming once timings come back:**
- `wad_contains_skin_bin` (project creation step 2b) opens AND mounts the
  champion WAD just to scan the TOC for one of two hashes. The full mount
  parses the entire chunk table — probably 10-100ms wasted per project
  create. Could be fused into `extract_skin_assets`'s mount.
- `WadReader::open` uses a `File` handle, not mmap. `Wad::mount(file)`
  reads TOC via syscalls. By contrast `extract_skin_assets` uses
  `Mmap::map(...)` then `Wad::mount(Cursor::new(&mmap[..]))`. Switching
  `WadReader` to mmap would mostly help repeated reads — likely small win
  for cold opens.
- 3000-chunk LMDB resolve loop in `resolve_hashes_lmdb` is ~3000 individual
  heed `db.get` calls in one read txn. heed serialization overhead per call
  may matter at this scale.
- Tauri IPC serialization of 3000 `ChunkInfo` structs per WAD load.
  Per-WAD payload is ~150KB JSON — adds up if explorer loads many.

**Remaining frontend suspects (separate from the user's main complaint):**
- BIN editor reparses on every keystroke — debounce + diff-parse.
- Tree build via per-folder IPC roundtrip — bulk listing could help.
- Image cache eviction policy under heavy preview load.

---

## Animated loadscreen preset — NEEDS USER SPEC
User mentioned: loadscreen banners can have a static-mat-with-mask setup
that makes them look animated. A right-click action would scaffold this.

I don't yet know:
- The exact BIN structure / class names involved in the animated mask
  pipeline (UV scrolling material? Mask texture binding? Where does the
  reference live — in the loadscreen tex's BIN or somewhere else?).
- What inputs the user expects the right-click to ask for (mask texture
  path? Scroll speed? Loop direction?).
- What "preset" means in this context — a fixed configuration, a chosen
  template from a list, or a parameterised generator?
- Whether this targets a specific class of skins (e.g. only legendary+
  loadscreens) or applies to anything.

**Action item:** user to describe the asset layout (one example skin that
already has this animated banner — point me at the BIN entries and
textures) before I spec the implementation.

---

## Map project fetch — NEEDS USER SPEC
User mentioned this is a detailed thing they want to explain.

I don't yet know:
- Does this mean fetching base map assets (Howling Abyss, SR variants) for
  modding the map itself? Or fetching map-specific overrides for a
  champion skin?
- The skin-id concept doesn't map cleanly to maps — maps have variants,
  not numbered skins. What's the project schema for "Map" mode? Is there
  a target map ID equivalent to a skin ID?
- Source WADs to crawl: `Maps/Shipping/Map11/Map11.wad.client` etc.?
- Refathering rules — do we still rewrite paths to
  `ASSETS/{creator}/{project}/`, or is there a map-specific convention?

**Action item:** user to describe the workflow end-to-end (input the user
provides → assets fetched → folder layout in the project) before I
implement.

---

## Suggested order
1. **Bug — "modified" tag** (small, daily annoyance)
2. **Bug — BIN save reload churn** (small, daily annoyance)
3. **General slowness** — profile first, fix top 3
4. **WAD explorer right side** — small, high polish/effort ratio
5. **BIN split** — needs the modal + Rust split logic
6. **Custom file explorer (re-scoped)** — depends on having a stable
   folder-selection path through PreviewPanel
7. **Animated loadscreen preset** — blocked on user spec
8. **Map project fetch** — blocked on user spec
