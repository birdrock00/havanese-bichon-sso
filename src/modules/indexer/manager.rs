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

use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
    time::Duration,
};

use crate::modules::{
    duckdb::init::duckdb,
    message::{
        attachment::AttachmentMetadata,
        content::{AttachmentDetail, AttachmentInfo},
        search::SortBy,
        tags::{TagCount, TagsRequest},
    },
};
use crate::{
    modules::{
        common::signal::SIGNAL_MANAGER,
        dashboard::{DashboardStats, LargestEmail},
        error::{code::ErrorCode, BichonResult},
        indexer::envelope::Envelope,
        message::search::SearchFilter,
        rest::response::DataPage,
    },
    raise_error,
};

use tokio::{sync::mpsc, task};

pub static ENVELOPE_INDEX_MANAGER: LazyLock<EnvelopeIndexManager> =
    LazyLock::new(EnvelopeIndexManager::new);

pub const ENVELOPE_BATCH_SIZE: usize = 500;
pub const EML_BATCH_SIZE: usize = 100;

const MAX_BUFFER_DURATION: Duration = Duration::from_secs(10);

pub enum MetadataOp {
    Record((Envelope, Vec<AttachmentInfo>)),
    Shutdown,
}

pub struct EnvelopeIndexManager {
    sender: mpsc::Sender<MetadataOp>,
}

impl EnvelopeIndexManager {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel::<MetadataOp>(1000);
        task::spawn(async move {
            let mut buffer: Vec<(Envelope, Vec<AttachmentInfo>)> =
                Vec::with_capacity(ENVELOPE_BATCH_SIZE);
            let mut interval = tokio::time::interval(MAX_BUFFER_DURATION);
            let mut shutdown = SIGNAL_MANAGER.subscribe();
            loop {
                tokio::select! {
                    maybe_msg = receiver.recv() => {
                        match maybe_msg {
                            Some(MetadataOp::Record(doc)) => {
                                buffer.push(doc);
                                if buffer.len() >= ENVELOPE_BATCH_SIZE {
                                    ENVELOPE_INDEX_MANAGER.drain_and_commit(&mut buffer).await;
                                }
                            }
                            Some(MetadataOp::Shutdown) => {
                                ENVELOPE_INDEX_MANAGER.drain_and_commit(&mut buffer).await;
                                break;
                            }
                            None => break,
                        }
                    }
                    _ = interval.tick() => {
                        if !buffer.is_empty() {
                            ENVELOPE_INDEX_MANAGER.drain_and_commit(&mut buffer).await;
                        }
                    }
                    _ = shutdown.recv() => {
                        let _ = ENVELOPE_INDEX_MANAGER.sender.send(MetadataOp::Shutdown).await;
                    }
                }
            }
        });
        Self { sender }
    }

    pub async fn add_document(&self, doc: (Envelope, Vec<AttachmentInfo>)) {
        let _ = self.sender.send(MetadataOp::Record(doc)).await;
    }

    async fn drain_and_commit(&self, buffer: &mut Vec<(Envelope, Vec<AttachmentInfo>)>) {
        if buffer.is_empty() {
            return;
        }

        let items: Vec<(Envelope, Vec<AttachmentInfo>)> = buffer.drain(..).collect();

        let result = (|| -> BichonResult<()> {
            duckdb()?.append_envelopes_with_attachments(&items)?;
            Ok(())
        })();

        if let Err(e) = result {
            tracing::error!("Failed to drain and commit envelopes to DuckDB: {:#?}", e);
        }
    }

    pub async fn total_emails(&self, accounts: Option<HashSet<u64>>) -> BichonResult<u64> {
        tokio::task::spawn_blocking(move || duckdb()?.total_emails(accounts))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn delete_account_envelopes(&self, account_id: u64) -> BichonResult<Vec<String>> {
        let content_hashes = tokio::task::spawn_blocking(move || {
            duckdb()?.delete_account_envelopes_with_orphans(account_id)
        })
        .await
        .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))??;
        Ok(content_hashes)
    }

    pub async fn delete_mailbox_envelopes(
        &self,
        account_id: u64,
        mailbox_ids: Vec<u64>,
    ) -> BichonResult<Vec<String>> {
        if mailbox_ids.is_empty() {
            tracing::warn!("delete_mailbox_envelopes: mailbox_ids is empty, nothing to delete");
            return Ok(vec![]);
        }

        let content_hashes = tokio::task::spawn_blocking(move || {
            duckdb()?.delete_mailbox_envelopes_with_orphans(account_id, mailbox_ids)
        })
        .await
        .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))??;
        Ok(content_hashes)
    }

    pub async fn get_all_tags(
        &self,
        accounts: Option<HashSet<u64>>,
    ) -> BichonResult<Vec<TagCount>> {
        tokio::task::spawn_blocking(move || duckdb()?.get_all_tags(accounts))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn get_all_contacts(
        &self,
        accounts: Option<HashSet<u64>>,
    ) -> BichonResult<HashSet<String>> {
        tokio::task::spawn_blocking(move || duckdb()?.get_all_contacts(accounts))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn get_attachment_metadata(
        &self,
        accounts: Option<HashSet<u64>>,
    ) -> BichonResult<AttachmentMetadata> {
        tokio::task::spawn_blocking(move || duckdb()?.get_attachment_metadata(accounts))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn get_orphan_hashes_in_memory(
        &self,
        deletes: HashMap<u64, Vec<String>>,
    ) -> BichonResult<Vec<String>> {
        tokio::task::spawn_blocking(move || duckdb()?.get_orphan_hashes_in_memory(deletes))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn delete_envelopes_multi_account(
        &self,
        deletes: HashMap<u64, Vec<String>>, // HashMap<account_id, envelope_ids>
    ) -> BichonResult<()> {
        if deletes.is_empty() {
            tracing::warn!("delete_envelopes_multi_account: deletes is empty, nothing to delete");
            return Ok(());
        }
        tokio::task::spawn_blocking(move || duckdb()?.delete_envelopes_multi_account(deletes))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn update_envelope_tags(&self, request: TagsRequest) -> BichonResult<()> {
        if request.updates.is_empty() {
            tracing::warn!("update_envelope_tags: request is empty, nothing to update");
            return Ok(());
        }
        tokio::task::spawn_blocking(move || duckdb()?.update_envelope_tags(request))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn search(
        &self,
        accounts: Option<HashSet<u64>>,
        filter: SearchFilter,
        page: u64,
        page_size: u64,
        desc: bool,
        sort_by: SortBy,
    ) -> BichonResult<DataPage<Envelope>> {
        assert!(page > 0, "Page number must be greater than 0");
        assert!(page_size > 0, "Page size must be greater than 0");
        tokio::task::spawn_blocking(move || {
            duckdb()?.search(accounts, filter, page, page_size, desc, sort_by)
        })
        .await
        .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn list_mailbox_envelopes(
        &self,
        account_id: u64,
        mailbox_id: u64,
        page: u64,
        page_size: u64,
        desc: bool,
    ) -> BichonResult<DataPage<Envelope>> {
        assert!(page > 0, "Page number must be greater than 0");
        assert!(page_size > 0, "Page size must be greater than 0");
        tokio::task::spawn_blocking(move || {
            duckdb()?.list_mailbox_envelopes(account_id, mailbox_id, page, page_size, desc)
        })
        .await
        .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn list_thread_envelopes(
        &self,
        account_id: u64,
        thread_id: String,
        page: u64,
        page_size: u64,
        desc: bool,
    ) -> BichonResult<DataPage<Envelope>> {
        assert!(page > 0, "Page number must be greater than 0");
        assert!(page_size > 0, "Page size must be greater than 0");
        tokio::task::spawn_blocking(move || {
            duckdb()?.list_thread_envelopes(account_id, thread_id, page, page_size, desc)
        })
        .await
        .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn get_envelope_by_id(
        &self,
        account_id: u64,
        envelope_id: String,
    ) -> BichonResult<Option<Envelope>> {
        tokio::task::spawn_blocking(move || duckdb()?.get_envelope_by_id(account_id, envelope_id))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn get_attachments_by_envelope_id(
        &self,
        account_id: u64,
        envelope_id: String,
    ) -> BichonResult<Vec<AttachmentDetail>> {
        tokio::task::spawn_blocking(move || {
            duckdb()?.get_attachments_by_envelope_id(account_id, envelope_id)
        })
        .await
        .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn top_10_largest_emails(
        &self,
        accounts: Option<HashSet<u64>>,
    ) -> BichonResult<Vec<LargestEmail>> {
        tokio::task::spawn_blocking(move || duckdb()?.top_10_largest_emails(accounts))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn get_max_uid(&self, account_id: u64, mailbox_id: u64) -> BichonResult<Option<u64>> {
        tokio::task::spawn_blocking(move || duckdb()?.get_max_uid(account_id, mailbox_id))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn num_messages_in_mailbox(
        &self,
        account_id: u64,
        mailbox_id: u64,
    ) -> BichonResult<u64> {
        tokio::task::spawn_blocking(move || {
            duckdb()?.num_messages_in_mailbox(account_id, mailbox_id)
        })
        .await
        .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn num_messages_in_thread(
        &self,
        account_id: u64,
        thread_id: String,
    ) -> BichonResult<u64> {
        tokio::task::spawn_blocking(move || duckdb()?.num_messages_in_thread(account_id, thread_id))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }

    pub async fn get_dashboard_stats(
        &self,
        accounts: Option<HashSet<u64>>,
    ) -> BichonResult<DashboardStats> {
        tokio::task::spawn_blocking(move || duckdb()?.get_dashboard_stats(accounts))
            .await
            .map_err(|e| raise_error!(format!("{:?}", e), ErrorCode::InternalError))?
    }
}
