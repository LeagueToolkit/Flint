use std::sync::Arc;
use crate::core::hash::{get_or_open_env, drop_lmdb_cache, build_hash_db};
use crate::core::wad::cache::WadCache;

/// Global WAD metadata cache for fast repeated access.
/// WADs are immutable once written, so caching headers is safe.
#[derive(Clone)]
pub struct WadCacheState(pub Arc<WadCache>);

impl Default for WadCacheState {
    fn default() -> Self {
        Self::new()
    }
}

impl WadCacheState {
    pub fn new() -> Self {
        Self(Arc::new(WadCache::new()))
    }

    pub fn get(&self) -> Arc<WadCache> {
        Arc::clone(&self.0)
    }
}

// =============================================================================
// LMDB env cache state
// =============================================================================

/// Tauri-managed handle to the global LMDB env cache.
///
/// The actual cache is a process-wide static (`LMDB_CACHE` in `lmdb_cache.rs`),
/// so this struct is a zero-cost wrapper that just exposes the API to Tauri
/// commands via `tauri::State<LmdbCacheState>`.
///
/// # Design
/// - `get_env()` — returns the `Arc<heed::Env>` for the current hash dir,
///   opening and caching it on first call.
/// - `build_db()` — rebuilds `hashes.lmdb` from the text files if stale,
///   then the next `get_env()` call re-opens the fresh DB.
/// - `clear()` — drops the cached env (call before deleting the LMDB dir).
#[derive(Clone, Default)]
pub struct LmdbCacheState;

impl LmdbCacheState {
    pub fn new() -> Self { Self }

    /// Return the `heed::Env` for `hash_dir`, opening it if not already cached.
    pub fn get_env(&self, hash_dir: &str) -> Option<Arc<heed::Env>> {
        get_or_open_env(hash_dir)
    }

    /// Build (or refresh) `hashes.lmdb` from text hash files, then return the env.
    ///
    /// Returns `None` if the build fails or the directory doesn't exist.
    pub fn prime(&self, hash_dir: &str) -> Option<Arc<heed::Env>> {
        if build_hash_db(hash_dir) {
            get_or_open_env(hash_dir)
        } else {
            None
        }
    }

    /// Drop the cached LMDB env — frees mmap pages and the file handle.
    pub fn clear(&self) {
        drop_lmdb_cache();
    }
}
