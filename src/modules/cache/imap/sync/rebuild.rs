//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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
        account::migration::AccountModel,
        cache::{
            imap::{
                mailbox::MailBox,
                sync::flow::{fetch_and_save_by_date, fetch_and_save_full_mailbox, FetchDirection},
            },
            SEMAPHORE,
        },
        error::{code::ErrorCode, BichonError, BichonResult},
        indexer::manager::{EML_INDEX_MANAGER, ENVELOPE_INDEX_MANAGER},
    },
    raise_error,
};
use std::{sync::Arc, time::Instant};
use tokio::sync::Semaphore;
use tracing::{error, info};

pub const DEFAULT_MAX_CONCURRENT_PER_ACCOUNT: usize = 5;

pub async fn rebuild_cache(
    account: &AccountModel,
    remote_mailboxes: &[MailBox],
) -> BichonResult<()> {
    let start_time = Instant::now();
    let mut total_inserted = 0;
    MailBox::batch_insert(remote_mailboxes).await?;

    let local_semaphore = Arc::new(Semaphore::new(DEFAULT_MAX_CONCURRENT_PER_ACCOUNT));

    let mut handles = Vec::new();
    for mailbox in remote_mailboxes {
        if mailbox.exists == 0 {
            info!(
                "Account {}: Mailbox '{}' on the remote server has no emails. Skipping fetch for this mailbox.",
                account.id, &mailbox.name
            );
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

        let handle: tokio::task::JoinHandle<Result<usize, BichonError>> =
            tokio::spawn(async move {
                let _global_permit = global_permit;
                let _local_permit = local_permit;

                fetch_and_save_full_mailbox(&account, &mailbox, mailbox.exists).await
            });
        handles.push(handle);
    }

    for task in handles {
        match task.await {
            Ok(Ok(count)) => {
                total_inserted += count;
            }
            Ok(Err(err)) => return Err(err),
            Err(e) => return Err(raise_error!(format!("{:#?}", e), ErrorCode::InternalError)),
        }
    }
    let elapsed_time = start_time.elapsed().as_secs();
    info!(
        "Rebuild account cache completed: {} envelopes inserted. {} secs elapsed. \
        This is a full data fetch as there was no local cache data available.",
        total_inserted, elapsed_time
    );
    Ok(())
}

pub async fn rebuild_cache_by_date(
    account: &AccountModel,
    remote_mailboxes: &[MailBox],
    date: &str,
    direction: FetchDirection,
) -> BichonResult<()> {
    let start_time = Instant::now();
    let mut total_inserted = 0;
    MailBox::batch_insert(remote_mailboxes).await?;

    let mut handles = Vec::new();

    let local_semaphore = Arc::new(Semaphore::new(DEFAULT_MAX_CONCURRENT_PER_ACCOUNT));

    for mailbox in remote_mailboxes {
        if mailbox.exists == 0 {
            info!(
                "Account {}: Mailbox '{}' on the remote server has no emails. Skipping fetch for this mailbox.",
                account.id, &mailbox.name
            );
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

        let handle: tokio::task::JoinHandle<Result<usize, BichonError>> =
            tokio::spawn(async move {
                let _global_permit = global_permit;
                let _local_permit = local_permit;

                fetch_and_save_by_date(&account, date.as_str(), &mailbox, direction).await
            });
        handles.push(handle);
    }
    for task in handles {
        match task.await {
            Ok(Ok(count)) => {
                total_inserted += count;
            }
            Ok(Err(err)) => return Err(err),
            Err(e) => return Err(raise_error!(format!("{:#?}", e), ErrorCode::InternalError)),
        }
    }
    let elapsed_time = start_time.elapsed().as_secs();
    let direction_desc = match direction {
        FetchDirection::Since => "starting from the specified date",
        FetchDirection::Before => "ending before the specified date",
    };
    info!(
        "Rebuild account cache completed: {} envelopes inserted. {} secs elapsed. \
     Data fetched from server {}: {}.",
        total_inserted, elapsed_time, direction_desc, date
    );
    Ok(())
}

pub async fn rebuild_mailbox_cache(
    account: &AccountModel,
    local_mailbox: &MailBox,
    remote_mailbox: &MailBox,
) -> BichonResult<()> {
    ENVELOPE_INDEX_MANAGER
        .delete_mailbox_envelopes(account.id, vec![local_mailbox.id])
        .await?;
    EML_INDEX_MANAGER
        .delete_mailbox_envelopes(account.id, vec![local_mailbox.id])
        .await?;
    if remote_mailbox.exists == 0 {
        info!(
            "Account {}: Mailbox '{}' has no emails on the remote server. The mailbox is empty, no envelopes to fetch.",
            account.id,
            &local_mailbox.name
        );
        return Ok(()); // Skip if the mailbox has no emails
    }

    let inserted_count =
        fetch_and_save_full_mailbox(account, remote_mailbox, remote_mailbox.exists).await?;
    info!(
        "Account {}: Successfully rebuild mailbox cache, inserted {} envelopes for mailbox '{}'.",
        account.id, inserted_count, &local_mailbox.name
    );
    Ok(())
}

pub async fn rebuild_mailbox_cache_by_date(
    account: &AccountModel,
    local_mailbox_id: u64,
    date: &str,
    remote: &MailBox,
    direction: FetchDirection,
) -> BichonResult<()> {
    ENVELOPE_INDEX_MANAGER
        .delete_mailbox_envelopes(account.id, vec![local_mailbox_id])
        .await?;
    EML_INDEX_MANAGER
        .delete_mailbox_envelopes(account.id, vec![local_mailbox_id])
        .await?;
    if remote.exists == 0 {
        info!(
            "Account {}: Mailbox '{}' has no emails on the remote server. The mailbox is empty, no envelopes to fetch.",
            account.id,
            &remote.name
        );
        return Ok(()); // Skip if the mailbox has no emails
    }

    let count = fetch_and_save_by_date(account, date, remote, direction).await?;
    info!(
        "Account {}: Successfully rebuild mailbox cache, inserted {} envelopes for mailbox '{}'.",
        account.id, count, &remote.name
    );
    Ok(())
}
