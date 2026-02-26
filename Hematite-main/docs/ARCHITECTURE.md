# Hematite Architecture

## Overview

Hematite uses a **three-phase pipeline** with remote configuration:

```
User Input → Analyzer → User Selection → Fixer → Output
     ↑                                              ↓
     └──────────── Remote Config (GitHub) ──────────┘
```

## Core Components

### 1. Config System (`src-tauri/src/config/`)

**Purpose:** Fetch and cache fix rules from GitHub

**Files:**
- `schema.rs` - Rust types for config JSON
- `fetcher.rs` - HTTP client for downloading config
- `cache.rs` - Local TTL-based cache

**Design Decision:**
We use remote config so users get updated fix logic without reinstalling the app. If GitHub is down, we fall back to cached version.

### 2. Analyzer (`src-tauri/src/analyzer/`)

**Purpose:** Scan files and detect issues WITHOUT modifying them

**Files:**
- `scanner.rs` - File/folder discovery (handles .fantome, .zip, .wad.client)
- `detector.rs` - Checks parsed files against config rules

**Flow:**
```
Input Path → Extract Archive → Parse BINs → Check Rules → Return ScanResult[]
```

### 3. Fixer (`src-tauri/src/fixer/`)

**Purpose:** Apply user-confirmed fixes

**Files:**
- `applier.rs` - Modifies BIN files using league-toolkit

**Flow:**
```
Selected Fixes → Load File → Apply Transforms → Serialize → Repack Archive
```

### 4. Commands (`src-tauri/src/commands/`)

**Purpose:** Tauri IPC layer between frontend and backend

**Commands:**
- `analyze_path(path: String) -> Result<Vec<ScanResult>, String>`
- `apply_fixes(path: String, fixes: Vec<String>) -> Result<String, String>`

## Data Flow

```
┌─────────────┐
│   Frontend  │
│  (Vanilla)  │
└──────┬──────┘
       │ invoke("analyze_path")
       ↓
┌─────────────┐
│  Commands   │
└──────┬──────┘
       │
       ↓
┌─────────────┐      ┌──────────────┐
│  Analyzer   │─────→│ Config Rules │
└──────┬──────┘      └──────────────┘
       │
       ↓
┌─────────────┐
│ ScanResult  │
│   (JSON)    │
└──────┬──────┘
       │ return to frontend
       ↓
┌─────────────┐
│ User checks │
│  checkboxes │
└──────┬──────┘
       │ invoke("apply_fixes")
       ↓
┌─────────────┐      ┌──────────────┐
│   Fixer     │─────→│league-toolkit│
└─────────────┘      └──────────────┘
```

## Design Principles

1. **Immutable Analysis:** Never modify files during scanning
2. **Config-Driven:** All fix logic in JSON, not hardcoded
3. **Type Safety:** BIN modifications use strongly-typed enums
4. **Fail-Safe:** Batch operations continue on individual file failures

---

**Last Updated:** 2026-01-11
