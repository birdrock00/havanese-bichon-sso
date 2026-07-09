use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use memmap2::Mmap;

use crate::error::Result;
use crate::types::{BUCKET_COUNT, INDEX_RECORD_SIZE};

/// On-disk format: 52 bytes per record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRecord {
    pub key: [u8; 32],
    pub segment_id: u32,
    pub offset: u64,
    pub data_size: u32,
    pub flags: u8,
}

impl IndexRecord {
    pub fn new(key: [u8; 32], segment_id: u32, offset: u64, data_size: u32, flags: u8) -> Self {
        Self {
            key,
            segment_id,
            offset,
            data_size,
            flags,
        }
    }

    pub fn is_tombstone(&self) -> bool {
        self.flags == 1
    }

    pub fn encode(&self) -> [u8; INDEX_RECORD_SIZE] {
        let mut buf = [0u8; INDEX_RECORD_SIZE];
        buf[0..32].copy_from_slice(&self.key);
        buf[32..36].copy_from_slice(&self.segment_id.to_le_bytes());
        buf[36..44].copy_from_slice(&self.offset.to_le_bytes());
        buf[44..48].copy_from_slice(&self.data_size.to_le_bytes());
        buf[48] = self.flags;
        buf
    }

    pub fn decode(buf: &[u8; INDEX_RECORD_SIZE]) -> Self {
        let mut key = [0u8; 32];
        key.copy_from_slice(&buf[0..32]);
        let segment_id = u32::from_le_bytes(buf[32..36].try_into().unwrap());
        let offset = u64::from_le_bytes(buf[36..44].try_into().unwrap());
        let data_size = u32::from_le_bytes(buf[44..48].try_into().unwrap());
        let flags = buf[48];
        Self {
            key,
            segment_id,
            offset,
            data_size,
            flags,
        }
    }
}

/// Compute bucket_id from a key's first 2 bytes.
pub fn bucket_id(key: &[u8; 32]) -> u16 {
    u16::from_be_bytes([key[0], key[1]]) % BUCKET_COUNT
}

// ── BucketStore ────────────────────────────────────────────────────────────

/// Per-bucket mutable state behind a Mutex.
struct BucketState {
    /// mmap of the clean, sorted, deduplicated portion of the bucket file.
    mmap: Mmap,
    /// Number of sorted records in the mmap.
    compacted_records: usize,
    /// Recent writes not yet merged into the mmap. Key → latest record.
    pending: HashMap<[u8; 32], IndexRecord>,
    /// Append-only file for durability of pending writes.
    file: File,
    /// Path to the bucket file.
    path: PathBuf,
}

/// Zero-heap bucket index store backed by mmap.
///
/// Each bucket's clean portion is mmap'd — binary search reads directly
/// from the OS page cache without allocating heap memory proportional to
/// the number of stored keys.  Only pending writes (since the last compact)
/// live in a small in-memory HashMap.
pub struct BucketStore {
    states: Vec<Mutex<BucketState>>,
    compact_threshold: usize,
}

impl BucketStore {
    /// Open all bucket files.  On first open or after a crash, each file is
    /// loaded, sorted, deduplicated, and rewritten into a clean mmap'd form.
    pub fn open(dir: &Path, compact_threshold: usize) -> Result<Self> {
        fs::create_dir_all(dir)?;

        let mut states = Vec::with_capacity(BUCKET_COUNT as usize);
        for bid in 0..BUCKET_COUNT {
            let path = bucket_path(dir, bid);
            let (mmap, compacted_records) = if path.exists() {
                // Load, sort, dedup, rewrite clean, then mmap.
                let records = load_records_from_file(&path)?;
                let deduped = sort_and_dedup(records);
                let count = deduped.len();
                rewrite_file(&path, &deduped)?;
                let file = fs::File::open(&path)?;
                let mmap = unsafe { Mmap::map(&file)? };
                (mmap, count)
            } else {
                // Create empty bucket file.
                let file = fs::File::create(&path)?;
                file.set_len(0)?;
                let mmap = unsafe { Mmap::map(&file)? };
                (mmap, 0)
            };

            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;

            states.push(Mutex::new(BucketState {
                mmap,
                compacted_records,
                pending: HashMap::new(),
                file,
                path,
            }));
        }

        Ok(Self {
            states,
            compact_threshold,
        })
    }

    /// Look up a key.  Returns the latest IndexRecord, or None if absent/tombstone.
    pub fn get(&self, key: &[u8; 32]) -> Result<Option<IndexRecord>> {
        let bid = bucket_id(key) as usize;
        let state = self.states[bid].lock().unwrap();

        // 1. Check pending (most recent wins)
        if let Some(rec) = state.pending.get(key) {
            return Ok(if rec.is_tombstone() { None } else { Some(rec.clone()) });
        }

        // 2. Binary search the mmap'd sorted portion
        let bytes: &[u8] = &state.mmap;
        if bytes.is_empty() {
            return Ok(None);
        }

        let count = bytes.len() / INDEX_RECORD_SIZE;
        let result = binary_search_records(bytes, key, count);
        match result {
            Some(idx) => {
                let rec = read_record_at(bytes, idx);
                Ok(if rec.is_tombstone() { None } else { Some(rec) })
            }
            None => Ok(None),
        }
    }

    /// Check whether a key exists (non-tombstone) in the store.
    pub fn exists(&self, key: &[u8; 32]) -> Result<bool> {
        self.get(key).map(|r| r.is_some())
    }

    /// Insert or update a record for a key.  Appends to the file for durability,
    /// then inserts into the pending HashMap.
    pub fn insert(&self, record: IndexRecord) -> Result<()> {
        let bid = bucket_id(&record.key) as usize;
        let mut state = self.states[bid].lock().unwrap();

        // Durability: append to file
        state.file.write_all(&record.encode())?;

        // Update pending
        state.pending.insert(record.key, record);

        // Auto-compact if pending grows too large
        if state.pending.len() >= self.compact_threshold {
            drop(state);
            self.compact_bucket(bid as u16)?;
        }

        Ok(())
    }

    /// Batch insert multiple records.  Appends all, then updates pending.
    pub fn insert_batch(&self, records: &[IndexRecord]) -> Result<()> {
        // Group by bucket
        let mut grouped: HashMap<u16, Vec<&IndexRecord>> = HashMap::new();
        for r in records {
            let bid = bucket_id(&r.key);
            grouped.entry(bid).or_default().push(r);
        }

        for (bid, recs) in &grouped {
            let bid_usize = *bid as usize;
            let mut state = self.states[bid_usize].lock().unwrap();

            for r in recs {
                state.file.write_all(&r.encode())?;
                state.pending.insert(r.key, (*r).clone());
            }
        }

        // Compact overfull buckets
        for &bid in grouped.keys() {
            let state = self.states[bid as usize].lock().unwrap();
            let needs_compact = state.pending.len() >= self.compact_threshold;
            drop(state);
            if needs_compact {
                self.compact_bucket(bid)?;
            }
        }

        Ok(())
    }

    /// Run stats across all buckets: total keys (non-tombstone) and total data bytes.
    pub fn total_keys(&self) -> usize {
        let mut count = 0usize;
        for state in self.states.iter() {
            let s = state.lock().unwrap();
            // Count from pending
            for r in s.pending.values() {
                if !r.is_tombstone() {
                    count += 1;
                }
            }
            // Count from mmap
            let bytes: &[u8] = &s.mmap;
            let n = bytes.len() / INDEX_RECORD_SIZE;
            for i in 0..n {
                let rec = read_record_at(bytes, i);
                // Skip keys that are overridden in pending
                if s.pending.contains_key(&rec.key) {
                    continue;
                }
                if !rec.is_tombstone() {
                    count += 1;
                }
            }
        }
        count
    }

    /// Compact a single bucket: merge mmap + pending, sort+dedup, rewrite file, remap.
    fn compact_bucket(&self, bid: u16) -> Result<()> {
        let idx = bid as usize;
        let mut state = self.states[idx].lock().unwrap();

        if state.pending.is_empty() {
            return Ok(());
        }

        // Collect mmap records + pending records
        let mut all: Vec<IndexRecord> = Vec::new();

        let bytes: &[u8] = &state.mmap;
        let n = bytes.len() / INDEX_RECORD_SIZE;
        all.reserve(n + state.pending.len());
        for i in 0..n {
            all.push(read_record_at(bytes, i));
        }
        for r in state.pending.values() {
            all.push(r.clone());
        }

        let deduped = sort_and_dedup(all);
        let new_count = deduped.len();

        // Write new file atomically
        rewrite_file(&state.path, &deduped)?;

        // Remap
        let file = fs::File::open(&state.path)?;
        let new_mmap = unsafe { Mmap::map(&file)? };

        // Re-open append file descriptor (old one was truncated)
        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&state.path)?;

        state.mmap = new_mmap;
        state.compacted_records = new_count;
        state.pending.clear();
        state.file = new_file;

        Ok(())
    }

    /// Compact all buckets.
    pub fn compact_all(&self) -> Result<()> {
        for bid in 0..BUCKET_COUNT {
            self.compact_bucket(bid)?;
        }
        Ok(())
    }

    /// Reload all mmaps from disk and reset pending state.
    /// Used after GC rewrites bucket files externally (via rebuild_from_segments).
    pub fn reload_all(&self) -> Result<()> {
        for bid in 0..BUCKET_COUNT {
            let mut state = self.states[bid as usize].lock().unwrap();
            let path = state.path.clone();

            // Load, sort, dedup, rewrite clean
            let records = load_records_from_file(&path)?;
            let deduped = sort_and_dedup(records);
            let count = deduped.len();
            rewrite_file(&path, &deduped)?;

            // Remap
            let file = fs::File::open(&path)?;
            let new_mmap = unsafe { Mmap::map(&file)? };

            // Reopen append file
            let new_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;

            state.mmap = new_mmap;
            state.compacted_records = count;
            state.pending.clear();
            state.file = new_file;
        }
        Ok(())
    }

    /// Rebuild all bucket files from scratch by scanning segment entries.
    /// Used by GC and recovery.
    pub fn rebuild_from_segments(
        dir: &Path,
        segments: &[(u32, &Path)],
    ) -> Result<()> {
        use crate::segment::SegmentReader;

        let mut bucket_records: HashMap<u16, Vec<IndexRecord>> = HashMap::new();
        for i in 0..BUCKET_COUNT {
            bucket_records.insert(i, Vec::new());
        }

        for &(seg_id, seg_path) in segments {
            if !seg_path.exists() {
                continue;
            }
            let reader = SegmentReader::open(seg_path.to_path_buf(), seg_id)?;
            reader.scan_entries(0, |entry, offset| {
                let bid = bucket_id(&entry.key);
                let rec = IndexRecord::new(
                    entry.key,
                    seg_id,
                    offset,
                    entry.data.len() as u32,
                    entry.flags,
                );
                bucket_records.entry(bid).or_default().push(rec);
                Ok(())
            })?;
        }

        for (bid, records) in &bucket_records {
            let deduped = sort_and_dedup(records.clone());
            let path = bucket_path(dir, *bid);
            rewrite_file(&path, &deduped)?;
        }

        Ok(())
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn bucket_path(dir: &Path, bid: u16) -> PathBuf {
    dir.join(format!("{:02x}.idx", bid))
}

/// Read records from a raw bucket file (may contain duplicates, not sorted).
fn load_records_from_file(path: &Path) -> Result<Vec<IndexRecord>> {
    let data = fs::read(path)?;
    let remainder = data.len() % INDEX_RECORD_SIZE;
    let count = data.len() / INDEX_RECORD_SIZE;
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let start = i * INDEX_RECORD_SIZE;
        let end = start + INDEX_RECORD_SIZE;
        let buf: &[u8; INDEX_RECORD_SIZE] = data[start..end].try_into().map_err(|_| {
            crate::error::Error::BucketIndexCorrupt {
                path: path.to_path_buf(),
                reason: "unexpected file size".into(),
            }
        })?;
        records.push(IndexRecord::decode(buf));
    }
    if remainder > 0 {
        tracing::warn!(
            "Bucket file {:?} has {} trailing bytes, ignoring",
            path,
            remainder
        );
    }
    Ok(records)
}

/// Sort records by key, deduplicate keeping the one with the highest (segment_id, offset).
fn sort_and_dedup(mut records: Vec<IndexRecord>) -> Vec<IndexRecord> {
    records.sort_by_key(|a| a.key);
    let mut out = Vec::with_capacity(records.len());
    let mut i = 0;
    while i < records.len() {
        let mut best = i;
        let mut j = i + 1;
        while j < records.len() && records[j].key == records[i].key {
            if records[j].segment_id > records[best].segment_id
                || (records[j].segment_id == records[best].segment_id
                    && records[j].offset > records[best].offset)
            {
                best = j;
            }
            j += 1;
        }
        out.push(records[best].clone());
        i = j;
    }
    out
}

/// Binary search for `key` in `bytes` (array of INDEX_RECORD_SIZE records, sorted).
fn binary_search_records(bytes: &[u8], key: &[u8; 32], count: usize) -> Option<usize> {
    let mut lo = 0usize;
    let mut hi = count;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let rec_key = read_key_at(bytes, mid);
        match rec_key.cmp(key) {
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
            std::cmp::Ordering::Equal => return Some(mid),
        }
    }
    None
}

/// Read the key at index `idx` from a byte slice of INDEX_RECORD_SIZE records.
fn read_key_at(bytes: &[u8], idx: usize) -> &[u8; 32] {
    let start = idx * INDEX_RECORD_SIZE;
    bytes[start..start + 32].try_into().unwrap()
}

/// Read a full IndexRecord at index `idx`.
fn read_record_at(bytes: &[u8], idx: usize) -> IndexRecord {
    let start = idx * INDEX_RECORD_SIZE;
    let buf: &[u8; INDEX_RECORD_SIZE] = bytes[start..start + INDEX_RECORD_SIZE]
        .try_into()
        .unwrap();
    IndexRecord::decode(buf)
}

/// Atomically rewrite a bucket file with sorted, deduplicated records.
fn rewrite_file(path: &Path, records: &[IndexRecord]) -> Result<()> {
    let mut buf = Vec::with_capacity(records.len() * INDEX_RECORD_SIZE);
    for r in records {
        buf.extend_from_slice(&r.encode());
    }
    crate::fs::create_atomic(path, &buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_index_record_encode_decode() {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&[1, 2, 3, 4]);
        let rec = IndexRecord::new(key, 5, 12345, 500, 0);
        let encoded = rec.encode();
        let decoded = IndexRecord::decode(&encoded);
        assert_eq!(rec, decoded);
    }

    #[test]
    fn test_bucket_id_deterministic() {
        let mut key = [0u8; 32];
        key[0] = 0x00;
        key[1] = 0x0F;
        assert_eq!(bucket_id(&key), 15);
        key[0] = 0x00;
        key[1] = 0x10;
        assert_eq!(bucket_id(&key), 16);
    }

    #[test]
    fn test_sort_and_dedup_keeps_latest() {
        let recs = vec![
            IndexRecord::new([1u8; 32], 1, 100, 50, 0),
            IndexRecord::new([1u8; 32], 2, 200, 50, 0),
            IndexRecord::new([2u8; 32], 1, 300, 60, 0),
        ];
        let deduped = sort_and_dedup(recs);
        assert_eq!(deduped.len(), 2);
        let found = &deduped[0];
        assert_eq!(found.segment_id, 2);
        assert_eq!(found.offset, 200);
    }

    #[test]
    fn test_bucket_store_put_and_get() {
        let dir = TempDir::new().unwrap();
        let store = BucketStore::open(dir.path(), 100).unwrap();

        let key = [0xAA; 32];
        let rec = IndexRecord::new(key, 1, 0, 500, 0);
        store.insert(rec).unwrap();

        let found = store.get(&key).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().data_size, 500);
    }

    #[test]
    fn test_bucket_store_get_missing() {
        let dir = TempDir::new().unwrap();
        let store = BucketStore::open(dir.path(), 100).unwrap();

        let result = store.get(&[0xFF; 32]).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_bucket_store_tombstone() {
        let dir = TempDir::new().unwrap();
        let store = BucketStore::open(dir.path(), 100).unwrap();

        let key = [0xBB; 32];

        // Write then tombstone
        store
            .insert(IndexRecord::new(key, 1, 0, 100, 0))
            .unwrap();
        store
            .insert(IndexRecord::new(key, 2, 0, 0, 1))
            .unwrap();

        let result = store.get(&key).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_bucket_store_compact() {
        let dir = TempDir::new().unwrap();
        // Use small threshold to trigger auto-compact
        let store = BucketStore::open(dir.path(), 5).unwrap();

        // Write 10 records for same bucket (all same first 2 bytes → same bucket)
        for i in 0u8..10 {
            let mut key = [0u8; 32];
            key[0..2].copy_from_slice(&[0x00, 0x00]); // same bucket
            key[2] = i;
            store
                .insert(IndexRecord::new(key, 1, i as u64 * 100, 50, 0))
                .unwrap();
        }

        // All should be readable after auto-compact
        for i in 0u8..10 {
            let mut key = [0u8; 32];
            key[0..2].copy_from_slice(&[0x00, 0x00]);
            key[2] = i;
            let found = store.get(&key).unwrap();
            assert!(found.is_some(), "key {} should exist after compact", i);
        }
    }

    #[test]
    fn test_bucket_store_persistence() {
        let dir = TempDir::new().unwrap();
        let dir_path = dir.path().to_path_buf();

        let key = [0xCC; 32];
        {
            let store = BucketStore::open(&dir_path, 100).unwrap();
            store
                .insert(IndexRecord::new(key, 1, 42, 512, 0))
                .unwrap();
            store.compact_all().unwrap();
        }

        // Reopen
        {
            let store = BucketStore::open(&dir_path, 100).unwrap();
            let found = store.get(&key).unwrap();
            assert!(found.is_some());
            assert_eq!(found.unwrap().offset, 42);
        }
    }
}
