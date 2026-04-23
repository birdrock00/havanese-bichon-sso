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
    {
        account::{
            migration::AccountModel,
            state::{DownloadState, DownloadStatus, FolderStatus},
        },
        cache::{
            imap::{
                download::rebuild::{rebuild_mailbox_cache, rebuild_mailbox_cache_by_date},
                find_intersecting_mailboxes, find_missing_mailboxes,
                mailbox::MailBox,
            },
            SEMAPHORE,
        },
        error::{code::ErrorCode, BichonResult},
        imap::executor::ImapExecutor,
        store::tantivy::envelope::ENVELOPE_MANAGER,
    },
    raise_error,
};
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

pub const DEFAULT_BATCH_SIZE: u32 = 30;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FetchDirection {
    Since,
    Before,
}

pub async fn fetch_and_save_by_date(
    account: &AccountModel,
    date: &str,
    mailbox: &MailBox,
    direction: FetchDirection,
    token: CancellationToken,
) -> BichonResult<()> {
    let account_id = account.id;
    let mut session = match ImapExecutor::create_connection(account_id).await {
        Ok(session) => session,
        Err(e) => {
            let err_msg = format!("Connection failed for this folder: {:#?}", e);
            DownloadState::update_folder_progress(
                account_id,
                mailbox.name.clone(),
                0,
                0,
                FolderStatus::Failed,
                Some(err_msg.clone()),
            )
            .await?;
            DownloadState::append_session_error(account_id, err_msg).await?;
            return Err(e);
        }
    };

    let search_criteria = match direction {
        FetchDirection::Since => format!("SINCE {date}"),
        FetchDirection::Before => format!("BEFORE {date}"),
    };

    let uid_list =
        match ImapExecutor::uid_search(&mut session, &mailbox.encoded_name(), &search_criteria)
            .await
        {
            Ok(uid_list) => uid_list,
            Err(e) => {
                let err_msg = format!("UID search failed in [{}]: {:#?}", mailbox.name, e);
                DownloadState::update_folder_progress(
                    account_id,
                    mailbox.name.clone(),
                    0,
                    0,
                    FolderStatus::Failed,
                    Some(err_msg.clone()),
                )
                .await?;
                DownloadState::append_session_error(account_id, err_msg).await?;
                return Err(e);
            }
        };

    let len = uid_list.len();
    if len == 0 {
        DownloadState::update_folder_progress(
            account_id,
            mailbox.name.clone(),
            0,
            0,
            FolderStatus::Success,
            None,
        )
        .await?;
        return Ok(());
    }

    let folder_limit = account.folder_limit;
    // sort small -> bigger
    let mut uid_vec: Vec<u32> = uid_list.into_iter().collect();
    uid_vec.sort();

    if let Some(limit) = folder_limit {
        let limit = limit.max(100) as usize;
        if len > limit {
            uid_vec = match direction {
                FetchDirection::Since => uid_vec.split_off(len - limit),
                FetchDirection::Before => {
                    uid_vec.truncate(limit);
                    uid_vec
                }
            };
        }
    }

    let planned = uid_vec.len() as u64;
    let uid_batches = generate_uid_sequence_hashset(
        uid_vec,
        account.download_batch_size.unwrap_or(DEFAULT_BATCH_SIZE) as usize,
        false,
    );
    DownloadState::update_folder_progress(
        account_id,
        mailbox.name.clone(),
        planned,
        0,
        FolderStatus::Pending,
        None,
    )
    .await?;

    let mut current_processed = 0u64;
    let mut has_error_or_cancel = false;
    for (index, batch) in uid_batches.into_iter().enumerate() {
        if token.is_cancelled() {
            DownloadState::update_session_status(
                account_id,
                DownloadStatus::Cancelled,
                Some("User stopped or system shutdown".to_string()),
            )
            .await?;
            DownloadState::update_folder_progress(
                account_id,
                mailbox.name.clone(),
                planned,
                current_processed,
                FolderStatus::Cancelled,
                None,
            )
            .await?;
            has_error_or_cancel = true;
            break;
        }
        // Fetch metadata for the current batch of UIDs
        match ImapExecutor::uid_batch_retrieve_emails(
            &mut session,
            account_id,
            mailbox.id,
            &batch.0,
            token.clone(),
        )
        .await
        {
            Ok(_) => {
                current_processed += batch.1;
                DownloadState::update_folder_progress(
                    account_id,
                    mailbox.name.clone(),
                    planned,
                    current_processed,
                    FolderStatus::Downloading,
                    None,
                )
                .await?;
            }
            Err(e) => {
                let err_msg = format!("Batch {} failed: {:#?}", index, e);
                DownloadState::append_session_error(account_id, err_msg.clone()).await?;
                DownloadState::update_folder_progress(
                    account_id,
                    mailbox.name.clone(),
                    planned,
                    current_processed,
                    FolderStatus::Failed,
                    Some(err_msg),
                )
                .await?;
                has_error_or_cancel = true;
                break;
            }
        }
    }
    if !has_error_or_cancel {
        DownloadState::update_folder_progress(
            account_id,
            mailbox.name.clone(),
            planned,
            current_processed,
            FolderStatus::Success,
            None,
        )
        .await?;
    }
    session.logout().await.ok();
    Ok(())
}

pub async fn fetch_and_save_full_mailbox(
    account: &AccountModel,
    mailbox: &MailBox,
    token: CancellationToken,
) -> BichonResult<()> {
    let mailbox_id = mailbox.id;
    let account_id = account.id;

    let mut session = match ImapExecutor::create_connection(account_id).await {
        Ok(session) => session,
        Err(e) => {
            let err_msg = format!("Connection failed for this folder: {:#?}", e);
            DownloadState::update_folder_progress(
                account_id,
                mailbox.name.clone(),
                0,
                0,
                FolderStatus::Failed,
                Some(err_msg.clone()),
            )
            .await?;
            DownloadState::append_session_error(account_id, err_msg).await?;
            return Err(e);
        }
    };

    let total = match session.examine(&mailbox.encoded_name()).await {
        Ok(mailbox) => mailbox.exists as u64,
        Err(e) => {
            let err_msg = format!("Failed to examine folder [{}]: {:#?}", mailbox.name, e);
            DownloadState::update_folder_progress(
                account_id,
                mailbox.name.clone(),
                mailbox.exists as u64,
                0,
                FolderStatus::Failed,
                Some(err_msg.clone()),
            )
            .await?;

            DownloadState::append_session_error(account_id, err_msg).await?;
            session.logout().await.ok();
            return Err(raise_error!(
                format!("{:#?}", e),
                ErrorCode::ImapCommandFailed
            ));
        }
    };

    let folder_limit = account.folder_limit;
    let total_to_fetch = match folder_limit {
        Some(limit) if (limit as u64) < total => {
            let limit64 = limit as u64;
            total.min(limit64.max(100))
        }
        _ => total,
    };

    let page_size = if let Some(limit) = folder_limit {
        limit
            .max(100)
            .min(account.download_batch_size.unwrap_or(DEFAULT_BATCH_SIZE))
    } else {
        account.download_batch_size.unwrap_or(DEFAULT_BATCH_SIZE)
    };

    let total_batches = total_to_fetch.div_ceil(page_size as u64);
    let desc = folder_limit.is_some();

    info!(
        "Starting full mailbox download for '{}', total={}, limit={:?}, batches={}, desc={}",
        mailbox.name, total, folder_limit, total_batches, desc
    );

    let mut current_processed = 0u64;
    let mut has_error_or_cancel = false;

    for page in 1..=total_batches {
        if token.is_cancelled() {
            DownloadState::update_session_status(
                account_id,
                DownloadStatus::Cancelled,
                Some("User stopped or system shutdown".to_string()),
            )
            .await?;
            DownloadState::update_folder_progress(
                account_id,
                mailbox.name.clone(),
                total_to_fetch,
                current_processed,
                FolderStatus::Cancelled,
                None,
            )
            .await?;
            has_error_or_cancel = true;
            break;
        }

        match ImapExecutor::batch_retrieve_emails(
            &mut session,
            account_id,
            mailbox_id,
            total_to_fetch,
            page as u64,
            page_size as u64,
            &mailbox.encoded_name(),
            desc,
            token.clone(),
        )
        .await
        {
            Ok(count) => {
                current_processed += count as u64;
                DownloadState::update_folder_progress(
                    account_id,
                    mailbox.name.clone(),
                    total_to_fetch,
                    current_processed,
                    FolderStatus::Downloading,
                    None,
                )
                .await?;
            }
            Err(e) => {
                let err_msg = format!("Batch {} failed: {:#?}", page, e);
                DownloadState::append_session_error(account_id, err_msg.clone()).await?;
                DownloadState::update_folder_progress(
                    account_id,
                    mailbox.name.clone(),
                    total_to_fetch,
                    current_processed,
                    FolderStatus::Failed,
                    Some(err_msg),
                )
                .await?;
                has_error_or_cancel = true;
                break;
            }
        };
    }

    if !has_error_or_cancel {
        DownloadState::update_folder_progress(
            account_id,
            mailbox.name.clone(),
            total_to_fetch,
            current_processed,
            FolderStatus::Success,
            None,
        )
        .await?;
    }
    session.logout().await.ok();
    Ok(())
}

pub fn generate_uid_sequence_hashset(
    unique_nums: Vec<u32>,
    chunk_size: usize,
    desc: bool,
) -> Vec<(String, u64)> {
    assert!(!unique_nums.is_empty());
    let mut nums = unique_nums;
    if desc {
        nums.reverse();
    }

    let mut result = Vec::new();

    for chunk in nums.chunks(chunk_size) {
        let size = chunk.len() as u64;
        let compressed = compress_uid_list(chunk.to_vec());
        result.push((compressed, size));
    }

    result
}

pub fn compress_uid_list(nums: Vec<u32>) -> String {
    if nums.is_empty() {
        return String::new();
    }

    let mut sorted_nums = nums;
    sorted_nums.sort();

    let mut result = Vec::new();
    let mut current_range_start = sorted_nums[0];
    let mut current_range_end = sorted_nums[0];

    for &n in sorted_nums.iter().skip(1) {
        if n == current_range_end + 1 {
            current_range_end = n;
        } else {
            if current_range_start == current_range_end {
                result.push(current_range_start.to_string());
            } else {
                result.push(format!("{}:{}", current_range_start, current_range_end));
            }
            current_range_start = n;
            current_range_end = n;
        }
    }

    if current_range_start == current_range_end {
        result.push(current_range_start.to_string());
    } else {
        result.push(format!("{}:{}", current_range_start, current_range_end));
    }

    result.join(",")
}

pub async fn reconcile_mailboxes(
    account: &AccountModel,
    remote_mailboxes: &[MailBox],
    local_mailboxes: &[MailBox],
    token: CancellationToken,
) -> BichonResult<()> {
    let start_time = Instant::now();
    let existing_mailboxes = find_intersecting_mailboxes(local_mailboxes, remote_mailboxes);
    let account_id = account.id;
    if !existing_mailboxes.is_empty() {
        let mut mailboxes_to_update = Vec::with_capacity(existing_mailboxes.len());

        DownloadState::init_folder_details(
            account.id,
            remote_mailboxes.iter().map(|m| m.name.clone()).collect(),
        )
        .await?;

        for (local_mailbox, remote_mailbox) in &existing_mailboxes {
            if token.is_cancelled() {
                DownloadState::update_session_status(
                    account.id,
                    DownloadStatus::Cancelled,
                    Some("Received termination signal (User stop or System shutdown)".to_string()),
                )
                .await?;
                break;
            }

            if local_mailbox.uid_validity != remote_mailbox.uid_validity {
                if remote_mailbox.uid_validity.is_none() {
                    let err_msg = format!(
                        "Mailbox '{}' logic error: Server did not provide UIDVALIDITY.",
                        local_mailbox.name
                    );

                    warn!("Account {}: {}", account_id, err_msg);

                    DownloadState::update_folder_progress(
                        account_id,
                        remote_mailbox.name.clone(),
                        0,
                        0,
                        FolderStatus::Failed,
                        Some(err_msg.clone()),
                    )
                    .await?;
                    DownloadState::append_session_error(account_id, err_msg).await?;
                    continue;
                }
                info!(
                    "Account {}: Mailbox '{}' detected with changed uid_validity (local: {:#?}, remote: {:#?}). \
                    The mailbox data may be invalid, resetting its envelopes and rebuilding the cache.",
                    account_id, local_mailbox.name, &local_mailbox.uid_validity, &remote_mailbox.uid_validity
                );

                DownloadState::update_folder_progress(
                    account_id,
                    local_mailbox.name.clone(),
                    remote_mailbox.exists as u64,
                    0,
                    FolderStatus::Downloading,
                    Some("UID validity changed, rebuilding...".into()),
                )
                .await?;

                match &account.date_since {
                    Some(date_since) => {
                        rebuild_mailbox_cache_by_date(
                            account,
                            local_mailbox.id,
                            &date_since.since_date()?,
                            remote_mailbox,
                            FetchDirection::Since,
                            token.clone(),
                        )
                        .await?;
                    }
                    None => match &account.date_before {
                        Some(r) => {
                            rebuild_mailbox_cache_by_date(
                                account,
                                local_mailbox.id,
                                &r.calculate_date()?,
                                remote_mailbox,
                                FetchDirection::Before,
                                token.clone(),
                            )
                            .await?;
                        }
                        None => {
                            rebuild_mailbox_cache(
                                account,
                                local_mailbox,
                                remote_mailbox,
                                token.clone(),
                            )
                            .await?
                        }
                    },
                }
            } else {
                perform_incremental_sync(account, local_mailbox, remote_mailbox, token.clone())
                    .await?;
            }

            mailboxes_to_update.push(remote_mailbox.clone());
        }
        //The metadata of this mailbox must only be updated after a successful synchronization;
        //otherwise, it may cause synchronization errors and result in missing emails in the local sync results.
        MailBox::batch_upsert(&mailboxes_to_update).await?;
    }

    debug!(
        "Checked mailbox folders for account ID: {}. Compared local and server folders to identify changes. Elapsed time: {} seconds",
        account.id,
        start_time.elapsed().as_secs()
    );

    let missing_mailboxes = find_missing_mailboxes(local_mailboxes, remote_mailboxes);
    //Mail folders that are not locally need to be downloaded.
    if !missing_mailboxes.is_empty() {
        MailBox::batch_insert(&missing_mailboxes).await?;

        let mut has_error = false;
        let mut last_err = None;
        for mailbox in &missing_mailboxes {
            if token.is_cancelled() {
                DownloadState::update_session_status(
                    account.id,
                    DownloadStatus::Cancelled,
                    Some("Received termination signal (User stop or System shutdown)".to_string()),
                )
                .await?;
                break;
            }
            if mailbox.exists > 0 {
                let account = account.clone();
                let mailbox = mailbox.clone();

                let _global_permit = match SEMAPHORE.clone().acquire_owned().await {
                    Ok(permit) => permit,
                    Err(err) => {
                        error!(
                            "Failed to acquire global semaphore permit for account {} mailbox '{}': {:#?}",
                            account.id, &mailbox.name, err
                        );
                        continue;
                    }
                };

                let result = match &account.date_since {
                    Some(date_since) => {
                        rebuild_mailbox_cache_by_date(
                            &account,
                            mailbox.id,
                            &date_since.since_date()?,
                            &mailbox,
                            FetchDirection::Since,
                            token.clone(),
                        )
                        .await
                    }
                    None => match &account.date_before {
                        Some(r) => {
                            rebuild_mailbox_cache_by_date(
                                &account,
                                mailbox.id,
                                &r.calculate_date()?,
                                &mailbox,
                                FetchDirection::Before,
                                token.clone(),
                            )
                            .await
                        }
                        None => {
                            rebuild_mailbox_cache(&account, &mailbox, &mailbox, token.clone()).await
                        }
                    },
                };

                match result {
                    Ok(_) => {}
                    Err(err) => {
                        has_error = true;
                        tracing::error!("Folder sync task failed: {:#?}", err);
                        last_err = Some(err);
                    }
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
    }
    Ok(())
}

//only check new emails and sync
async fn perform_incremental_sync(
    account: &AccountModel,
    local_mailbox: &MailBox,
    remote_mailbox: &MailBox,
    token: CancellationToken,
) -> BichonResult<()> {
    if remote_mailbox.exists > 0 {
        let local_max_uid = ENVELOPE_MANAGER
            .get_max_uid(account.id, local_mailbox.id)
            .await?;
        match local_max_uid {
            Some(max_uid) => {
                let mut session = ImapExecutor::create_connection(account.id).await?;
                let before_date = account
                    .date_before
                    .as_ref()
                    .map(|r| r.calculate_date())
                    .transpose()?;

                ImapExecutor::fetch_new_mail(
                    &mut session,
                    account,
                    local_mailbox,
                    max_uid + 1,
                    before_date.as_deref(),
                    token,
                )
                .await?;
                session.logout().await.ok();
            }
            None => {
                info!(
                    "No maximum UID found in index for mailbox, assuming local cache is missing."
                );

                match &account.date_since {
                    Some(date_since) => {
                        fetch_and_save_by_date(
                            account,
                            date_since.since_date()?.as_str(),
                            remote_mailbox,
                            FetchDirection::Since,
                            token,
                        )
                        .await?;
                    }
                    None => {
                        fetch_and_save_full_mailbox(account, remote_mailbox, token).await?;
                    }
                }
            }
        }
    }

    Ok(())
}
