use crate::modules::{
    common::signal::SIGNAL_MANAGER,
    envelope::extractor::reattach_eml_content,
    error::{code::ErrorCode, BichonResult},
    settings::dir::DATA_DIR_MANAGER,
};
use crate::raise_error;
use bytes::Bytes;
use fjall::{CompressionType, Database, Keyspace, KeyspaceCreateOptions, KvSeparationOptions};

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
        store: &Database,
        email_ks: &Keyspace,
        attach_ks: &Keyspace,
    ) {
        let (email_hash, email_data) = eml.email;
        let mut batch = store.batch();
        let mut needs_commit = false;

        match email_ks.contains_key(&email_hash) {
            Ok(false) => {
                batch.insert(email_ks, email_hash.as_bytes(), email_data);
                needs_commit = true;
            }
            Err(e) => tracing::error!("Fjall email_ks error: {:?}", e),
            _ => {}
        }

        if let Some(attachments) = eml.attachments {
            for (a_hash, a_data) in attachments {
                match attach_ks.contains_key(&a_hash) {
                    Ok(false) => {
                        batch.insert(attach_ks, a_hash.as_bytes(), a_data);
                        needs_commit = true;
                    }
                    Err(e) => tracing::error!("Fjall attach_ks error: {:?}", e),
                    _ => {}
                }
            }
        }

        if needs_commit {
            if let Err(e) = batch.commit() {
                tracing::error!("Fjall Batch Commit Error: {:?}", e);
            }
        }
    }

    pub fn new() -> Self {
        let db = Database::builder(&DATA_DIR_MANAGER.eml_dir)
            .open()
            .expect("Failed to initialize Fjall database: Check if the directory exists and has write permissions.");
        let email_keyspace = db
            .keyspace("email", || {
                KeyspaceCreateOptions::default()
                    .with_kv_separation(Some(
                        KvSeparationOptions::default()
                            .separation_threshold(0)
                            .compression(CompressionType::Lz4)
                            .file_target_size(128 * 1024 * 1024)
                            .staleness_threshold(0.5)
                            .age_cutoff(0.6),
                    ))
                    .max_memtable_size(64 * 1024 * 1024)
            })
            .expect("Failed to open 'email' keyspace: The partition metadata might be corrupted or inaccessible.");

        let attachments_keyspace = db
            .keyspace("attachments", || {
                KeyspaceCreateOptions::default()
                    .with_kv_separation(Some(
                        KvSeparationOptions::default()
                            .separation_threshold(0)
                            .compression(CompressionType::Lz4)
                            .file_target_size(256 * 1024 * 1024)
                            .staleness_threshold(0.5)
                            .age_cutoff(0.6),
                    ))
                    .max_memtable_size(64 * 1024 * 1024)
            })
            .expect("Failed to open 'attachments' keyspace: Check disk space for blob storage initialization.");

        let (sender, mut receiver) = mpsc::channel::<DetachedEmail>(100);

        let store = db.clone();
        let email_ks = email_keyspace.clone();
        let attach_ks = attachments_keyspace.clone();
        let handler = task::spawn(async move {
            let mut shutdown = SIGNAL_MANAGER.subscribe();
            loop {
                tokio::select! {
                    res = receiver.recv() => {
                        match res {
                            Some(eml) => {
                                Self::process_detached_email(eml, &store, &email_ks, &attach_ks);
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
                            Self::process_detached_email(eml, &store, &email_ks, &attach_ks);
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

    pub fn delete(
        &self,
        email_content_hashes: &[String],
        attachment_content_hashes: &[String],
    ) -> BichonResult<()> {
        let mut batch = self.db.batch();
        for hash in email_content_hashes {
            batch.remove(&self.email_keyspace, hash);
        }
        for hash in attachment_content_hashes {
            batch.remove(&self.attachments_keyspace, hash);
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
