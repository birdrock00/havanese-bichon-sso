use std::path::{Path, PathBuf};

use crate::settings::cli::SETTINGS;

/// Current storage layout version.
/// - 1: fjall-based blob storage (post v0.3.7 migration)
/// - 2: bichon-blob based storage
pub const CURRENT_STORAGE_VERSION: u32 = 2;

const VERSION_FILE: &str = "STORAGE_VERSION";

/// Read the storage layout version from `root_dir/STORAGE_VERSION`.
pub fn read_storage_version(root_dir: &Path) -> Option<u32> {
    let content = std::fs::read_to_string(root_dir.join(VERSION_FILE)).ok()?;
    content.trim().parse().ok()
}

/// Write the storage layout version to `root_dir/STORAGE_VERSION`.
pub fn write_storage_version(root_dir: &Path, version: u32) -> std::io::Result<()> {
    std::fs::write(root_dir.join(VERSION_FILE), format!("{}\n", version))
}

pub fn is_tantivy_index_dir(dir: &PathBuf) -> std::io::Result<bool> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(false);
    }

    let tantivy_extensions = [".store", ".term", ".idx", ".fieldnorm", ".pos"];
    let mut match_count = 0;
    let mut has_meta_json = false;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if name == "meta.json" {
            has_meta_json = true;
            continue;
        }

        if tantivy_extensions.iter().any(|ext| name.ends_with(ext)) {
            match_count += 1;
        }
    }

    Ok(has_meta_json && match_count >= 3)
}

/// Check whether the data layout is compatible with the current server.
/// Returns `false` when legacy data (v0.3.7 or v1.x) is detected and migration is required.
pub fn check_data_status() -> std::io::Result<bool> {
    let root_dir = PathBuf::from(&SETTINGS.bichon_root_dir);

    // 1. Version file takes precedence
    if let Some(version) = read_storage_version(&root_dir) {
        return Ok(version >= CURRENT_STORAGE_VERSION);
    }

    // 2. No version file — check for existing v1.x-style storage (fjall era)
    let new_data_base = SETTINGS
        .bichon_data_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.clone());
    let new_storage_path = new_data_base.join("bichon-storage");

    if is_dir_not_empty(&new_storage_path)? {
        // Existing v1.x install predates version file — mark it as v1
        let _ = write_storage_version(&root_dir, 1);
        return Ok(false); // Needs migration: v1.x → v2.x
    }

    // 3. Check for legacy v0.3.7 Tantivy layout
    let legacy_index_root = SETTINGS
        .bichon_index_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.join("envelope"));
    let legacy_data_root = SETTINGS
        .bichon_data_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.join("eml"));

    let has_legacy_index = is_tantivy_index_dir(&legacy_index_root)?;
    let has_legacy_data = is_tantivy_index_dir(&legacy_data_root)?;

    if has_legacy_index || has_legacy_data {
        Ok(false) // Needs migration
    } else {
        Ok(true) // Fresh install
    }
}

fn is_dir_not_empty(path: &PathBuf) -> std::io::Result<bool> {
    if !path.exists() || !path.is_dir() {
        return Ok(false);
    }
    let mut entries = std::fs::read_dir(path)?;
    Ok(entries.next().is_some())
}
