use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use crate::bucket::{BucketFile, BucketIndex, IndexRecord};
use crate::error::Result;

type CacheKey = (String, u16);

/// Thread-safe LRU bucket cache with single-lock interior.
/// Eliminates the TOCTOU race in the old two-Mutex design.
pub struct BucketCache {
    inner: Mutex<CacheInner>,
}

struct CacheInner {
    max_entries: usize,
    entries: Vec<CacheEntry>,
    index: HashMap<CacheKey, usize>,
}

struct CacheEntry {
    key: CacheKey,
    index: BucketIndex,
}

impl BucketCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Mutex::new(CacheInner {
                max_entries: max_entries.max(1),
                entries: Vec::new(),
                index: HashMap::new(),
            }),
        }
    }

    /// Get or load a bucket index. Eliminates TOCTOU via double-checked locking.
    pub fn get_or_load(
        &self,
        account: &str,
        bucket_id: u16,
        account_dir: &Path,
    ) -> Result<Vec<IndexRecord>> {
        let key: CacheKey = (account.to_string(), bucket_id);

        // Check cache
        {
            let inner = self.inner.lock().unwrap();
            if let Some(&pos) = inner.index.get(&key) {
                return Ok(inner.entries[pos].index.records.clone());
            }
        }

        // Load from disk
        let bucket_file = BucketFile::open(account_dir, bucket_id);
        let bucket_index = bucket_file.load_index()?;
        let records = bucket_index.records.clone();

        // Insert with double-check (another thread might have beaten us)
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(&pos) = inner.index.get(&key) {
                return Ok(inner.entries[pos].index.records.clone());
            }
            // Evict if full
            if inner.entries.len() >= inner.max_entries {
                if let Some(evicted) = inner.entries.pop() {
                    inner.index.remove(&evicted.key);
                }
            }
            // Insert at front
            inner.entries.insert(0, CacheEntry {
                key: key.clone(),
                index: bucket_index,
            });
            // Rebuild index
            inner.index.clear();
            for i in 0..inner.entries.len() {
                let key = inner.entries[i].key.clone();
                inner.index.insert(key, i);
            }
        }

        Ok(records)
    }

    /// Insert or update a single record in a cached bucket.
    pub fn update_record(&self, account: &str, bucket_id: u16, record: IndexRecord) {
        let key: CacheKey = (account.to_string(), bucket_id);
        let mut inner = self.inner.lock().unwrap();

        if let Some(&pos) = inner.index.get(&key) {
            inner.entries[pos].index.insert(record);
            // Move to front
            let entry = inner.entries.remove(pos);
            inner.entries.insert(0, entry);
            // Rebuild index
            inner.index.clear();
            for i in 0..inner.entries.len() {
                let entry_key = inner.entries[i].key.clone();
                inner.index.insert(entry_key, i);
            }
        }
    }

    /// Invalidate a cached bucket (after GC rewrites bucket files).
    pub fn invalidate(&self, account: &str, bucket_id: u16) {
        let key: CacheKey = (account.to_string(), bucket_id);
        let mut inner = self.inner.lock().unwrap();

        if let Some(&pos) = inner.index.get(&key) {
            inner.entries.remove(pos);
            inner.index.clear();
            for i in 0..inner.entries.len() {
                let entry_key = inner.entries[i].key.clone();
                inner.index.insert(entry_key, i);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().entries.is_empty()
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

    #[test]
    fn test_concurrent_get_or_load_no_deadlock() {
        use std::sync::Arc;
        use std::thread;

        let dir = TempDir::new().unwrap();
        crate::bucket::BucketFile::ensure_dir(dir.path()).unwrap();
        let bf = BucketFile::open(dir.path(), 0);
        for i in 0..10u8 {
            bf.append(&IndexRecord::new([i; 32], 1, i as u64 * 100, 50, 0))
                .unwrap();
        }

        let cache = Arc::new(BucketCache::new(10));
        let dir_path = dir.path().to_path_buf();

        let mut handles = vec![];
        for _ in 0..4 {
            let cache = cache.clone();
            let dir_path = dir_path.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let records = cache
                        .get_or_load("test", 0, &dir_path)
                        .unwrap();
                    assert_eq!(records.len(), 10);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
    }
}
