use std::sync::Arc;
use flint_ltk::heed;
use flint_ltk::hash::{drop_lmdb_cache, get_or_open_env, get_wad_env, hashes_present};
use flint_ltk::wad::cache::WadCache;

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

/// Tauri-managed handle to the global LMDB env caches.
///
/// Backed by process-wide statics in `flint_ltk::hash::lmdb_cache`; this struct
/// is a zero-cost wrapper exposing the API to Tauri commands via
/// `tauri::State<LmdbCacheState>`.
///
/// Two separate LMDBs are managed:
/// - WAD hashes — `hashes-wad.lmdb` (64-bit xxh64 keys, named DB `"wad"`).
/// - BIN hashes — `hashes-bin.lmdb` (32-bit FNV1a keys, named DB `"bin"`).
///
/// Both are pre-built and downloaded from the `LeagueToolkit/lmdb-hashes`
/// GitHub release — no local build step.
#[derive(Clone, Default)]
pub struct LmdbCacheState;

impl LmdbCacheState {
    pub fn new() -> Self { Self }

    /// Return the WAD env, opening it on first call.
    pub fn get_wad_env(&self, hash_dir: &str) -> Option<Arc<heed::Env>> {
        get_wad_env(hash_dir)
    }

    /// Legacy alias — returns the WAD env. Prefer [`Self::get_wad_env`] in new code.
    pub fn get_env(&self, hash_dir: &str) -> Option<Arc<heed::Env>> {
        get_or_open_env(hash_dir)
    }

    /// Ensure the WAD env is open and return it.
    ///
    /// The DB is downloaded as a pre-built zstd-compressed artifact, so priming
    /// just opens the env. Returns `None` if the LMDB files are missing.
    pub fn prime(&self, hash_dir: &str) -> Option<Arc<heed::Env>> {
        if !hashes_present(std::path::Path::new(hash_dir)) {
            tracing::warn!("Hash LMDBs not present at {} — run download_hashes first", hash_dir);
            return None;
        }
        get_wad_env(hash_dir)
    }

    /// Drop the cached WAD and BIN envs — frees mmap pages and closes file handles.
    pub fn clear(&self) {
        drop_lmdb_cache();
    }
}
