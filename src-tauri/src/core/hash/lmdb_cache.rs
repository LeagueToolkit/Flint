/// Global LMDB Environment Cache
///
/// Opened once per hash directory, reused for all reads.
/// OS memory-maps the file — only physically pages in what's actually touched.
///
/// This is a direct port of the cache described in the sibling `lib.rs` design file:
/// ```
///   static LMDB_CACHE: OnceLock<Mutex<Option<(String, Arc<heed::Env>)>>> = OnceLock::new();
/// ```
///
/// # Memory characteristics
/// - 1 GB virtual address space reserved (no physical RAM committed)
/// - Only accessed B-tree pages are faulted in from disk
/// - A typical resolve of ~4 000 hashes warms ≈ 5-20 MB of real RAM
///
/// # Thread safety
/// The inner `Mutex` serialises open/swap operations.
/// Once an `Arc<heed::Env>` is cloned out, concurrent read transactions are
/// lock-free (LMDB MVCC allows unlimited concurrent readers).
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use heed::{EnvOpenOptions};
use heed::types::{Bytes, Str};

// ── Statics ─────────────────────────────────────────────────────────────────────

/// Cache of the currently-open heed::Env.
#[allow(clippy::type_complexity)]
static LMDB_CACHE: OnceLock<Mutex<Option<(String, Arc<heed::Env>)>>> = OnceLock::new();

/// Serialises concurrent `build_hash_db` callers.
///
/// Without this, a background startup build (6-8 s) and an immediate
/// foreground "Create Project" call both call `build_hash_db` concurrently.
/// The second caller sees a partially-written DB, opens it, gets 0 lookups.
///
/// With this lock, the second caller simply **waits** behind the first and
/// then opens the fully-committed DB.
static BUILD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lmdb_mutex() -> &'static Mutex<Option<(String, Arc<heed::Env>)>> {
    LMDB_CACHE.get_or_init(|| Mutex::new(None))
}

fn build_mutex() -> &'static Mutex<()> {
    BUILD_LOCK.get_or_init(|| Mutex::new(()))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Return the cached `heed::Env` for `hash_dir`, opening it if needed.
///
/// `hash_dir` must contain a `hashes.lmdb/` sub-directory (created by
/// `build_hash_db`). Returns `None` if the directory doesn't exist or
/// the env cannot be opened.
pub fn get_or_open_env(hash_dir: &str) -> Option<Arc<heed::Env>> {
    let lmdb_dir = Path::new(hash_dir).join("hashes.lmdb");
    if !lmdb_dir.exists() {
        return None;
    }
    let key = lmdb_dir.to_string_lossy().into_owned();

    let mut g = lmdb_mutex().lock().unwrap_or_else(|e| e.into_inner());

    // Fast path — same dir already open.
    if let Some((ref k, ref env)) = *g {
        if *k == key {
            return Some(Arc::clone(env));
        }
    }

    // Slow path — open (or swap to) a new env.
    let env = match unsafe {
        EnvOpenOptions::new()
            .map_size(1024 * 1024 * 1024) // 1 GB virtual — OS pages in only what's touched
            .max_dbs(1)
            .open(&lmdb_dir)
    } {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Failed to open LMDB at {}: {}", lmdb_dir.display(), e);
            return None;
        }
    };

    let arc = Arc::new(env);
    *g = Some((key, Arc::clone(&arc)));
    Some(arc)
}

/// Drop the cached env — frees mmap'd pages and closes the file handle.
///
/// Call this before deleting/rebuilding `hashes.lmdb` so Windows doesn't
/// refuse to delete the open file.
pub fn drop_lmdb_cache() {
    let mut g = lmdb_mutex().lock().unwrap_or_else(|e| e.into_inner());
    *g = None;
    tracing::debug!("LMDB env cache cleared");
}

/// Resolve a deduplicated set of `u64` path hashes to a lookup map.
///
/// Opens *one* read transaction, *one* `open_database` call, then resolves
/// every unique hash. Returns a `HashMap<u64, String>` so callers can look up
/// results for any hash in O(1).
///
/// Use this for bulk operations (e.g. loading hundreds of WADs at once) where
/// many WADs share the same asset hashes — deduplication avoids redundant
/// B-tree traversals.
pub fn resolve_hashes_lmdb_bulk(
    hashes: &[u64],
    env: &heed::Env,
) -> HashMap<u64, String> {
    let rtxn = match env.read_txn() {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("LMDB read_txn failed: {}", e);
            return hashes.iter().map(|h| (*h, format!("{:016x}", h))).collect();
        }
    };

    let db = match env.open_database::<Bytes, Str>(&rtxn, None) {
        Ok(Some(d)) => d,
        _ => return hashes.iter().map(|h| (*h, format!("{:016x}", h))).collect(),
    };

    hashes
        .iter()
        .map(|h| {
            let key = h.to_be_bytes();
            let resolved = db
                .get(&rtxn, &key[..])
                .ok()
                .flatten()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{:016x}", h));
            (*h, resolved)
        })
        .collect()
}

/// Resolve a slice of `u64` path hashes to path strings using a single LMDB
/// read transaction.
///
/// - On cache hit: returns the stored path string.
/// - On cache miss: returns the hash formatted as a 16-digit lowercase hex string.
///
/// Opens *one* read transaction for all hashes — cheap (microseconds after
/// OS page warm-up).
pub fn resolve_hashes_lmdb(hashes: &[u64], env: &heed::Env) -> Vec<String> {
    let rtxn = match env.read_txn() {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("LMDB read_txn failed: {}", e);
            return hashes.iter().map(|h| format!("{:016x}", h)).collect();
        }
    };

    let db = match env.open_database::<Bytes, Str>(&rtxn, None) {
        Ok(Some(d)) => d,
        _ => return hashes.iter().map(|h| format!("{:016x}", h)).collect(),
    };

    hashes
        .iter()
        .map(|h| {
            let key = h.to_be_bytes();
            db.get(&rtxn, &key[..])
                .ok()
                .flatten()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{:016x}", h))
        })
        .collect()
}

// ── Build / prime helpers ─────────────────────────────────────────────────────

/// Build (or update) `hashes.lmdb` from the canonical text hash files.
///
/// Acquires a process-wide build lock so concurrent callers (e.g. the
/// background startup task and an immediate "Create Project") don't race
/// and open a partial database. The second caller waits behind the first,
/// then both see the fully-committed DB.
///
/// Only rebuilds when a source `.txt` file is newer than the existing LMDB.
/// Returns `true` on success (including "no rebuild needed").
pub fn build_hash_db(hash_dir: &str) -> bool {
    // Acquire the build lock. This blocks any concurrent caller until
    // the current build (or staleness check) finishes.
    let _guard = build_mutex()
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    build_hash_db_inner(hash_dir)
}

/// Inner implementation — must only be called while holding `BUILD_LOCK`.
///
/// Rebuilds in-place: opens the env, clears the DB, and reinserts all entries
/// in a single write transaction. This avoids the `remove_dir_all` call that
/// fails on Windows when the memory-mapped files are still held open by
/// outstanding `Arc<heed::Env>` clones.
fn build_hash_db_inner(hash_dir: &str) -> bool {
    let dir = Path::new(hash_dir);
    let lmdb_dir = dir.join("hashes.lmdb");

    let sources: &[(&str, usize)] = &[
        ("hashes.game.txt",      16),
        ("hashes.lcu.txt",       16),
        ("hashes.extracted.txt", 16),
    ];

    let db_mtime = std::fs::metadata(lmdb_dir.join("data.mdb"))
        .and_then(|m| m.modified())
        .ok();

    let needs_rebuild = !lmdb_dir.exists() || sources.iter().any(|(name, _)| {
        let file_mtime = std::fs::metadata(dir.join(name))
            .and_then(|m| m.modified())
            .ok();
        match (db_mtime, file_mtime) {
            (Some(db_t), Some(f_t)) => f_t > db_t,
            (None, Some(_)) => true,
            _ => false,
        }
    });

    if !needs_rebuild {
        tracing::debug!("LMDB hash DB is up-to-date, skipping rebuild");
        return true;
    }

    tracing::info!("Rebuilding LMDB hash DB at {}", lmdb_dir.display());

    // Ensure the directory exists (first-run case).
    if !lmdb_dir.exists() && std::fs::create_dir_all(&lmdb_dir).is_err() {
        tracing::error!("Failed to create LMDB directory");
        return false;
    }

    // Drop the cached env so the next `get_or_open_env` picks up the fresh data.
    // We do NOT delete the directory — rebuild in-place avoids Windows file-lock issues.
    drop_lmdb_cache();

    // Use 1 GB map for writes — 1.9M entries with B-tree overhead can exceed 512 MB.
    // This is virtual address space only; the OS pages in what's actually touched.
    let env = match unsafe {
        EnvOpenOptions::new()
            .map_size(1024 * 1024 * 1024)
            .max_dbs(1)
            .open(&lmdb_dir)
    } {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Failed to open LMDB env: {}", e);
            return false;
        }
    };

    let mut wtxn = match env.write_txn() {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to open write txn: {}", e);
            return false;
        }
    };

    let db = match env.create_database::<Bytes, Str>(&mut wtxn, None) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Failed to create DB: {}", e);
            return false;
        }
    };

    // Clear existing data in-place (no directory deletion needed).
    if let Err(e) = db.clear(&mut wtxn) {
        tracing::error!("Failed to clear LMDB DB: {}", e);
        return false;
    }

    // Collect all entries across all sources, sort by key for fast MDB_APPEND-style insert.
    let mut entries: Vec<([u8; 8], String)> = Vec::with_capacity(2_000_000);

    for (filename, sep) in sources {
        let file_path = dir.join(filename);
        let Ok(content) = std::fs::read_to_string(&file_path) else { continue };

        for line in content.lines() {
            if line.len() <= sep + 1 || line.starts_with('#') { continue; }
            let hash_hex = &line[..*sep];
            let path = line[*sep + 1..].trim_end_matches('\r');
            let Ok(hash_u64) = u64::from_str_radix(hash_hex, 16) else { continue };
            entries.push((hash_u64.to_be_bytes(), path.to_string()));
        }
    }

    // Sorted inserts are ~2× faster on LMDB's B-tree.
    entries.sort_unstable_by_key(|(k, _)| *k);
    entries.dedup_by_key(|(k, _)| *k);

    tracing::info!("Inserting {} hash entries into LMDB", entries.len());

    for (i, (key, path)) in entries.iter().enumerate() {
        if let Err(e) = db.put(&mut wtxn, key.as_slice(), path.as_str()) {
            tracing::error!(
                "LMDB put failed at entry {}/{}: {} (path len={})",
                i, entries.len(), e, path.len()
            );
            return false;
        }
    }

    match wtxn.commit() {
        Ok(()) => {
            tracing::info!("LMDB hash DB built successfully ({} entries)", entries.len());
            true
        }
        Err(e) => {
            tracing::error!("LMDB commit failed: {}", e);
            false
        }
    }
}
