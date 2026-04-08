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

use crate::modules::{
    common::signal::SIGNAL_MANAGER,
    envelope::extractor::reattach_eml_content,
    error::{code::ErrorCode, BichonResult},
    settings::dir::DATA_DIR_MANAGER,
};
use crate::raise_error;
use bytes::Bytes;
use fjall::{CompressionType, Database, Keyspace, KeyspaceCreateOptions, KvSeparationOptions, config::{BlockSizePolicy, CompressionPolicy}};

use std::{io::Cursor, sync::LazyLock};
use tokio::{
    sync::{mpsc, Mutex},
    task::{self, JoinHandle},
};

pub static BLOB_MANAGER: LazyLock<BlobManager> = LazyLock::new(BlobManager::new);

pub struct DetachedEmail {
    pub email: (String, Bytes),
    pub attachments: Option<Vec<(String, Bytes)>>,
}

pub struct BlobManager {
    sender: mpsc::Sender<DetachedEmail>,
    db: Database,
    email_keyspace: Keyspace,
    attachments_keyspace: Keyspace,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl BlobManager {
    pub async fn shutdown(&self) {
        let mut guard = self.handle.lock().await;
        if let Some(handle) = guard.take() {
            let _ = handle.await;
        }
    }

    fn process_detached_email(
        eml: DetachedEmail,
        email_ks: &Keyspace,
        attach_ks: &Keyspace,
    ) {
        let (email_hash, email_data) = eml.email;


        match email_ks.contains_key(&email_hash) {
            Ok(false) => {
                if let Err(e) = email_ks.insert(email_hash, email_data) {
                    tracing::error!("CRITICAL: Failed to insert email: {:?}",  e);
                }
            }
            Err(e) => tracing::error!("Fjall email_ks error: {:?}", e),
            _ => {}
        }

        if let Some(attachments) = eml.attachments {
            for (a_hash, a_data) in attachments {
                match attach_ks.contains_key(&a_hash) {
                    Ok(false) => {
                        if let Err(e) = attach_ks.insert(a_hash, a_data) {
                            tracing::error!("CRITICAL: Failed to insert attachment: {:?}", e);
                        }
                    }
                    Err(e) => tracing::error!("Fjall attach_ks error: {:?}", e),
                    _ => {}
                }
            }
        }
    }

    pub fn new() -> Self {
        let db = Database::builder(&DATA_DIR_MANAGER.eml_dir)
        .cache_size(64 * 1024 * 1024)
        .max_cached_files(Some(400))
        .journal_compression(CompressionType::None)
        .max_journaling_size(64 * 1024 * 1024)
            .open()
            .expect("Failed to initialize Fjall database: Check if the directory exists and has write permissions.");


        let email_keyspace = db
            .keyspace("email", || {
                KeyspaceCreateOptions::default()
                .max_memtable_size(16 * 1024 * 1024)
                .data_block_size_policy(BlockSizePolicy::all(4 * 1024))
                .data_block_compression_policy(  
                    CompressionPolicy::all(CompressionType::Lz4)  
                )  
                .with_kv_separation(Some(
                    KvSeparationOptions::default()
                        .separation_threshold(1024)
                        .compression(CompressionType::Lz4)
                        .file_target_size(512 * 1024 * 1024)
                        .staleness_threshold(0.5)
                        .age_cutoff(0.6),
                ))
            })
            .expect("Failed to open 'email' keyspace: The partition metadata might be corrupted or inaccessible.");
        
        let attachments_keyspace = db
            .keyspace("attachments", || {
                KeyspaceCreateOptions::default()
                .data_block_size_policy(BlockSizePolicy::all(4 * 1024))
                .data_block_compression_policy(  
                    CompressionPolicy::all(CompressionType::Lz4)  
                )
                .with_kv_separation(Some(
                    KvSeparationOptions::default()
                        .separation_threshold(1024)
                        .compression(CompressionType::Lz4)
                        .file_target_size(512 * 1024 * 1024)
                        .staleness_threshold(0.5)
                        .age_cutoff(0.6),
                ))
                .max_memtable_size(16 * 1024 * 1024)
            })
            .expect("Failed to open 'attachments' keyspace: Check disk space for blob storage initialization.");
        
        let (sender, mut receiver) = mpsc::channel::<DetachedEmail>(100);

        let email_ks = email_keyspace.clone();
        let attach_ks = attachments_keyspace.clone();
        let handler = task::spawn(async move {
            let mut shutdown = SIGNAL_MANAGER.subscribe();
            loop {
                tokio::select! {
                    res = receiver.recv() => {
                        match res {
                            Some(eml) => {
                                Self::process_detached_email(eml, &email_ks, &attach_ks);
                                while let Ok(next_eml) = receiver.try_recv() {
                                    Self::process_detached_email(next_eml, &email_ks, &attach_ks);
                                }
                            }
                            None => {
                                tracing::info!("BlobManager: All senders dropped, closing storage.");
                                break;
                            }
                        }
                    }
                    _ = shutdown.recv() => {
                        receiver.close();
                        let remaining = receiver.len();
                        tracing::info!(
                            "BlobManager: Shutdown signal received. Processing {} remaining tasks...",
                            remaining
                        );

                        while let Some(eml) = receiver.recv().await {
                            Self::process_detached_email(eml, &email_ks, &attach_ks);
                        }

                        tracing::info!("BlobManager: All remaining tasks processed. Closing Fjall.");
                        break;
                    }
                }
            }
        });

        Self {
            sender,
            db,
            email_keyspace,
            attachments_keyspace,
            handle: Mutex::new(Some(handler)),
        }
    }

    pub async fn queue(&self, email: DetachedEmail) {
        let _ = self.sender.send(email).await;
    }

    pub fn get_email(&self, content_hash: &str) -> BichonResult<Option<Bytes>> {
        self.email_keyspace
            .get(content_hash)
            .map(|user_value| user_value.map(|s| s.into()))
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
    }

    pub fn get_attachment(&self, content_hash: &str) -> BichonResult<Option<Bytes>> {
        self.attachments_keyspace
            .get(content_hash)
            .map(|user_value| user_value.map(|s| s.into()))
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
    }

    pub fn delete<I1, I2>(
        &self,
        email_content_hashes: I1,
        attachment_content_hashes: I2,
    ) -> BichonResult<()>
    where
        I1: IntoIterator,
        I1::Item: AsRef<str>,
        I2: IntoIterator,
        I2::Item: AsRef<str> {
        let mut batch = self.db.batch();
        for hash in email_content_hashes {
            batch.remove(&self.email_keyspace, hash.as_ref());
        }
        for hash in attachment_content_hashes {
            batch.remove(&self.attachments_keyspace, hash.as_ref());
        }
        batch
            .commit()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Ok(())
    }
}

pub async fn get_reader(account_id: u64, eid: String) -> BichonResult<Cursor<Bytes>> {
    let (_, data) = reattach_eml_content(account_id, eid).await?;
    Ok(Cursor::new(data))
}
