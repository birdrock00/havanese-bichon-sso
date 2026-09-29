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

use std::ops::{Deref, DerefMut};

use tantivy::{schema::Facet, IndexWriter};

use crate::{
    error::{code::ErrorCode, BichonResult},
    raise_error,
};

/// Holds an `IndexWriter` behind the index manager's mutex.
///
/// Tantivy's only deterministic "wait for all in-flight merges" API
/// (`IndexWriter::wait_merging_threads`) takes ownership of the writer and
/// permanently shuts it down, so a backup flush must consume the writer and
/// install a freshly created one afterwards. Wrapping it in this slot lets the
/// flush arm do the consume/recreate dance while every other caller keeps using
/// the writer through plain deref — no lock site needs to change.
pub(crate) struct WriterSlot {
    inner: Option<IndexWriter>,
}

impl WriterSlot {
    pub(crate) fn new(writer: IndexWriter) -> Self {
        Self {
            inner: Some(writer),
        }
    }

    /// Removes the writer from the slot. Only the backup flush arm calls this,
    /// while the write gate is paused and it holds the writer lock, so no other
    /// code path can observe the slot as empty.
    pub(crate) fn take(&mut self) -> IndexWriter {
        self.inner
            .take()
            .expect("index writer missing (backup flush in progress?)")
    }

    /// Replaces the (consumed) writer after a backup flush.
    pub(crate) fn put(&mut self, writer: IndexWriter) {
        debug_assert!(
            self.inner.is_none(),
            "index writer slot already occupied when restoring after backup flush"
        );
        self.inner = Some(writer);
    }
}

impl Deref for WriterSlot {
    type Target = IndexWriter;
    fn deref(&self) -> &IndexWriter {
        self.inner
            .as_ref()
            .expect("index writer missing (backup flush in progress?)")
    }
}

impl DerefMut for WriterSlot {
    fn deref_mut(&mut self) -> &mut IndexWriter {
        self.inner
            .as_mut()
            .expect("index writer missing (backup flush in progress?)")
    }
}

pub mod attachment;
pub mod dedup;
pub mod dedup_cache;
pub mod envelope;
pub mod fields;
pub mod filter;
pub mod model;
pub mod schema;
pub mod tokenizers;

pub fn fatal_commit(writer: &mut IndexWriter) {
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
                tantivy::TantivyError::IoError(_) | tantivy::TantivyError::OpenWriteError(_) => {
                    if attempt < MAX_RETRIES {
                        eprintln!(
                            "[WARN] Commit failed (attempt {}/{}): {:?}. Retrying in {}ms...",
                            attempt + 1,
                            MAX_RETRIES + 1,
                            e,
                            RETRY_DELAY_MS * (attempt as u64 + 1)
                        );
                        std::thread::sleep(std::time::Duration::from_millis(
                            RETRY_DELAY_MS * (attempt as u64 + 1),
                        ));
                    } else {
                        eprintln!(
                            "[FATAL] Tantivy commit failed after {} attempts: {:?}",
                            MAX_RETRIES + 1,
                            e
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

pub fn validate_facet(tag: &str) -> BichonResult<()> {
    Facet::from_text(tag)
        .map(|_| ())
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::{schema, Index};

    fn test_index() -> Index {
        let mut builder = schema::Schema::builder();
        builder.add_text_field("body", schema::TEXT);
        Index::create_in_ram(builder.build())
    }

    fn test_writer() -> IndexWriter {
        test_index().writer_with_num_threads(1, 30_000_000).unwrap()
    }

    /// Ordinary call sites see the writer through deref; the backup flush
    /// consumes it out of the slot and installs a fresh one afterwards.
    #[test]
    fn writer_slot_derefs_then_take_and_put() {
        let mut slot = WriterSlot::new(test_writer());
        let _ = slot.commit_opstamp(); // Deref
        let _ = slot.commit(); // DerefMut

        let consumed = slot.take();
        drop(consumed);

        slot.put(test_writer());
        let _ = slot.commit_opstamp();
        let _ = slot.commit();
    }

    #[test]
    #[should_panic(expected = "index writer missing")]
    fn writer_slot_deref_panics_while_consumed() {
        let mut slot = WriterSlot::new(test_writer());
        let _ = slot.take();
        let _ = slot.commit_opstamp();
    }
}
