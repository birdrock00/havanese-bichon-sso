use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

use crate::store::tantivy::envelope::ENVELOPE_MANAGER;
use crate::store::tantivy::fields::{F_ACCOUNT_ID, F_CONTENT_HASH, F_INGEST_AT, F_MAILBOX_ID};
use crate::utc_now;

/// Max entries before evicting the oldest.
/// At ~152 bytes/entry, 300_000 ≈ 45 MB, within the 50 MB budget.
const MAX_ENTRIES: usize = 300_000;

/// Fraction of entries to keep when evicting (newest 3/4).
const KEEP_FRACTION_NUM: usize = 3;
const KEEP_FRACTION_DEN: usize = 4;

/// Populate only loads entries ingested within this window.
const POPULATE_WINDOW_MS: i64 = 7 * 24 * 60 * 60 * 1000; // 7 days

pub static DEDUP_CACHE: LazyLock<DedupCache> = LazyLock::new(DedupCache::new);

pub struct DedupCache {
    entries: Mutex<HashMap<(u64, u64, String), i64>>,
    max_entries: usize,
    populated: AtomicBool,
}

impl DedupCache {
    fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_entries: MAX_ENTRIES,
            populated: AtomicBool::new(false),
        }
    }

    #[cfg(test)]
    fn new_for_test() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_entries: MAX_ENTRIES,
            populated: AtomicBool::new(true),
        }
    }

    #[cfg(test)]
    fn new_for_test_small(max_entries: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_entries,
            populated: AtomicBool::new(true),
        }
    }

    /// Returns true if this `(account_id, mailbox_id, content_hash)` triple
    /// has already been seen.
    ///
    /// On the very first call the cache is populated from the Tantivy index
    /// FAST columns (only entries ingested within [`POPULATE_WINDOW_MS`]).
    /// If that scan fails the cache starts empty and still operates correctly
    /// for newly-arriving emails.
    pub fn contains(&self, account_id: u64, mailbox_id: u64, hash: &str) -> bool {
        self.ensure_populated();

        let entries = self.entries.lock().unwrap();
        entries.contains_key(&(account_id, mailbox_id, hash.to_string()))
    }

    /// Insert a triple into the cache after it has been queued for indexing.
    ///
    /// Each entry is stamped with the current time. When the cache exceeds
    /// [`MAX_ENTRIES`], the oldest entries are evicted, keeping the newest
    /// `MAX_ENTRIES * 3/4`.
    pub fn insert(&self, account_id: u64, mailbox_id: u64, hash: &str) {
        let mut entries = self.entries.lock().unwrap();
        let now = utc_now!();
        entries.insert((account_id, mailbox_id, hash.to_string()), now);

        if entries.len() > self.max_entries {
            let keep = self.max_entries * KEEP_FRACTION_NUM / KEEP_FRACTION_DEN;
            let mut vec: Vec<_> = entries.drain().collect();
            // Sort descending by timestamp (newest first)
            vec.sort_by(|a, b| b.1.cmp(&a.1));
            for (k, v) in vec.into_iter().take(keep) {
                entries.insert(k, v);
            }
            tracing::warn!(
                "DedupCache evicted oldest entries, kept {}/{}",
                entries.len(),
                keep
            );
        }
    }

    // ── private ──────────────────────────────────────────────────────────────

    fn ensure_populated(&self) {
        if self.populated.load(Ordering::Acquire) {
            return;
        }
        self.do_populate();
    }

    fn do_populate(&self) {
        if self
            .populated
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            return;
        }

        let reader = match ENVELOPE_MANAGER.create_reader() {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("DedupCache: failed to create reader for populate: {e}");
                return;
            }
        };

        let searcher = reader.searcher();
        let cutoff = utc_now!() - POPULATE_WINDOW_MS;
        let mut entries = self.entries.lock().unwrap();

        for segment_reader in searcher.segment_readers() {
            let account_col = match segment_reader.fast_fields().u64(F_ACCOUNT_ID) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mailbox_col = match segment_reader.fast_fields().u64(F_MAILBOX_ID) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let hash_col = match segment_reader.fast_fields().str(F_CONTENT_HASH) {
                Ok(Some(c)) => c,
                _ => continue,
            };
            let ingest_col = match segment_reader.fast_fields().i64(F_INGEST_AT) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let max_doc = segment_reader.max_doc();
            for doc_id in 0..max_doc {
                if segment_reader.is_deleted(doc_id) {
                    continue;
                }
                let ingest_at = ingest_col.values.get_val(doc_id);
                if ingest_at < cutoff {
                    continue;
                }
                let account_id = account_col.values.get_val(doc_id);
                let mailbox_id = mailbox_col.values.get_val(doc_id);

                let hash_ord = hash_col
                    .ords()
                    .values_for_doc(doc_id as u32)
                    .next()
                    .unwrap_or(0);
                let mut hash_buf = String::new();
                if hash_col.ord_to_str(hash_ord, &mut hash_buf).is_err() {
                    continue;
                }

                entries.insert((account_id, mailbox_id, hash_buf), ingest_at);
            }
        }

        tracing::info!(
            "DedupCache populated with {} entries from index (cutoff {}d ago)",
            entries.len(),
            POPULATE_WINDOW_MS / (24 * 60 * 60 * 1000),
        );
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tantivy::fields::EmailFields;
    use crate::store::tantivy::schema::SchemaTools;
    use crate::store::tantivy::tokenizers::EuroTokenizer;
    use std::fs;
    use tantivy::{Index, TantivyDocument};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join("bichon-dedup-cache-test")
            .join(name)
            .join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ── basic contains / insert ─────────────────────────────────────────────

    #[test]
    fn contains_after_insert() {
        let cache = DedupCache::new_for_test();
        assert!(!cache.contains(1, 10, "hash-aaa"));
        cache.insert(1, 10, "hash-aaa");
        assert!(cache.contains(1, 10, "hash-aaa"));
    }

    #[test]
    fn different_hash_not_matched() {
        let cache = DedupCache::new_for_test();
        cache.insert(1, 10, "hash-aaa");
        assert!(!cache.contains(1, 10, "hash-bbb"));
    }

    #[test]
    fn different_account_not_matched() {
        let cache = DedupCache::new_for_test();
        cache.insert(1, 10, "hash-aaa");
        assert!(!cache.contains(2, 10, "hash-aaa"));
    }

    #[test]
    fn different_mailbox_not_matched() {
        let cache = DedupCache::new_for_test();
        cache.insert(1, 10, "hash-aaa");
        assert!(!cache.contains(1, 20, "hash-aaa"));
    }

    #[test]
    fn cross_account_allowed() {
        let cache = DedupCache::new_for_test();
        cache.insert(1, 10, "hash-aaa");
        cache.insert(2, 10, "hash-aaa");
        assert!(cache.contains(2, 10, "hash-aaa"));
        assert!(cache.contains(1, 10, "hash-aaa"));
    }

    #[test]
    fn cross_mailbox_allowed() {
        let cache = DedupCache::new_for_test();
        cache.insert(1, 10, "hash-aaa");
        cache.insert(1, 20, "hash-aaa");
        assert!(cache.contains(1, 20, "hash-aaa"));
        assert!(cache.contains(1, 10, "hash-aaa"));
    }

    // ── time-based eviction ─────────────────────────────────────────────────

    #[test]
    fn eviction_keeps_newest() {
        let cap = 100;
        let cache = DedupCache::new_for_test_small(cap);

        // Fill to exact capacity. Entry hash-0 is oldest.
        for i in 0..cap {
            cache.insert(1, 1, &format!("hash-{}", i));
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        assert!(cache.contains(1, 1, "hash-0"));
        assert!(cache.contains(1, 1, &format!("hash-{}", cap - 1)));

        // One more triggers eviction
        cache.insert(1, 1, "hash-overflow");

        // Newest survives, oldest evicted
        assert!(cache.contains(1, 1, "hash-overflow"));
        assert!(cache.contains(1, 1, &format!("hash-{}", cap - 1)));
        assert!(!cache.contains(1, 1, "hash-0"));

        let keep = cap * KEEP_FRACTION_NUM / KEEP_FRACTION_DEN;
        assert!(cache.entries.lock().unwrap().len() <= keep);
    }

    // ── memory bound ────────────────────────────────────────────────────────

    #[test]
    fn memory_bound_within_budget() {
        let cache = DedupCache::new_for_test();

        for i in 0..MAX_ENTRIES {
            cache.insert(1, 1, &format!("{:064x}", i));
        }

        let entries = cache.entries.lock().unwrap();
        assert_eq!(entries.len(), MAX_ENTRIES);

        let capacity = entries.capacity();
        // HashMap with (u64,u64,String) key + i64 value ≈ 112 + map overhead
        let approx_bytes = capacity * (104 + 8 + 8);
        let approx_mb = approx_bytes as f64 / (1024.0 * 1024.0);
        println!(
            "DedupCache: {} entries, {} buckets, ~{:.1} MB",
            MAX_ENTRIES, capacity, approx_mb
        );
        assert!(
            approx_mb < 55.0,
            "memory estimate {:.1} MB exceeds 55 MB buffer",
            approx_mb
        );
    }

    // ── populate guard ──────────────────────────────────────────────────────

    #[test]
    fn populate_cas_is_idempotent() {
        let cache = DedupCache::new_for_test();
        assert!(cache.populated.load(Ordering::Acquire));
        cache.ensure_populated();
        assert!(cache.populated.load(Ordering::Acquire));
        cache.do_populate();
    }

    // ── populate from test index ────────────────────────────────────────────

    fn build_test_index() -> (Index, &'static EmailFields) {
        let dir = temp_dir("populate");
        let schema = SchemaTools::email_schema();
        let fields = SchemaTools::email_fields();
        let index = Index::create_in_dir(&dir, schema).unwrap();
        index.tokenizers().register("euro", EuroTokenizer::new());
        (index, fields)
    }

    fn add_email_doc(
        fields: &EmailFields,
        writer: &mut tantivy::IndexWriter,
        account: u64,
        mailbox: u64,
        hash: &str,
        ingest_at: i64,
    ) {
        let mut doc = TantivyDocument::new();
        doc.add_u64(fields.f_account_id, account);
        doc.add_u64(fields.f_mailbox_id, mailbox);
        doc.add_text(fields.f_content_hash, hash);
        doc.add_i64(fields.f_ingest_at, ingest_at);
        doc.add_text(fields.f_id, &uuid::Uuid::new_v4().to_string());
        doc.add_text(fields.f_subject, "test");
        doc.add_text(fields.f_body, "test body");
        doc.add_u64(fields.f_uid, 1);
        doc.add_i64(fields.f_date, 1);
        doc.add_i64(fields.f_internal_date, 1);
        doc.add_u64(fields.f_size, 100);
        writer.add_document(doc).unwrap();
    }

    #[test]
    fn populate_reads_all_docs_in_window() {
        let (index, fields) = build_test_index();
        let mut writer = index.writer_with_num_threads(1, 50_000_000).unwrap();

        let recent = utc_now!();
        add_email_doc(&fields, &mut writer, 1, 10, "hash-recent", recent);
        add_email_doc(&fields, &mut writer, 2, 10, "hash-recent", recent);
        add_email_doc(&fields, &mut writer, 1, 20, "hash-recent", recent);
        writer.commit().unwrap();
        drop(writer);

        let reader = index.reader().unwrap();
        let cache = DedupCache::new_for_test();
        {
            let searcher = reader.searcher();
            let cutoff = utc_now!() - POPULATE_WINDOW_MS;
            let mut entries = cache.entries.lock().unwrap();
            entries.clear();

            for segment_reader in searcher.segment_readers() {
                let account_col = segment_reader.fast_fields().u64(F_ACCOUNT_ID).unwrap();
                let mailbox_col = segment_reader.fast_fields().u64(F_MAILBOX_ID).unwrap();
                let hash_col = segment_reader.fast_fields().str(F_CONTENT_HASH).unwrap().unwrap();
                let ingest_col = segment_reader.fast_fields().i64(F_INGEST_AT).unwrap();

                let max_doc = segment_reader.max_doc();
                for doc_id in 0..max_doc {
                    if segment_reader.is_deleted(doc_id) {
                        continue;
                    }
                    let ingest_at = ingest_col.values.get_val(doc_id);
                    if ingest_at < cutoff {
                        continue;
                    }
                    let account_id = account_col.values.get_val(doc_id);
                    let mailbox_id = mailbox_col.values.get_val(doc_id);

                    let hash_ord = hash_col.ords().values_for_doc(doc_id as u32).next().unwrap_or(0);
                    let mut hash_buf = String::new();
                    hash_col.ord_to_str(hash_ord, &mut hash_buf).unwrap();

                    entries.insert((account_id, mailbox_id, hash_buf), ingest_at);
                }
            }
        }

        assert_eq!(cache.entries.lock().unwrap().len(), 3);
        assert!(cache.contains(1, 10, "hash-recent"));
        assert!(cache.contains(2, 10, "hash-recent"));
        assert!(cache.contains(1, 20, "hash-recent"));
    }

    #[test]
    fn populate_skips_old_entries() {
        let (index, fields) = build_test_index();
        let mut writer = index.writer_with_num_threads(1, 50_000_000).unwrap();

        let recent = utc_now!();
        let old = recent - POPULATE_WINDOW_MS - 60_000; // 1 minute past the window
        add_email_doc(&fields, &mut writer, 1, 10, "hash-recent", recent);
        add_email_doc(&fields, &mut writer, 1, 10, "hash-old", old);
        writer.commit().unwrap();
        drop(writer);

        let reader = index.reader().unwrap();
        let cache = DedupCache::new_for_test();
        {
            let searcher = reader.searcher();
            let cutoff = utc_now!() - POPULATE_WINDOW_MS;
            let mut entries = cache.entries.lock().unwrap();
            entries.clear();

            for segment_reader in searcher.segment_readers() {
                let account_col = segment_reader.fast_fields().u64(F_ACCOUNT_ID).unwrap();
                let mailbox_col = segment_reader.fast_fields().u64(F_MAILBOX_ID).unwrap();
                let hash_col = segment_reader.fast_fields().str(F_CONTENT_HASH).unwrap().unwrap();
                let ingest_col = segment_reader.fast_fields().i64(F_INGEST_AT).unwrap();

                let max_doc = segment_reader.max_doc();
                for doc_id in 0..max_doc {
                    if segment_reader.is_deleted(doc_id) {
                        continue;
                    }
                    let ingest_at = ingest_col.values.get_val(doc_id);
                    if ingest_at < cutoff {
                        continue;
                    }
                    let account_id = account_col.values.get_val(doc_id);
                    let mailbox_id = mailbox_col.values.get_val(doc_id);

                    let hash_ord = hash_col.ords().values_for_doc(doc_id as u32).next().unwrap_or(0);
                    let mut hash_buf = String::new();
                    hash_col.ord_to_str(hash_ord, &mut hash_buf).unwrap();

                    entries.insert((account_id, mailbox_id, hash_buf), ingest_at);
                }
            }
        }

        assert!(cache.contains(1, 10, "hash-recent"));
        assert!(!cache.contains(1, 10, "hash-old"));
        assert_eq!(cache.entries.lock().unwrap().len(), 1);
    }

    #[test]
    fn populate_skips_deleted_docs() {
        let (index, fields) = build_test_index();
        let mut writer = index.writer_with_num_threads(1, 50_000_000).unwrap();

        let recent = utc_now!();
        add_email_doc(&fields, &mut writer, 1, 10, "hash-keep", recent);
        add_email_doc(&fields, &mut writer, 1, 10, "hash-delete", recent);
        writer.commit().unwrap();

        let term = tantivy::Term::from_field_text(fields.f_content_hash, "hash-delete");
        writer.delete_term(term);
        writer.commit().unwrap();
        drop(writer);

        let reader = index.reader().unwrap();
        let cache = DedupCache::new_for_test();
        {
            let searcher = reader.searcher();
            let cutoff = utc_now!() - POPULATE_WINDOW_MS;
            let mut entries = cache.entries.lock().unwrap();
            entries.clear();

            for segment_reader in searcher.segment_readers() {
                let account_col = segment_reader.fast_fields().u64(F_ACCOUNT_ID).unwrap();
                let mailbox_col = segment_reader.fast_fields().u64(F_MAILBOX_ID).unwrap();
                let hash_col = segment_reader.fast_fields().str(F_CONTENT_HASH).unwrap().unwrap();
                let ingest_col = segment_reader.fast_fields().i64(F_INGEST_AT).unwrap();

                let max_doc = segment_reader.max_doc();
                for doc_id in 0..max_doc {
                    if segment_reader.is_deleted(doc_id) {
                        continue;
                    }
                    let ingest_at = ingest_col.values.get_val(doc_id);
                    if ingest_at < cutoff {
                        continue;
                    }
                    let account_id = account_col.values.get_val(doc_id);
                    let mailbox_id = mailbox_col.values.get_val(doc_id);

                    let hash_ord = hash_col.ords().values_for_doc(doc_id as u32).next().unwrap_or(0);
                    let mut hash_buf = String::new();
                    hash_col.ord_to_str(hash_ord, &mut hash_buf).unwrap();

                    entries.insert((account_id, mailbox_id, hash_buf), ingest_at);
                }
            }
        }

        assert!(cache.contains(1, 10, "hash-keep"));
        assert!(!cache.contains(1, 10, "hash-delete"));
    }
}
