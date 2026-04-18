//! Global LMDB environment cache.
//!
//! Two separate LMDBs, matching Quartz's layout:
//! - `hashes-wad.lmdb` — named DB `"wad"`, 8-byte BE keys (xxh64), WAD path hashes.
//! - `hashes-bin.lmdb` — named DB `"bin"`, 4-byte BE keys (FNV1a), BIN hashes.
//!
//! Each env is opened once per process and cached. OS memory-maps the data files;
//! only touched B-tree pages get paged in. `Arc<heed::Env>` clones are lock-free
//! readers — heed's MVCC allows unlimited concurrent read transactions.

use heed::types::{Bytes, Str};
use heed::{Database, EnvOpenOptions};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

// ── Statics ───────────────────────────────────────────────────────────────────

struct EnvCache {
    key: String,
    env: Arc<heed::Env>,
}

static WAD_LMDB_CACHE: OnceLock<Mutex<Option<EnvCache>>> = OnceLock::new();
static BIN_LMDB_CACHE: OnceLock<Mutex<Option<EnvCache>>> = OnceLock::new();

fn wad_mutex() -> &'static Mutex<Option<EnvCache>> {
    WAD_LMDB_CACHE.get_or_init(|| Mutex::new(None))
}

fn bin_mutex() -> &'static Mutex<Option<EnvCache>> {
    BIN_LMDB_CACHE.get_or_init(|| Mutex::new(None))
}

// ── Open helpers ──────────────────────────────────────────────────────────────

fn open_env(lmdb_dir: &Path) -> Option<heed::Env> {
    if !lmdb_dir.join("data.mdb").exists() {
        tracing::debug!("LMDB data.mdb missing at: {}", lmdb_dir.display());
        return None;
    }

    // 1 GB virtual address reservation — actual RAM use is only what's touched.
    // `max_dbs(2)` so we can open the named DB by name.
    match unsafe {
        EnvOpenOptions::new()
            .map_size(1024 * 1024 * 1024)
            .max_dbs(2)
            .open(lmdb_dir)
    } {
        Ok(e) => Some(e),
        Err(e) => {
            tracing::warn!("Failed to open LMDB at {}: {}", lmdb_dir.display(), e);
            None
        }
    }
}

fn get_cached_env(
    slot: &Mutex<Option<EnvCache>>,
    lmdb_dir: &Path,
) -> Option<Arc<heed::Env>> {
    let key = lmdb_dir.to_string_lossy().into_owned();
    let mut g = slot.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(ref cache) = *g {
        if cache.key == key {
            return Some(Arc::clone(&cache.env));
        }
    }

    tracing::info!("Opening LMDB env: {}", lmdb_dir.display());
    let env = open_env(lmdb_dir)?;
    let arc = Arc::new(env);
    *g = Some(EnvCache { key, env: Arc::clone(&arc) });
    Some(arc)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Get (and cache) the WAD hash env rooted at `hash_dir`.
///
/// Returns `None` if `hash_dir/hashes-wad.lmdb/data.mdb` is missing.
pub fn get_wad_env(hash_dir: &str) -> Option<Arc<heed::Env>> {
    let lmdb_dir = Path::new(hash_dir).join("hashes-wad.lmdb");
    get_cached_env(wad_mutex(), &lmdb_dir)
}

/// Get (and cache) the BIN hash env rooted at `hash_dir`.
///
/// Returns `None` if `hash_dir/hashes-bin.lmdb/data.mdb` is missing.
pub fn get_bin_env(hash_dir: &str) -> Option<Arc<heed::Env>> {
    let lmdb_dir = Path::new(hash_dir).join("hashes-bin.lmdb");
    get_cached_env(bin_mutex(), &lmdb_dir)
}

/// Legacy alias — callers that need a single env default to WAD.
///
/// Prefer [`get_wad_env`] or [`get_bin_env`] in new code.
pub fn get_or_open_env(hash_dir: &str) -> Option<Arc<heed::Env>> {
    get_wad_env(hash_dir)
}

/// Drop both cached envs. Call before replacing the on-disk `data.mdb` so
/// Windows doesn't refuse the overwrite (the file is memory-mapped while open).
pub fn drop_lmdb_cache() {
    {
        let mut g = wad_mutex().lock().unwrap_or_else(|e| e.into_inner());
        *g = None;
    }
    {
        let mut g = bin_mutex().lock().unwrap_or_else(|e| e.into_inner());
        *g = None;
    }
    tracing::debug!("LMDB env caches cleared");
}

// ── Resolve helpers ───────────────────────────────────────────────────────────

/// Open a read txn and the named DB in one shot, with a shared lifetime.
fn open_read_db<'a>(
    env: &'a heed::Env,
    name: &str,
) -> Option<(heed::RoTxn<'a>, Database<Bytes, Str>)> {
    let rtxn = env.read_txn().ok()?;
    let db = env.open_database::<Bytes, Str>(&rtxn, Some(name)).ok().flatten()?;
    Some((rtxn, db))
}

/// Resolve a slice of 64-bit WAD path hashes. Unresolved entries fall back to
/// their 16-char hex form, which is the format downstream code expects.
pub fn resolve_hashes_lmdb(hashes: &[u64], env: &heed::Env) -> Vec<String> {
    let Some((rtxn, db)) = open_read_db(env, "wad") else {
        return hashes.iter().map(|h| format!("{:016x}", h)).collect();
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

/// Bulk WAD hash resolution. Opens one txn + one DB handle, then looks up every hash.
///
/// Returns a `HashMap` so callers can query any input hash in O(1). Duplicate
/// input hashes are deduplicated automatically by the map.
pub fn resolve_hashes_lmdb_bulk(
    hashes: &[u64],
    env: &heed::Env,
) -> HashMap<u64, String> {
    let Some((rtxn, db)) = open_read_db(env, "wad") else {
        return hashes.iter().map(|h| (*h, format!("{:016x}", h))).collect();
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

/// Resolve a slice of 32-bit FNV1a BIN hashes. Unresolved entries fall back
/// to 8-char hex form.
pub fn resolve_bin_hashes_lmdb(hashes: &[u32], env: &heed::Env) -> HashMap<u32, String> {
    let Some((rtxn, db)) = open_read_db(env, "bin") else {
        return hashes.iter().map(|h| (*h, format!("{:08x}", h))).collect();
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
                .unwrap_or_else(|| format!("{:08x}", h));
            (*h, resolved)
        })
        .collect()
}
