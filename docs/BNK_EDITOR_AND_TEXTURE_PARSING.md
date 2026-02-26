# BNK/WPK Editor & BIN Texture Parsing — Rust Implementation Guide

This document details everything needed to port Quartz's BNK/WPK audio editor and BIN-based texture parsing into Flint's Rust/Tauri backend.

---

## Table of Contents

1. [Overview & Goals](#1-overview--goals)
2. [BNK/WPK File Formats (Binary Spec)](#2-bnkwpk-file-formats-binary-spec)
3. [WEM Audio Format & Decoding](#3-wem-audio-format--decoding)
4. [HIRC Section — Wwise Hierarchy](#4-hirc-section--wwise-hierarchy)
5. [BIN Event Mapping (Audio Events)](#5-bin-event-mapping-audio-events)
6. [Rust Module Structure](#6-rust-module-structure)
7. [Rust Structs & Types](#7-rust-structs--types)
8. [Tauri Commands to Implement](#8-tauri-commands-to-implement)
9. [Frontend API & Types](#9-frontend-api--types)
10. [Frontend Components & Architecture](#10-frontend-components--architecture)
11. [BIN Texture Parsing for SKN/SCB](#11-bin-texture-parsing-for-skn-scb)
12. [Crate Dependencies](#12-crate-dependencies)
13. [Implementation Order](#13-implementation-order)
14. [Reference: Quartz Source Files](#14-reference-quartz-source-files)

---

## 1. Overview & Goals

### What we're building

**BNK/WPK Audio System** with two modes:

| Mode | Context | Capabilities |
|------|---------|-------------|
| **Player** | WAD Explorer | Parse BNK/WPK, list WEM entries, decode WEM→OGG, play audio |
| **Editor** | Project tabs | Everything above + replace WEM data, make silent, adjust gain, rebuild BNK/WPK, undo/redo, drag-drop, audio splitter |

**BIN Texture Discovery** for 3D model preview:
- Parse `.bin` files to extract `materialOverride` → submesh→texture mappings
- Resolve `StaticMaterialDef` entries for material→texture paths
- Map textures to SKN/SCB submeshes for Three.js rendering

### Why Rust, not JS

Quartz runs BNK parsing in JavaScript (Electron main process). In Flint's Tauri architecture:
- JS runs only in the webview (no Node.js)
- All file I/O and binary parsing belongs in the Rust backend
- Rust parsing is ~10-50x faster for binary formats
- Memory management is deterministic (no GC pauses on large BNK files)

### Existing Flint patterns to follow

- Commands: `#[tauri::command] pub async fn name(...) -> Result<T, String>`
- State: `State<'_, HashtableState>`, `State<'_, WadCacheState>`
- Errors: `crate::error::Error` enum with path context
- Modules: `core/` for logic, `commands/` for Tauri handlers
- File detection: `LeagueFileKind::WwiseBank` / `WwisePackage` already exists in `commands/file.rs:188-189`

---

## 2. BNK/WPK File Formats (Binary Spec)

### BNK (Wwise Soundbank)

All integers are **little-endian**. Sections appear in order: BKHD, DIDX, DATA, HIRC (+ others we skip).

```
┌─────────────────────────────────────────────┐
│ BNK File Layout                             │
├─────────┬───────┬───────────────────────────┤
│ Section │ Size  │ Description               │
├─────────┼───────┼───────────────────────────┤
│ BKHD    │ var   │ Bank Header               │
│ DIDX    │ N*12  │ Data Index (audio entries) │
│ DATA    │ var   │ Raw WEM audio blobs        │
│ HIRC    │ var   │ Hierarchy objects          │
│ STID    │ var   │ String table (optional)    │
│ ENVS    │ var   │ Environments (optional)    │
└─────────┴───────┴───────────────────────────┘
```

#### BKHD (Bank Header)
```
Offset  Type    Field
0x00    [u8;4]  magic = "BKHD"
0x04    u32     section_length
0x08    u32     version          // 0x58..=0x91+ (Wwise version)
0x0C    u32     bank_id
0x10    ...     (padding to section_length)
```

#### DIDX (Data Index)
```
Offset  Type    Field
0x00    [u8;4]  magic = "DIDX"
0x04    u32     section_length   // entry_count = section_length / 12
0x08    [DIDXEntry; N]

DIDXEntry (12 bytes):
  0x00  u32     file_id          // WEM file ID
  0x04  u32     offset           // Offset within DATA section
  0x08  u32     length           // Size of WEM data in bytes
```

#### DATA (Raw Audio)
```
Offset  Type    Field
0x00    [u8;4]  magic = "DATA"
0x04    u32     section_length
0x08    [u8; section_length]     // WEM blobs at DIDX[i].offset
```

Audio entries are aligned to 16-byte boundaries within DATA.

#### Writing BNK

From `bnkParser.js:writeBnkFile()`:
```
1. Write BKHD (version=0x86, section_length=0x14)
2. Write DIDX:
   - For each audio entry: (file_id, offset, length)
   - Offsets padded to 16-byte alignment
3. Write DATA:
   - Place each WEM blob at its computed offset
```

### WPK (Audio Pack)

```
┌─────────────────────────────────────┐
│ WPK File Layout                     │
├─────────┬───────────────────────────┤
│ Header  │ magic="r3d2", ver, count  │
│ Offsets │ u32[count] → entry infos  │
│ Entries │ (data_off, data_len, name)│
│ Data    │ Raw WEM blobs             │
└─────────┴───────────────────────────┘
```

#### WPK Header
```
Offset  Type    Field
0x00    [u8;4]  magic = "r3d2"
0x04    u32     version (usually 1)
0x08    u32     file_count
0x0C    [u32; file_count]  // Offset to each EntryInfo (0 = padding/skip)
```

#### WPK Entry Info (at each offset)
```
Offset  Type    Field
0x00    u32     data_offset      // Absolute offset of WEM data
0x04    u32     data_length
0x08    u32     filename_length  // Number of chars (not bytes)
0x0C    [u16; filename_length]   // UTF-16LE filename (e.g. "12345.wem")
```

#### Writing WPK

From `bnkParser.js:writeWpkFile()`:
```
1. Write header: "r3d2" + version(1) + count
2. Calculate entry info positions (8-byte aligned after header)
3. Calculate data positions (8-byte aligned after entries)
4. Write offset table → entry info positions
5. Write entry infos (data_offset, data_length, filename as UTF-16LE)
6. Write raw WEM data at computed positions
```

---

## 3. WEM Audio Format & Decoding

### What WEM is

WEM files are **Wwise-encoded audio** — typically Vorbis-in-RIFF, sometimes raw PCM WAV.

### Detection

```
Magic bytes:
- "RIFF" at offset 0 → WEM (RIFF container wrapping Vorbis or PCM)
- Check for "vorb" chunk → Vorbis encoded
- Check for "fmt " chunk with wFormatTag == 0x0001 → PCM (already WAV)
```

### Decoding Strategy for Rust

**Option A: Use the `vgmstream` C library via FFI** (recommended)
- Battle-tested decoder for all Wwise versions
- Handles every WEM variant (Vorbis, ADPCM, Opus, etc.)
- Build as static lib, link via `cc` crate or `bindgen`

**Option B: Port ww2ogg to Rust**
- Quartz's `wemConverter.js` (1520 lines) is a JS port of ww2ogg
- Core flow: parse RIFF→extract vorb chunk→rebuild OGG pages→reconstruct Vorbis setup headers
- Requires shipping `packed_codebooks_aoTuV_603.bin` (codebook data, ~300KB)
- Complex but self-contained — no external dependencies

**Option C: Use `lewton` (Rust Vorbis decoder) + custom RIFF parsing**
- Parse RIFF structure manually
- Extract Vorbis parameters from "vorb" chunk
- Reconstruct OGG stream headers
- Feed to lewton for decoding
- Most complex option but pure Rust

### Recommended approach

**Option A (vgmstream FFI)** for maximum compatibility, or **Option B (port ww2ogg)** for zero external dependencies. The ww2ogg algorithm from Quartz is well-documented in `wemConverter.js`.

### Key ww2ogg data structures (from `wemConverter.js`)

```
WwiseRiffVorbis:
  channels: u8
  sample_rate: u32
  avg_bytes_per_second: u32
  sample_count: u32
  setup_packet_offset: u32
  first_audio_packet_offset: u32
  uid: u32 (unused, from "smpl" chunk)
  blocksize_0_pow: u8
  blocksize_1_pow: u8
  mod_signal: u8 (0 or 1 — packet header format)

  // Header mode (inline vs external codebooks):
  setup_header_mode: enum { Triad, InlineCodebooks, ExternalCodebooks, ... }
```

### Audio output format

The frontend needs **OGG Vorbis bytes** or **WAV PCM bytes** that the browser's Web Audio API can decode. The Rust command returns `Vec<u8>` (raw audio bytes), and the frontend creates a `Blob` for playback.

---

## 4. HIRC Section — Wwise Hierarchy

HIRC maps Wwise objects to audio files via a tree of containers. This is needed for **event name mapping** (associating BIN event strings with WEM file IDs).

### Section layout
```
Offset  Type    Field
0x00    [u8;4]  magic = "HIRC"
0x04    u32     section_length
0x08    u32     object_count
0x0C    [HircObject; object_count]

HircObject:
  0x00  u8      type
  0x04  u32     object_length (bytes after this field)
  0x08  [u8; object_length]  // Type-specific data
```

### Object types to parse

| Type | Name | Key fields |
|------|------|-----------|
| 2 | Sound | self_id, file_id, is_streamed |
| 3 | EventAction | self_id, scope, action_type, sound_object_id, switch_group_id, switch_state_id |
| 4 | Event | self_id, event_count, event_action_ids[] |
| 5 | RandomSequenceContainer | self_id, sound_ids[] |
| 6 | SwitchContainer | self_id, group_id, children[] |
| 10 | MusicSegment | self_id, music_track_ids[] |
| 11 | MusicTrack | self_id, track_count, file_ids[], switch_group_id, switch_ids[] |
| 12 | MusicSwitchContainer | self_id, children[], arguments[], decision_tree_nodes[] |
| 13 | MusicPlaylistContainer | (same structure as MusicSegment) |

### BaseParams skip logic

Many objects contain "BaseParams" — a complex variable-length structure that must be skipped to reach the useful fields. The skip functions are version-dependent:

```rust
fn skip_base_params(reader: &mut R, version: u32) -> u32 {
    skip_initial_fx_params(reader, version);
    if version > 0x88 { skip_metadata_fx(reader); }
    if version > 0x59 && version <= 0x91 { reader.skip(1); }
    let bus_id = reader.read_u32();
    let parent_id = reader.read_u32();
    reader.skip(if version <= 0x59 { 2 } else { 1 });
    skip_initial_params(reader);
    skip_positioning_params(reader, version);
    skip_aux_params(reader, version);
    reader.skip(6);
    skip_state_chunk(reader);
    skip_rtpc(reader, version);
    parent_id
}
```

See `bnkParser.js` lines 443-547 for all skip functions with exact byte counts per version.

### Event→WEM resolution chain

```
BIN event string → FNV-1 hash
  → HIRC Event (type 4) matching hash
    → EventAction (type 3) with action_type
      → if Play (4): follow sound_object_id
        → Sound (type 2): get file_id → WEM
        → RandomContainer (type 5): iterate sound_ids
        → SwitchContainer (type 6): iterate children
        → MusicSegment (type 10): iterate tracks
          → MusicTrack (type 11): get file_ids
      → if SetSwitch (25): match switch_group + state
        → MusicTrack with matching switch → file_ids
      → if SetState (18): match state_group + target
        → MusicSwitchContainer decision tree → children
```

---

## 5. BIN Event Mapping (Audio Events)

BIN files contain audio event names as embedded strings. Quartz uses **pattern matching** on magic bytes to find them (not full BIN parsing).

### Event container pattern
```
Search for: [0x84, 0xE3, 0xD8, 0x12, 0x80, 0x10]
Then:
  skip(4)           // object size
  amount = u32      // number of event strings
  for each:
    len = u16       // string length
    string = [u8; len]  // event name (ASCII)
    hash = fnv1(string) // FNV-1 hash for HIRC lookup
```

### Music container pattern
```
Search for: [0xD4, 0x4F, 0x9C, 0x9F, 0x83]
Then:
  type_hash = u32   // skip if 0
  skip(4)           // object size
  amount = u16      // number of entries
  for each:
    skip(4)         // name hash
    bin_type = u8   // must be 0x10 (string)
    len = u16
    string = [u8; len]
    hash = fnv1(string)
```

### FNV-1 Hash (NOT FNV-1a)

Wwise uses **FNV-1** (multiply-then-XOR), not FNV-1a:

```rust
fn fnv1_hash(input: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(0x01000193);
        let c = if byte >= b'A' && byte <= b'Z' { byte + 32 } else { byte };
        hash ^= c as u32;
    }
    hash
}
```

Note: This is different from the FNV-1**a** used in BIN field hashing. The order of operations differs (FNV-1: multiply then XOR; FNV-1a: XOR then multiply).

---

## 6. Rust Module Structure

### New files to create

```
src-tauri/src/
├── core/
│   └── audio/
│       ├── mod.rs              // Module declarations
│       ├── bnk.rs              // BNK parser + writer
│       ├── wpk.rs              // WPK parser + writer
│       ├── wem.rs              // WEM→OGG decoder (ww2ogg port or FFI)
│       ├── hirc.rs             // HIRC section parser (all object types)
│       └── event_mapper.rs     // BIN event extraction + HIRC→WEM mapping
│
├── commands/
│   └── audio.rs                // Tauri command handlers
```

### Existing files to modify

```
src-tauri/src/main.rs           // Register new commands
src-tauri/src/commands/mod.rs   // Add `pub mod audio;`
src-tauri/src/core/mod.rs       // Add `pub mod audio;`
src-tauri/src/error.rs          // Add Audio variant (optional)

src/lib/api.ts                  // Add frontend API wrappers
src/lib/types.ts                // Add audio types
```

---

## 7. Rust Structs & Types

### core/audio/bnk.rs

```rust
use serde::{Serialize, Deserialize};

/// A single WEM audio entry extracted from BNK/WPK
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEntry {
    pub id: u32,              // WEM file ID
    pub size: u32,            // Data length in bytes
    #[serde(skip)]
    pub data: Vec<u8>,        // Raw WEM bytes (skip in JSON serialization)
}

/// Metadata-only audio entry (for listing without loading data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEntryInfo {
    pub id: u32,
    pub size: u32,
}

/// Parsed BNK file
#[derive(Debug)]
pub struct BnkFile {
    pub version: u32,
    pub bank_id: u32,
    pub entries: Vec<AudioEntry>,
    /// Raw HIRC section bytes (if present) — parsed separately
    pub hirc_data: Option<Vec<u8>>,
    /// Full file bytes for re-serialization
    raw_sections: Vec<BnkSection>,
}

/// Represents a raw section we need to preserve during round-trip
#[derive(Debug, Clone)]
struct BnkSection {
    magic: [u8; 4],
    data: Vec<u8>,
}

/// Parsed WPK file
#[derive(Debug)]
pub struct WpkFile {
    pub version: u32,
    pub entries: Vec<AudioEntry>,
}

impl BnkFile {
    /// Parse from raw bytes
    pub fn parse(data: &[u8]) -> Result<Self, String> { ... }

    /// List entries without loading audio data (metadata only)
    pub fn parse_metadata(data: &[u8]) -> Result<Vec<AudioEntryInfo>, String> { ... }

    /// Get a single entry's audio data by ID
    pub fn read_entry_data(data: &[u8], file_id: u32) -> Result<Vec<u8>, String> { ... }

    /// Write modified BNK back to bytes
    pub fn write(entries: &[AudioEntry]) -> Vec<u8> { ... }

    /// Replace an entry's audio data (returns new BNK bytes)
    pub fn replace_entry(data: &[u8], file_id: u32, new_wem: &[u8]) -> Result<Vec<u8>, String> { ... }

    /// Replace an entry with silence
    pub fn silence_entry(data: &[u8], file_id: u32, silence_wem: &[u8]) -> Result<Vec<u8>, String> { ... }
}

impl WpkFile {
    pub fn parse(data: &[u8]) -> Result<Self, String> { ... }
    pub fn parse_metadata(data: &[u8]) -> Result<Vec<AudioEntryInfo>, String> { ... }
    pub fn read_entry_data(data: &[u8], file_id: u32) -> Result<Vec<u8>, String> { ... }
    pub fn write(entries: &[AudioEntry]) -> Vec<u8> { ... }
    pub fn replace_entry(data: &[u8], file_id: u32, new_wem: &[u8]) -> Result<Vec<u8>, String> { ... }
}
```

### core/audio/hirc.rs

```rust
/// All HIRC object types we care about
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HircData {
    pub sounds: Vec<HircSound>,
    pub event_actions: Vec<HircEventAction>,
    pub events: Vec<HircEvent>,
    pub random_containers: Vec<HircRandomContainer>,
    pub switch_containers: Vec<HircSwitchContainer>,
    pub music_segments: Vec<HircMusicContainer>,
    pub music_tracks: Vec<HircMusicTrack>,
    pub music_switches: Vec<HircMusicSwitch>,
    pub music_playlists: Vec<HircMusicContainer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HircSound {
    pub self_id: u32,
    pub file_id: u32,
    pub is_streamed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HircEventAction {
    pub self_id: u32,
    pub scope: u8,
    pub action_type: u8,
    pub sound_object_id: u32,
    pub switch_group_id: u32,
    pub switch_state_id: u32,
    pub state_group_id: u32,
    pub target_state_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HircEvent {
    pub self_id: u32,
    pub action_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HircRandomContainer {
    pub self_id: u32,
    pub parent_id: u32,
    pub sound_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HircSwitchContainer {
    pub self_id: u32,
    pub parent_id: u32,
    pub group_type: u8,
    pub group_id: u32,
    pub children: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HircMusicContainer {
    pub self_id: u32,
    pub parent_id: u32,
    pub track_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HircMusicTrack {
    pub self_id: u32,
    pub parent_id: u32,
    pub track_count: u32,
    pub file_ids: Vec<u32>,
    pub has_switch_ids: bool,
    pub switch_group_id: u32,
    pub switch_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HircMusicSwitch {
    pub self_id: u32,
    pub parent_id: u32,
    pub children: Vec<u32>,
    pub arguments: Vec<MusicSwitchArg>,
    pub decision_nodes: Vec<DecisionNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicSwitchArg {
    pub group_id: u32,
    pub group_type: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionNode {
    pub key: u32,
    pub audio_id: u32,
}

impl HircData {
    /// Parse HIRC from a BNK file's raw bytes
    pub fn parse(bnk_data: &[u8]) -> Result<Option<Self>, String> { ... }
}
```

### core/audio/event_mapper.rs

```rust
/// A BIN event string with its FNV-1 hash
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinEventString {
    pub name: String,
    pub hash: u32,
}

/// A mapped event → WEM file relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMapping {
    pub event_name: String,
    pub wem_id: u32,
    pub container_id: u32,
    pub music_segment_id: Option<u32>,
    pub switch_id: Option<u32>,
}

/// Extract event strings from a BIN file using pattern matching
pub fn extract_bin_events(bin_data: &[u8]) -> Vec<BinEventString> { ... }

/// Map BIN events to WEM IDs via HIRC hierarchy
pub fn map_events_to_wem(
    events: &[BinEventString],
    hirc: &HircData,
) -> Vec<EventMapping> { ... }
```

### core/audio/wem.rs

```rust
/// Decode WEM bytes to playable audio (OGG Vorbis or WAV PCM)
///
/// Returns (audio_bytes, format) where format is "ogg" or "wav"
pub fn decode_wem(wem_data: &[u8], codebook: &[u8]) -> Result<(Vec<u8>, String), String> { ... }

/// Check if WEM data is already a standard WAV file
pub fn is_wem_pcm(wem_data: &[u8]) -> bool { ... }
```

---

## 8. Tauri Commands to Implement

### commands/audio.rs

```rust
// ============================================================
// READ-ONLY COMMANDS (WAD Explorer + Project)
// ============================================================

/// Parse BNK/WPK file from disk, return entry list (no audio data)
#[tauri::command]
pub async fn parse_audio_bank(path: String) -> Result<AudioBankInfo, String>

/// Parse BNK/WPK from raw bytes (for WAD Explorer in-memory chunks)
#[tauri::command]
pub async fn parse_audio_bank_bytes(data: Vec<u8>) -> Result<AudioBankInfo, String>

/// Read a single WEM entry from a BNK/WPK file
#[tauri::command]
pub async fn read_audio_entry(path: String, file_id: u32) -> Result<Vec<u8>, String>

/// Read a single WEM entry from in-memory BNK/WPK bytes
#[tauri::command]
pub async fn read_audio_entry_bytes(data: Vec<u8>, file_id: u32) -> Result<Vec<u8>, String>

/// Decode WEM bytes to playable audio (OGG or WAV)
#[tauri::command]
pub async fn decode_wem(wem_data: Vec<u8>) -> Result<DecodedAudio, String>

/// Parse HIRC from a BNK file for event hierarchy
#[tauri::command]
pub async fn parse_bnk_hirc(path: String) -> Result<HircData, String>

/// Extract event names from a BIN file
#[tauri::command]
pub async fn extract_bin_events(data: Vec<u8>) -> Result<Vec<BinEventString>, String>

/// Map BIN events to WEM IDs via HIRC hierarchy
#[tauri::command]
pub async fn map_audio_events(
    bin_data: Vec<u8>,
    events_bnk_data: Vec<u8>,
) -> Result<Vec<EventMapping>, String>

// ============================================================
// EDIT COMMANDS (Project mode only)
// ============================================================

/// Replace a WEM entry in a BNK/WPK file, return modified file bytes
#[tauri::command]
pub async fn replace_audio_entry(
    bank_data: Vec<u8>,
    file_id: u32,
    new_wem_data: Vec<u8>,
) -> Result<Vec<u8>, String>

/// Replace multiple WEM entries at once
#[tauri::command]
pub async fn replace_audio_entries(
    bank_data: Vec<u8>,
    replacements: Vec<AudioReplacement>,
) -> Result<Vec<u8>, String>

/// Silence a WEM entry (replace with silence.wem)
#[tauri::command]
pub async fn silence_audio_entry(
    bank_data: Vec<u8>,
    file_id: u32,
) -> Result<Vec<u8>, String>

/// Rebuild a full BNK file from a list of audio entries
#[tauri::command]
pub async fn write_bnk(entries: Vec<AudioEntryData>) -> Result<Vec<u8>, String>

/// Rebuild a full WPK file from a list of audio entries
#[tauri::command]
pub async fn write_wpk(entries: Vec<AudioEntryData>) -> Result<Vec<u8>, String>

/// Save audio bytes to disk (BNK, WPK, or extracted WEM)
#[tauri::command]
pub async fn save_audio_file(path: String, data: Vec<u8>) -> Result<(), String>

// ============================================================
// RETURN TYPES
// ============================================================

#[derive(Serialize, Deserialize)]
pub struct AudioBankInfo {
    pub format: String,          // "bnk" or "wpk"
    pub version: u32,
    pub entry_count: usize,
    pub entries: Vec<AudioEntryInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct DecodedAudio {
    pub data: Vec<u8>,           // Raw OGG/WAV bytes
    pub format: String,          // "ogg" or "wav"
    pub sample_rate: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct AudioReplacement {
    pub file_id: u32,
    pub new_data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct AudioEntryData {
    pub id: u32,
    pub data: Vec<u8>,
}
```

### Register in main.rs

```rust
// Audio commands (BNK/WPK editor)
commands::audio::parse_audio_bank,
commands::audio::parse_audio_bank_bytes,
commands::audio::read_audio_entry,
commands::audio::read_audio_entry_bytes,
commands::audio::decode_wem,
commands::audio::parse_bnk_hirc,
commands::audio::extract_bin_events,
commands::audio::map_audio_events,
commands::audio::replace_audio_entry,
commands::audio::replace_audio_entries,
commands::audio::silence_audio_entry,
commands::audio::write_bnk,
commands::audio::write_wpk,
commands::audio::save_audio_file,
```

---

## 9. Frontend API & Types

### src/lib/types.ts additions

```typescript
// =============================================================================
// Audio / BNK Editor Types
// =============================================================================

export interface AudioEntryInfo {
    id: number;
    size: number;
}

export interface AudioBankInfo {
    format: 'bnk' | 'wpk';
    version: number;
    entry_count: number;
    entries: AudioEntryInfo[];
}

export interface DecodedAudio {
    data: number[];   // Raw bytes (Tauri serializes Vec<u8> as number[])
    format: 'ogg' | 'wav';
    sample_rate: number | null;
}

export interface BinEventString {
    name: string;
    hash: number;
}

export interface EventMapping {
    event_name: string;
    wem_id: number;
    container_id: number;
    music_segment_id: number | null;
    switch_id: number | null;
}

/** Tree node for the BNK editor UI */
export interface AudioTreeNode {
    id: string;
    name: string;
    audioEntry: AudioEntryInfo | null;  // null = folder node
    children: AudioTreeNode[];
}
```

### src/lib/api.ts additions

```typescript
// =============================================================================
// Audio / BNK Editor API
// =============================================================================

export async function parseAudioBank(path: string): Promise<AudioBankInfo> {
    return invoke<AudioBankInfo>('parse_audio_bank', { path });
}

export async function parseAudioBankBytes(data: Uint8Array): Promise<AudioBankInfo> {
    return invoke<AudioBankInfo>('parse_audio_bank_bytes', { data: Array.from(data) });
}

export async function readAudioEntry(path: string, fileId: number): Promise<Uint8Array> {
    const bytes = await invoke<number[]>('read_audio_entry', { path, fileId });
    return new Uint8Array(bytes);
}

export async function readAudioEntryBytes(data: Uint8Array, fileId: number): Promise<Uint8Array> {
    const bytes = await invoke<number[]>('read_audio_entry_bytes', {
        data: Array.from(data), fileId
    });
    return new Uint8Array(bytes);
}

export async function decodeWem(wemData: Uint8Array): Promise<DecodedAudio> {
    return invoke<DecodedAudio>('decode_wem', { wemData: Array.from(wemData) });
}

export async function mapAudioEvents(
    binData: Uint8Array,
    eventsBnkData: Uint8Array,
): Promise<EventMapping[]> {
    return invoke<EventMapping[]>('map_audio_events', {
        binData: Array.from(binData),
        eventsBnkData: Array.from(eventsBnkData),
    });
}

export async function replaceAudioEntry(
    bankData: Uint8Array, fileId: number, newWemData: Uint8Array,
): Promise<Uint8Array> {
    const bytes = await invoke<number[]>('replace_audio_entry', {
        bankData: Array.from(bankData), fileId, newWemData: Array.from(newWemData),
    });
    return new Uint8Array(bytes);
}

export async function writeBnk(entries: { id: number; data: Uint8Array }[]): Promise<Uint8Array> {
    const bytes = await invoke<number[]>('write_bnk', {
        entries: entries.map(e => ({ id: e.id, data: Array.from(e.data) })),
    });
    return new Uint8Array(bytes);
}

export async function writeWpk(entries: { id: number; data: Uint8Array }[]): Promise<Uint8Array> {
    const bytes = await invoke<number[]>('write_wpk', {
        entries: entries.map(e => ({ id: e.id, data: Array.from(e.data) })),
    });
    return new Uint8Array(bytes);
}

export async function saveAudioFile(path: string, data: Uint8Array): Promise<void> {
    return invoke('save_audio_file', { path, data: Array.from(data) });
}
```

---

## 10. Frontend Components & Architecture

### Component hierarchy

```
BnkEditor (project mode — full editor)
├── BnkEditorHeader
│   ├── File inputs (BNK path, BIN path, WPK path)
│   ├── Parse button
│   ├── View mode toggle (Normal / Split)
│   └── Status bar
├── BnkEditorContent
│   ├── LeftPane (main BNK tree)
│   │   ├── SearchBar
│   │   └── AudioTreeView (recursive AudioTreeNode)
│   ├── RightPane (split mode — reference/comparison)
│   │   ├── SearchBar
│   │   ├── Sort controls
│   │   └── AudioTreeView (drop target)
│   └── ActionSidebar
│       ├── Undo / Redo
│       ├── Extract
│       ├── Replace
│       ├── Make Silent
│       ├── Save BNK/WPK
│       ├── Play / Stop
│       └── Volume slider
├── BnkContextMenu (right-click)
├── AudioSplitter (overlay — waveform editor)
└── BnkSettingsModal

BnkPlayer (WAD Explorer mode — read-only)
├── AudioTreeView (flat list of WEM entries)
├── Play / Stop controls
├── Volume slider
└── (no edit controls)
```

### Hooks to implement (mirrors Quartz)

| Hook | Purpose |
|------|---------|
| `useBnkParser` | Call Rust parse commands, manage parsed state |
| `useBnkTreeState` | Selection, expansion, search filtering |
| `useBnkPlayback` | Web Audio API playback, decode WEM via Rust |
| `useBnkFileOps` | Extract, replace, silence, save (calls Rust commands) |
| `useBnkHistory` | Undo/redo with memory-aware snapshots |
| `useBnkHotkeys` | Keyboard shortcuts (Delete, Space, Ctrl+Z/Y) |

### Playback flow

```
User clicks WEM entry
  → Frontend calls decodeWem(wemData) via Rust
  ← Rust returns OGG/WAV bytes
  → Frontend creates Blob → URL.createObjectURL
  → Web Audio API: AudioContext.decodeAudioData → BufferSource.start()
```

### Tree building from event mappings

```
1. Parse BNK → get flat AudioEntryInfo[]
2. Parse BIN → get BinEventString[]
3. Parse events BNK HIRC → get HircData
4. Map events → EventMapping[]
5. Build tree:
   - Root node (filename)
     - Event name folders (from EventMapping.event_name)
       - Container folders (from EventMapping.container_id, if >1 child)
         - WEM leaves (ID.wem with AudioEntryInfo)
     - Unmatched WEM entries (no event mapping)
```

---

## 11. BIN Texture Parsing for SKN/SCB

### What Flint already has

Flint already has `core/mesh/bin_texture_discovery.rs` and `core/mesh/texture.rs` for texture resolution. Check these files before implementing — they may already handle this. The existing system parses `.ritobin` text files for texture hints.

### What Quartz does differently

Quartz parses **raw binary .bin files** (not converted to text first) using a full BIN type system. The key logic is in `modelInspect.js:discoverMaterialTextureHints()`.

### Three-pass texture discovery

**Pass 1: Material Override Extraction**

Scan all BIN entries for `materialOverride` fields (type LIST of EMBED):
```
For each BIN entry:
  Find field named "materialoverride" (type 128/129 = LIST/LIST2)
    For each EMBED item (type 131):
      Find "submesh" field → submesh name (string)
      Find "material" field → material reference path (string/hash/link)
      Find "texture" field → direct texture path (string, type FILE=18)

      Store: submeshToMaterial[submesh] = material
      If texture path found: submeshToTexture[submesh] = texture (HIGH PRIORITY)
```

Also scan for `SkinMeshDataProperties` entries:
```
Look for EMBED with type containing "skinmeshdataproperties"
  OR fields containing both "simpleskin" and "materialoverride"
  Extract: simpleSkinPath, materialRef, texturePath
  Store as default fallback texture
```

**Pass 2: StaticMaterialDef → Diffuse Texture**

Scan entries where type contains "staticmaterialdef" or name contains "/materials/":
```
For each StaticMaterialDef entry:
  Find "samplervalues" field (LIST)
    For each sampler:
      Find "texturename" → must contain "diffuse_texture"
      Find "texturepath" → the actual .dds/.tex path

      Store: materialToTexture[materialName] = texturePath
      (Also store aliases: basename, basename without _inst, normalized)
```

**Pass 3: Join**

```
For each submesh → material mapping:
  texture = submeshToTexture[submesh]  // Direct texture (highest priority)
           ?? materialToTexture[material]  // Via material definition

  If texture valid and matches character/skin filter:
    hints[submesh] = texture

  Fallback: use SkinMeshDataProperties default texture
  Store as hints["__default__"]
```

### Character/skin filtering

Quartz filters texture paths to ensure they match the selected champion + skin:
```
Allow: paths containing /shared/, /global/, /common/
Block: paths from other champions (different /characters/X/ folder)
Block: paths from other skins (different /skins/X/ folder)
```

### Texture file resolution

Given a hint path like `characters/ahri/skins/base/textures/ahri_base_tx.dds`:

1. **Exact match**: Check if extracted files contain this exact relative path
2. **Suffix match**: Check if any file path ends with this path (handles `assets/` prefix differences)
3. **Basename match**: Check if any file has matching filename (e.g. `ahri_base_tx.dds`)

### What to implement in Flint

Since Flint already has `bin_texture_discovery.rs`, extend it to support **raw binary BIN parsing** in addition to the existing `.ritobin` text parsing. The existing `ltk_bridge.rs` already has `read_bin(bytes) → BinTree` which gives structured access to all fields. Use this instead of porting Quartz's pattern-matching approach.

Recommended approach:
```rust
// In core/mesh/bin_texture_discovery.rs

/// Extract material texture hints from raw BIN bytes
pub fn discover_texture_hints_from_bin(
    bin_data: &[u8],
    hashtable: &Hashtable,
    skin_filter: &SkinFilter,
) -> TextureHints {
    let tree = read_bin(bin_data);  // Uses existing ltk_bridge

    let mut submesh_to_material = HashMap::new();
    let mut submesh_to_texture = HashMap::new();
    let mut material_to_texture = HashMap::new();

    for entry in tree.entries {
        // Pass 1: Find materialOverride fields
        scan_material_overrides(&entry, &mut submesh_to_material, &mut submesh_to_texture);

        // Pass 2: Find StaticMaterialDef entries
        if is_static_material_def(&entry, hashtable) {
            extract_diffuse_texture(&entry, hashtable, &mut material_to_texture);
        }
    }

    // Pass 3: Join
    join_hints(submesh_to_material, submesh_to_texture, material_to_texture, skin_filter)
}
```

---

## 12. Crate Dependencies

Add to `src-tauri/Cargo.toml`:

```toml
[dependencies]
# Existing deps (already present)
# serde, tokio, rayon, tracing, etc.

# NEW: Audio format handling
# Option A: vgmstream FFI
# vgmstream-sys = { path = "../vgmstream-sys" }  # Custom FFI wrapper

# Option B: Pure Rust OGG construction
ogg = "0.9"              # OGG page framing (if porting ww2ogg)
# OR
lewton = "0.10"          # Vorbis decoding (if needed for validation)

# For WEM→WAV PCM handling
hound = "3.5"            # WAV reading/writing (lightweight)
```

If porting ww2ogg to Rust, you also need to ship `packed_codebooks_aoTuV_603.bin` as a bundled resource. Add to `tauri.conf.json`:
```json
{
  "bundle": {
    "resources": ["resources/packed_codebooks_aoTuV_603.bin", "resources/silence.wem"]
  }
}
```

---

## 13. Implementation Order

### Phase 1: BNK/WPK Parser (read-only)

1. Create `core/audio/mod.rs`, `bnk.rs`, `wpk.rs`
2. Implement `BnkFile::parse()` and `WpkFile::parse()` — just BKHD + DIDX + DATA
3. Implement `parse_metadata()` for listing without loading all audio data
4. Create `commands/audio.rs` with `parse_audio_bank` and `parse_audio_bank_bytes`
5. Register commands in `main.rs`
6. Add frontend API wrappers
7. Test: load a BNK file, verify entry count and IDs match Quartz

### Phase 2: WEM Decoding

1. Create `core/audio/wem.rs`
2. Choose approach (vgmstream FFI vs ww2ogg port)
3. If ww2ogg port: translate `WwiseRiffVorbis` class from `wemConverter.js`
   - RIFF parsing → vorb chunk extraction
   - OGG page construction
   - Vorbis header rebuilding (identification, comment, setup packets)
   - Codebook library loading
4. Implement `decode_wem` command
5. Bundle codebook file
6. Test: decode WEM → play in browser

### Phase 3: Frontend Player (WAD Explorer)

1. Create `BnkPlayer` component (read-only)
2. Implement `useBnkPlayback` hook (Web Audio API)
3. Integrate into WAD Explorer for `audio/x-wwise-bnk` and `audio/x-wwise-wpk` files
4. Add volume control, play/stop

### Phase 4: HIRC + Event Mapping

1. Create `core/audio/hirc.rs` — parse all object types
2. Create `core/audio/event_mapper.rs` — pattern matching on BIN + HIRC traversal
3. Implement `parse_bnk_hirc`, `extract_bin_events`, `map_audio_events` commands
4. Build tree structure on frontend from event mappings
5. Test with champion BNK files that have BIN event data

### Phase 5: BNK/WPK Writer (edit mode)

1. Implement `BnkFile::write()` and `WpkFile::write()` in Rust
2. Implement `replace_entry()`, `silence_entry()`
3. Bundle `silence.wem` resource
4. Add edit commands: `replace_audio_entry`, `silence_audio_entry`, `write_bnk`, `write_wpk`
5. Test round-trip: parse → modify → write → re-parse → verify

### Phase 6: Full Editor UI (Project mode)

1. Create `BnkEditor` component (full editor with split view)
2. Implement all hooks: tree state, file ops, history, hotkeys
3. Add: replace, extract, make silent, save, undo/redo
4. Drag-and-drop support
5. Context menu
6. Audio splitter (optional — complex, can be deferred)

### Phase 7: BIN Texture Discovery Enhancement

1. Extend `core/mesh/bin_texture_discovery.rs` with raw BIN parsing
2. Use existing `ltk_bridge::read_bin()` for structured access
3. Implement the 3-pass material→texture resolution
4. Add skin/character filtering
5. Wire into existing mesh preview commands

---

## 14. Reference: Quartz Source Files

### BNK/WPK Parser & Writer
| File | Lines | Key exports |
|------|-------|-------------|
| `src/pages/bnkextract/utils/bnkParser.js` | 1105 | `parseBnkFile`, `parseWpkFile`, `parseHirc`, `parseBinFile`, `getEventMappings`, `fnv1Hash`, `groupAudioFiles`, `writeBnkFile`, `writeWpkFile` |

### WEM Converter
| File | Lines | Key exports |
|------|-------|-------------|
| `src/pages/bnkextract/utils/wemConverter.js` | 1520 | `wemToOgg`, `isWemWav`, `wemToWav`, `wemToMp3`, `decodeToAudioBuffer`, `audioBufferToWav` |

### Editor Hooks
| File | Lines | Purpose |
|------|-------|---------|
| `hooks/useBnkFileParsing.js` | ~80 | File selection dialogs, parse trigger |
| `hooks/useBnkAudioPlayback.js` | 157 | Web Audio playback, volume, WEM→OGG |
| `hooks/useBnkFileOps.js` | 370 | Extract, replace, silence, save |
| `hooks/useBnkTreeState.js` | ~100 | Selection, expansion, clear |
| `hooks/useBnkSelectionActions.js` | ~100 | Play selected, context menu, delete, copy |
| `hooks/useBnkHistory.js` | 102 | Undo/redo with memory cap (256MB) |
| `hooks/useBnkDropOps.js` | 229 | Drag-drop replacement, external file import |
| `hooks/useBnkGainOps.js` | 115 | Audio gain/amplify via Wwise |
| `hooks/useBnkSplitterActions.js` | 183 | Audio splitter integration |
| `hooks/useBnkWwiseBridge.js` | ~150 | External Wwise SDK install/conversion |
| `hooks/useBnkCodebookLoader.js` | 65 | Codebook binary loading |
| `hooks/useBnkHotkeys.js` | 35 | Keyboard shortcuts |
| `hooks/useBnkSearch.js` | ~60 | Debounced search filtering |
| `hooks/useBnkPersistence.js` | ~40 | localStorage for paths/history |

### Editor Components
| File | Lines | Purpose |
|------|-------|---------|
| `components/BnkMainContent.js` | 346 | Dual-pane tree + action sidebar |
| `components/BnkHeaderPanel.js` | 284 | Header bar with file inputs |
| `components/TreeNode.js` | 221 | Recursive tree node (memo, drag-drop) |
| `components/BnkContextMenu.js` | ~80 | Right-click menu |
| `components/AudioSplitter.js` | 1095 | WaveSurfer.js waveform editor |
| `components/BnkSettingsModal.js` | ~60 | Settings checkboxes |
| `components/BnkInstallModal.js` | ~40 | Wwise install dialog |
| `components/BnkGainModal.js` | ~50 | Gain adjustment dialog |
| `components/BnkLoadingOverlay.js` | ~30 | Loading animation |
| `components/BnkConvertOverlay.js` | ~30 | Conversion progress |

### Texture Parsing (BIN → Model)
| File | Lines | Purpose |
|------|-------|---------|
| `src/main/ipc/channels/modelInspect.js` | 724 | Full BIN texture discovery pipeline |
| `src/services/modelInspectViewerService.js` | 450 | SKN loading, texture map building |
| `src/jsritofile/bin.js` | 462 | BIN binary parser (PROP/PTCH) |
| `src/jsritofile/binReader.js` | 222 | BIN field/entry reader (35+ types) |
| `src/jsritofile/binTypes.js` | 53 | BINType enum (0-135) |
| `src/jsritofile/binHasher.js` | 76 | FNV-1a hash lookup |
| `src/jsritofile/wadHasher.js` | 154 | xxhash64 WAD path hashing |
| `src/jsritofile/skn.js` | 325 | SKN mesh parser |
| `src/jsritofile/scb.js` | 225 | SCB static mesh parser |

---

## Key Differences from Quartz

1. **No Electron IPC** — Tauri commands replace all `ipcRenderer.invoke()` calls
2. **No Node.js fs** — All file I/O in Rust, not JS
3. **No external Wwise SDK** — WEM decoding is built into the Rust backend (no install modal needed)
4. **Binary data transfer** — Tauri serializes `Vec<u8>` as `number[]`; frontend converts via `new Uint8Array()`
5. **Audio playback stays in JS** — Web Audio API runs in the browser; Rust only does decoding
6. **Existing BIN parsing** — Flint already has `ltk_bridge::read_bin()` for structured BIN access; use it instead of porting Quartz's pattern-matching approach
7. **Existing SKN/SCB** — Flint already has `core/mesh/skn.rs` and `core/mesh/scb.rs`; texture discovery enhances these, doesn't replace them
