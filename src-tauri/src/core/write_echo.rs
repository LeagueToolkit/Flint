//! Write-echo suppression for the project file watcher.
//!
//! When a Tauri command writes a file (e.g. `save_ritobin_to_bin` writes both
//! the `.bin` and the `.ritobin` sidecar), the file watcher fires `Modify`
//! events for those paths. The frontend then thinks the file changed
//! externally — it invalidates caches, reloads previews, flips the editor's
//! "dirty" indicator, and so on. The result is jarring flicker / scroll-reset
//! after every save.
//!
//! The fix is "self-write filtering": before writing, the command marks the
//! target paths as "expected echoes". The watcher checks this set on every
//! event and drops matches whose timestamp is still within the suppression
//! window (default 1.5s — safely covers the debouncer's 100ms window plus
//! filesystem flush latency on Windows).
//!
//! Path normalization: we always store the canonicalized form when the file
//! exists, and fall back to a lexical-normalized absolute path when it
//! doesn't (covers the create case). The watcher canonicalizes the same way
//! before lookup.
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// How long an entry stays "expected" after being marked.
const SUPPRESSION_WINDOW: Duration = Duration::from_millis(1500);

static PENDING: OnceLock<Mutex<HashMap<PathBuf, Instant>>> = OnceLock::new();

fn pending() -> &'static Mutex<HashMap<PathBuf, Instant>> {
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Best-effort canonicalization. Falls back to the input if the file does not
/// exist yet (e.g. when marking a path right before writing it for the first
/// time). On Windows this strips the `\\?\` UNC prefix that
/// `fs::canonicalize` adds, since `notify` does not produce it.
fn normalize(path: &Path) -> PathBuf {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    strip_unc(&canonical)
}

#[cfg(windows)]
fn strip_unc(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        // Don't strip if this is a true UNC path (e.g. \\?\UNC\server\share).
        if rest.starts_with("UNC\\") {
            path.to_path_buf()
        } else {
            PathBuf::from(rest)
        }
    } else {
        path.to_path_buf()
    }
}

#[cfg(not(windows))]
fn strip_unc(path: &Path) -> PathBuf {
    path.to_path_buf()
}

/// Mark a path as an expected self-write. Call this *immediately before*
/// writing the file. Subsequent watcher events for this path within
/// `SUPPRESSION_WINDOW` will be dropped.
pub fn mark<P: AsRef<Path>>(path: P) {
    let key = normalize(path.as_ref());
    let mut map = pending().lock().unwrap();
    map.insert(key, Instant::now());
}

/// Returns true if the given path was recently marked as a self-write and the
/// mark hasn't expired. Intended to be called from the watcher's event
/// handler — drops the entry on hit so a single mark only consumes a single
/// echo (we don't want to silently swallow legitimate later modifications).
pub fn consume<P: AsRef<Path>>(path: P) -> bool {
    let key = normalize(path.as_ref());
    let mut map = pending().lock().unwrap();

    // Opportunistic prune of expired entries so the map can't grow unbounded.
    let now = Instant::now();
    map.retain(|_, t| now.duration_since(*t) < SUPPRESSION_WINDOW);

    if let Some(t) = map.remove(&key) {
        if now.duration_since(t) < SUPPRESSION_WINDOW {
            return true;
        }
    }
    false
}
