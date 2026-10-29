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
    database::{
        delete_impl, find_impl, manager::DB_MANAGER, update_impl,
        upsert_impl, MemDbModel,
    },
    error::BichonResult,
    metrics::classify::{classify_error, ErrorKind},
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
    /// Messages dropped by the account's archive rules in this folder. They
    /// are included in `current` (they were processed) but never archived.
    #[serde(default)]
    pub filtered: u64,
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
    /// Messages actually backfilled (new envelopes created).
    pub downloaded: u64,
    pub failed: u64,
    /// Messages skipped by the archive filter before their bodies were
    /// downloaded. Not included in `downloaded`.
    #[serde(default)]
    pub filtered: u64,
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
    /// Total emails skipped by the account's archive rules.
    #[serde(default)]
    pub filtered: u64,
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
    /// Classified kind of this error. Older persisted entries deserialize
    /// as Internal via `#[serde(default)]`.
    #[serde(default)]
    pub kind: ErrorKind,
}

/// Record a finished download session in the metrics registry (session
/// counter, duration histogram). Terminal-state bookkeeping only; call
/// exactly once per session that leaves the Running state.
fn record_finished_session(account_id: u64, session: &DownloadSession) {
    let duration_ms = session.end_time.unwrap_or(session.start_time) - session.start_time;
    crate::metrics::count_sync_session(
        account_id,
        crate::metrics::trigger_label(&session.trigger),
        crate::metrics::download_status_label(&session.status),
        duration_ms,
    );
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
                    record_finished_session(account_id, &old_session);
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
                    record_finished_session(account_id, &session);
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
                        record_finished_session(account_id, &session);
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
        Self::update_folder_progress_inner(
            account_id, folder_name, planned, current, None, status, message,
        )
    }

    /// Updates progress and the archive-filtered count in one read-modify-write.
    /// The download loop changes both on every batch, so writing them through
    /// two calls doubled the state clones and redb writes.
    pub fn update_folder_progress_and_filtered(
        account_id: u64,
        folder_name: String,
        planned: u64,
        current: u64,
        filtered: u64,
        status: FolderStatus,
        message: Option<String>,
    ) -> BichonResult<()> {
        Self::update_folder_progress_inner(
            account_id,
            folder_name,
            planned,
            current,
            Some(filtered),
            status,
            message,
        )
    }

    fn update_folder_progress_inner(
        account_id: u64,
        folder_name: String,
        planned: u64,
        current: u64,
        filtered: Option<u64>,
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
                if let Some(filtered) = filtered {
                    progress.filtered = filtered;
                }
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
                            filtered: 0,
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
        let kind = classify_error(&error);
        // Count before persisting: the error did happen even if the session
        // record cannot be written (e.g. the database is unavailable).
        crate::metrics::count_sync_error(account_id, kind);
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            let new_error = AccountError {
                error,
                at: utc_now!(),
                kind,
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
                run.filtered += stats.filtered;
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
        Self::update_folder_progress_inner(
            account_id,
            folder_name,
            candidate_count,
            downloaded,
            None,
            message,
        )
    }

    /// Updates live progress and the filtered count in one read-modify-write,
    /// mirroring the download state's combined update.
    pub fn update_folder_progress_and_filtered(
        account_id: u64,
        folder_name: String,
        candidate_count: u64,
        downloaded: u64,
        filtered: u64,
        message: Option<String>,
    ) -> BichonResult<()> {
        Self::update_folder_progress_inner(
            account_id,
            folder_name,
            candidate_count,
            downloaded,
            Some(filtered),
            message,
        )
    }

    fn update_folder_progress_inner(
        account_id: u64,
        folder_name: String,
        candidate_count: u64,
        downloaded: u64,
        filtered: Option<u64>,
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
                        filtered: 0,
                        candidate_count,
                        message: None,
                    });
                entry.candidate_count = candidate_count;
                entry.downloaded = downloaded;
                if let Some(filtered) = filtered {
                    entry.filtered = filtered;
                }
                entry.message = message;
            }
            Ok(updated)
        })
    }

    /// Records how many candidates the archive filter rejected before any body
    /// download, so the live gap-fill view can show it alongside progress.
    pub fn set_folder_filtered(
        account_id: u64,
        folder_name: String,
        filtered: u64,
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
                        filtered: 0,
                        candidate_count: 0,
                        message: None,
                    });
                entry.filtered = filtered;
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
        filtered: u64,
    ) -> BichonResult<()> {
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            if let Some(mut run) = updated.active.take() {
                run.status = status;
                run.finished_at = Some(utc_now!());
                run.downloaded = downloaded;
                run.failed = failed;
                run.filtered = filtered;
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

// ─── Source-mailbox cleanup ────────────────────────────────────────────────

/// How many finished cleanup runs are kept per account.
pub const REMOTE_CLEANUP_HISTORY: usize = 30;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum RemoteCleanupStatus {
    #[default]
    Running,
    Success,
    Cancelled,
    /// A guard rail (legal hold, stale backup, missing capability) refused the
    /// run before anything was moved.
    Blocked,
    Failed,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum RemoteCleanupTrigger {
    #[default]
    Scheduled,
    Manual,
}

/// At most this many failed messages are listed **per run** (not per folder).
/// Counters stay exact; the list is for "which ones?" and must not make a run
/// write a megabyte of report on every message. Routine policy skips are
/// counted but not listed — they need no operator action.
pub const REMOTE_CLEANUP_REPORTED_MESSAGES: usize = 200;

/// Display subjects are truncated to this many characters in a report.
///
/// A `Subject:` header is attacker-controlled and has no length limit — a
/// folded header can be tens of kilobytes, and ingest stores it verbatim.
/// Reports only need enough to recognise a message, so this bounds each entry
/// instead of letting a crafted mailbox inflate the state row.
pub const REMOTE_CLEANUP_SUBJECT_CHARS: usize = 160;

/// One message the run did not move, named so an operator can find it.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct RemoteCleanupSkippedMessage {
    /// Source folder the message was found in.
    pub folder: String,
    pub uid: u32,
    pub message_id: String,
    /// Display subject; empty when the archive has none.
    #[serde(default)]
    pub subject: Option<String>,
    /// Stable reason key, the same set counted in `skipped_reasons`.
    pub reason: String,
    /// `true` when the message counted as a failure, `false` for a policy
    /// skip. Mirrors how the counters were incremented.
    pub failed: bool,
    pub size: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct RemoteCleanupFolderStats {
    #[serde(default)]
    pub candidate_count: u64,
    #[serde(default)]
    pub verified: u64,
    #[serde(default)]
    pub moved: u64,
    #[serde(default)]
    pub skipped: u64,
    #[serde(default)]
    pub failed: u64,
    #[serde(default)]
    pub bytes_moved: u64,
    #[serde(default)]
    pub message: Option<String>,
    /// Why messages were skipped/stopped in this folder, for the run report.
    /// Counts only: the UI shows successes and failures, not routine skips.
    #[serde(default)]
    pub skipped_reasons: BTreeMap<String, u64>,
    /// Which messages this folder failed to move, oldest first, capped at
    /// [`REMOTE_CLEANUP_REPORTED_MESSAGES`]. Routine skips are counted in
    /// `skipped` but not listed. `None` — rather than an empty list — marks a
    /// report written before this field existed, which the UI must not present
    /// as "nothing failed".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unprocessed: Option<Vec<RemoteCleanupSkippedMessage>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct RemoteCleanupRun {
    pub run_id: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub trigger: RemoteCleanupTrigger,
    /// The API user who triggered a manual run; `None` for the scheduler.
    pub user: Option<String>,
    pub status: RemoteCleanupStatus,
    pub destination_folder: String,
    /// Per-source-folder outcome, keyed by mailbox name.
    pub folders: BTreeMap<String, RemoteCleanupFolderStats>,
    pub moved: u64,
    pub skipped: u64,
    pub failed: u64,
    pub bytes_moved: u64,
    pub message: Option<String>,
}

/// Independent state for the source-mailbox cleanup task. Moving is repeatable
/// and tracked separately from downloads and gap-fill.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct RemoteCleanupState {
    pub account_id: u64,
    pub active: Option<RemoteCleanupRun>,
    pub history: Vec<RemoteCleanupRun>,
    /// Epoch millis of the last finished (or blocked) run. The scheduler uses
    /// this to avoid re-firing the same cron slot.
    #[serde(default)]
    pub last_finished_at: Option<i64>,
}

impl MemDbModel for RemoteCleanupState {
    fn collection() -> &'static str {
        "remote_cleanup_states"
    }
    fn key(&self) -> String {
        self.account_id.to_string()
    }
}

impl RemoteCleanupState {
    pub fn get(account_id: u64) -> BichonResult<Option<Self>> {
        find_impl::<RemoteCleanupState>(DB_MANAGER.db(), &account_id.to_string())
    }

    /// Starts a run, moving any stale active run into history as failed. The
    /// caller must already hold the account-level cleanup lock.
    pub fn start_run(
        account_id: u64,
        run_id: String,
        trigger: RemoteCleanupTrigger,
        user: Option<String>,
        destination_folder: String,
    ) -> BichonResult<Self> {
        let mut state = Self::get(account_id)?.unwrap_or(RemoteCleanupState {
            account_id,
            ..Default::default()
        });
        if let Some(mut stale) = state.active.take() {
            if stale.status == RemoteCleanupStatus::Running {
                stale.status = RemoteCleanupStatus::Failed;
                stale.finished_at = Some(utc_now!());
                stale.message = Some("interrupted by a new run".into());
                state.history.push(stale);
            }
        }
        state.active = Some(RemoteCleanupRun {
            run_id,
            started_at: utc_now!(),
            finished_at: None,
            trigger,
            user,
            status: RemoteCleanupStatus::Running,
            destination_folder,
            ..Default::default()
        });
        upsert_impl(DB_MANAGER.db(), state.clone())?;
        Ok(state)
    }

    /// Records the latest per-folder counters of the active run.
    ///
    /// The entry is **replaced**, not accumulated, so callers may persist a
    /// cumulative snapshot after every message. Run totals are recomputed from
    /// the per-folder stats, which also keeps them correct across folders.
    pub fn update_folder(
        account_id: u64,
        folder_name: String,
        stats: RemoteCleanupFolderStats,
    ) -> BichonResult<()> {
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            if let Some(ref mut run) = updated.active {
                run.folders.insert(folder_name, stats);
                run.moved = run.folders.values().map(|folder| folder.moved).sum();
                run.skipped = run.folders.values().map(|folder| folder.skipped).sum();
                run.failed = run.folders.values().map(|folder| folder.failed).sum();
                run.bytes_moved = run.folders.values().map(|folder| folder.bytes_moved).sum();
            }
            Ok(updated)
        })
    }

    pub fn finish_run(
        account_id: u64,
        status: RemoteCleanupStatus,
        moved: u64,
        skipped: u64,
        failed: u64,
        bytes_moved: u64,
        message: Option<String>,
    ) -> BichonResult<()> {
        Self::update_state(account_id, move |current| {
            let mut updated = current.clone();
            if let Some(mut run) = updated.active.take() {
                run.status = status;
                run.finished_at = Some(utc_now!());
                run.moved = moved;
                run.skipped = skipped;
                run.failed = failed;
                run.bytes_moved = bytes_moved;
                run.message = message;
                updated.last_finished_at = run.finished_at;
                updated.history.push(run);
                let keep = updated.history.len().saturating_sub(REMOTE_CLEANUP_HISTORY);
                if keep > 0 {
                    updated.history.drain(0..keep);
                }
            }
            Ok(updated)
        })
    }

    /// Finalizes an active run left behind by a crash. Returns `true` when one
    /// was finalized. Called on startup, like the gap-fill stale-run sweep.
    pub fn finalize_stale_run(account_id: u64) -> BichonResult<bool> {
        let stale = Self::get(account_id)?
            .and_then(|state| state.active)
            .map(|run| run.status == RemoteCleanupStatus::Running)
            .unwrap_or(false);
        if stale {
            Self::update_state(account_id, move |current| {
                let mut updated = current.clone();
                if let Some(mut run) = updated.active.take() {
                    if run.status == RemoteCleanupStatus::Running {
                        run.status = RemoteCleanupStatus::Failed;
                        run.finished_at = Some(utc_now!());
                        run.message = Some(
                            "interrupted before completion; review the run report".into(),
                        );
                        updated.last_finished_at = run.finished_at;
                        updated.history.push(run);
                        let keep = updated.history.len().saturating_sub(REMOTE_CLEANUP_HISTORY);
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

    /// Finalize a run that was interrupted by an unexpected error. Preserves
    /// the per-folder totals already recorded. Returns whether a run was
    /// finalized.
    pub fn abort_active_run(account_id: u64, reason: &str) -> BichonResult<bool> {
        Ok(Self::abort_active_run_report(account_id, reason)?.is_some())
    }

    /// Like [`Self::abort_active_run`], but returns the finalized run so the
    /// caller can emit the same audit event a normal finish would.
    pub fn abort_active_run_report(
        account_id: u64,
        reason: &str,
    ) -> BichonResult<Option<RemoteCleanupRun>> {
        let Some(state) = Self::get(account_id)? else {
            return Ok(None);
        };
        let Some(run) = state.active else {
            return Ok(None);
        };
        let moved: u64 = run.folders.values().map(|folder| folder.moved).sum();
        let skipped: u64 = run.folders.values().map(|folder| folder.skipped).sum();
        let failed: u64 = run.folders.values().map(|folder| folder.failed).sum();
        let bytes: u64 = run.folders.values().map(|folder| folder.bytes_moved).sum();
        Self::finish_run(
            account_id,
            RemoteCleanupStatus::Failed,
            moved,
            skipped,
            failed,
            bytes,
            Some(reason.to_string()),
        )?;
        Ok(Self::get(account_id)?
            .and_then(|state| state.history.last().cloned()))
    }

    fn update_state(
        account_id: u64,
        updater: impl FnOnce(RemoteCleanupState) -> BichonResult<RemoteCleanupState> + Send + 'static,
    ) -> BichonResult<()> {
        if Self::get(account_id)?.is_some() {
            update_impl(DB_MANAGER.db(), &account_id.to_string(), updater)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AccountError, ErrorKind, FolderProgress, GapFillFolderStats, GapFillRun,
        RemoteCleanupState,
    };

    /// Download history written by an older version has no `kind` field. It
    /// must keep deserializing (as Internal) instead of breaking the account
    /// state.
    #[test]
    fn legacy_account_error_without_kind_deserializes_as_internal() {
        let error: AccountError =
            serde_json::from_str(r#"{"error":"connection reset","at":123}"#)
                .expect("legacy AccountError must deserialize");
        assert_eq!(error.kind, ErrorKind::Internal);
    }

    #[test]
    fn account_error_round_trips_with_kind() {
        let error = AccountError {
            error: "authentication failed".into(),
            at: 123,
            kind: ErrorKind::Auth,
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"kind\":\"auth\""));
        assert_eq!(serde_json::from_str::<AccountError>(&json).unwrap(), error);
    }

    #[test]
    fn legacy_folder_progress_without_filtered_deserializes() {
        let progress: FolderProgress = serde_json::from_str(
            r#"{"folder_name":"INBOX","planned":10,"current":4,"status":"Downloading","message":null}"#,
        )
        .expect("legacy FolderProgress must deserialize");
        assert_eq!(progress.filtered, 0);
    }

    #[test]
    fn legacy_gap_fill_stats_without_filtered_deserialize() {
        let stats: GapFillFolderStats = serde_json::from_str(
            r#"{"downloaded":2,"failed":0,"candidate_count":5,"message":null}"#,
        )
        .expect("legacy GapFillFolderStats must deserialize");
        assert_eq!(stats.filtered, 0);

        let run: GapFillRun = serde_json::from_str(
            r#"{"started_at":1,"finished_at":null,"status":"Success","folders":{},"downloaded":2,"failed":0}"#,
        )
        .expect("legacy GapFillRun must deserialize");
        assert_eq!(run.filtered, 0);
    }

    /// Cleanup state written before `last_finished_at`/per-folder
    /// `skipped_reasons` existed must keep deserializing after an upgrade.
    #[test]
    fn legacy_remote_cleanup_state_deserializes() {
        let state: RemoteCleanupState = serde_json::from_str(
            r#"{"account_id":1,"active":null,"history":[{"run_id":"r1","started_at":1,"finished_at":2,"trigger":"Scheduled","user":null,"status":"Success","destination_folder":"bichon_auto_created_trash","folders":{"INBOX":{"candidate_count":2,"verified":2,"moved":2,"skipped":0,"failed":0,"bytes_moved":10,"message":null}},"moved":2,"skipped":0,"failed":0,"bytes_moved":10,"message":null}]}"#,
        )
        .expect("legacy RemoteCleanupState must deserialize");
        assert!(state.last_finished_at.is_none());
        assert_eq!(state.history.len(), 1);
        assert!(state.history[0].folders["INBOX"].skipped_reasons.is_empty());
        // A report written before the message list existed must report `None`,
        // not an empty list: the UI has to tell "nothing skipped" apart from
        // "we did not record which ones".
        assert!(state.history[0].folders["INBOX"].unprocessed.is_none());
    }
}
