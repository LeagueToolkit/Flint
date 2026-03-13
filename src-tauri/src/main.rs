// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod core;
mod error;
mod state;

use commands::project_watcher::WatcherState;
use core::hash::get_ritoshark_hash_dir;
use core::frontend_log::{FrontendLogLayer, set_app_handle};
use state::{LmdbCacheState, WadCacheState};
use tauri::Manager;
use tracing_subscriber::{fmt, prelude::*, reload, EnvFilter};

fn main() {
    // Initialize tracing/logging with frontend layer
    // Use reload layer so log level can be changed at runtime via set_log_level command
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let (filter_layer, reload_handle) = reload::Layer::new(filter);

    // Store a closure that captures the reload handle — avoids spelling out the full type
    commands::logging::set_reload_fn(Box::new(move |filter_str: &str| {
        let new_filter = EnvFilter::try_new(filter_str)
            .map_err(|e| format!("Invalid filter: {}", e))?;
        reload_handle.reload(new_filter)
            .map_err(|e| format!("Failed to reload filter: {}", e))
    }));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(FrontendLogLayer)
        .with(filter_layer)
        .init();

    tracing::info!("Starting Flint");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(WadCacheState::new())
        .manage(LmdbCacheState::new())
        .manage(WatcherState::new())
        .setup(|app| {
            // Set app handle for frontend logging
            set_app_handle(app.handle().clone());
            
            // Use RitoShark directory for hash files (shared with other RitoShark tools)
            let hash_dir = get_ritoshark_hash_dir().unwrap_or_else(|e| {
                tracing::warn!("Failed to get RitoShark hash directory: {}", e);
                // Fallback to Tauri app data directory if RitoShark path not available
                app.path().app_data_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("./hashes"))
                    .join("hashes")
            });
            
            tracing::info!("Hash directory: {}", hash_dir.display());
            
            // Prime LMDB: build hashes.lmdb from .txt files if stale, then mmap it.
            // This replaces the old in-memory Hashtable load (was 200-400 MB RAM, seconds).
            // LMDB only pages in what's actually touched — typically 5-20 MB warm.
            let hash_dir_str = hash_dir.to_string_lossy().into_owned();
            let lmdb_state = app.state::<LmdbCacheState>().inner().clone();
            tauri::async_runtime::spawn(async move {
                tracing::info!("Checking for hash updates...");
                match crate::core::hash::download_hashes(&hash_dir, false).await {
                    Ok(stats) => {
                        if stats.downloaded > 0 {
                            tracing::info!(
                                "Hash update: {} downloaded, {} up-to-date",
                                stats.downloaded, stats.skipped
                            );
                        } else {
                            tracing::debug!("Hashes up-to-date ({} files)", stats.skipped);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to update hashes (will use existing): {}", e);
                    }
                }
                // Prime LMDB after download completes (build if stale, then mmap)
                match lmdb_state.prime(&hash_dir_str) {
                    Some(_) => tracing::info!("LMDB hash env ready — point lookups active"),
                    None    => tracing::warn!("LMDB hash env not available — hashes may not resolve"),
                }
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::hash::download_hashes,
            commands::hash::get_hash_status,
            commands::hash::reload_hashes,
            commands::wad::read_wad,
            commands::wad::get_wad_chunks,
            commands::wad::load_all_wad_chunks,
            commands::wad::extract_wad,
            commands::wad::read_wad_chunk_data,
            commands::wad::scan_game_wads,
            commands::bin::convert_bin_to_text,
            commands::bin::convert_bin_to_json,
            commands::bin::convert_text_to_bin,
            commands::bin::convert_json_to_bin,
            commands::bin::read_bin_info,
            commands::bin::convert_bin_bytes_to_text,
            commands::bin::convert_bin_bytes_to_json,
            commands::bin::parse_bin_file_to_text,
            commands::bin::read_or_convert_bin,
            commands::bin::save_ritobin_to_bin,
            // League detection commands

            commands::league::detect_league,
            commands::league::validate_league,
            // Project management commands
            commands::project::create_project,
            commands::project::create_loading_screen_project,
            commands::project::open_project,
            commands::project::save_project,
            commands::project::delete_project,
            commands::project::list_project_files,
            commands::project::preconvert_project_bins,
            // Champion discovery commands
            commands::champion::discover_champions,
            commands::champion::get_champion_skins,
            commands::champion::search_champions,
            // Validation commands
            commands::validation::extract_asset_references,
            commands::validation::validate_assets,
            // File commands (preview system)
            commands::file::read_file_bytes,
            commands::file::read_file_info,
            commands::file::decode_dds_to_png,
            commands::file::decode_bytes_to_png,
            commands::file::get_bundled_floor_png,
            commands::file::read_text_file,
            commands::file::write_text_file,
            commands::file::recolor_image,
            commands::file::recolor_folder,
            commands::file::colorize_image,
            commands::file::colorize_folder,
            // File management commands
            commands::file::rename_file,
            commands::file::delete_file,
            commands::file::open_in_explorer,
            commands::file::open_with_default_app,
            commands::file::create_directory,
            commands::file::duplicate_file,
            // Export commands
            commands::export::repath_project_cmd,
            commands::export::export_fantome,
            commands::export::export_modpkg,
            commands::export::get_fantome_filename,
            commands::export::get_export_preview,
            // Mesh commands (3D preview)
            commands::mesh::read_skn_mesh,
            commands::mesh::read_scb_mesh,
            commands::mesh::read_skl_skeleton,
            commands::mesh::read_animation_list,
            commands::mesh::read_animation,
            commands::mesh::evaluate_animation,
            commands::mesh::resolve_asset_path,
            // Auto-update commands
            commands::updater::get_current_version,
            commands::updater::check_for_updates,
            commands::updater::download_and_install_update,
            // Checkpoint commands
            commands::checkpoint::create_checkpoint,
            commands::checkpoint::list_checkpoints,
            commands::checkpoint::restore_checkpoint,
            commands::checkpoint::compare_checkpoints,
            commands::checkpoint::delete_checkpoint,
            commands::checkpoint::read_checkpoint_file,
            commands::checkpoint::get_file_changes,
            // Audio commands (BNK/WPK editor)
            commands::audio::parse_audio_bank,
            commands::audio::parse_audio_bank_bytes,
            commands::audio::read_audio_entry,
            commands::audio::read_audio_entry_bytes,
            commands::audio::decode_wem,
            commands::audio::parse_bnk_hirc,
            commands::audio::parse_bnk_hirc_bytes,
            commands::audio::extract_bin_audio_events,
            commands::audio::map_audio_events,
            commands::audio::replace_audio_entry,
            commands::audio::replace_audio_entries,
            commands::audio::silence_audio_entry,
            commands::audio::write_bnk,
            commands::audio::write_wpk,
            commands::audio::save_audio_file,
            // Fixer commands (Hematite integration)
            commands::fixer::get_fixer_config,
            commands::fixer::analyze_project,
            commands::fixer::fix_project,
            commands::fixer::batch_fix_projects,
            // Logging commands
            commands::logging::set_log_level,
            // LTK Manager integration commands
            commands::ltk_manager::get_ltk_manager_mod_path,
            commands::ltk_manager::sync_project_to_launcher,
            // Project watcher commands (auto-sync + preview hot reload)
            commands::project_watcher::start_project_watcher,
            commands::project_watcher::stop_project_watcher,
            commands::project_watcher::start_preview_watcher,
            commands::project_watcher::stop_preview_watcher,
            // Fantome import commands
            commands::fantome_import::analyze_fantome,
            commands::fantome_import::import_fantome_wad,
            // HUD Editor commands
            commands::hud::parse_hud_py_file,
            commands::hud::save_hud_py_file,
            commands::hud::create_hud_project,
            commands::hud::get_hud_file_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
