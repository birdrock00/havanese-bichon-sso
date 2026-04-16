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
    modules::{
        account::{
            migration::AccountModel,
            state::{DownloadState, DownloadStatus, FolderStatus},
        },
        cache::{
            imap::{
                mailbox::MailBox,
                download::flow::{fetch_and_save_by_date, fetch_and_save_full_mailbox, FetchDirection},
            },
            SEMAPHORE,
        },
        error::{code::ErrorCode, BichonError, BichonResult},
        store::tantivy::manager::INDEX_MANAGER,
    },
    raise_error,
};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub const DEFAULT_MAX_CONCURRENT_PER_ACCOUNT: usize = 3;

pub async fn rebuild_cache(
    account: &AccountModel,
    remote_mailboxes: &[MailBox],
    token: CancellationToken,
) -> BichonResult<()> {
    MailBox::batch_insert(remote_mailboxes).await?;
    DownloadState::init_folder_details(
        account.id,
        remote_mailboxes.iter().map(|m| m.name.clone()).collect(),
    )
    .await?;

    let local_semaphore = Arc::new(Semaphore::new(DEFAULT_MAX_CONCURRENT_PER_ACCOUNT));

    let mut handles = Vec::new();
    for mailbox in remote_mailboxes {
        if token.is_cancelled() {
            DownloadState::update_session_status(
                account.id,
                DownloadStatus::Cancelled,
                Some("Received termination signal (User stop or System shutdown)".to_string()),
            )
            .await?;
            break;
        }
        if mailbox.exists == 0 {
            info!(
                "Account {}: Mailbox '{}' on the remote server has no emails. Skipping fetch for this mailbox.",
                account.id, &mailbox.name
            );
            DownloadState::update_folder_progress(
                account.id,
                mailbox.name.clone(),
                0,
                0,
                FolderStatus::Success,
                None,
            )
            .await?;
            continue;
        }
        let account = account.clone();
        let mailbox = mailbox.clone();

        let global_permit = match SEMAPHORE.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(err) => {
                error!(
                    "Failed to acquire global semaphore permit for account {} mailbox '{}': {:#?}",
                    account.id, &mailbox.name, err
                );
                continue;
            }
        };

        let local_permit = match local_semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(err) => {
                error!(
                    "Failed to acquire local semaphore permit for account {} mailbox '{}': {:#?}",
                    account.id, &mailbox.name, err
                );
                drop(global_permit);
                continue;
            }
        };
        let token_clone = token.clone();
        let handle: tokio::task::JoinHandle<Result<(), BichonError>> = tokio::spawn(async move {
            let _global_permit = global_permit;
            let _local_permit = local_permit;
            fetch_and_save_full_mailbox(&account, &mailbox, token_clone).await
        });
        handles.push(handle);
    }

    let mut has_error = false;
    let mut last_err = None;

    for task in handles {
        match task.await {
            Ok(Ok(_)) => {}
            Ok(Err(err)) => {
                has_error = true;
                tracing::error!("Folder sync task failed: {:#?}", err);
                last_err = Some(err);
            }
            Err(e) => {
                has_error = true;
                tracing::error!("Task panicked or runtime error: {:#?}", e);
            }
        }
    }
    if has_error {
        if let Some(e) = last_err {
            return Err(e);
        }
        return Err(raise_error!(
            "Some tasks failed".into(),
            ErrorCode::InternalError
        ));
    }
    Ok(())
}

pub async fn rebuild_cache_by_date(
    account: &AccountModel,
    remote_mailboxes: &[MailBox],
    date: &str,
    direction: FetchDirection,
    token: CancellationToken,
) -> BichonResult<()> {
    MailBox::batch_insert(remote_mailboxes).await?;
    DownloadState::init_folder_details(
        account.id,
        remote_mailboxes.iter().map(|m| m.name.clone()).collect(),
    )
    .await?;

    let mut handles = Vec::new();
    let local_semaphore = Arc::new(Semaphore::new(DEFAULT_MAX_CONCURRENT_PER_ACCOUNT));

    for mailbox in remote_mailboxes {
        if token.is_cancelled() {
            DownloadState::update_session_status(
                account.id,
                DownloadStatus::Cancelled,
                Some("Received termination signal (User stop or System shutdown)".to_string()),
            )
            .await?;
            break;
        }
        if mailbox.exists == 0 {
            info!(
                "Account {}: Mailbox '{}' on the remote server has no emails. Skipping fetch for this mailbox.",
                account.id, &mailbox.name
            );

            DownloadState::update_folder_progress(
                account.id,
                mailbox.name.clone(),
                0,
                0,
                FolderStatus::Success,
                None,
            )
            .await?;
            continue;
        }
        let account = account.clone();
        let mailbox = mailbox.clone();
        let date = date.to_string();
        let direction = direction.clone();

        let global_permit = match SEMAPHORE.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(err) => {
                error!(
                    "Failed to acquire global semaphore permit for account {} mailbox '{}': {:#?}",
                    account.id, &mailbox.name, err
                );
                continue;
            }
        };

        let local_permit = match local_semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(err) => {
                error!(
                    "Failed to acquire local semaphore permit for account {} mailbox '{}': {:#?}",
                    account.id, &mailbox.name, err
                );
                drop(global_permit);
                continue;
            }
        };
        let token_clone = token.clone();
        let handle: tokio::task::JoinHandle<Result<(), BichonError>> =
            tokio::spawn(async move {
                let _global_permit = global_permit;
                let _local_permit = local_permit;
                fetch_and_save_by_date(&account, date.as_str(), &mailbox, direction, token_clone)
                    .await
            });
        handles.push(handle);
    }

    let mut has_error = false;
    let mut last_err = None;

    for task in handles {
        match task.await {
            Ok(Ok(_)) => {}
            Ok(Err(err)) => {
                has_error = true;
                tracing::error!("Folder sync task failed: {:#?}", err);
                last_err = Some(err);
            }
            Err(e) => {
                has_error = true;
                tracing::error!("Task panicked or runtime error: {:#?}", e);
            }
        }
    }
    if has_error {
        if let Some(e) = last_err {
            return Err(e);
        }
        return Err(raise_error!(
            "Some tasks failed".into(),
            ErrorCode::InternalError
        ));
    }

    Ok(())
}

pub async fn rebuild_mailbox_cache(
    account: &AccountModel,
    local_mailbox: &MailBox,
    remote_mailbox: &MailBox,
    token: CancellationToken,
) -> BichonResult<()> {
    INDEX_MANAGER
        .delete_mailbox_envelopes(account.id, vec![local_mailbox.id])
        .await?;

    if remote_mailbox.exists == 0 {
        info!(
            "Account {}: Mailbox '{}' has no emails on the remote server. The mailbox is empty, no envelopes to fetch.",
            account.id,
            &local_mailbox.name
        );
        DownloadState::update_folder_progress(
            account.id,
            remote_mailbox.name.clone(),
            0,
            0,
            FolderStatus::Success,
            None,
        )
        .await?;
        return Ok(());
    }

    fetch_and_save_full_mailbox(account, remote_mailbox, token).await?;
    Ok(())
}

pub async fn rebuild_mailbox_cache_by_date(
    account: &AccountModel,
    local_mailbox_id: u64,
    date: &str,
    remote: &MailBox,
    direction: FetchDirection,
    token: CancellationToken,
) -> BichonResult<()> {
    INDEX_MANAGER
        .delete_mailbox_envelopes(account.id, vec![local_mailbox_id])
        .await?;
    if remote.exists == 0 {
        info!(
            "Account {}: Mailbox '{}' has no emails on the remote server. The mailbox is empty, no envelopes to fetch.",
            account.id,
            &remote.name
        );
        DownloadState::update_folder_progress(
            account.id,
            remote.name.clone(),
            0,
            0,
            FolderStatus::Success,
            None,
        )
        .await?;
        return Ok(());
    }

    fetch_and_save_by_date(account, date, remote, direction, token).await?;
    Ok(())
}
