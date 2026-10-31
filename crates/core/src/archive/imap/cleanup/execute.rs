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

//! Moves source mail into `bichon_auto_created_trash` on the account's own IMAP
//! server. The move is reversible, and Bichon never deletes from the server:
//! emptying that folder is left to the operator, with their own mail client.

use std::collections::BTreeMap;

use tokio_util::sync::CancellationToken;

use crate::account::migration::{
    AccountModel, MAX_REMOTE_CLEANUP_PER_RUN, REMOTE_CLEANUP_FOLDER,
};
use crate::account::state::{
    RemoteCleanupFolderStats, RemoteCleanupRun, RemoteCleanupSkippedMessage, RemoteCleanupState,
    RemoteCleanupStatus, RemoteCleanupTrigger, REMOTE_CLEANUP_REPORTED_MESSAGES,
    REMOTE_CLEANUP_SUBJECT_CHARS,
};
use crate::archive::imap::cleanup::plan;
use crate::archive::imap::cleanup::preview::{self, CleanupPreviewKind};
use crate::archive::imap::cleanup::verify::{self, VerifiedMove, VerifyIssue};

/// Stable reason keys for move failures. Counted in the report and translated
/// by the WebUI; the detail string is for logs and the run's stop message.
const FAIL_SOURCE_SELECT: &str = "source_select_failed";
const FAIL_MOVE: &str = "move_failed";
const FAIL_SOURCE_CHECK: &str = "source_check_failed";
const FAIL_SOURCE_UID_SURVIVES: &str = "source_uid_survives_move";
use crate::archive::imap::cleanup::{
    try_acquire, MAX_LOCAL_INTEGRITY_FAILURES, MESSAGE_DELAY,
};
use crate::archive::imap::task::SYNC_TASKS;
use crate::backup::gate::WRITE_GATE;
use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::ext::event_bus::{emit, Event};
use crate::imap::executor::ImapExecutor;
use crate::imap::session::SessionStream;
use crate::raise_error;
use crate::store::tantivy::envelope::ENVELOPE_MANAGER;
use crate::utc_now;
use async_imap::Session;
use tracing::{debug, info, warn};

#[derive(Clone, Debug)]
pub struct MoveRequest {
    pub trigger: RemoteCleanupTrigger,
    pub user: Option<String>,
    /// Required for `RemoteCleanupTrigger::Manual`; `None` for the scheduler.
    pub preview_token: Option<String>,
    pub cancel: CancellationToken,
}

impl MoveRequest {
    pub fn scheduled() -> Self {
        Self {
            trigger: RemoteCleanupTrigger::Scheduled,
            user: None,
            preview_token: None,
            cancel: CancellationToken::new(),
        }
    }

    pub fn manual(user: String, preview_token: String) -> Self {
        Self {
            trigger: RemoteCleanupTrigger::Manual,
            user: Some(user),
            preview_token: Some(preview_token),
            cancel: CancellationToken::new(),
        }
    }
}

fn trigger_name(trigger: &RemoteCleanupTrigger) -> String {
    match trigger {
        RemoteCleanupTrigger::Scheduled => "scheduled",
        RemoteCleanupTrigger::Manual => "manual",
    }
    .to_string()
}

fn status_name(status: &RemoteCleanupStatus) -> String {
    match status {
        RemoteCleanupStatus::Running => "running",
        RemoteCleanupStatus::Success => "success",
        RemoteCleanupStatus::Cancelled => "cancelled",
        RemoteCleanupStatus::Blocked => "blocked",
        RemoteCleanupStatus::Failed => "failed",
    }
    .to_string()
}

fn emit_finished(
    account_id: u64,
    run_id: &str,
    trigger: &RemoteCleanupTrigger,
    status: &RemoteCleanupStatus,
    moved: u64,
    skipped: u64,
    failed: u64,
    bytes_moved: u64,
    user: Option<&str>,
    message: Option<&str>,
) {
    emit(Event::RemoteCleanupRunFinished {
        user: user.map(str::to_string),
        account_id,
        run_id: run_id.to_string(),
        trigger: trigger_name(trigger),
        status: status_name(status),
        moved,
        skipped,
        failed,
        bytes_moved,
        message: message.map(str::to_string),
    });
}

fn latest_run(account_id: u64) -> BichonResult<RemoteCleanupRun> {
    let state = RemoteCleanupState::get(account_id)?;
    if let Some(active) = state.as_ref().and_then(|state| state.active.clone()) {
        return Ok(active);
    }
    Ok(state
        .and_then(|state| state.history.last().cloned())
        .unwrap_or_default())
}

fn blocked_run(
    account_id: u64,
    run_id: &str,
    request: &MoveRequest,
    destination: &str,
    reason: &str,
) -> BichonResult<RemoteCleanupRun> {
    RemoteCleanupState::start_run(
        account_id,
        run_id.to_string(),
        request.trigger.clone(),
        request.user.clone(),
        destination.to_string(),
    )?;
    RemoteCleanupState::finish_run(
        account_id,
        RemoteCleanupStatus::Blocked,
        0,
        0,
        0,
        0,
        Some(reason.to_string()),
    )?;
    emit(Event::RemoteCleanupBlocked {
        user: request.user.clone(),
        account_id,
        reason: reason.to_string(),
    });
    emit_finished(
        account_id,
        run_id,
        &request.trigger,
        &RemoteCleanupStatus::Blocked,
        0,
        0,
        0,
        0,
        request.user.as_deref(),
        Some(reason),
    );
    latest_run(account_id)
}

fn finish_run(
    account_id: u64,
    run_id: &str,
    request: &MoveRequest,
    status: RemoteCleanupStatus,
    moved: u64,
    skipped: u64,
    failed: u64,
    bytes_moved: u64,
    message: Option<String>,
) -> BichonResult<RemoteCleanupRun> {
    RemoteCleanupState::finish_run(
        account_id,
        status.clone(),
        moved,
        skipped,
        failed,
        bytes_moved,
        message.clone(),
    )?;
    emit_finished(
        account_id,
        run_id,
        &request.trigger,
        &status,
        moved,
        skipped,
        failed,
        bytes_moved,
        request.user.as_deref(),
        message.as_deref(),
    );
    latest_run(account_id)
}

fn add_skip(map: &mut BTreeMap<String, u64>, reason: &str) {
    *map.entry(reason.to_string()).or_insert(0) += 1;
}

/// A display subject, trimmed for the report. `None` when there is nothing
/// worth showing: the header is attacker-controlled and unbounded, and a report
/// entry only needs enough to recognise the message.
fn report_subject(subject: &str) -> Option<String> {
    let trimmed = subject.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().count() <= REMOTE_CLEANUP_SUBJECT_CHARS {
        return Some(trimmed.to_string());
    }
    let head: String = trimmed.chars().take(REMOTE_CLEANUP_SUBJECT_CHARS).collect();
    Some(format!("{head}…"))
}

/// Budget shared by every folder of one run.
///
/// The list is capped per **run**, not per folder: a folder contributes at most
/// what is left of the budget. Capping per folder would let the number of
/// source folders multiply the report row, and that row is rewritten before
/// every single message.
struct ReportBudget {
    remaining: usize,
}

impl ReportBudget {
    fn new() -> Self {
        Self {
            remaining: REMOTE_CLEANUP_REPORTED_MESSAGES,
        }
    }

    fn take(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        true
    }
}

/// Record one message the run did not move. The reason counters and this list
/// are updated together so the report can always answer "which ones?" for the
/// numbers it shows; the counters are exact, the list is bounded.
fn add_unprocessed(
    list: &mut Vec<RemoteCleanupSkippedMessage>,
    budget: &mut ReportBudget,
    candidate: &plan::Candidate,
    reason: &str,
    failed: bool,
) {
    // Skips are expected policy outcomes and are not shown to the user, so the
    // capped list is spent on failures only: those are the messages an
    // operator can act on, and a long tail of skips must not crowd them out.
    if !failed {
        return;
    }
    if !budget.take() {
        return;
    }
    list.push(RemoteCleanupSkippedMessage {
        folder: candidate.mailbox.name.clone(),
        uid: candidate.uid,
        message_id: candidate.message_id.clone(),
        subject: report_subject(&candidate.subject),
        reason: reason.to_string(),
        failed,
        size: candidate.size,
    });
}

/// Move one verified message with `UID MOVE` (RFC 6851).
///
/// MOVE is the **only** move path. There is deliberately no copy-and-expunge
/// fallback: Bichon must never issue `EXPUNGE`, even scoped to one UID, so a
/// server that does not advertise `MOVE` cannot run cleanup at all.
///
/// Only a source that still holds the message after the move is fatal. Every
/// other outcome leaves the archive and the server consistent, so it must not
/// stop the run.
async fn move_one(
    session: &mut Session<Box<dyn SessionStream>>,
    folder: &crate::archive::imap::mailbox::MailBox,
    verified: &VerifiedMove,
) -> Result<(), VerifyIssue> {
    let destination = REMOTE_CLEANUP_FOLDER;
    let uid = verified.uid;

    let remote_uid_validity = ImapExecutor::select_mailbox(session, &folder.encoded_name())
        .await
        .map_err(|e| VerifyIssue::fatal(FAIL_SOURCE_SELECT, format!("source select failed: {e}")))?;
    if remote_uid_validity != verified.uid_validity {
        return Err(VerifyIssue::fatal(
            verify::FATAL_UIDVALIDITY_CHANGED,
            "UIDVALIDITY changed before the move",
        ));
    }

    // RFC 6851: the server must not answer OK until the message is in the
    // destination, so its presence there needs no follow-up search — and
    // searching for it is actively harmful on servers whose SEARCH lags or
    // ignores `HEADER Message-ID`. The source post-check below is the one
    // that matters.
    ImapExecutor::uid_mv(session, &uid.to_string(), destination)
        .await
        .map_err(|e| VerifyIssue::fatal(FAIL_MOVE, format!("UID MOVE failed: {e}")))?;

    // The source must no longer hold the message. This is the post-condition
    // that actually proves the move: if the UID survives, the message was not
    // moved and the run must not count it.
    let still_there = ImapExecutor::uid_search_in(
        session,
        &folder.encoded_name(),
        &format!("UID {uid}"),
    )
    .await
    .map_err(|e| {
        VerifyIssue::fatal(FAIL_SOURCE_CHECK, format!("source post-check failed: {e}"))
    })?;
    if still_there.contains(&uid) {
        return Err(VerifyIssue::fatal(
            FAIL_SOURCE_UID_SURVIVES,
            format!("uid {uid} is still present in '{}' after the move", folder.name),
        ));
    }

    Ok(())
}

/// Run the scheduled/manual move task. Returns the finished run report.
pub async fn execute_move(
    account: &AccountModel,
    request: MoveRequest,
) -> BichonResult<RemoteCleanupRun> {
    let config = account.remote_cleanup.clone().ok_or_else(|| {
        raise_error!(
            "Source-mailbox cleanup is not configured".into(),
            ErrorCode::InvalidParameter
        )
    })?;
    let destination = REMOTE_CLEANUP_FOLDER.to_string();
    let run_id = uuid::Uuid::new_v4().to_string();

    // Manual runs must carry a preview token. It is validated after the flag
    // pre-filter, because that filter defines the candidate set the preview
    // showed (flagged messages must not consume the per-run cap).
    let scan = plan::collect(account, utc_now!())?;
    if request.trigger == RemoteCleanupTrigger::Manual
        && request.preview_token.is_none()
    {
        return Err(raise_error!(
            "manual cleanup requires a preview token".into(),
            ErrorCode::InvalidParameter
        ));
    }

    if account.deleting {
        return blocked_run(
            account.id,
            &run_id,
            &request,
            &destination,
            "account is being deleted",
        );
    }
    if !account.enabled {
        return blocked_run(
            account.id,
            &run_id,
            &request,
            &destination,
            "account is disabled",
        );
    }
    if account.is_on_hold() {
        return blocked_run(account.id, &run_id, &request, &destination, "legal hold");
    }
    if request.trigger == RemoteCleanupTrigger::Scheduled && !config.enabled {
        return blocked_run(
            account.id,
            &run_id,
            &request,
            &destination,
            "scheduled cleanup is disabled",
        );
    }

    if !try_acquire(account.id).await {
        return Err(raise_error!(
            "A cleanup operation is already running for this account".into(),
            ErrorCode::InvalidParameter
        ));
    }
    // On any early return (including a write-gate rejection), make sure the run
    // cannot stay "Running" forever and the account lock is released.
    // This guard owns only the cleanup lock. The sync slot has its own guard
    // below: releasing a slot we never acquired would clear a download task's
    // busy marker and break mutual exclusion.
    let _cleanup_guard = scopeguard::guard(account.id, |id| {
        if let Ok(Some(run)) =
            RemoteCleanupState::abort_active_run_report(id, "interrupted before completion")
        {
            emit_finished(
                id,
                &run.run_id,
                &run.trigger,
                &run.status,
                run.moved,
                run.skipped,
                run.failed,
                run.bytes_moved,
                run.user.as_deref(),
                run.message.as_deref(),
            );
        }
        super::release(id);
    });

    // Transient conflicts are deferred, not recorded as blocked runs: writing
    // last_finished_at here would consume the cron slot for a whole day.
    if SYNC_TASKS.is_account_busy(account.id).await
        || SYNC_TASKS.is_manual_running(account.id).await
    {
        return Err(raise_error!(
            "a sync or manual download is running; cleanup will retry on the next slot".into(),
            ErrorCode::InvalidParameter
        ));
    }
    if WRITE_GATE.is_paused() {
        return Err(raise_error!(
            "a backup capture window is open; cleanup will retry on the next slot".into(),
            ErrorCode::InvalidParameter
        ));
    }
    // Hold the same account slot the download tasks use for the whole run, so
    // a sync cannot start between this check and the first move.
    if !SYNC_TASKS.try_lock_account(account.id).await {
        return Err(raise_error!(
            "the account was claimed by a sync; cleanup will retry on the next slot".into(),
            ErrorCode::InvalidParameter
        ));
    }
    let _sync_guard = scopeguard::guard(account.id, |id| {
        tokio::spawn(async move {
            SYNC_TASKS.unlock_account(id).await;
        });
    });

    let mut session = ImapExecutor::create_connection(account.id).await?;
    // Cleanup only ever moves mail. A server that cannot MOVE cannot free the
    // source mailbox, and there is no copy/expunge fallback by design, so say
    // so instead of doing something that would not help.
    if !ImapExecutor::supports_move(&mut session).await? {
        session.logout().await.ok();
        return blocked_run(
            account.id,
            &run_id,
            &request,
            &destination,
            "server does not advertise MOVE (RFC 6851); source-mailbox cleanup is unsupported",
        );
    }

    // Filter flagged messages first. The token is bound to this whole window,
    // not to the first N entries: verification skips (gone from the source,
    // failed checks) must not shrink the run below its batch size, so the loop
    // below keeps pulling candidates from the window until `per_run` messages
    // have actually moved.
    let per_run = config
        .max_messages_per_run
        .clamp(1, MAX_REMOTE_CLEANUP_PER_RUN) as usize;
    let (kept, flagged_skipped, flagged_dropped) =
        plan::apply_flagged_filter(&mut session, &scan.candidates, config.skip_flagged).await?;
    if let Some(token) = request.preview_token.as_deref() {
        preview::consume_ticket(
            token,
            account.id,
            CleanupPreviewKind::Move,
            &plan::candidate_digest(&kept),
            &plan::config_digest(&config),
        )
        .map_err(|e| raise_error!(e, ErrorCode::InvalidParameter))?;
    }

    if !ImapExecutor::mailbox_exists(&mut session, &destination).await? {
        if let Err(e) = ImapExecutor::create_mailbox(&mut session, &destination).await {
            session.logout().await.ok();
            return blocked_run(
                account.id,
                &run_id,
                &request,
                &destination,
                &format!("failed to create '{destination}': {e}"),
            );
        }
        if !ImapExecutor::mailbox_exists(&mut session, &destination).await? {
            session.logout().await.ok();
            return blocked_run(
                account.id,
                &run_id,
                &request,
                &destination,
                &format!("'{destination}' could not be created"),
            );
        }
    }
    if let Err(e) = ImapExecutor::examine_mailbox(&mut session, &destination).await {
        session.logout().await.ok();
        return blocked_run(
            account.id,
            &run_id,
            &request,
            &destination,
            &format!("'{destination}' is not selectable: {e}"),
        );
    }

    RemoteCleanupState::start_run(
        account.id,
        run_id.clone(),
        request.trigger.clone(),
        request.user.clone(),
        destination.clone(),
    )?;
    info!(
        account_id = account.id,
        run_id = %run_id,
        trigger = %trigger_name(&request.trigger),
        user = request.user.as_deref().unwrap_or("system"),
        window = kept.len(),
        batch = per_run,
        "remote-cleanup: run started"
    );

    let mut total_moved = 0u64;
    let mut total_skipped: u64 = flagged_skipped.values().copied().sum();
    let mut total_failed = 0u64;
    let mut total_bytes = 0u64;
    let mut stop: Option<(RemoteCleanupStatus, String)> = None;
    // Envelope ids moved in this run, flagged in the index once the run ends.
    // Marking per message would re-index the whole document (body, attachments,
    // trigram fields) on every single move, so the batch is written once.
    let mut moved_envelope_ids: Vec<String> = Vec::new();
    // One budget for the whole run, so the number of source folders cannot
    // multiply the size of the stored report.
    let mut report_budget = ReportBudget::new();

    'folders: for folder in &scan.folders {
        // The batch is full: verification skips earlier in the run have
        // already been compensated for from the same window.
        if total_moved as usize >= per_run {
            break;
        }
        let candidates: Vec<_> = kept
            .iter()
            .filter(|candidate| candidate.mailbox.id == folder.id)
            .cloned()
            .collect();
        // Flagged messages are filtered before the window is walked, so a
        // folder whose candidates were *all* flagged has nothing left to
        // process. Report those messages instead of silently omitting it.
        let folder_flagged: Vec<_> = flagged_dropped
            .iter()
            .filter(|candidate| candidate.mailbox.id == folder.id)
            .cloned()
            .collect();
        if candidates.is_empty() {
            if folder_flagged.is_empty() {
                continue;
            }
            let mut stats = RemoteCleanupFolderStats {
                candidate_count: folder_flagged.len() as u64,
                skipped: folder_flagged.len() as u64,
                ..Default::default()
            };
            let mut unprocessed = Vec::new();
            for candidate in &folder_flagged {
                add_unprocessed(
                    &mut unprocessed,
                    &mut report_budget,
                    candidate,
                    super::SKIP_FLAGGED,
                    false,
                );
            }
            stats.skipped_reasons.insert(
                super::SKIP_FLAGGED.to_string(),
                folder_flagged.len() as u64,
            );
            stats.unprocessed = Some(unprocessed);
            RemoteCleanupState::update_folder(account.id, folder.name.clone(), stats)?;
            continue;
        }

        let mut stats = RemoteCleanupFolderStats::default();
        let mut skipped: BTreeMap<String, u64> = BTreeMap::new();
        let mut unprocessed: Vec<RemoteCleanupSkippedMessage> = Vec::new();
        let mut integrity_failures = 0u32;

        let remote_uid_validity =
            match ImapExecutor::select_mailbox(&mut session, &folder.encoded_name()).await {
                Ok(uid_validity) => uid_validity,
                Err(e) => {
                    stop = Some((
                        RemoteCleanupStatus::Failed,
                        format!("could not select '{}': {e}", folder.name),
                    ));
                    break 'folders;
                }
            };
        if remote_uid_validity != folder.uid_validity.unwrap_or(0) {
            stop = Some((
                RemoteCleanupStatus::Failed,
                format!("UIDVALIDITY changed for '{}'", folder.name),
            ));
            break 'folders;
        }

        for (index, candidate) in candidates.iter().enumerate() {
            // The batch size counts *moves*, not examined candidates: a
            // skipped message (gone from the source, failed check) must not
            // shrink the run. `candidate_count` therefore counts what this run
            // actually looked at.
            if total_moved as usize >= per_run {
                break;
            }
            stats.candidate_count += 1;
            // Persist a cumulative snapshot before every message so a polling
            // UI can show the run advancing; without it the active run only
            // learns new totals after the whole folder is done.
            //
            // Deliberately **without** the per-message list: this row is
            // rewritten once per message (up to 500 times a run), and folding
            // a growing list into every one of those writes turns a bounded
            // report into quadratic write amplification. The list is written
            // when the folder ends, which is when it can be complete.
            let mut snapshot = stats.clone();
            snapshot.skipped_reasons = skipped.clone();
            snapshot.unprocessed = None;
            if let Err(e) =
                RemoteCleanupState::update_folder(account.id, folder.name.clone(), snapshot)
            {
                warn!(
                    account_id = account.id,
                    folder = %folder.name,
                    "remote-cleanup: progress update failed: {e}"
                );
            }

            if request.cancel.is_cancelled() {
                stop = Some((
                    RemoteCleanupStatus::Cancelled,
                    "cancelled by the user".to_string(),
                ));
                break;
            }

            // Re-read the account so a legal hold takes effect mid-run.
            if let Ok(current) = AccountModel::get(account.id) {
                if current.is_on_hold() {
                    stop = Some((
                        RemoteCleanupStatus::Failed,
                        "legal hold was placed during the run".to_string(),
                    ));
                    break;
                }
            }

            match verify::verify_local(account.id, candidate) {
                Ok(()) => {
                    // The threshold counts *consecutive* failures: one intact
                    // message proves the local store is readable again.
                    integrity_failures = 0;
                }
                Err(VerifyIssue::LocalIntegrity(code)) => {
                    integrity_failures += 1;
                    stats.failed += 1;
                    total_failed += 1;
                    add_skip(&mut skipped, code);
                    add_unprocessed(&mut unprocessed, &mut report_budget, candidate, code, true);
                    if integrity_failures >= MAX_LOCAL_INTEGRITY_FAILURES {
                        stop = Some((
                            RemoteCleanupStatus::Failed,
                            format!(
                                "stopped after {integrity_failures} local archive integrity failures"
                            ),
                        ));
                        break;
                    }
                    continue;
                }
                Err(VerifyIssue::Skip(reason)) => {
                    stats.skipped += 1;
                    total_skipped += 1;
                    add_skip(&mut skipped, reason);
                    add_unprocessed(&mut unprocessed, &mut report_budget, candidate, reason, false);
                    continue;
                }
                Err(VerifyIssue::Fatal { code, detail }) => {
                    stats.failed += 1;
                    total_failed += 1;
                    add_skip(&mut skipped, code);
                    add_unprocessed(&mut unprocessed, &mut report_budget, candidate, code, true);
                    stop = Some((RemoteCleanupStatus::Failed, detail));
                    break;
                }
            }

            match verify::verify_remote(
                &mut session,
                candidate,
                config.skip_flagged,
                &request.cancel,
            )
            .await
            {
                Ok(verified) => {
                    stats.verified += 1;
                    match move_one(&mut session, folder, &verified).await {
                        Ok(()) => {
                            total_moved += 1;
                            total_bytes += candidate.size;
                            stats.moved += 1;
                            stats.bytes_moved += candidate.size;
                            moved_envelope_ids.push(candidate.envelope_id.clone());
                            debug!(
                                account_id = account.id,
                                run_id = %run_id,
                                folder = %folder.name,
                                source_uid = candidate.uid,
                                message_id = %candidate.message_id,
                                "remote-cleanup: message moved"
                            );
                            emit(Event::RemoteCleanupMessageMoved {
                                user: request.user.clone(),
                                account_id: account.id,
                                source_mailbox: folder.name.clone(),
                                source_uid: candidate.uid,
                                destination_folder: destination.clone(),
                                message_id: candidate.message_id.clone(),
                                size: candidate.size,
                            });
                            tokio::time::sleep(MESSAGE_DELAY).await;
                        }
                        Err(VerifyIssue::Fatal { code, detail }) => {
                            stats.failed += 1;
                            total_failed += 1;
                            add_unprocessed(&mut unprocessed, &mut report_budget, candidate, code, true);
                            stop = Some((RemoteCleanupStatus::Failed, detail));
                            break;
                        }
                        Err(VerifyIssue::Skip(reason)) => {
                            stats.skipped += 1;
                            total_skipped += 1;
                            add_skip(&mut skipped, reason);
                            add_unprocessed(&mut unprocessed, &mut report_budget, candidate, reason, false);
                        }
                        Err(VerifyIssue::LocalIntegrity(code)) => {
                            stats.failed += 1;
                            total_failed += 1;
                            add_unprocessed(&mut unprocessed, &mut report_budget, candidate, code, true);
                            stop = Some((
                                RemoteCleanupStatus::Failed,
                                format!("local archive not intact: {code}"),
                            ));
                            break;
                        }
                    }
                }
                Err(VerifyIssue::Skip(reason)) => {
                    stats.skipped += 1;
                    total_skipped += 1;
                    add_skip(&mut skipped, reason);
                    add_unprocessed(&mut unprocessed, &mut report_budget, candidate, reason, false);
                }
                Err(VerifyIssue::LocalIntegrity(code)) => {
                    integrity_failures += 1;
                    stats.failed += 1;
                    total_failed += 1;
                    add_skip(&mut skipped, code);
                    add_unprocessed(&mut unprocessed, &mut report_budget, candidate, code, true);
                    if integrity_failures >= MAX_LOCAL_INTEGRITY_FAILURES {
                        stop = Some((
                            RemoteCleanupStatus::Failed,
                            format!(
                                "stopped after {integrity_failures} local archive integrity failures"
                            ),
                        ));
                        break;
                    }
                }
                Err(VerifyIssue::Fatal { code, detail }) => {
                    stats.failed += 1;
                    total_failed += 1;
                    add_skip(&mut skipped, code);
                    add_unprocessed(&mut unprocessed, &mut report_budget, candidate, code, true);
                    stop = Some((RemoteCleanupStatus::Failed, detail));
                    break;
                }
            }

            if (index as u64 + 1) % super::BATCH_SIZE == 0 {
                tokio::time::sleep(super::BATCH_DELAY).await;
                match ImapExecutor::select_mailbox(&mut session, &folder.encoded_name()).await {
                    Ok(uid_validity) if uid_validity == folder.uid_validity.unwrap_or(0) => {}
                    Ok(_) => {
                        stop = Some((
                            RemoteCleanupStatus::Failed,
                            format!("UIDVALIDITY changed for '{}'", folder.name),
                        ));
                        break;
                    }
                    Err(e) => {
                        stop = Some((
                            RemoteCleanupStatus::Failed,
                            format!("could not re-select '{}': {e}", folder.name),
                        ));
                        break;
                    }
                }
            }
        }

        stats.skipped_reasons = skipped;
        stats.unprocessed = Some(unprocessed);
        RemoteCleanupState::update_folder(account.id, folder.name.clone(), stats)?;

        // A stop condition inside the candidate loop breaks out of *that* loop
        // only, so the folder's final report (including the message list) is
        // still written. The run itself stops here.
        if stop.is_some() {
            break 'folders;
        }
    }

    session.logout().await.ok();

    // Flag what this run moved so the next run does not offer it again. Done
    // after the loop, before the run is finalized: one index batch instead of
    // one full-document rewrite per message. A failure here is not fatal — the
    // move already happened and the run report records it — but it is logged,
    // because those messages will reappear as candidates until the flag lands.
    if !moved_envelope_ids.is_empty() {
        match ENVELOPE_MANAGER
            .mark_cleanup_moved(account.id, &moved_envelope_ids)
            .await
        {
            Ok(flagged) => info!(
                account_id = account.id,
                run_id = %run_id,
                flagged,
                "remote-cleanup: marked moved messages in the index"
            ),
            Err(e) => warn!(
                account_id = account.id,
                run_id = %run_id,
                moved = moved_envelope_ids.len(),
                "remote-cleanup: moved messages could not be marked in the index; \
                 they will reappear as candidates: {e}"
            ),
        }
    }

    let stopped = stop.is_some();
    let (status, message) = stop.unwrap_or((RemoteCleanupStatus::Success, String::new()));
    info!(
        account_id = account.id,
        run_id = %run_id,
        status = %status_name(&status),
        moved = total_moved,
        skipped = total_skipped,
        failed = total_failed,
        bytes = total_bytes,
        "remote-cleanup: run finished"
    );
    let mut message = if message.is_empty() {
        None
    } else {
        Some(message)
    };
    // A stopped run never reaches the rest of its batch. Report that, so a
    // "3 moved, 3 failed, 0 skipped" result cannot silently hide the 44
    // messages the batch size promised.
    if stopped {
        let flagged_total: u64 = flagged_skipped.values().sum();
        let total = (per_run as u64).min(kept.len() as u64);
        let processed = total_moved
            .saturating_add(total_failed)
            .saturating_add(total_skipped.saturating_sub(flagged_total));
        if total > processed {
            let note = format!(
                "stopped after {processed} of {total} planned message(s); {} not processed",
                total - processed
            );
            message = Some(match message {
                Some(existing) => format!("{existing}; {note}"),
                None => note,
            });
        }
    }
    if !flagged_skipped.is_empty() {
        let flagged_total: u64 = flagged_skipped.values().sum();
        let note = format!(
            "{flagged_total} flagged message(s) were left in the source mailbox"
        );
        message = Some(match message {
            Some(existing) => format!("{existing}; {note}"),
            None => note,
        });
    }
    if scan.scan_limited {
        let note = "scan budget reached; some skipped messages may hide newer eligible mail";
        message = Some(match message {
            Some(existing) => format!("{existing}; {note}"),
            None => note.to_string(),
        });
    }
    finish_run(
        account.id,
        &run_id,
        &request,
        status,
        total_moved,
        total_skipped,
        total_failed,
        total_bytes,
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::imap::mailbox::MailBox;
    use crate::imap::mock_server::{
        examine_response, uid_search_response, MockImapServer, MockImapServerHandle,
    };
    use crate::imap::session::SessionStream;
    use tokio_io_timeout::TimeoutStream;

    async fn mock_session(
        handle: &MockImapServerHandle,
    ) -> Session<Box<dyn SessionStream>> {
        let tcp = tokio::net::TcpStream::connect((handle.host(), handle.port()))
            .await
            .unwrap();
        let timeout_stream = TimeoutStream::new(tcp);
        let pinned: std::pin::Pin<Box<TimeoutStream<tokio::net::TcpStream>>> =
            Box::pin(timeout_stream);
        let stream: Box<dyn SessionStream> = Box::new(pinned);
        let mut client = async_imap::Client::new(stream);
        client.read_response().await.unwrap();
        let mut session = client
            .login("user", "pass")
            .await
            .map_err(|(e, _)| panic!("Login failed: {e:?}"))
            .unwrap();
        session.examine("INBOX").await.unwrap();
        session
    }

    fn folder() -> MailBox {
        MailBox {
            name: "INBOX".into(),
            uid_validity: Some(42),
            ..Default::default()
        }
    }

    fn verified(folder: &MailBox) -> VerifiedMove {
        VerifiedMove {
            folder: folder.name.clone(),
            folder_id: folder.id,
            uid: 5,
            uid_validity: 42,
            message_id: "<m@test.example>".into(),
            content_hash: "hash".into(),
            size: 10,
        }
    }

    #[tokio::test]
    async fn move_stops_when_the_source_uid_survives() {
        let handle = MockImapServer::new()
            .respond("LOGIN", "{TAG} OK LOGIN done\r\n")
            .respond("EXAMINE", examine_response("INBOX", 3, 42, 4))
            .respond("SELECT", examine_response("INBOX", 3, 42, 4))
            .respond("UID MOVE", "{TAG} OK MOVE done\r\n")
            .respond("UID SEARCH UID", uid_search_response(&[5]))
            .respond("LOGOUT", "* BYE\r\n{TAG} OK\r\n")
            .start()
            .await;
        let mut session = mock_session(&handle).await;
        let folder = folder();
        let result = move_one(&mut session, &folder, &verified(&folder)).await;
        session.logout().await.ok();

        match result {
            Err(VerifyIssue::Fatal { code, detail }) => {
                assert_eq!(code, "source_uid_survives_move");
                assert!(detail.contains("still present"), "unexpected detail: {detail}")
            }
            other => panic!("a surviving source UID must stop the move: {other:?}"),
        }
    }

    #[tokio::test]
    async fn move_does_not_search_the_destination() {
        // MOVE is atomic (RFC 6851): once the server answers OK the message is
        // in the destination. The mock only answers the source post-check, so
        // any destination search would block and fail the run.
        let handle = MockImapServer::new()
            .respond("LOGIN", "{TAG} OK LOGIN done\r\n")
            .respond("EXAMINE", examine_response("INBOX", 3, 42, 4))
            .respond("SELECT", examine_response("INBOX", 3, 42, 4))
            .respond("UID MOVE", "{TAG} OK MOVE done\r\n")
            .respond("UID SEARCH UID", uid_search_response(&[]))
            .respond("LOGOUT", "* BYE\r\n{TAG} OK\r\n")
            .start()
            .await;
        let mut session = mock_session(&handle).await;
        let folder = folder();
        let result = move_one(&mut session, &folder, &verified(&folder)).await;
        session.logout().await.ok();

        assert!(
            result.is_ok(),
            "the MOVE path must not query the destination: {result:?}"
        );
    }

    /// The move path is MOVE-only: a session that is never told MOVE succeeded
    /// must fail without ever issuing COPY, STORE or EXPUNGE. The mock answers
    /// nothing for those commands, so a fallback would hang or fail here.
    #[tokio::test]
    async fn no_copy_or_expunge_fallback_exists() {
        let handle = MockImapServer::new()
            .respond("LOGIN", "{TAG} OK LOGIN done\r\n")
            .respond("EXAMINE", examine_response("INBOX", 3, 42, 4))
            .respond("SELECT", examine_response("INBOX", 3, 42, 4))
            .respond("UID MOVE", "{TAG} NO MOVE unsupported\r\n")
            .respond("LOGOUT", "* BYE\r\n{TAG} OK\r\n")
            .start()
            .await;
        let mut session = mock_session(&handle).await;
        let folder = folder();
        let result = move_one(&mut session, &folder, &verified(&folder)).await;
        session.logout().await.ok();

        match result {
            Err(VerifyIssue::Fatal { code, .. }) => assert_eq!(code, "move_failed"),
            other => panic!("a failed MOVE must stop the move, not fall back: {other:?}"),
        }
    }

    fn candidate(uid: u32, subject: &str) -> plan::Candidate {
        plan::Candidate {
            mailbox: MailBox {
                name: "INBOX".into(),
                ..Default::default()
            },
            envelope_id: format!("eid-{uid}"),
            message_id: format!("<{uid}@test.example>"),
            subject: subject.into(),
            content_hash: "hash".into(),
            uid,
            size: 10,
            date: 1,
            internal_date: 1,
            ingest_at: 1,
        }
    }

    #[test]
    fn a_policy_skip_is_not_listed() {
        let mut list = Vec::new();
        let mut budget = ReportBudget::new();
        add_unprocessed(&mut list, &mut budget, &candidate(7, "Quarterly report"), "flagged", false);
        assert!(
            list.is_empty(),
            "routine skips are counted, not named: the UI does not show them"
        );
    }

    #[test]
    fn a_failure_is_named_with_its_subject_and_reason() {
        let mut list = Vec::new();
        let mut budget = ReportBudget::new();
        add_unprocessed(&mut list, &mut budget, &candidate(9, "Invoice"), "local_hash_mismatch", true);
        assert_eq!(list.len(), 1);
        assert!(list[0].failed);
        assert_eq!(list[0].subject.as_deref(), Some("Invoice"));
        assert_eq!(list[0].reason, "local_hash_mismatch");
        assert_eq!(list[0].uid, 9);
        assert_eq!(list[0].folder, "INBOX");
    }

    #[test]
    fn an_empty_subject_becomes_none_rather_than_an_empty_string() {
        let mut list = Vec::new();
        let mut budget = ReportBudget::new();
        add_unprocessed(&mut list, &mut budget, &candidate(3, ""), "local_blob_empty", true);
        assert_eq!(
            list[0].subject, None,
            "the UI shows its own '(no subject)' instead of a blank"
        );
    }

    #[test]
    fn the_message_list_is_capped_while_counters_stay_exact() {
        let mut list = Vec::new();
        let mut budget = ReportBudget::new();
        for uid in 1..=(REMOTE_CLEANUP_REPORTED_MESSAGES as u32 + 25) {
            add_unprocessed(
                &mut list,
                &mut budget,
                &candidate(uid, "s"),
                "local_hash_mismatch",
                true,
            );
        }
        assert_eq!(list.len(), REMOTE_CLEANUP_REPORTED_MESSAGES);
    }

    /// A `Subject:` header is attacker-controlled and unbounded. The report
    /// keeps a recognisable prefix instead of copying the whole header into a
    /// row that is rewritten on every message.
    #[test]
    fn an_absurd_subject_is_truncated_in_the_report() {
        let huge = "A".repeat(50_000);
        let mut list = Vec::new();
        let mut budget = ReportBudget::new();
        add_unprocessed(&mut list, &mut budget, &candidate(1, &huge), "local_hash_mismatch", true);

        let stored = list[0].subject.as_deref().unwrap();
        assert!(
            stored.chars().count() <= REMOTE_CLEANUP_SUBJECT_CHARS + 1,
            "stored subject must be bounded, got {} chars",
            stored.chars().count()
        );
        assert!(stored.ends_with('…'), "truncation must be visible");
    }

    /// The budget is per run, not per folder: capping per folder would let the
    /// number of source folders multiply the stored report.
    #[test]
    fn one_budget_bounds_the_list_across_folders() {
        let mut list = Vec::new();
        let mut budget = ReportBudget::new();
        // Two folders x the whole cap: the second folder contributes nothing.
        for folder in ["INBOX", "Archive"] {
            for uid in 1..=REMOTE_CLEANUP_REPORTED_MESSAGES as u32 {
                let mut c = candidate(uid, "s");
                c.mailbox.name = folder.into();
                add_unprocessed(&mut list, &mut budget, &c, "local_hash_mismatch", true);
            }
        }
        assert_eq!(list.len(), REMOTE_CLEANUP_REPORTED_MESSAGES);
    }
}
