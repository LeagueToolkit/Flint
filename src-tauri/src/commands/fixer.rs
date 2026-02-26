//! Tauri commands for Hematite fixer integration
//!
//! Provides project analysis and fixing using the Hematite fixer library.
//! Scans BIN files for known issues (broken health bars, deprecated fields, etc.)
//! and applies config-driven fixes.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::Emitter;
use walkdir::WalkDir;

use hematite_lib::analyzer::{self, HashDict, ScanResult, WadCache};
use hematite_lib::config::schema::FixConfig;
use hematite_lib::fixer::{self, FixContext, FixResult};

// =============================================================================
// Types
// =============================================================================

/// Summary of analyzing a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAnalysis {
    /// Project directory path
    pub project_path: String,
    /// Per-file scan results (only files with issues)
    pub results: Vec<ScanResult>,
    /// Total number of BIN files scanned
    pub files_scanned: u32,
    /// Total number of issues detected
    pub issues_found: u32,
}

/// Summary of fixing a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFixResult {
    /// Project directory path
    pub project_path: String,
    /// Per-file fix results
    pub results: Vec<FixResult>,
    /// Total fixes applied
    pub total_applied: u32,
    /// Total fixes failed
    pub total_failed: u32,
}

/// Summary of batch fixing multiple projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFixResult {
    /// Per-project results
    pub projects: Vec<ProjectFixResult>,
    /// Total projects processed
    pub total_projects: u32,
    /// Total fixes applied across all projects
    pub total_applied: u32,
    /// Total fixes failed across all projects
    pub total_failed: u32,
}

// =============================================================================
// Helpers
// =============================================================================

/// Find all .bin files under a project's content directory
fn find_bin_files(project_path: &Path) -> Vec<PathBuf> {
    let content_dir = project_path.join("content");
    if !content_dir.exists() {
        return Vec::new();
    }

    WalkDir::new(&content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("bin"))
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

// =============================================================================
// Commands
// =============================================================================

/// Fetch the fixer configuration (from GitHub with cache fallback)
#[tauri::command]
pub async fn get_fixer_config() -> Result<FixConfig, String> {
    tokio::task::spawn_blocking(|| {
        hematite_lib::config::fetcher::get_config().map_err(|e| format!("{:#}", e))
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

/// Analyze a Flint project for fixable issues
///
/// Scans all BIN files under `{project_path}/content/` and reports detected issues.
#[tauri::command]
pub async fn analyze_project(
    project_path: String,
    app: tauri::AppHandle,
) -> Result<ProjectAnalysis, String> {
    let project_dir = PathBuf::from(&project_path);
    if !project_dir.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }

    let _ = app.emit("fixer-progress", serde_json::json!({
        "phase": "scan",
        "message": "Scanning project for BIN files..."
    }));

    let analysis = tokio::task::spawn_blocking(move || -> Result<ProjectAnalysis, String> {
        // Load hash dictionary for type/field resolution
        let hash_dict = HashDict::load().map_err(|e| format!("Failed to load hash dict: {:#}", e))?;

        // Fetch fix config
        let config = hematite_lib::config::fetcher::get_config()
            .map_err(|e| format!("Failed to load fix config: {:#}", e))?;

        // Empty WAD cache (we're scanning extracted BIN files, not inside a WAD)
        let wad_cache = WadCache::new();

        // Find all BIN files
        let bin_files = find_bin_files(&project_dir);
        let files_scanned = bin_files.len() as u32;

        let mut results = Vec::new();
        let mut issues_found = 0u32;

        for bin_path in &bin_files {
            let path_str = bin_path.to_string_lossy().to_string();
            match analyzer::analyze_file(&path_str, &wad_cache, &config.fixes, &hash_dict) {
                Ok(scan_result) => {
                    if !scan_result.detected_issues.is_empty() {
                        issues_found += scan_result.detected_issues.len() as u32;
                        results.push(scan_result);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to analyze {}: {:#}", path_str, e);
                }
            }
        }

        Ok(ProjectAnalysis {
            project_path: project_dir.to_string_lossy().to_string(),
            results,
            files_scanned,
            issues_found,
        })
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))??;

    let _ = app.emit("fixer-progress", serde_json::json!({
        "phase": "done",
        "message": format!("Found {} issues in {} files", analysis.issues_found, analysis.files_scanned)
    }));

    Ok(analysis)
}

/// Fix a Flint project by applying selected fixes
///
/// Scans BIN files, detects issues, and applies the specified fixes in-place.
/// If `selected_fix_ids` is empty, all detected fixes are applied.
#[tauri::command]
pub async fn fix_project(
    project_path: String,
    selected_fix_ids: Vec<String>,
    app: tauri::AppHandle,
) -> Result<ProjectFixResult, String> {
    let project_dir = PathBuf::from(&project_path);
    if !project_dir.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }

    let _ = app.emit("fixer-progress", serde_json::json!({
        "phase": "fix",
        "message": "Applying fixes..."
    }));

    let fix_result = tokio::task::spawn_blocking(move || -> Result<ProjectFixResult, String> {
        let hash_dict = HashDict::load().map_err(|e| format!("Failed to load hash dict: {:#}", e))?;
        let config = hematite_lib::config::fetcher::get_config()
            .map_err(|e| format!("Failed to load fix config: {:#}", e))?;
        let wad_cache = WadCache::new();

        let bin_files = find_bin_files(&project_dir);

        // If no specific fixes selected, use all enabled fix IDs
        let fix_ids: Vec<String> = if selected_fix_ids.is_empty() {
            config.fixes.iter()
                .filter(|(_, rule)| rule.enabled)
                .map(|(id, _)| id.clone())
                .collect()
        } else {
            selected_fix_ids
        };

        let mut results = Vec::new();
        let mut total_applied = 0u32;
        let mut total_failed = 0u32;

        for bin_path in &bin_files {
            let path_str = bin_path.to_string_lossy().to_string();

            // Parse the BIN file
            let bin_tree = match analyzer::parse_bin_file(&path_str) {
                Ok(tree) => tree,
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {:#}", path_str, e);
                    continue;
                }
            };

            // Create fix context
            let mut ctx = FixContext::new(bin_tree, &wad_cache, &hash_dict);

            // Apply transforms
            let fix_result = fixer::apply_transforms(&path_str, &mut ctx, &config.fixes, &fix_ids);

            if !fix_result.fixes_applied.is_empty() {
                // Write modified BIN back to disk
                if let Err(e) = analyzer::write_bin_file(&ctx.bin_tree, bin_path) {
                    tracing::error!("Failed to write fixed BIN {}: {:#}", path_str, e);
                    total_failed += 1;
                } else {
                    total_applied += fix_result.fixes_applied.len() as u32;
                    tracing::info!(
                        "Applied {} fixes to {}",
                        fix_result.fixes_applied.len(),
                        path_str
                    );
                }
            }

            total_failed += fix_result.fixes_failed.len() as u32;

            if !fix_result.fixes_applied.is_empty() || !fix_result.fixes_failed.is_empty() {
                results.push(fix_result);
            }
        }

        Ok(ProjectFixResult {
            project_path: project_dir.to_string_lossy().to_string(),
            results,
            total_applied,
            total_failed,
        })
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))??;

    let _ = app.emit("fixer-progress", serde_json::json!({
        "phase": "done",
        "message": format!("Applied {} fixes ({} failed)", fix_result.total_applied, fix_result.total_failed)
    }));

    Ok(fix_result)
}

/// Batch fix multiple Flint projects
///
/// Each path in `project_paths` should be a Flint project directory.
/// Applies all enabled fixes to each project.
#[tauri::command]
pub async fn batch_fix_projects(
    project_paths: Vec<String>,
    selected_fix_ids: Vec<String>,
    app: tauri::AppHandle,
) -> Result<BatchFixResult, String> {
    let total_projects = project_paths.len() as u32;

    let _ = app.emit("fixer-progress", serde_json::json!({
        "phase": "batch",
        "message": format!("Fixing {} projects...", total_projects)
    }));

    let batch_result = tokio::task::spawn_blocking(move || -> Result<BatchFixResult, String> {
        let hash_dict = HashDict::load().map_err(|e| format!("Failed to load hash dict: {:#}", e))?;
        let config = hematite_lib::config::fetcher::get_config()
            .map_err(|e| format!("Failed to load fix config: {:#}", e))?;
        let wad_cache = WadCache::new();

        let fix_ids: Vec<String> = if selected_fix_ids.is_empty() {
            config.fixes.iter()
                .filter(|(_, rule)| rule.enabled)
                .map(|(id, _)| id.clone())
                .collect()
        } else {
            selected_fix_ids
        };

        let mut projects = Vec::new();
        let mut grand_total_applied = 0u32;
        let mut grand_total_failed = 0u32;

        for (idx, project_path) in project_paths.iter().enumerate() {
            let project_dir = PathBuf::from(project_path);
            if !project_dir.exists() {
                tracing::warn!("Skipping non-existent project: {}", project_path);
                continue;
            }

            tracing::info!("Fixing project {}/{}: {}", idx + 1, total_projects, project_path);

            let bin_files = find_bin_files(&project_dir);
            let mut results = Vec::new();
            let mut total_applied = 0u32;
            let mut total_failed = 0u32;

            for bin_path in &bin_files {
                let path_str = bin_path.to_string_lossy().to_string();

                let bin_tree = match analyzer::parse_bin_file(&path_str) {
                    Ok(tree) => tree,
                    Err(e) => {
                        tracing::warn!("Failed to parse {}: {:#}", path_str, e);
                        continue;
                    }
                };

                let mut ctx = FixContext::new(bin_tree, &wad_cache, &hash_dict);
                let fix_result = fixer::apply_transforms(&path_str, &mut ctx, &config.fixes, &fix_ids);

                if !fix_result.fixes_applied.is_empty() {
                    if let Err(e) = analyzer::write_bin_file(&ctx.bin_tree, bin_path) {
                        tracing::error!("Failed to write fixed BIN {}: {:#}", path_str, e);
                        total_failed += 1;
                    } else {
                        total_applied += fix_result.fixes_applied.len() as u32;
                    }
                }

                total_failed += fix_result.fixes_failed.len() as u32;

                if !fix_result.fixes_applied.is_empty() || !fix_result.fixes_failed.is_empty() {
                    results.push(fix_result);
                }
            }

            grand_total_applied += total_applied;
            grand_total_failed += total_failed;

            projects.push(ProjectFixResult {
                project_path: project_path.clone(),
                results,
                total_applied,
                total_failed,
            });
        }

        Ok(BatchFixResult {
            projects,
            total_projects,
            total_applied: grand_total_applied,
            total_failed: grand_total_failed,
        })
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))??;

    let _ = app.emit("fixer-progress", serde_json::json!({
        "phase": "done",
        "message": format!(
            "Batch complete: {} projects, {} fixes applied, {} failed",
            batch_result.total_projects, batch_result.total_applied, batch_result.total_failed
        )
    }));

    Ok(batch_result)
}
