//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Persisted records of backup runs.
//!
//! Stored in the memdb `backup_records` collection. The backup manager writes
//! these **directly** (bypassing the gated write helpers): the "Running"
//! record is created inside the write window and the final record after the
//! run completes, both while the gate is paused — the gated helpers would
//! rightfully reject those writes.

use serde::{Deserialize, Serialize};

use crate::backup::backend::BackupSummary;
use crate::database::MemDbModel;

/// Outcome of a backup run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
#[cfg_attr(feature = "web-api", oai(rename_all = "snake_case"))]
pub enum BackupRunStatus {
    /// The run is currently executing.
    Running,
    /// The run completed and the engine committed a restore point.
    #[default]
    Success,
    /// The run failed at some phase; `BackupRecord::error` has the detail.
    Failed,
}

/// One backup run persisted in the memdb.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct BackupRecord {
    pub id: String,
    /// `"manual"` (WebUI/API button) or `"schedule"` (cron tick).
    pub trigger: String,
    /// UTC millis when the run started.
    pub started_at: i64,
    /// UTC millis when the run finished (success or failure); `None` while
    /// running or if the process died mid-run.
    pub finished_at: Option<i64>,
    pub status: BackupRunStatus,
    /// The last phase reached before the run ended (e.g. `"preflight"`,
    /// `"quiesce"`, `"prepare"`, `"backup"`, `"finalize"`).
    pub phase: String,
    pub error: Option<String>,
    pub snapshot_id: Option<String>,
    pub summary: Option<BackupSummary>,
    /// Non-fatal warnings reported by preparers during the run (e.g. a
    /// database growing large enough to slow backups). Empty when the run had
    /// nothing to flag.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Which backup chain this run wrote to — the target plus the prefix, as
    /// [`crate::backup::config::destination`] renders it.
    ///
    /// The run history is a single global list, but a restore point only
    /// exists on the target that wrote it. Without this the list mixes runs
    /// from two different destinations and the status card can point at a
    /// manifest the browser (which reads the *current* target only) cannot
    /// find. `None` on records written before this field existed — those can
    /// only have come from the single target configured at the time, so
    /// readers treat `None` as "belongs to whatever is configured now".
    #[serde(default)]
    pub target: Option<String>,
}

impl MemDbModel for BackupRecord {
    fn collection() -> &'static str {
        "backup_records"
    }
    fn key(&self) -> String {
        self.id.clone()
    }
}

impl BackupRecord {
    /// Cap on persisted run records. Every write prunes records beyond the
    /// most recent [`Self::HISTORY_LIMIT`], so the history rolls without a
    /// separate cleanup task — the page shows the last 20, the API serves up
    /// to 200, and anything older is noise no one reads.
    pub const HISTORY_LIMIT: usize = 200;

    pub fn list_recent(limit: usize) -> Vec<BackupRecord> {
        let mut all: Vec<BackupRecord> = crate::database::list_all_impl(
            crate::database::manager::DB_MANAGER.db(),
        )
        .unwrap_or_default();
        all.sort_by_key(|r| std::cmp::Reverse(r.started_at));
        all.truncate(limit);
        all
    }

    /// The most recent successful run against `destination`.
    ///
    /// The history is one global list, but a restore point only exists on the
    /// target that wrote it, so anything asking "what did the last backup do?"
    /// must scope to a destination — otherwise a target change makes the
    /// answer describe a target the caller cannot see.
    ///
    /// A record with no `target` (written before the field existed) is treated
    /// as belonging to any destination: those runs predate target switching,
    /// so they can only have come from the single target configured at the
    /// time.
    pub fn last_success_for(destination: &str) -> Option<BackupRecord> {
        Self::list_recent(20).into_iter().find(|r| {
            r.status == BackupRunStatus::Success
                && r.target.as_deref().is_none_or(|t| t == destination)
        })
    }

    /// Delete run records beyond the most recent `limit` (newest first by
    /// `started_at`, the same ordering `list_recent` shows). Returns how many
    /// were removed.
    pub fn prune_beyond(limit: usize) -> usize {
        let db = crate::database::manager::DB_MANAGER.db();
        let Ok(all) = db
            .collection(Self::collection())
            .list_all::<BackupRecord>()
        else {
            return 0;
        };
        if all.len() <= limit {
            return 0;
        }
        let mut all = all;
        all.sort_by_key(|r| std::cmp::Reverse(r.started_at));
        let mut removed = 0;
        for r in &all[limit..] {
            if db
                .collection(Self::collection())
                .delete(r.key())
                .unwrap_or(false)
            {
                removed += 1;
            }
        }
        removed
    }
}
