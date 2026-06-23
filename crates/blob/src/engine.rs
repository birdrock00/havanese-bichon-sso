use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::account::AccountHandle;
use crate::bucket::{self, IndexRecord};
use crate::cache::BucketCache;
use crate::compress;
use crate::error::{Error, Result};
use crate::gc::{self, GcStats};
use crate::meta::GlobalMeta;
use crate::segment::SegmentReader;
use crate::types::{Codec, Config, ENTRY_HEADER_SIZE};

pub struct Engine {
    root: PathBuf,
    config: Config,
    cache: BucketCache,
    accounts: RwLock<HashMap<String, Arc<AccountHandle>>>,
}

#[derive(Debug, Clone)]
pub struct AccountStats {
    pub account_id: String,
    pub total_keys: u64,
    pub total_bytes: u64,
    pub deleted_bytes: u64,
    pub segment_count: usize,
}

impl Engine {
    pub fn open(path: &Path, config: Config) -> Result<Self> {
        config.validate()?;
        fs::create_dir_all(path)?;
        fs::create_dir_all(path.join("accounts"))?;

        let mut global = GlobalMeta::load(path)?;
        global.save(path)?;

        let cache = BucketCache::new(config.lru_bucket_count);

        let accounts_dir = path.join("accounts");
        let mut accounts = HashMap::new();

        if accounts_dir.exists() {
            for entry in fs::read_dir(&accounts_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    let account_name = entry.file_name().to_string_lossy().into_owned();

                    let _ = crate::recovery::cleanup_temp_files(&entry.path());

                    match crate::recovery::recover_account(&entry.path()) {
                        Ok(_meta) => {
                            match AccountHandle::open(path, &account_name) {
                                Ok(handle) => {
                                    accounts.insert(account_name, handle);
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to open account {}: {}",
                                        account_name,
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to recover account {}: {}",
                                account_name,
                                e
                            );
                        }
                    }
                }
            }
        }

        global.accounts = accounts.keys().cloned().collect();
        global.save(path)?;

        Ok(Self {
            root: path.to_path_buf(),
            config,
            cache,
            accounts: RwLock::new(accounts),
        })
    }

    // ── Account management ──────────────────────────────────────────────

    pub fn create_account(&self, account_id: &str) -> Result<()> {
        let mut accounts = self.accounts.write().unwrap();
        if accounts.contains_key(account_id) {
            return Err(Error::AccountAlreadyExists(account_id.to_string()));
        }
        let handle = AccountHandle::create(&self.root, account_id)?;
        accounts.insert(account_id.to_string(), handle);

        let mut global = GlobalMeta::load(&self.root)?;
        global.accounts = accounts.keys().cloned().collect();
        global.save(&self.root)?;

        Ok(())
    }

    pub fn delete_account(&self, account_id: &str) -> Result<()> {
        let mut accounts = self.accounts.write().unwrap();
        let handle = accounts
            .remove(account_id)
            .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?;

        let account_dir = handle.dir().to_path_buf();
        drop(handle);
        fs::remove_dir_all(&account_dir)?;

        let mut global = GlobalMeta::load(&self.root)?;
        global.accounts = accounts.keys().cloned().collect();
        global.save(&self.root)?;

        Ok(())
    }

    pub fn list_accounts(&self) -> Vec<String> {
        let accounts = self.accounts.read().unwrap();
        accounts.keys().cloned().collect()
    }

    // ── Read / Write / Delete ───────────────────────────────────────────

    pub fn write(
        &self,
        account_id: &str,
        key: [u8; 32],
        value: &[u8],
        codec: Codec,
    ) -> Result<()> {
        if value.len() > crate::types::MAX_VALUE_SIZE {
            return Err(Error::ValueTooLarge { size: value.len() });
        }

        let handle = {
            let accounts = self.accounts.read().unwrap();
            accounts
                .get(account_id)
                .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?
                .clone()
        };

        let _write_lock = handle.write_mutex.lock().unwrap();
        let mut inner = handle.write();

        let (data, actual_codec) =
            compress::compress(value, codec, self.config.compress_threshold, self.config.compression_level);

        let (segment_id, offset, data_size) =
            inner.write_entry(key, &data, 0, actual_codec)?;

        let record = IndexRecord::new(key, segment_id, offset, data_size, 0);
        inner.append_index(&record)?;

        let entry_end = offset + ENTRY_HEADER_SIZE as u64 + data_size as u64;
        inner.mark_indexed(segment_id, entry_end)?;

        let bucket_id = bucket::bucket_id(&key);
        self.cache.update_record(account_id, bucket_id, record);

        Ok(())
    }

    pub fn read(&self, account_id: &str, key: &[u8; 32]) -> Result<Option<Vec<u8>>> {
        let bucket_id = bucket::bucket_id(key);

        let handle = {
            let accounts = self.accounts.read().unwrap();
            accounts
                .get(account_id)
                .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?
                .clone()
        };

        let (record, seg_path): (IndexRecord, PathBuf) = {
            let inner = handle.read();
            let records = self
                .cache
                .get_or_load(account_id, bucket_id, handle.dir())?;
            match records.binary_search_by(|r| r.key.cmp(key)) {
                Ok(idx) => {
                    let r = records[idx].clone();
                    if r.is_tombstone() {
                        return Ok(None);
                    }
                    let seg_path = inner.segment_path(r.segment_id)?;
                    (r, seg_path)
                }
                Err(_) => return Ok(None),
            }
        };

        if !seg_path.exists() {
            return Err(Error::SegmentNotFound(record.segment_id));
        }

        let reader = SegmentReader::open(seg_path.clone(), record.segment_id)?;
        let file = handle.get_segment_file(record.segment_id, &seg_path)?;
        let (entry, _) = reader.read_entry_at_file(record.offset, &file)?;

        let value = compress::decompress(&entry.data, entry.codec, entry.raw_size as usize)?;

        Ok(Some(value))
    }

    pub fn delete(&self, account_id: &str, key: &[u8; 32]) -> Result<()> {
        let handle = {
            let accounts = self.accounts.read().unwrap();
            accounts
                .get(account_id)
                .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?
                .clone()
        };

        let _write_lock = handle.write_mutex.lock().unwrap();
        let mut inner = handle.write();

        let (segment_id, offset, data_size) =
            inner.write_entry(*key, &[], 1, Codec::None)?;

        let record = IndexRecord::new(*key, segment_id, offset, data_size, 1);
        inner.append_index(&record)?;

        let entry_end = offset + ENTRY_HEADER_SIZE as u64 + data_size as u64;
        inner.mark_indexed(segment_id, entry_end)?;

        let bucket_id = bucket::bucket_id(key);
        self.cache.update_record(account_id, bucket_id, record);

        Ok(())
    }

    // ── Batch write ─────────────────────────────────────────────────────

    pub fn write_batch(&self, account_id: &str, entries: &[([u8; 32], Vec<u8>, Codec)]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let handle = {
            let accounts = self.accounts.read().unwrap();
            accounts
                .get(account_id)
                .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?
                .clone()
        };

        let _write_lock = handle.write_mutex.lock().unwrap();
        let mut inner = handle.write();

        let mut pending: Vec<(IndexRecord, u64)> = Vec::with_capacity(entries.len());
        for (key, value, codec) in entries {
            if value.len() > crate::types::MAX_VALUE_SIZE {
                return Err(Error::ValueTooLarge { size: value.len() });
            }
            let (data, actual_codec) =
                compress::compress(value, *codec, self.config.compress_threshold, self.config.compression_level);

            let (segment_id, offset, data_size) =
                inner.append_entry(*key, &data, 0, actual_codec)?;

            let entry_end = offset + ENTRY_HEADER_SIZE as u64 + data_size as u64;
            let record = IndexRecord::new(*key, segment_id, offset, data_size, 0);
            pending.push((record, entry_end));
        }

        inner.flush_active()?;

        for (record, entry_end) in &pending {
            inner.append_index(record)?;
            inner.mark_indexed(record.segment_id, *entry_end)?;

            let bucket_id = bucket::bucket_id(&record.key);
            self.cache.update_record(account_id, bucket_id, record.clone());
        }

        Ok(())
    }

    // ── GC ──────────────────────────────────────────────────────────────

    pub fn gc(&self, account_id: &str) -> Result<Option<GcStats>> {
        let handle = {
            let accounts = self.accounts.read().unwrap();
            accounts
                .get(account_id)
                .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?
                .clone()
        };

        // Hold write_mutex to prevent concurrent writes from racing
        // with GC's bucket rebuild phase.
        let _write_lock = handle.write_mutex.lock().unwrap();

        let result = gc::gc_account(handle.dir(), self.config.gc_deleted_ratio)?;

        // Invalidate FilePool for GC'd segments (they were rewritten via rename)
        if let Some(ref stats) = result {
            handle.invalidate_file_cache(stats.segment_id);
        }

        for bid in 0..crate::types::BUCKET_COUNT {
            self.cache.invalidate(account_id, bid);
        }

        Ok(result)
    }

    pub fn compact_buckets(&self, account_id: &str) -> Result<()> {
        let handle = {
            let accounts = self.accounts.read().unwrap();
            accounts
                .get(account_id)
                .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?
                .clone()
        };

        // Hold write_mutex — compact rewrites all bucket files.
        let _write_lock = handle.write_mutex.lock().unwrap();

        gc::compact_buckets(handle.dir())?;

        for bid in 0..crate::types::BUCKET_COUNT {
            self.cache.invalidate(account_id, bid);
        }

        Ok(())
    }

    // ── Stats / Shutdown ────────────────────────────────────────────────

    pub fn stats(&self, account_id: &str) -> Result<AccountStats> {
        let handle = {
            let accounts = self.accounts.read().unwrap();
            accounts
                .get(account_id)
                .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?
                .clone()
        };

        let inner = handle.read();
        let meta = inner.meta();
        let mut total_bytes = 0u64;
        let mut deleted_bytes = 0u64;

        for seg in meta.segments.values() {
            total_bytes += seg.total_bytes;
            deleted_bytes += seg.deleted_bytes;
        }

        let mut total_keys = 0u64;
        for bid in 0..crate::types::BUCKET_COUNT {
            if let Ok(records) =
                self.cache
                    .get_or_load(account_id, bid, handle.dir())
            {
                total_keys += records.iter().filter(|r| !r.is_tombstone()).count() as u64;
            }
        }

        Ok(AccountStats {
            account_id: account_id.to_string(),
            total_keys,
            total_bytes,
            deleted_bytes,
            segment_count: meta.segments.len(),
        })
    }

    pub fn shutdown(&self) -> Result<()> {
        let accounts = self.accounts.read().unwrap();
        for (_, handle) in accounts.iter() {
            let mut inner = handle.write();
            inner.flush_active()?;
        }
        let global = GlobalMeta::load(&self.root)?;
        global.save(&self.root)?;
        tracing::info!("bichon-blob shut down cleanly");
        Ok(())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Err(e) = self.shutdown() {
            tracing::error!("bichon-blob shutdown error: {}", e);
        }
    }
}
