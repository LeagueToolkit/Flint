use std::path::{Path, PathBuf};
use std::process::Command;

/// Detect Jade League Bin Editor installation
#[tauri::command]
pub async fn detect_jade_installation() -> Result<Option<String>, String> {
    let search_locations = get_jade_search_locations();

    for path in search_locations {
        if path.exists() && path.is_file() {
            tracing::info!("[external_apps] Found Jade at: {}", path.display());
            return Ok(Some(path.to_string_lossy().to_string()));
        }
    }

    tracing::info!("[external_apps] Jade not found in any search location");
    Ok(None)
}

/// Get potential Jade installation paths
fn get_jade_search_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();

    // LocalAppData\Programs\Jade\Jade.exe (typical Tauri installation)
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        locations.push(PathBuf::from(&localappdata).join("Programs").join("Jade").join("Jade.exe"));
        locations.push(PathBuf::from(&localappdata).join("Jade").join("Jade.exe"));
    }

    // AppData\Roaming\LeagueToolkit\Jade\Jade.exe
    if let Ok(appdata) = std::env::var("APPDATA") {
        locations.push(PathBuf::from(&appdata).join("LeagueToolkit").join("Jade").join("Jade.exe"));
    }

    // Common installation directories
    locations.push(PathBuf::from("C:\\Program Files\\Jade\\Jade.exe"));
    locations.push(PathBuf::from("C:\\Program Files (x86)\\Jade\\Jade.exe"));

    // Desktop (users sometimes keep executables there)
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        locations.push(PathBuf::from(&userprofile).join("Desktop").join("Jade.exe"));
    }

    locations
}

/// Detect Quartz installation
#[tauri::command]
pub async fn detect_quartz_installation() -> Result<Option<String>, String> {
    let search_locations = get_quartz_search_locations();

    for path in search_locations {
        if path.exists() && path.is_file() {
            tracing::info!("[external_apps] Found Quartz at: {}", path.display());
            return Ok(Some(path.to_string_lossy().to_string()));
        }
    }

    tracing::info!("[external_apps] Quartz not found in any search location");
    Ok(None)
}

/// Get potential Quartz installation paths
fn get_quartz_search_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();

    // LocalAppData\Programs\Quartz\Quartz.exe (typical Tauri/Electron installation)
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        locations.push(PathBuf::from(&localappdata).join("Programs").join("Quartz").join("Quartz.exe"));
        locations.push(PathBuf::from(&localappdata).join("Quartz").join("Quartz.exe"));
    }

    // AppData\Roaming\LeagueToolkit\Quartz\Quartz.exe
    if let Ok(appdata) = std::env::var("APPDATA") {
        locations.push(PathBuf::from(&appdata).join("LeagueToolkit").join("Quartz").join("Quartz.exe"));
    }

    // Common installation directories
    locations.push(PathBuf::from("C:\\Program Files\\Quartz\\Quartz.exe"));
    locations.push(PathBuf::from("C:\\Program Files (x86)\\Quartz\\Quartz.exe"));

    // Desktop
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        locations.push(PathBuf::from(&userprofile).join("Desktop").join("Quartz.exe"));
    }

    locations
}

/// Launch Jade with a file path
#[tauri::command]
pub async fn launch_jade(file_path: String, jade_path: String) -> Result<(), String> {
    let jade_exe = Path::new(&jade_path);
    let file = Path::new(&file_path);

    // Validate Jade executable exists
    if !jade_exe.exists() {
        return Err(format!("Jade executable not found: {}", jade_path));
    }

    // Validate file exists
    if !file.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    tracing::info!("[external_apps] Launching Jade with file: {}", file_path);

    // Launch Jade with the file as argument
    Command::new(jade_exe)
        .arg(&file_path)
        .spawn()
        .map_err(|e| format!("Failed to launch Jade: {}", e))?;

    Ok(())
}

/// Launch Quartz with a file path
#[tauri::command]
pub async fn launch_quartz(file_path: String, quartz_path: String) -> Result<(), String> {
    let quartz_exe = Path::new(&quartz_path);
    let file = Path::new(&file_path);

    // Validate Quartz executable exists
    if !quartz_exe.exists() {
        return Err(format!("Quartz executable not found: {}", quartz_path));
    }

    // Validate file exists
    if !file.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    tracing::info!("[external_apps] Launching Quartz with file: {}", file_path);

    // Launch Quartz with the file as argument
    Command::new(quartz_exe)
        .arg(&file_path)
        .spawn()
        .map_err(|e| format!("Failed to launch Quartz: {}", e))?;

    Ok(())
}
