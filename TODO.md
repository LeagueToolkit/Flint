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

## Bug — "modified" tag triggered by sidecar files
**Repro:** open any `.bin` in the editor. Flint converts it to a sidecar
`.ritobin` for editing. The file watcher sees a `modify` event for the
sidecar and flags it. The sidecar is hidden from the tree, but its parent
folder ends up looking dirty when nothing the user cares about changed.

**Root cause:** `App.tsx:222-225` sets `'modified'` on every file-watcher
`modify` event regardless of file kind. There is no allow/deny list.

**Fix:**
- Filter the watcher event in `App.tsx` so `modify`/`create` for paths
  ending in `.ritobin` (and any other sidecar/derived format we add later
  — `.tex.png` cache, `.dds.png` cache, etc.) does NOT update
  `fileStatuses`. They should still bump `fileVersions` for hot-reload —
  only the user-facing badge has to be suppressed.
- Define the deny list in one place: `src/lib/sidecarFiles.ts`, exported as
  `isSidecarFile(path: string): boolean`. Reuse from anywhere that needs
  to distinguish derived-from-source files.

Files: `App.tsx:200-244`, new `lib/sidecarFiles.ts`, `appMetadataStore.ts`.

---

## Bug — BIN save reload churn
**Repro:** edit ritobin → save. The file flickers / re-renders / scrolls
because the save command writes `.bin`, which fires a watcher event, which
reloads the editor content, which reconverts ritobin, which writes a new
sidecar, which fires another event, etc.

**Root cause:** `save_ritobin_to_bin` writes the `.bin` and the conversion
also updates the `.ritobin` sidecar. The watcher doesn't know "this write
came from the app" so it treats it like an external change and triggers
the cache invalidation + version bump pipeline.

**Fix (write echo suppression):**
- Add a `pending_writes: Mutex<HashSet<PathBuf>>` to app state.
- Before any Rust command writes a file, insert the path. After the write
  completes, schedule removal on a 250ms timer (covers the watcher
  debounce window).
- In `commands/project_watcher.rs`, drop events whose path is in
  `pending_writes`.
- This is the standard pattern; VS Code calls it "self-write filtering".

Alternative (simpler, worse): on save, the editor already knows it just
wrote. Have `BinEditor.tsx` set a "just saved" flag for 500ms and ignore
the next watcher-triggered version bump for that path. Less robust because
multiple sources can save, but easier to ship.

Files: `src-tauri/src/commands/project_watcher.rs`, `commands/bin.rs`
(`save_ritobin_to_bin`), `BinEditor.tsx`.

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

## WAD explorer right side — base/empty state
Currently the right side shows nothing useful when no WAD is open.
Replace with:
- "Recently opened WADs" list (last 10), each row: WAD name, full path,
  "open" button. Source: a new `recentWads: string[]` in `configStore`.
- "Folder preview" — when the user selects a folder node in the WAD tree,
  show grid of immediate children (file icons + sizes + count). This
  doubles as the entry point for the custom file explorer (below).

Files: `WadExplorer.tsx`, `WadPreviewPanel.tsx`, `configStore.ts`.

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

## General slowness — profile, then fix
Promote this. Don't guess at the fix.

**Step 1 — measure.**
- Frontend: run a "do a thing, wait, do another thing" session with the
  React DevTools Profiler recording. Export the flame graph.
- Rust: add `tracing-flame` layer behind a `--features profile` cargo
  flag. Run a session, dump `flame.folded`, render with `inferno` to SVG.

**Step 2 — known suspects to confirm or rule out.**
- `useAppState()` in any component still subscribing to the whole store
  (per `MEMORY.md` → `store-render-performance.md`) — re-render cascades.
- BIN editor reparses on every keystroke — already known, may be the
  biggest single hit.
- Tauri IPC roundtrip overhead for many small calls (e.g. building the
  tree by listing each folder). Bulk equivalents help; we did this for
  hash resolution, may need it elsewhere.
- Image cache size and eviction policy.

**Step 3 — fix the top 3 hotspots only.** Don't refactor anything not
backed by the profile.

No new files until profiling lands.

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
