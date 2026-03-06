//! LTK Manager integration commands
//!
//! These commands provide integration with LTK Manager for syncing projects to the launcher.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LtkManagerSettings {
    pub league_path: Option<String>,
    pub mod_storage_path: Option<String>,
    pub workshop_path: Option<String>,
    pub first_run_complete: bool,
    pub theme: String,
}

/// Read LTK Manager settings to get the mod storage path
#[tauri::command]
pub async fn get_ltk_manager_mod_path(app: tauri::AppHandle) -> Result<Option<String>, String> {
    tracing::info!("Reading LTK Manager settings...");

    let app_data = app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let roaming_dir = app_data
        .parent()
        .ok_or_else(|| "Failed to get parent directory".to_string())?;

    // Get Local AppData directory (sibling to Roaming)
    let local_dir = roaming_dir
        .parent()
        .ok_or_else(|| "Failed to get AppData parent directory".to_string())?
        .join("Local");

    // Try multiple possible locations and app identifiers
    // LTK Manager can be in Local or Roaming, and uses different identifiers for dev/prod
    let search_configs = [
        // Roaming AppData (most common for Tauri apps in dev mode)
        (roaming_dir, "dev.leaguetoolkit.manager"),
        (roaming_dir, "dev.leaguetoolkit.ltk-manager"),
        (roaming_dir, "com.leaguetoolkit.ltk-manager"),
        // Local AppData (for installed versions)
        (local_dir.as_path(), "LTK Manager"),
        (local_dir.as_path(), "dev.leaguetoolkit.manager"),
        (local_dir.as_path(), "dev.leaguetoolkit.ltk-manager"),
        (local_dir.as_path(), "com.leaguetoolkit.ltk-manager"),
    ];

    for (base_dir, dir_name) in &search_configs {
        let ltk_settings_path = base_dir.join(dir_name).join("settings.json");

        tracing::info!("Looking for LTK Manager settings at: {:?}", ltk_settings_path);

        if ltk_settings_path.exists() {
            let contents = fs::read_to_string(&ltk_settings_path)
                .map_err(|e| format!("Failed to read LTK Manager settings: {}", e))?;

            let settings: LtkManagerSettings = serde_json::from_str(&contents)
                .map_err(|e| format!("Failed to parse LTK Manager settings: {}", e))?;

            if let Some(mod_path) = settings.mod_storage_path {
                tracing::info!("Found LTK Manager mod storage path: {}", mod_path);
                return Ok(Some(mod_path));
            } else {
                // If modStoragePath is not set, LTK Manager uses the app data directory
                let ltk_data_dir = base_dir.join(dir_name);
                tracing::info!("Using LTK Manager default data directory: {:?}", ltk_data_dir);
                return Ok(Some(ltk_data_dir.to_string_lossy().to_string()));
            }
        }
    }

    tracing::warn!("LTK Manager settings file not found in any of the expected locations");
    Ok(None)
}

/// Package a Flint project and install it to LTK Manager
///
/// This command:
/// 1. Packages the project as a .modpkg file
/// 2. Installs it into LTK Manager's mod library
/// 3. Returns the installed mod ID
#[tauri::command]
pub async fn sync_project_to_launcher(
    project_path: String,
    ltk_storage_path: String,
) -> Result<String, String> {
    tracing::info!("Syncing project to launcher: {}", project_path);

    let project_path_buf = PathBuf::from(&project_path);
    let ltk_storage_buf = PathBuf::from(&ltk_storage_path);

    // 1. Load the project
    let project = crate::core::project::open_project(&project_path_buf)
        .map_err(|e| format!("Failed to open project: {}", e))?;

    tracing::info!("Loaded project: {} v{}", project.name, project.version);

    // 2. Package the project as .modpkg
    let temp_dir = std::env::temp_dir();
    let modpkg_filename = format!("{}.modpkg", project.name);
    let modpkg_path = temp_dir.join(&modpkg_filename);

    tracing::info!("Packaging project to: {:?}", modpkg_path);

    // Use the ltk_mod_project library to package
    package_project(&project_path_buf, &modpkg_path)
        .map_err(|e| format!("Failed to package project: {}", e))?;

    tracing::info!("Successfully packaged project as .modpkg");

    // 3. Install to LTK Manager
    let mod_id = install_to_ltk_manager(&modpkg_path, &ltk_storage_buf)
        .map_err(|e| format!("Failed to install to LTK Manager: {}", e))?;

    tracing::info!("Successfully installed to LTK Manager with ID: {}", mod_id);

    // Clean up temp file
    let _ = fs::remove_file(&modpkg_path);

    Ok(mod_id)
}

/// Package a Flint project as a .modpkg file
fn package_project(project_path: &std::path::Path, output_path: &std::path::Path) -> Result<(), String> {
    use ltk_mod_project::ModProject;
    use ltk_modpkg::builder::{ModpkgBuilder, ModpkgChunkBuilder, ModpkgLayerBuilder};
    use ltk_modpkg::{ModpkgMetadata, ModpkgAuthor};
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Write;
    use walkdir::WalkDir;

    // Read mod.config.json
    let mod_config_path = project_path.join("mod.config.json");
    let config_data = std::fs::read_to_string(&mod_config_path)
        .map_err(|e| format!("Failed to read mod.config.json: {}", e))?;

    let mod_project: ModProject = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse mod.config.json: {}", e))?;

    // Collect all files from the content directory
    let content_base = project_path.join("content");
    if !content_base.exists() {
        return Err("Project content directory not found".to_string());
    }

    let mut file_map: HashMap<String, Vec<u8>> = HashMap::new();

    for entry in WalkDir::new(&content_base)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
    {
        let file_path = entry.path();
        let relative_path = file_path
            .strip_prefix(&content_base)
            .map_err(|e| format!("Failed to get relative path: {}", e))?;

        let file_data = std::fs::read(file_path)
            .map_err(|e| format!("Failed to read file {}: {}", file_path.display(), e))?;

        // Normalize path separators and lowercase
        let normalized_path = relative_path.to_string_lossy().replace("\\", "/").to_lowercase();
        file_map.insert(normalized_path, file_data);
    }

    // Parse version
    let version = semver::Version::parse(&mod_project.version)
        .unwrap_or_else(|_| semver::Version::new(1, 0, 0));

    // Create metadata
    let metadata = ModpkgMetadata {
        name: mod_project.name.clone(),
        display_name: mod_project.display_name.clone(),
        version,
        description: if mod_project.description.is_empty() {
            None
        } else {
            Some(mod_project.description.clone())
        },
        authors: mod_project.authors.iter().map(|author| {
            match author {
                ltk_mod_project::ModProjectAuthor::Name(name) => ModpkgAuthor::new(name.clone(), None),
                ltk_mod_project::ModProjectAuthor::Role { name, role } => ModpkgAuthor::new(name.clone(), Some(role.clone())),
            }
        }).collect(),
        ..Default::default()
    };

    // Build the modpkg
    let mut builder = ModpkgBuilder::default()
        .with_metadata(metadata)
        .map_err(|e| format!("Failed to set metadata: {}", e))?
        .with_layer(ModpkgLayerBuilder::base());

    // Add all files as chunks
    for path in file_map.keys() {
        let chunk = ModpkgChunkBuilder::new()
            .with_path(path)
            .map_err(|e| format!("Failed to set chunk path: {}", e))?
            .with_layer("base");
        builder = builder.with_chunk(chunk);
    }

    // Create output file
    let mut output_file = File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;

    // Build to writer
    builder.build_to_writer(&mut output_file, |chunk_builder, cursor| {
        if let Some(data) = file_map.get(&chunk_builder.path) {
            cursor.write_all(data)?;
        }
        Ok(())
    })
    .map_err(|e| format!("Failed to build modpkg: {}", e))?;

    Ok(())
}

/// Install a .modpkg file to LTK Manager's library
fn install_to_ltk_manager(
    modpkg_path: &std::path::Path,
    ltk_storage_path: &std::path::Path,
) -> Result<String, String> {
    use ltk_modpkg::Modpkg;
    use uuid::Uuid;
    use std::io::Cursor;

    // Read the modpkg to get metadata first
    let modpkg_bytes = fs::read(modpkg_path)
        .map_err(|e| format!("Failed to read modpkg: {}", e))?;

    let mut modpkg = Modpkg::mount_from_reader(Cursor::new(&modpkg_bytes))
        .map_err(|e| format!("Failed to parse modpkg: {}", e))?;

    let metadata = modpkg.load_metadata()
        .map_err(|e| format!("Failed to load metadata from modpkg: {}", e))?;

    // Check if a mod with this name already exists in the library
    let library_path = ltk_storage_path.join("library.json");
    let existing_mod_id = if library_path.exists() {
        let contents = fs::read_to_string(&library_path)
            .map_err(|e| format!("Failed to read library.json: {}", e))?;
        let library: LibraryIndex = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse library.json: {}", e))?;

        // Find existing mod by name
        library.mods.iter()
            .find(|m| {
                // Check if the mod's name matches
                let mod_dir = ltk_storage_path.join("mods").join(&m.id);
                if let Ok(config_contents) = fs::read_to_string(mod_dir.join("mod.config.json")) {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_contents) {
                        if let Some(name) = config.get("name").and_then(|n| n.as_str()) {
                            return name == metadata.name;
                        }
                    }
                }
                false
            })
            .map(|m| m.id.clone())
    } else {
        None
    };

    // Use existing ID or generate new one
    let (mod_id, is_update) = if let Some(existing_id) = existing_mod_id {
        tracing::info!("Updating existing mod with ID: {}", existing_id);
        (existing_id, true)
    } else {
        let new_id = Uuid::new_v4().to_string();
        tracing::info!("Creating new mod with ID: {}", new_id);
        (new_id, false)
    };

    // Create archives directory if it doesn't exist
    let archives_dir = ltk_storage_path.join("archives");
    fs::create_dir_all(&archives_dir)
        .map_err(|e| format!("Failed to create archives directory: {}", e))?;

    // Copy the modpkg to the archives directory
    let archive_dest = archives_dir.join(format!("{}.modpkg", mod_id));
    fs::copy(modpkg_path, &archive_dest)
        .map_err(|e| format!("Failed to copy modpkg to archives: {}", e))?;

    // Create mods metadata directory
    let mods_dir = ltk_storage_path.join("mods").join(&mod_id);
    fs::create_dir_all(&mods_dir)
        .map_err(|e| format!("Failed to create mods metadata directory: {}", e))?;

    // Write mod.config.json to metadata directory (use minimal representation)
    let mod_config = serde_json::json!({
        "name": metadata.name,
        "display_name": metadata.display_name,
        "version": metadata.version.to_string(),
        "description": metadata.description,
        "authors": metadata.authors.iter().map(|a| {
            if let Some(role) = &a.role {
                serde_json::json!({"name": a.name, "role": role})
            } else {
                serde_json::json!(a.name)
            }
        }).collect::<Vec<_>>(),
    });

    let config_json = serde_json::to_string_pretty(&mod_config)
        .map_err(|e| format!("Failed to serialize project config: {}", e))?;

    fs::write(mods_dir.join("mod.config.json"), config_json)
        .map_err(|e| format!("Failed to write mod.config.json: {}", e))?;

    // Update library.json
    update_library_index(ltk_storage_path, &mod_id)
        .map_err(|e| format!("Failed to update library index: {}", e))?;

    Ok(mod_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryIndex {
    mods: Vec<LibraryModEntry>,
    profiles: Vec<Profile>,
    active_profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryModEntry {
    id: String,
    installed_at: String,
    format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    id: String,
    name: String,
    slug: String,
    enabled_mods: Vec<String>,
    mod_order: Vec<String>,
    created_at: String,
    last_used: String,
}

impl Default for LibraryIndex {
    fn default() -> Self {
        use uuid::Uuid;
        let default_profile = Profile {
            id: Uuid::new_v4().to_string(),
            name: "Default".to_string(),
            slug: "default".to_string(),
            enabled_mods: Vec::new(),
            mod_order: Vec::new(),
            created_at: Utc::now().to_rfc3339(),
            last_used: Utc::now().to_rfc3339(),
        };
        Self {
            mods: Vec::new(),
            profiles: vec![default_profile.clone()],
            active_profile_id: default_profile.id,
        }
    }
}

/// Update LTK Manager's library.json to include the new mod
fn update_library_index(ltk_storage_path: &std::path::Path, mod_id: &str) -> Result<(), String> {
    let library_path = ltk_storage_path.join("library.json");

    // Load existing library or create new one
    let mut library: LibraryIndex = if library_path.exists() {
        let contents = fs::read_to_string(&library_path)
            .map_err(|e| format!("Failed to read library.json: {}", e))?;
        serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse library.json: {}", e))?
    } else {
        LibraryIndex::default()
    };

    // Check if mod already exists
    let existing_mod = library.mods.iter_mut().find(|m| m.id == mod_id);

    if let Some(existing) = existing_mod {
        // Update existing mod's installed_at timestamp
        existing.installed_at = Utc::now().to_rfc3339();
        tracing::info!("Updated existing mod entry in library");
    } else {
        // Add new mod entry
        library.mods.push(LibraryModEntry {
            id: mod_id.to_string(),
            installed_at: Utc::now().to_rfc3339(),
            format: "modpkg".to_string(),
        });
        tracing::info!("Added new mod entry to library");

        // Add to active profile's mod_order (only for new mods)
        if let Some(profile) = library.profiles.iter_mut().find(|p| p.id == library.active_profile_id) {
            profile.mod_order.push(mod_id.to_string());
        }
    }

    // Save library.json
    let contents = serde_json::to_string_pretty(&library)
        .map_err(|e| format!("Failed to serialize library index: {}", e))?;

    fs::write(&library_path, contents)
        .map_err(|e| format!("Failed to write library.json: {}", e))?;

    Ok(())
}
