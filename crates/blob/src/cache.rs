use std::collections::HashMap;
use std::sync::Mutex;

use crate::bucket::{BucketFile, BucketIndex, IndexRecord};
use crate::error::Result;

/// Cache key: (account_name, bucket_id)
type CacheKey = (String, u16);

/// LRU bucket cache. Thread-safe.
pub struct BucketCache {
    max_entries: usize,
    entries: Mutex<Vec<CacheEntry>>,
    index: Mutex<HashMap<CacheKey, usize>>,
}

struct CacheEntry {
    key: CacheKey,
    index: BucketIndex,
}

impl BucketCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries: max_entries.max(1),
            entries: Mutex::new(Vec::new()),
            index: Mutex::new(HashMap::new()),
        }
    }

    /// Get or load a bucket index. Returns the sorted, deduplicated records for the bucket.
    pub fn get_or_load(
        &self,
        account: &str,
        bucket_id: u16,
        account_dir: &std::path::Path,
    ) -> Result<Vec<IndexRecord>> {
        let key: CacheKey = (account.to_string(), bucket_id);

        // Check cache
        {
            let index = self.index.lock().unwrap();
            if let Some(&pos) = index.get(&key) {
                let entries = self.entries.lock().unwrap();
                return Ok(entries[pos].index.records.clone());
            }
        }

        // Load from disk
        let bucket_file = BucketFile::open(account_dir, bucket_id);
        let bucket_index = bucket_file.load_index()?;
        let records = bucket_index.records.clone();

        // Insert into cache
        self.insert(key, bucket_index);

        Ok(records)
    }

    fn insert(&self, key: CacheKey, index: BucketIndex) {
        let mut idx_map = self.index.lock().unwrap();
        let mut entries = self.entries.lock().unwrap();

        // If already exists, update and move to front
        if let Some(&pos) = idx_map.get(&key) {
            entries[pos].index = index;
            let entry = entries.remove(pos);
            entries.insert(0, entry);
            // Rebuild index
            idx_map.clear();
            for (i, e) in entries.iter().enumerate() {
                idx_map.insert(e.key.clone(), i);
            }
            return;
        }

        // Evict if full
        if entries.len() >= self.max_entries {
            if let Some(evicted) = entries.pop() {
                idx_map.remove(&evicted.key);
            }
        }

        // Insert at front (most recently used)
        entries.insert(0, CacheEntry { key: key.clone(), index });
        // Rebuild index (positions shifted)
        idx_map.clear();
        for (i, e) in entries.iter().enumerate() {
            idx_map.insert(e.key.clone(), i);
        }
    }

    /// Insert or update a single record in a cached bucket. If bucket not cached, no-op.
    pub fn update_record(&self, account: &str, bucket_id: u16, record: IndexRecord) {
        let key: CacheKey = (account.to_string(), bucket_id);
        let mut idx_map = self.index.lock().unwrap();

        if let Some(&pos) = idx_map.get(&key) {
            let mut entries = self.entries.lock().unwrap();
            entries[pos].index.insert(record);
            // Move to front
            let entry = entries.remove(pos);
            entries.insert(0, entry);
            // Rebuild index
            idx_map.clear();
            for (i, e) in entries.iter().enumerate() {
                idx_map.insert(e.key.clone(), i);
            }
        }
    }

    /// Invalidate a cached bucket.
    pub fn invalidate(&self, account: &str, bucket_id: u16) {
        let key: CacheKey = (account.to_string(), bucket_id);
        let mut idx_map = self.index.lock().unwrap();
        if let Some(&pos) = idx_map.get(&key) {
            let mut entries = self.entries.lock().unwrap();
            entries.remove(pos);
            idx_map.clear();
            for (i, e) in entries.iter().enumerate() {
                idx_map.insert(e.key.clone(), i);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.lock().unwrap().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bucket::IndexRecord;
    use tempfile::TempDir;

    #[test]
    fn test_cache_miss_loads_from_disk() {
        let dir = TempDir::new().unwrap();
        crate::bucket::BucketFile::ensure_dir(dir.path()).unwrap();
        let bf = BucketFile::open(dir.path(), 0);
        bf.append(&IndexRecord::new([1u8; 32], 1, 100, 50, 0))
            .unwrap();

        let cache = BucketCache::new(10);
        let records = cache
            .get_or_load("test", 0, dir.path())
            .unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_cache_hit() {
        let dir = TempDir::new().unwrap();
        crate::bucket::BucketFile::ensure_dir(dir.path()).unwrap();
        let bf = BucketFile::open(dir.path(), 0);
        bf.append(&IndexRecord::new([2u8; 32], 1, 200, 60, 0))
            .unwrap();

        let cache = BucketCache::new(10);
        let _ = cache.get_or_load("test", 0, dir.path()).unwrap();
        // Second call should hit cache
        let records = cache
            .get_or_load("test", 0, dir.path())
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_eviction() {
        let dir = TempDir::new().unwrap();
        crate::bucket::BucketFile::ensure_dir(dir.path()).unwrap();
        let cache = BucketCache::new(2);

        for b in 0..4 {
            let bf = BucketFile::open(dir.path(), b);
            bf.append(&IndexRecord::new([b as u8; 32], 1, 100, 50, 0))
                .unwrap();
            let _ = cache.get_or_load("test", b, dir.path()).unwrap();
        }

        assert!(cache.len() <= 2);
    }
}
