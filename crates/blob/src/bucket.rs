use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

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
        // bytes 49..52 are padding (keep zero)
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

/// Represents a loaded and deduplicated bucket in memory.
pub struct BucketIndex {
    pub bucket_id: u16,
    /// Records sorted by key, deduplicated (one record per key, latest wins).
    pub records: Vec<IndexRecord>,
}

impl BucketIndex {
    /// Build from raw records: sort by key, dedup keeping the one with max offset.
    pub fn from_records(mut records: Vec<IndexRecord>, bucket_id: u16) -> Self {
        records.sort_by_key(|a| a.key);
        // Dedup: keep last (max offset) for each key
        let mut deduped = Vec::with_capacity(records.len());
        let mut i = 0;
        while i < records.len() {
            let mut best = i;
            let mut j = i + 1;
            while j < records.len() && records[j].key == records[i].key {
                if records[j].offset > records[best].offset {
                    best = j;
                }
                j += 1;
            }
            deduped.push(records[best].clone());
            i = j;
        }
        Self {
            bucket_id,
            records: deduped,
        }
    }

    /// Binary search for a key. Returns the record if found.
    pub fn find(&self, key: &[u8; 32]) -> Option<&IndexRecord> {
        match self.records.binary_search_by(|r| r.key.cmp(key)) {
            Ok(idx) => Some(&self.records[idx]),
            Err(_) => None,
        }
    }

    /// Append a new record and maintain sorted order.
    pub fn insert(&mut self, record: IndexRecord) {
        match self.records.binary_search_by(|r| r.key.cmp(&record.key)) {
            Ok(idx) => {
                // Replace if newer (larger offset)
                if record.offset > self.records[idx].offset {
                    self.records[idx] = record;
                }
            }
            Err(idx) => {
                self.records.insert(idx, record);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// Manages a bucket index file on disk.
pub struct BucketFile {
    path: PathBuf,
    bucket_id: u16,
}

impl BucketFile {
    pub fn path_for(account_dir: &Path, bucket_id: u16) -> PathBuf {
        account_dir.join("buckets").join(format!("{:02x}.idx", bucket_id))
    }

    pub fn open(account_dir: &Path, bucket_id: u16) -> Self {
        Self {
            path: Self::path_for(account_dir, bucket_id),
            bucket_id,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bucket_id(&self) -> u16 {
        self.bucket_id
    }

    /// Ensure the buckets directory exists.
    pub fn ensure_dir(account_dir: &Path) -> Result<()> {
        let dir = account_dir.join("buckets");
        std::fs::create_dir_all(&dir)?;
        Ok(())
    }

    /// Append a single record to the bucket file.
    pub fn append(&self, record: &IndexRecord) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(&record.encode())?;
        Ok(())
    }

    /// Append multiple records at once.
    pub fn append_batch(&self, records: &[IndexRecord]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        for r in records {
            file.write_all(&r.encode())?;
        }
        Ok(())
    }

    /// Load all records from the bucket file.
    /// If the file size is not a multiple of INDEX_RECORD_SIZE (partial write),
    /// the trailing bytes are silently ignored.
    pub fn load_all(&self) -> Result<Vec<IndexRecord>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read(&self.path)?;
        let remainder = data.len() % INDEX_RECORD_SIZE;
        let count = data.len() / INDEX_RECORD_SIZE;
        let mut records = Vec::with_capacity(count);
        for i in 0..count {
            let start = i * INDEX_RECORD_SIZE;
            let end = start + INDEX_RECORD_SIZE;
            let buf: &[u8; INDEX_RECORD_SIZE] = data[start..end]
                .try_into()
                .map_err(|_| crate::error::Error::BucketIndexCorrupt {
                    path: self.path.clone(),
                    reason: format!("unexpected file size {}, not a multiple of {}", data.len(), INDEX_RECORD_SIZE),
                })?;
            records.push(IndexRecord::decode(buf));
        }
        if remainder > 0 {
            tracing::warn!(
                "Bucket file {:?} has {} trailing bytes (expected multiple of {}), ignoring",
                self.path, remainder, INDEX_RECORD_SIZE
            );
        }
        Ok(records)
    }

    /// Load all records, sort, and deduplicate into a BucketIndex.
    pub fn load_index(&self) -> Result<BucketIndex> {
        let records = self.load_all()?;
        Ok(BucketIndex::from_records(records, self.bucket_id))
    }

    /// Rewrite the bucket file with a sorted, deduplicated set of records.
    pub fn rewrite(&self, records: &[IndexRecord]) -> Result<()> {
        let temp_path = self.path.with_extension("idx.tmp");
        {
            let mut file = File::create(&temp_path)?;
            for r in records {
                file.write_all(&r.encode())?;
            }
            file.sync_all()?;
        }
        std::fs::rename(&temp_path, &self.path)?;
        Ok(())
    }

    /// Delete the bucket file.
    pub fn delete(&self) -> Result<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

/// Compute bucket_id from a key's first 2 bytes.
pub fn bucket_id(key: &[u8; 32]) -> u16 {
    u16::from_be_bytes([key[0], key[1]]) % BUCKET_COUNT
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
        assert_eq!(bucket_id(&key), 0);
    }

    #[test]
    fn test_bucket_append_and_load() {
        let dir = TempDir::new().unwrap();
        let bucket = BucketFile::open(dir.path(), 0);
        BucketFile::ensure_dir(dir.path()).unwrap();

        let r1 = IndexRecord::new([1u8; 32], 1, 100, 50, 0);
        let r2 = IndexRecord::new([2u8; 32], 1, 200, 60, 0);

        bucket.append(&r1).unwrap();
        bucket.append(&r2).unwrap();

        let loaded = bucket.load_all().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].key, [1u8; 32]);
        assert_eq!(loaded[1].key, [2u8; 32]);
    }

    #[test]
    fn test_bucket_index_dedup() {
        let recs = vec![
            IndexRecord::new([1u8; 32], 1, 100, 50, 0),
            IndexRecord::new([1u8; 32], 2, 200, 50, 0), // newer offset wins
            IndexRecord::new([2u8; 32], 1, 300, 60, 0),
        ];
        let idx = BucketIndex::from_records(recs, 0);
        assert_eq!(idx.len(), 2);
        let found = idx.find(&[1u8; 32]).unwrap();
        assert_eq!(found.segment_id, 2);
        assert_eq!(found.offset, 200);
    }

    #[test]
    fn test_bucket_index_find_missing() {
        let recs = vec![IndexRecord::new([1u8; 32], 1, 100, 50, 0)];
        let idx = BucketIndex::from_records(recs, 0);
        assert!(idx.find(&[99u8; 32]).is_none());
    }

    #[test]
    fn test_bucket_rewrite() {
        let dir = TempDir::new().unwrap();
        let bucket = BucketFile::open(dir.path(), 0);
        BucketFile::ensure_dir(dir.path()).unwrap();

        let r1 = IndexRecord::new([3u8; 32], 1, 300, 70, 0);
        let r2 = IndexRecord::new([1u8; 32], 1, 100, 50, 0);
        bucket.append(&r1).unwrap();
        bucket.append(&r2).unwrap();

        // Rewrite sorted
        let sorted = vec![r2.clone(), r1.clone()];
        bucket.rewrite(&sorted).unwrap();

        let loaded = bucket.load_all().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].key, [1u8; 32]);
        assert_eq!(loaded[1].key, [3u8; 32]);
    }
}
