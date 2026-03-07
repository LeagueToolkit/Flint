//! Project file watcher for auto-sync to LTK Manager
//!
//! Watches the project's content directory for changes and automatically syncs to LTK Manager
//! when auto-sync is enabled.

use notify_debouncer_full::{new_debouncer, notify::*, DebounceEventResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Manager, Emitter};
use tokio::sync::mpsc;

/// Global watcher state stored in Tauri's managed state
pub struct WatcherState {
    /// Currently active watcher (if any)
    pub watcher: Arc<Mutex<Option<WatcherHandle>>>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self {
            watcher: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for WatcherState {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a running file watcher
pub struct WatcherHandle {
    /// The debouncer (must be kept alive)
    _debouncer: Box<dyn Send>,
    /// Sender to stop the watcher task (not read, but dropping it signals the task to stop)
    #[allow(dead_code)]
    stop_tx: mpsc::UnboundedSender<()>,
}

/// Start watching a project directory for changes
#[tauri::command]
pub async fn start_project_watcher(
    app: tauri::AppHandle,
    project_path: String,
    ltk_storage_path: String,
) -> std::result::Result<(), String> {
    tracing::info!("Starting project watcher for: {}", project_path);

    let watcher_state = app.state::<WatcherState>();
    let mut watcher_guard = watcher_state.watcher.lock().unwrap();

    // Stop existing watcher if any
    if watcher_guard.is_some() {
        tracing::info!("Stopping existing watcher before starting new one");
        *watcher_guard = None;
    }

    let content_path = PathBuf::from(&project_path).join("content");
    if !content_path.exists() {
        return Err(format!("Project content directory not found: {}", content_path.display()));
    }

    // Create channel for stopping the watcher task
    let (stop_tx, mut stop_rx) = mpsc::unbounded_channel();

    // Clone values for the async task
    let project_path_clone = project_path.clone();
    let ltk_storage_clone = ltk_storage_path.clone();
    let app_clone = app.clone();

    // Create debounced watcher (2 second debounce)
    let (tx, mut rx) = mpsc::unbounded_channel();

    let mut debouncer = new_debouncer(
        Duration::from_secs(2),
        None,
        move |result: DebounceEventResult| match result {
            Ok(events) => {
                // Log all received events for debugging
                for event in &events {
                    tracing::debug!("File event: {:?} - {:?}", event.kind, event.paths);
                }

                // Filter out events we don't care about (metadata changes, directory ops)
                let relevant_events: Vec<_> = events.iter()
                    .filter(|e| matches!(
                        e.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                    ))
                    .collect();

                if !relevant_events.is_empty() {
                    tracing::info!(
                        "Detected {} relevant file change(s), triggering sync in 2s...",
                        relevant_events.len()
                    );
                    let _ = tx.send(());
                }
            }
            Err(e) => {
                tracing::error!("File watcher error: {:?}", e);
            }
        },
    )
    .map_err(|e| format!("Failed to create file watcher: {}", e))?;

    // Watch the content directory recursively
    debouncer
        .watch(&content_path, RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch directory: {}", e))?;

    tracing::info!("Watching directory: {}", content_path.display());

    // Spawn background task to handle sync events
    tokio::spawn(async move {
        tracing::info!("Auto-sync background task started");
        loop {
            tokio::select! {
                Some(_) = rx.recv() => {
                    tracing::info!("Debounce complete! Starting auto-sync to LTK Manager...");

                    // Call the sync command
                    match crate::commands::ltk_manager::sync_project_to_launcher(
                        project_path_clone.clone(),
                        ltk_storage_clone.clone(),
                    )
                    .await
                    {
                        Ok(mod_id) => {
                            tracing::info!("✓ Auto-sync completed successfully: {}", mod_id);
                            // Emit event to frontend
                            let _ = app_clone.emit("auto-sync-complete", mod_id);
                        }
                        Err(e) => {
                            tracing::error!("✗ Auto-sync failed: {}", e);
                            // Emit error event to frontend
                            let _ = app_clone.emit("auto-sync-error", e);
                        }
                    }
                }
                _ = stop_rx.recv() => {
                    tracing::info!("Auto-sync background task stopping");
                    break;
                }
            }
        }
        tracing::info!("Auto-sync background task ended");
    });

    // Store the watcher handle
    *watcher_guard = Some(WatcherHandle {
        _debouncer: Box::new(debouncer),
        stop_tx,
    });

    Ok(())
}

/// Stop the active project watcher
#[tauri::command]
pub async fn stop_project_watcher(app: tauri::AppHandle) -> std::result::Result<(), String> {
    tracing::info!("Stopping project watcher");

    let watcher_state = app.state::<WatcherState>();
    let mut watcher_guard = watcher_state.watcher.lock().unwrap();

    if watcher_guard.is_some() {
        *watcher_guard = None;
        tracing::info!("Project watcher stopped");
    } else {
        tracing::debug!("No active watcher to stop");
    }

    Ok(())
}
