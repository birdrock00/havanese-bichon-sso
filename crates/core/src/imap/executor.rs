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

use crate::account::migration::AccountModel;
use crate::account::state::{DownloadState, FolderStatus};
use crate::cache::imap::mailbox::MailBox;
use crate::cache::imap::download::flow::{generate_uid_sequence_hashset, DEFAULT_BATCH_SIZE};
use crate::envelope::extractor::extract_envelope_and_store_it;
use crate::error::code::ErrorCode;
use crate::imap::session::SessionStream;
use crate::{error::BichonResult, imap::manager::ImapConnectionManager};
use crate::raise_error;
use async_imap::types::Name;
use async_imap::Session;
use futures::TryStreamExt;
use std::collections::HashSet;
use tokio_util::sync::CancellationToken;
use tracing::info;

const BODY_FETCH_COMMAND: &str = "(UID INTERNALDATE RFC822.SIZE BODY.PEEK[])";

pub struct ImapExecutor;

impl ImapExecutor {
    pub async fn list_all_mailboxes(
        session: &mut Session<Box<dyn SessionStream>>,
    ) -> BichonResult<Vec<Name>> {
        let list = session
            .list(Some(""), Some("*"))
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))?;
        let result = list
            .try_collect::<Vec<Name>>()
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))?;
        Ok(result)
    }

    pub async fn uid_search(
        session: &mut Session<Box<dyn SessionStream>>,
        mailbox_name: &str,
        query: &str,
    ) -> BichonResult<HashSet<u32>> {
        session
            .examine(mailbox_name)
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))?;
        let result = session
            .uid_search(query)
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))?;
        Ok(result)
    }

    pub async fn append(
        session: &mut Session<Box<dyn SessionStream>>,
        mailbox_name: impl AsRef<str>,
        flags: Option<&str>,
        internaldate: Option<&str>,
        content: impl AsRef<[u8]>,
    ) -> BichonResult<()> {
        session
            .append(mailbox_name, flags, internaldate, content)
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))
    }

    pub async fn fetch_new_mail(
        session: &mut Session<Box<dyn SessionStream>>,
        account: &AccountModel,
        mailbox: &MailBox,
        start_uid: u64,
        before: Option<&str>,
        token: CancellationToken,
    ) -> BichonResult<()> {
        assert!(start_uid > 0, "start_uid must be greater than 0");

        let query = match before {
            Some(date) => format!("UID {start_uid}:* BEFORE {date}"),
            None => format!("UID {start_uid}:*"),
        };

        let uid_list = match Self::uid_search(session, &mailbox.encoded_name(), &query).await {
            Ok(uid_list) => uid_list,
            Err(e) => {
                let err_msg = format!("UID search failed in [{}]: {:#?}", mailbox.name, e);
                DownloadState::update_folder_progress(
                    account.id,
                    mailbox.name.clone(),
                    0,
                    0,
                    FolderStatus::Failed,
                    Some(err_msg.clone()),
                )
                .await?;
                DownloadState::append_session_error(account.id, err_msg).await?;
                return Err(e);
            }
        };

        let len = uid_list.len();
        if len == 0 {
            let msg = match before {
                Some(date) => format!("No emails found before {}.", date),
                None => "No new emails found.".into(),
            };
            DownloadState::update_folder_progress(
                account.id,
                mailbox.name.clone(),
                0,
                0,
                FolderStatus::Success,
                Some(msg),
            )
            .await?;
            return Ok(());
        }
        info!(
            "[account {}][mailbox {}] {} envelopes need to be fetched",
            account.id, mailbox.name, len
        );

        let mut uid_vec: Vec<u32> = uid_list.into_iter().collect();
        uid_vec.sort();
        let uid_batches = generate_uid_sequence_hashset(
            uid_vec,
            account.download_batch_size.unwrap_or(DEFAULT_BATCH_SIZE) as usize,
            false,
        );
        let mut current_processed = 0u64;
        let mut has_error_or_cancel = false;
        for (index, batch) in uid_batches.into_iter().enumerate() {
            if token.is_cancelled() {
                break;
            }
            match Self::uid_batch_retrieve_emails(
                session,
                account.id,
                mailbox.id,
                &batch.0,
                token.clone(),
            )
            .await
            {
                Ok(_) => {
                    current_processed += batch.1;
                    DownloadState::update_folder_progress(
                        account.id,
                        mailbox.name.clone(),
                        len as u64,
                        current_processed,
                        FolderStatus::Downloading,
                        None,
                    )
                    .await?;
                }
                Err(e) => {
                    let err_msg = format!("Batch {} failed: {:#?}", index, e);
                    DownloadState::append_session_error(account.id, err_msg.clone()).await?;
                    DownloadState::update_folder_progress(
                        account.id,
                        mailbox.name.clone(),
                        len as u64,
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
                account.id,
                mailbox.name.clone(),
                len as u64,
                current_processed,
                FolderStatus::Success,
                None,
            )
            .await?;
        }

        Ok(())
    }

    pub async fn batch_retrieve_emails(
        session: &mut Session<Box<dyn SessionStream>>,
        account_id: u64,
        mailbox_id: u64,
        total: u64,
        page: u64,
        page_size: u64,
        encoded_mailbox_name: &str,
        desc: bool,
        token: CancellationToken,
    ) -> BichonResult<usize> {
        assert!(page > 0, "Page number must be greater than 0");
        assert!(page_size > 0, "Page size must be greater than 0");

        let (start, end) = if desc {
            // Fetch messages starting from the newest (descending order)
            let end = total.saturating_sub((page - 1) * page_size);
            if end == 0 {
                return Ok(0);
            }
            // Calculate start as end - page_size + 1 to avoid off-by-one errors
            let start = end.saturating_sub(page_size - 1).max(1);
            (start, end)
        } else {
            // Fetch messages starting from the oldest (ascending order)
            let start = (page - 1) * page_size + 1;
            if start > total {
                return Ok(0);
            }
            // Calculate end, capped by the total number of messages
            let end = (start + page_size - 1).min(total);
            (start, end)
        };

        let sequence_set = format!("{}:{}", start, end);
        info!(
            "Fetching mailbox '{}' messages: sequence {} (page {}, page_size {}, desc={})",
            encoded_mailbox_name, sequence_set, page, page_size, desc
        );

        let mut stream = session
            .fetch(sequence_set.as_str(), BODY_FETCH_COMMAND)
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))?;

        let mut count = 0;
        while let Some(fetch) = stream
            .try_next()
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))?
        {
            if token.is_cancelled() {
                tracing::info!("Account {}: UID fetch stream interrupted.", account_id);
                return Err(raise_error!(
                    "Stream cancelled".into(),
                    ErrorCode::InternalError
                ));
            }
            extract_envelope_and_store_it(fetch, account_id, mailbox_id).await?;
            count += 1;
        }
        Ok(count)
    }

    pub async fn uid_batch_retrieve_emails(
        session: &mut Session<Box<dyn SessionStream>>,
        account_id: u64,
        mailbox_id: u64,
        uid_set: &str,
        token: CancellationToken,
    ) -> BichonResult<()> {
        let mut stream = session
            .uid_fetch(uid_set, BODY_FETCH_COMMAND)
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))?;
        while let Some(fetch) = stream
            .try_next()
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))?
        {
            if token.is_cancelled() {
                tracing::info!("Account {}: UID fetch stream interrupted.", account_id);
                return Err(raise_error!(
                    "Stream cancelled".into(),
                    ErrorCode::InternalError
                ));
            }
            extract_envelope_and_store_it(fetch, account_id, mailbox_id).await?;
        }
        Ok(())
    }

    pub async fn create_connection(
        account_id: u64,
    ) -> BichonResult<Session<Box<dyn SessionStream>>> {
        ImapConnectionManager::build(account_id).await
    }
}
