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
    account::{
        migration::{AccountModel, AccountType},
        state::{
            DownloadState, DownloadStatus, GapFillFolderStats, GapFillState, GapFillStatus,
            TriggerType,
        },
    },
    archive::imap::{download::flow::FetchDirection, mailbox::MailBox},
    error::BichonResult,
    imap::executor::ImapExecutor,
};
use download_folders::get_download_folders;
use download_type::{decide_next_download_task, DownloadTask};
use flow::reconcile_mailboxes;
use rebuild::{rebuild_cache, rebuild_cache_by_date};
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

pub mod download_folders;
pub mod download_type;
pub mod gap_fill;
pub mod flow;
pub mod rebuild;

pub async fn process_imap_download(
    account: &AccountModel,
    token: CancellationToken,
    trigger_type: TriggerType,
    run_gap_fill: bool,
) -> BichonResult<()> {
    assert_eq!(account.account_type, AccountType::IMAP);
    let start_time = Instant::now();
    let account_id = account.id;
    let download_task = decide_next_download_task(account, trigger_type).await?;
    if matches!(download_task, DownloadTask::Idle) {
        return Ok(());
    }
    let mut session = match ImapExecutor::create_connection(account_id).await {
        Ok(session) => session,
        Err(e) => {
            let err_msg = format!("Failed to connect to IMAP server: {:#?}", e);
            DownloadState::append_session_error(account_id, err_msg.clone())?;
            DownloadState::update_session_status(
                account_id,
                DownloadStatus::Failed,
                Some(err_msg),
            )?;
            return Err(e);
        }
    };
    let remote_mailboxes = match get_download_folders(account, &mut session).await {
        Ok(mailboxes) => mailboxes,
        Err(err) => {
            let err_msg = format!("Failed to fetch mailboxes: {:#?}", err);
            warn!(account_id = account.id, error = %err, "{}", err_msg);
            DownloadState::append_session_error(account_id, err_msg.clone())?;
            DownloadState::update_session_status(
                account_id,
                DownloadStatus::Failed,
                Some(err_msg),
            )?;
            return Ok(());
        }
    };
    session.logout().await.ok();
    if matches!(download_task, DownloadTask::FullFetch) {
        let result = match &account.date_since {
            Some(date_since) => {
                rebuild_cache_by_date(
                    account,
                    &remote_mailboxes,
                    &date_since.since_date()?,
                    FetchDirection::Since,
                    token,
                )
                .await
            }
            None => match &account.date_before {
                Some(r) => {
                    rebuild_cache_by_date(
                        account,
                        &remote_mailboxes,
                        &r.calculate_date()?,
                        FetchDirection::Before,
                        token,
                    )
                    .await
                }
                None => rebuild_cache(account, &remote_mailboxes, token).await,
            },
        };
        match result {
            Ok(_) => {
                DownloadState::update_session_status(account_id, DownloadStatus::Success, None)?;
            }
            Err(e) => {
                let err_msg = format!("Email Download interrupted: {:#?}", e);
                DownloadState::append_session_error(account_id, err_msg.clone())?;
                DownloadState::update_session_status(
                    account_id,
                    DownloadStatus::Failed,
                    Some(err_msg),
                )?;
            }
        }
        return Ok(());
    }

    let local_mailboxes = MailBox::list_all(account_id)?;
    let reconcile_result =
        reconcile_mailboxes(account, &remote_mailboxes, &local_mailboxes, token.clone()).await;

    // Gap-fill phase: only on explicit user request (manual download with
    // "run gap-fill" checked). Enumerate every UID in the download folders and
    // download anything missing locally. Not run on scheduled syncs. Gap-fill
    // runs are tracked in their own state (independent of the download session)
    // because they are repeatable until `failed == 0`.
    if run_gap_fill {
        GapFillState::start_run(account_id)?;
        // Run inside a helper so a failure anywhere still finalizes the run:
        // an abandoned active run would otherwise show as Running forever.
        let run_outcome = gap_fill_phase(
            account,
            &local_mailboxes,
            &remote_mailboxes,
            token,
            account_id,
        )
        .await;
        let (cancelled, total_downloaded, total_failed) = match run_outcome {
            Ok(v) => v,
            Err(e) => {
                warn!(account_id = account_id, "Gap-fill phase error: {:#?}", e);
                (false, 0, 1)
            }
        };
        let status = if cancelled {
            GapFillStatus::Cancelled
        } else if total_failed > 0 {
            GapFillStatus::Failed
        } else {
            GapFillStatus::Success
        };
        GapFillState::finish_run(account_id, status, total_downloaded, total_failed)?;
        let summary = if cancelled {
            format!(
                "Gap-fill cancelled: {} downloaded, {} failed",
                total_downloaded, total_failed
            )
        } else {
            format!(
                "Gap-fill finished: {} downloaded, {} failed",
                total_downloaded, total_failed
            )
        };
        DownloadState::update_session_message(account_id, summary.clone())?;
        info!(account_id = account_id, "{}", summary);
    }

    // Finalize session status AFTER all phases so stats/progress written
    // during gap-fill are not dropped (update_session_status closes the active
    // session, moving it into history).
    match reconcile_result {
        Ok(_) => DownloadState::update_session_status(account_id, DownloadStatus::Success, None)?,
        Err(e) => {
            let err_msg = format!("Email Download interrupted: {:#?}", e);
            DownloadState::append_session_error(account_id, err_msg.clone())?;
            DownloadState::update_session_status(
                account_id,
                DownloadStatus::Failed,
                Some(err_msg),
            )?;
        }
    }

    let elapsed_time = start_time.elapsed().as_secs();
    debug!(
        "Account{{{}}} Incremental sync completed: {} seconds elapsed.",
        account.email, elapsed_time
    );
    Ok(())
}

/// Runs the gap-fill phase across all download-folder mailboxes. Errors inside
/// are converted into a failed-run outcome instead of propagating, so the
/// caller can always finalize the active run.
async fn gap_fill_phase(
    account: &AccountModel,
    local_mailboxes: &[MailBox],
    remote_mailboxes: &[MailBox],
    token: CancellationToken,
    account_id: u64,
) -> BichonResult<(bool, u64, u64)> {
    let mut total_downloaded = 0u64;
    let mut total_failed = 0u64;
    let mut cancelled = false;
    for local_mailbox in local_mailboxes {
        let Some(remote) = remote_mailboxes.iter().find(|r| r.name == local_mailbox.name) else {
            continue;
        };
        if token.is_cancelled() {
            cancelled = true;
            break;
        }
        DownloadState::set_current_folder(account_id, local_mailbox.name.clone())?;
        match gap_fill::gap_fill_mailbox(account, local_mailbox, remote, token.clone()).await {
            Ok(stats) => {
                total_downloaded += stats.downloaded;
                total_failed += stats.failed;
                GapFillState::add_folder_result(account_id, local_mailbox.name.clone(), stats)?;
            }
            Err(e) => {
                let err_msg = format!(
                    "Gap-fill failed for mailbox '{}': {:#?}",
                    local_mailbox.name, e
                );
                warn!(account_id = account_id, "{}", err_msg);
                DownloadState::append_session_error(account_id, err_msg)?;
                total_failed += 1; // count the mailbox as a failed unit
                GapFillState::add_folder_result(
                    account_id,
                    local_mailbox.name.clone(),
                    GapFillFolderStats {
                        downloaded: 0,
                        failed: 1,
                        candidate_count: 0,
                        message: None,
                    },
                )?;
            }
        }
    }
    Ok((cancelled, total_downloaded, total_failed))
}
