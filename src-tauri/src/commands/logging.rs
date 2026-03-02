//! Logging commands — runtime log level switching

use parking_lot::Mutex;

type ReloadFn = Box<dyn Fn(&str) -> Result<(), String> + Send>;

static RELOAD_FN: Mutex<Option<ReloadFn>> = Mutex::new(None);

/// Store the reload function (called once from main before any commands run).
pub fn set_reload_fn(f: ReloadFn) {
    *RELOAD_FN.lock() = Some(f);
}

/// Switch between normal (`info`) and verbose (`debug`) log levels.
#[tauri::command]
pub async fn set_log_level(verbose: bool) -> Result<(), String> {
    let filter_str = if verbose { "debug" } else { "info" };

    let guard = RELOAD_FN.lock();
    let reload = guard.as_ref().ok_or("Reload handle not initialized")?;
    reload(filter_str)?;

    tracing::info!("Log level set to: {}", filter_str);
    Ok(())
}
