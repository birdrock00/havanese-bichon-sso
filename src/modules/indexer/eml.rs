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
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, LazyLock},
    time::Duration,
};

use crate::modules::{
    envelope::extractor::reattach_eml_content, indexer::DocumentOp, settings::cli::SETTINGS,
};
use crate::{
    modules::{
        common::signal::SIGNAL_MANAGER,
        error::{code::ErrorCode, BichonResult},
        indexer::schema::SchemaTools,
        settings::dir::DATA_DIR_MANAGER,
    },
    raise_error,
};
use tantivy::indexer::{NoMergePolicy, UserOperation};
use tantivy::{
    collector::TopDocs,
    query::TermQuery,
    schema::{IndexRecordOption, Value},
    store::{Compressor, ZstdCompressor},
    Index, IndexBuilder, IndexReader, IndexSettings, IndexWriter, TantivyDocument, Term,
};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::{mpsc, Mutex},
    task,
};
use tracing::info;

pub const EML_BATCH_SIZE: usize = 100;
const MAX_BUFFER_DURATION: Duration = Duration::from_secs(10);

pub static EML_INDEX_MANAGER: LazyLock<EmlIndexManager> = LazyLock::new(EmlIndexManager::new);

pub struct EmlIndexManager {
    index_writer: Arc<Mutex<IndexWriter>>,
    sender: mpsc::Sender<DocumentOp>,
    reader: IndexReader,
}

impl EmlIndexManager {
    pub fn new() -> Self {
        let index = Self::open_or_create_index(&DATA_DIR_MANAGER.eml_dir);

        let writer: IndexWriter<TantivyDocument> = index
            .writer_with_num_threads(
                SETTINGS.bichon_tantivy_threads as usize,
                SETTINGS.bichon_tantivy_buffer_size,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to create IndexWriter (threads: {}, buffer: {}B) for {:?}: {}",
                    SETTINGS.bichon_tantivy_threads,
                    SETTINGS.bichon_tantivy_buffer_size,
                    DATA_DIR_MANAGER.eml_dir,
                    e
                )
            });

        writer.set_merge_policy(Box::new(NoMergePolicy));
        let index_writer = Arc::new(Mutex::new(writer));

        let reader = index.reader().unwrap_or_else(|e| {
            panic!(
                "Failed to create IndexReader for {:?}: {}",
                DATA_DIR_MANAGER.eml_dir, e
            )
        });
        let (sender, mut receiver) = mpsc::channel::<DocumentOp>(100);
        task::spawn(async move {
            let mut buffer: HashMap<String, TantivyDocument> =
                HashMap::with_capacity(EML_BATCH_SIZE);
            let mut interval = tokio::time::interval(MAX_BUFFER_DURATION);
            let mut shutdown = SIGNAL_MANAGER.subscribe();
            loop {
                tokio::select! {
                    maybe_msg = receiver.recv() => {
                        match maybe_msg {
                            Some(DocumentOp::Document((eid, doc))) => {
                                buffer.insert(eid, doc);
                                if buffer.len() >= EML_BATCH_SIZE {
                                    EML_INDEX_MANAGER.drain_and_commit(&mut buffer).await;
                                }
                            }
                            Some(DocumentOp::Shutdown) => {
                                EML_INDEX_MANAGER.drain_and_commit(&mut buffer).await;
                                break;
                            }
                            None => break,
                        }
                    }
                    _ = interval.tick() => {
                        if !buffer.is_empty() {
                            EML_INDEX_MANAGER.drain_and_commit(&mut buffer).await;
                        }
                    }
                    _ = shutdown.recv() => {
                        let _ = EML_INDEX_MANAGER.sender.send(DocumentOp::Shutdown).await;
                    }
                }
            }
        });
        Self {
            index_writer,
            sender,
            reader,
        }
    }

    /// Adds a document to the indexer.
    ///
    /// # Parameters
    /// - `eid`: A hash derived from **Account ID + Message ID**.
    ///   This acts as a unique identifier for the EML content itself.
    ///
    /// - `doc`: The `TantivyDocument` representing the mail body/content.
    ///
    /// # Logical Design
    /// Unlike the `envelope_id` (which is a hash of Account + Folder + Message ID),
    /// this `eid` ignores the folder context. This ensures that while metadata
    /// (envelopes) can be duplicated across different folders, the physical
    /// EML/document storage remains de-duplicated and unique.
    pub async fn add_document(&self, content_hash: String, doc: TantivyDocument) {
        let _ = self
            .sender
            .send(DocumentOp::Document((content_hash, doc)))
            .await;
    }

    fn open_or_create_index(index_dir: &PathBuf) -> Index {
        let need_create = !index_dir.exists()
            || index_dir
                .read_dir()
                .map(|mut d| d.next().is_none())
                .unwrap_or(true);

        if need_create {
            info!(
                "Email storage not found or empty, creating new mail storage at {}",
                index_dir.display()
            );
            std::fs::create_dir_all(&index_dir).unwrap_or_else(|e| {
                panic!("Failed to create index directory {:?}: {}", index_dir, e)
            });
            IndexBuilder::new()
                .schema(SchemaTools::schema())
                .settings(IndexSettings {
                    docstore_compression: Compressor::Zstd(ZstdCompressor {
                        compression_level: Some(SETTINGS.bichon_eml_compression_level as i32),
                    }),
                    docstore_compress_dedicated_thread: true,
                    docstore_blocksize: SETTINGS.bichon_eml_blocksize,
                })
                .create_in_dir(&index_dir)
                .unwrap_or_else(|e| panic!("Failed to create index in {:?}: {}", index_dir, e))
        } else {
            info!("Opening existing email storage at {}", index_dir.display());
            open(&index_dir)
        }
    }

    fn term(&self, content_hash: &str) -> Term {
        Term::from_field_text(SchemaTools::fields().f_id, content_hash)
    }

    pub async fn get(&self, content_hash: &str) -> BichonResult<Option<Vec<u8>>> {
        let searcher = self.reader.searcher();
        let term = Term::from_field_text(SchemaTools::fields().f_id, content_hash);
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let docs = searcher
            .search(&query, &TopDocs::with_limit(1))
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        if docs.is_empty() {
            return Ok(None);
        }

        let (_, doc_address) = docs.first().unwrap();
        let doc: TantivyDocument = searcher
            .doc_async(*doc_address)
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        let fields = SchemaTools::fields();
        let value = doc.get_first(fields.f_blob).ok_or_else(|| {
            raise_error!(
                format!("miss '{}' field in tantivy document", stringify!(field)),
                ErrorCode::InternalError
            )
        })?;
        let bytes = value.as_bytes().ok_or_else(|| {
            raise_error!(
                format!("'{}' field is not a bytes", stringify!(field)),
                ErrorCode::InternalError
            )
        })?;

        Ok(Some(bytes.to_vec()))
    }

    pub async fn get_reader(&self, account_id: u64, eid: String) -> BichonResult<File> {
        let (envelope, data) = reattach_eml_content(account_id, eid).await?;
        let mut path = DATA_DIR_MANAGER.temp_dir.clone();
        path.push(format!("{}.eml", envelope.content_hash));
        {
            let mut file = File::create(&path)
                .await
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            file.write_all(&data)
                .await
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        }
        let file = File::open(&path)
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Ok(file)
    }

    pub async fn delete(
        &self,
        content_hashes: &Vec<String>, // HashMap<account_id, envelope_ids>
    ) -> BichonResult<()> {
        if content_hashes.is_empty() {
            tracing::warn!("delete_email_multi_account: deletes is empty, nothing to delete");
            return Ok(());
        }

        let mut writer = self.index_writer.lock().await;
        for hash in content_hashes {
            let term = self.term(hash);
            writer.delete_term(term);
        }
        writer
            .commit()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        Ok(())
    }

    // Deduplicate directly by content_hash, regardless of the account.
    async fn drain_and_commit(&self, buffer: &mut HashMap<String, TantivyDocument>) {
        if buffer.is_empty() {
            return;
        }
        let mut writer = self.index_writer.lock().await;
        let mut operations = Vec::new();

        for (content_hash, doc) in buffer.drain() {
            let delete_term = Term::from_field_text(SchemaTools::fields().f_id, &content_hash);
            operations.push(UserOperation::Delete(delete_term));
            operations.push(UserOperation::Add(doc));
        }
        if let Err(e) = writer.run(operations) {
            eprintln!("[FATAL] Tantivy run failed: {e:?}");
            std::process::exit(1);
        }

        fatal_commit(&mut writer);
    }
}

fn fatal_commit(writer: &mut IndexWriter) {
    const MAX_RETRIES: usize = 3;
    const RETRY_DELAY_MS: u64 = 1000;

    for attempt in 0..=MAX_RETRIES {
        match writer.commit() {
            Ok(_) => {
                if attempt > 0 {
                    eprintln!("[INFO] Commit succeeded on attempt {}", attempt + 1);
                }
                return;
            }
            Err(e) => match &e {
                tantivy::TantivyError::IoError(io_error) => {
                    if attempt < MAX_RETRIES {
                        eprintln!(
                            "[WARN] Commit failed (attempt {}/{}): {:?}. Retrying in {}ms...",
                            attempt + 1,
                            MAX_RETRIES + 1,
                            io_error,
                            RETRY_DELAY_MS * (attempt as u64 + 1)
                        );
                        std::thread::sleep(std::time::Duration::from_millis(
                            RETRY_DELAY_MS * (attempt as u64 + 1),
                        ));
                    } else {
                        eprintln!(
                            "[FATAL] Tantivy commit failed after {} attempts: {:?}",
                            MAX_RETRIES + 1,
                            io_error
                        );
                        std::process::exit(1);
                    }
                }
                _ => {
                    eprintln!("[FATAL] Tantivy commit failed with non-IO error: {e:?}");
                    std::process::exit(1);
                }
            },
        }
    }
}

fn open(index_dir: &PathBuf) -> Index {
    Index::open_in_dir(index_dir)
        .unwrap_or_else(|e| panic!("Failed to open index in {:?}: {}", index_dir, e))
}
