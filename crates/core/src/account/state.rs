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

use crate::{
    database::{delete_impl, find_impl, manager::DB_MANAGER, update_impl, upsert_impl, MemDbModel},
    error::BichonResult,
    utc_now,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum DownloadStatus {
    Running,
    Success,
    Failed,
    #[default]
    Cancelled,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum TriggerType {
    Manual,
    #[default]
    Scheduled,
    /// Full re-sync (UID SEARCH ALL) invoked explicitly, e.g. to repair a
    /// mailbox whose incremental download was interrupted. Semantically a
    /// manual trigger, tracked distinctly for diagnostics.
    SyncFull,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum FolderStatus {
    #[default]
    Pending,
    Downloading,
    Success,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct FolderProgress {
    pub folder_name: String,
    pub planned: u64,
    pub current: u64,
    pub status: FolderStatus,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum GapFillStatus {
    #[default]
    Running,
    Success,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct GapFillFolderStats {
    pub downloaded: u64,
    pub failed: u64,
    pub candidate_count: u64,
    /// Live progress hint (e.g. "IMAP server is slow...") shown while the
    /// folder is being scanned; usually `None` once the folder is done.
    pub message: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct GapFillRun {
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub status: GapFillStatus,
    /// Per-mailbox gap-fill outcome, keyed by mailbox name.
    pub folders: BTreeMap<String, GapFillFolderStats>,
    /// Total emails newly downloaded by gap-fill.
    pub downloaded: u64,
    /// Total emails that failed to download during gap-fill.
    pub failed: u64,
}

/// Independent, repeatable gap-fill history for an account. Gap-fill is a
/// distinct operation from downloading (it can be run again and again until
/// `failed == 0`), so its runs are tracked separately from `DownloadState`
/// instead of being mixed into download sessions.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct GapFillState {
    pub account_id: u64,
    /// The gap-fill run currently in progress, if any.
    pub active: Option<GapFillRun>,
    /// Finished runs, most recent last.
    pub history: Vec<GapFillRun>,
}

impl MemDbModel for GapFillState {
    fn collection() -> &'static str {
        "gap_fill_states"
    }
    fn key(&self) -> String {
        self.account_id.to_string()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct DownloadSession {
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub status: DownloadStatus,
    pub message: Option<String>,
    pub trigger: TriggerType,
    pub folder_details: BTreeMap<String, FolderProgress>,
    pub current_folder: Option<String>,
    pub errors: Vec<AccountError>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct DownloadState {
    pub account_id: u64,
    pub active_session: Option<DownloadSession>,
    pub history: Vec<DownloadSession>,
    pub last_trigger_at: i64,
    pub last_finished_at: Option<i64>,
}

impl MemDbModel for DownloadState {
    fn collection() -> &'static str {
        "download_states"
    }
    fn key(&self) -> String {
        self.account_id.to_string()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AccountError {
    pub error: String,
    pub at: i64,
}

impl DownloadState {
    pub fn empty(account_id: u64) -> Self {
        DownloadState {
            account_id,
            ..Default::default()
        }
    }

    pub async fn init(account_id: u64) -> BichonResult<()> {
        let now = utc_now!();
        let state = DownloadState {
            account_id,
            last_trigger_at: now,
            active_session: Some(DownloadSession {
                start_time: now,
                status: DownloadStatus::Running,
                trigger: TriggerType::Scheduled,
                ..Default::default()
            }),
            history: Default::default(),
            last_finished_at: Default::default(),
        };
        upsert_impl(DB_MANAGER.db(), state)
    }

    pub fn get(account_id: u64) -> BichonResult<Option<DownloadState>> {
        find_impl::<DownloadState>(DB_MANAGER.db(), &account_id.to_string())
    }

    pub fn start_new_session(account_id: u64, trigger: TriggerType) -> BichonResult<()> {
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            updated.last_trigger_at = utc_now!();

            if let Some(mut old_session) = updated.active_session.take() {
                if old_session.status == DownloadStatus::Running {
                    old_session.status = DownloadStatus::Cancelled;
                    old_session.end_time = Some(utc_now!());
                    old_session.message = Some("Interrupted by a new download session.".into());
                }
                updated.history.push(old_session);
                if updated.history.len() > 30 {
                    updated.history.remove(0);
                }
            }

            let new_session = DownloadSession {
                start_time: utc_now!(),
                status: DownloadStatus::Running,
                trigger,
                ..Default::default()
            };

            updated.active_session = Some(new_session);
            Ok(updated)
        })
    }

    pub fn update_session_status(
        account_id: u64,
        status: DownloadStatus,
        message: Option<String>,
    ) -> BichonResult<()> {
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            if let Some(mut session) = updated.active_session.take() {
                session.status = status.clone();
                if message.is_some() {
                    session.message = message;
                }
                if status == DownloadStatus::Running {
                    updated.active_session = Some(session);
                } else {
                    let now = utc_now!();
                    session.end_time = Some(now);
                    updated.last_finished_at = Some(now);
                    updated.history.push(session);
                    let to_remove = updated.history.len().saturating_sub(10);
                    if to_remove > 0 {
                        updated.history.drain(0..to_remove);
                    }
                }
            }
            Ok(updated)
        })
    }

    /// Moves a stale Running session into history as Cancelled.
    ///
    /// A Running `active_session` that survives an Idle decision means the
    /// previous run was interrupted without a clean shutdown (e.g. process
    /// killed mid-download). Leaving it in place makes the UI show a phantom
    /// "syncing" state even though nothing is downloading. Callers invoke this
    /// only when no download is actually running for the account, so a
    /// legitimately active session is never touched.
    ///
    /// Returns `true` if a stale session was finalized (i.e. the previous sync
    /// did not finish) and `false` otherwise.
    pub fn finalize_stale_session(account_id: u64) -> BichonResult<bool> {
        let stale = Self::get(account_id)?
            .and_then(|s| s.active_session)
            .map_or(false, |s| s.status == DownloadStatus::Running);
        if stale {
            Self::update_state(account_id, move |current| {
                let mut updated = current.clone();
                if updated
                    .active_session
                    .as_ref()
                    .map_or(false, |s| s.status == DownloadStatus::Running)
                {
                    if let Some(mut session) = updated.active_session.take() {
                        session.status = DownloadStatus::Cancelled;
                        session.end_time = Some(utc_now!());
                        session.message = Some(
                            "Previous sync did not finish cleanly; marked as cancelled on startup."
                                .into(),
                        );
                        updated.history.push(session);
                        updated.last_finished_at = Some(utc_now!());
                        updated.active_session = None;
                    }
                }
                Ok(updated)
            })?;
        }
        Ok(stale)
    }

    pub fn update_folder_progress(
        account_id: u64,
        folder_name: String,
        planned: u64,
        current: u64,
        status: FolderStatus,
        message: Option<String>,
    ) -> BichonResult<()> {
        Self::update_state(account_id, move |state| {
            let mut updated = state.clone();
            if let Some(ref mut session) = updated.active_session {
                session.current_folder = Some(folder_name.clone());

                let progress =
                    session
                        .folder_details
                        .entry(folder_name.clone())
                        .or_insert(FolderProgress {
                            folder_name,
                            ..Default::default()
                        });

                progress.planned = planned;
                progress.current = current;
                progress.status = status;
                progress.message = message;
            }
            Ok(updated)
        })
    }

    /// Touches only `current_folder` without rewriting folder progress. Lets
    /// long-running IMAP operations (e.g. waiting on a slow server mid-batch)
    /// keep the UI's "last activity" indicator fresh without spamming writes.
    pub fn set_current_folder(account_id: u64, folder_name: String) -> BichonResult<()> {
        Self::update_state(account_id, move |state| {
            let mut updated = state.clone();
            if let Some(ref mut session) = updated.active_session {
                session.current_folder = Some(folder_name);
            }
            Ok(updated)
        })
    }

    pub fn init_folder_details(account_id: u64, folders: Vec<String>) -> BichonResult<()> {
        Self::update_state(account_id, move |state| {
            let mut updated = state.clone();
            if let Some(ref mut session) = updated.active_session {
                for name in folders {
                    session.folder_details.insert(
                        name.clone(),
                        FolderProgress {
                            folder_name: name,
                            planned: 0,
                            current: 0,
                            status: FolderStatus::Pending,
                            message: None,
                        },
                    );
                }
            }
            Ok(updated)
        })
    }

    pub fn append_session_error(account_id: u64, error: String) -> BichonResult<()> {
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            let new_error = AccountError {
                error,
                at: utc_now!(),
            };
            let target = updated
                .active_session
                .as_mut()
                .or_else(|| updated.history.last_mut());
            if let Some(session) = target {
                session.errors.push(new_error);
                let to_remove = session.errors.len().saturating_sub(30);
                if to_remove > 0 {
                    session.errors.drain(0..to_remove);
                }
            }
            Ok(updated)
        })
    }

    /// Appends/updates a free-form message on the active session without
    /// changing its status. Used to record the gap-fill summary.
    pub fn update_session_message(account_id: u64, message: String) -> BichonResult<()> {
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            if let Some(ref mut session) = updated.active_session {
                session.message = Some(message);
            }
            Ok(updated)
        })
    }

    fn update_state(
        account_id: u64,
        updater: impl FnOnce(DownloadState) -> BichonResult<DownloadState> + Send + 'static,
    ) -> BichonResult<()> {
        if Self::get(account_id)?.is_some() {
            update_impl(DB_MANAGER.db(), &account_id.to_string(), updater)?;
        }
        Ok(())
    }

    pub fn delete(account_id: u64) -> BichonResult<()> {
        if Self::get(account_id)?.is_none() {
            return Ok(());
        }

        delete_impl::<DownloadState>(DB_MANAGER.db(), &account_id.to_string())
    }
}

impl GapFillState {
    pub fn get(account_id: u64) -> BichonResult<Option<GapFillState>> {
        find_impl::<GapFillState>(DB_MANAGER.db(), &account_id.to_string())
    }

    /// Starts a new gap-fill run, moving any stale active run into history as
    /// Cancelled. Creates the state record on first use.
    pub fn start_run(account_id: u64) -> BichonResult<()> {
        let now = utc_now!();
        let run = GapFillRun {
            started_at: now,
            status: GapFillStatus::Running,
            ..Default::default()
        };
        if Self::get(account_id)?.is_none() {
            let state = GapFillState {
                account_id,
                active: Some(run),
                history: Vec::new(),
            };
            return upsert_impl(DB_MANAGER.db(), state);
        }
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            if let Some(mut old) = updated.active.take() {
                if old.status == GapFillStatus::Running {
                    old.status = GapFillStatus::Cancelled;
                    old.finished_at = Some(utc_now!());
                }
                updated.history.push(old);
                let keep = updated.history.len().saturating_sub(10);
                if keep > 0 {
                    updated.history.drain(0..keep);
                }
            }
            updated.active = Some(run);
            Ok(updated)
        })
    }

    /// Accumulates a per-folder outcome into the active run.
    pub fn add_folder_result(
        account_id: u64,
        folder_name: String,
        stats: GapFillFolderStats,
    ) -> BichonResult<()> {
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            if let Some(ref mut run) = updated.active {
                run.downloaded += stats.downloaded;
                run.failed += stats.failed;
                run.folders.insert(folder_name, stats);
            }
            Ok(updated)
        })
    }
    /// Updates the live per-folder progress of the active run (used during a
    /// gap-fill scan so the UI can show per-folder progress without waiting
    /// for the folder to finish). `candidate_count` is the planned total,
    /// `downloaded` the current count, `message` an optional live hint
    /// (e.g. slow-server notice).
    pub fn update_folder_progress(
        account_id: u64,
        folder_name: String,
        candidate_count: u64,
        downloaded: u64,
        message: Option<String>,
    ) -> BichonResult<()> {
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            if let Some(ref mut run) = updated.active {
                let entry = run
                    .folders
                    .entry(folder_name.clone())
                    .or_insert(GapFillFolderStats {
                        downloaded: 0,
                        failed: 0,
                        candidate_count,
                        message: None,
                    });
                entry.candidate_count = candidate_count;
                entry.downloaded = downloaded;
                entry.message = message;
            }
            Ok(updated)
        })
    }

    /// Finalizes the active run: moves it to history with the given status and
    /// totals. `failed`/`downloaded` are the authoritative accumulated values.
    pub fn finish_run(
        account_id: u64,
        status: GapFillStatus,
        downloaded: u64,
        failed: u64,
    ) -> BichonResult<()> {
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            if let Some(mut run) = updated.active.take() {
                run.status = status;
                run.finished_at = Some(utc_now!());
                run.downloaded = downloaded;
                run.failed = failed;
                updated.history.push(run);
                let keep = updated.history.len().saturating_sub(10);
                if keep > 0 {
                    updated.history.drain(0..keep);
                }
            }
            Ok(updated)
        })
    }

    /// Moves a stale Running active run into history as Cancelled.
    ///
    /// A Running `active` that survives a restart means the previous gap-fill
    /// run was interrupted without finishing (process killed mid-scan). Leaving
    /// it in place makes the UI show a phantom "Running" gap-fill. Callers
    /// invoke this on startup, when no gap-fill is actually running.
    ///
    /// Returns `true` if a stale run was finalized.
    pub fn finalize_stale_run(account_id: u64) -> BichonResult<bool> {
        let stale = Self::get(account_id)?
            .and_then(|s| s.active)
            .map_or(false, |r| r.status == GapFillStatus::Running);
        if stale {
            Self::update_state(account_id, move |current| {
                let mut updated = current.clone();
                if let Some(mut run) = updated.active.take() {
                    if run.status == GapFillStatus::Running {
                        run.status = GapFillStatus::Cancelled;
                        run.finished_at = Some(utc_now!());
                        updated.history.push(run);
                        let keep = updated.history.len().saturating_sub(10);
                        if keep > 0 {
                            updated.history.drain(0..keep);
                        }
                    } else {
                        updated.active = Some(run);
                    }
                }
                Ok(updated)
            })?;
        }
        Ok(stale)
    }

    fn update_state(
        account_id: u64,
        updater: impl FnOnce(GapFillState) -> BichonResult<GapFillState> + Send + 'static,
    ) -> BichonResult<()> {
        if Self::get(account_id)?.is_some() {
            update_impl(DB_MANAGER.db(), &account_id.to_string(), updater)?;
        }
        Ok(())
    }
}
