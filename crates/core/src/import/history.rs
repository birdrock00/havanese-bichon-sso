//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project

use crate::database::MemDbModel;
use crate::import::{ImportProgress, ImportStatus};
use serde::{Deserialize, Serialize};

/// Maximum number of import history entries to keep per user.
pub const MAX_HISTORY_PER_USER: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ImportHistory {
    /// Composite key: "{user_id}:{import_id}"
    pub id: String,
    pub user_id: u64,
    pub import_id: String,
    pub account_id: u64,
    pub folder: String,
    pub format: String,
    pub status: String,
    pub total: usize,
    pub success: usize,
    pub duplicates: usize,
    pub failed: usize,
    pub failed_details: Vec<crate::import::FailedItemDetail>,
    /// Unix timestamp in milliseconds.
    pub created_at: i64,
}

impl MemDbModel for ImportHistory {
    fn collection() -> &'static str {
        "import_history"
    }
    fn key(&self) -> String {
        self.id.clone()
    }
}

impl ImportHistory {
    pub fn from_progress(
        user_id: u64,
        import_id: &str,
        account_id: u64,
        folder: &str,
        progress: &ImportProgress,
    ) -> Self {
        Self {
            id: format!("{}:{}", user_id, import_id),
            user_id,
            import_id: import_id.to_string(),
            account_id,
            folder: folder.to_string(),
            format: progress.format.clone(),
            status: match progress.status {
                ImportStatus::Pending => "pending",
                ImportStatus::Processing => "processing",
                ImportStatus::Completed => "completed",
                ImportStatus::Failed => "failed",
            }
            .to_string(),
            total: progress.total,
            success: progress.success,
            duplicates: progress.duplicates,
            failed: progress.failed,
            failed_details: progress.failed_details.clone(),
            created_at: crate::utc_now!(),
        }
    }
}

/// Prune old entries for a user so only the latest `MAX_HISTORY_PER_USER` remain.
pub fn prune_user_history(user_id: u64) -> crate::error::BichonResult<()> {
    use crate::database::manager::DB_MANAGER;
    use crate::database::batch_delete_impl;
    use crate::raise_error;
    use crate::error::code::ErrorCode;
    let db = DB_MANAGER.db();
    let coll = db.collection(ImportHistory::collection());
    let prefix = format!("{}:", user_id);

    let mut entries: Vec<ImportHistory> = coll
        .scan_prefix(&prefix)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    if entries.len() <= MAX_HISTORY_PER_USER {
        return Ok(());
    }

    // Sort by created_at descending (newest first), keep the first N
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let to_delete: Vec<String> = entries
        .iter()
        .skip(MAX_HISTORY_PER_USER)
        .map(|e| e.id.clone())
        .collect();

    if !to_delete.is_empty() {
        batch_delete_impl::<ImportHistory>(db, to_delete)?;
    }

    Ok(())
}

/// Save an import history record and prune old entries for the user.
pub fn save_import_history(
    user_id: u64,
    account_id: u64,
    folder: &str,
    progress: &ImportProgress,
) {
    use crate::database::manager::DB_MANAGER;
    use crate::database::upsert_impl;

    let entry = ImportHistory::from_progress(user_id, &progress.import_id, account_id, folder, progress);
    let db = DB_MANAGER.db();

    if let Err(e) = upsert_impl::<ImportHistory>(db, entry) {
        tracing::error!("Failed to save import history: {:?}", e);
        return;
    }

    if let Err(e) = prune_user_history(user_id) {
        tracing::warn!("Failed to prune import history: {:?}", e);
    }
}
