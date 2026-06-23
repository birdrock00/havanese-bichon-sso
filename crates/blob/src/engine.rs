use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::account::Account;
use crate::bucket::{self, IndexRecord};
use crate::cache::BucketCache;
use crate::compress;
use crate::error::{Error, Result};
use crate::gc::{self, GcStats};
use crate::meta::GlobalMeta;
use crate::recovery;
use crate::segment::{self, SegmentReader};
use crate::types::{Codec, Config, ENTRY_HEADER_SIZE};

pub struct Engine {
    root: PathBuf,
    config: Config,
    cache: BucketCache,
    accounts: RwLock<HashMap<String, Account>>,
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
    /// Open or create the store at `path`. Runs recovery on startup.
    pub fn open(path: &Path, config: Config) -> Result<Self> {
        config.validate()?;
        fs::create_dir_all(path)?;
        fs::create_dir_all(path.join("accounts"))?;

        let mut global = GlobalMeta::load(path)?;
        global.save(path)?;

        let cache = BucketCache::new(config.lru_bucket_count);

        // Discover accounts on disk
        let accounts_dir = path.join("accounts");
        let mut accounts = HashMap::new();

        if accounts_dir.exists() {
            for entry in fs::read_dir(&accounts_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    let account_name = entry.file_name().to_string_lossy().into_owned();

                    // Clean up temp files from interrupted GC
                    let _ = recovery::cleanup_temp_files(&entry.path());

                    // Run recovery
                    match recovery::recover_account(&entry.path()) {
                        Ok(_meta) => {
                            match Account::open(path, &account_name) {
                                Ok(account) => {
                                    accounts.insert(account_name, account);
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

        // Update global account list
        global.accounts = accounts.keys().cloned().collect();
        global.save(path)?;

        Ok(Self {
            root: path.to_path_buf(),
            config,
            cache,
            accounts: RwLock::new(accounts),
        })
    }

    // ── Account management ──────────────────────────────────────────────────

    pub fn create_account(&self, account_id: &str) -> Result<()> {
        let mut accounts = self.accounts.write().unwrap();
        if accounts.contains_key(account_id) {
            return Err(Error::AccountAlreadyExists(account_id.to_string()));
        }
        let account = Account::create(&self.root, account_id)?;
        accounts.insert(account_id.to_string(), account);

        let mut global = GlobalMeta::load(&self.root)?;
        global.accounts = accounts.keys().cloned().collect();
        global.save(&self.root)?;

        Ok(())
    }

    pub fn delete_account(&self, account_id: &str) -> Result<()> {
        let mut accounts = self.accounts.write().unwrap();
        let account = accounts
            .remove(account_id)
            .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?;

        let account_dir = account.dir().to_path_buf();
        drop(account);
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

    // ── Read / Write / Delete ───────────────────────────────────────────────

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

        let mut accounts = self.accounts.write().unwrap();
        let account = accounts
            .get_mut(account_id)
            .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?;

        // Acquire per-account write lock
        {
            let _lock = account.lock_write();
        }

        let (data, actual_codec) =
            compress::compress(value, codec, self.config.compress_threshold, self.config.compression_level);

        let (segment_id, offset, data_size) =
            account.write_entry(key, &data, 0, actual_codec)?;

        let record = IndexRecord::new(key, segment_id, offset, data_size, 0);
        account.append_index(&record)?;

        // Update indexed_up_to_offset for incremental recovery
        let entry_end = offset + ENTRY_HEADER_SIZE as u64 + data_size as u64;
        account.mark_indexed(segment_id, entry_end)?;

        let bucket_id = bucket::bucket_id(&key);
        self.cache.update_record(account_id, bucket_id, record);

        Ok(())
    }

    pub fn read(&self, account_id: &str, key: &[u8; 32]) -> Result<Option<Vec<u8>>> {
        let bucket_id = bucket::bucket_id(key);

        // Acquire lock only to get bucket records and segment routing info.
        let record: Option<IndexRecord> = {
            let accounts = self.accounts.read().unwrap();
            let account = accounts
                .get(account_id)
                .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?;
            let records = self
                .cache
                .get_or_load(account_id, bucket_id, account.dir())?;
            match records.binary_search_by(|r| r.key.cmp(key)) {
                Ok(idx) => Some(records[idx].clone()),
                Err(_) => None,
            }
        }; // accounts read lock dropped here — I/O happens outside the lock

        let record = match record {
            Some(r) => r,
            None => return Ok(None),
        };

        if record.is_tombstone() {
            return Ok(None);
        }

        // I/O outside the global lock
        let seg_path = self
            .root
            .join("accounts")
            .join(account_id)
            .join("segments")
            .join(segment::segment_filename(record.segment_id));

        if !seg_path.exists() {
            return Err(Error::SegmentNotFound(record.segment_id));
        }

        let reader = SegmentReader::open(seg_path, record.segment_id)?;
        let (entry, _) = reader.read_entry_at(record.offset)?;

        let value = compress::decompress(&entry.data, entry.codec, entry.raw_size as usize)?;

        Ok(Some(value))
    }

    pub fn delete(&self, account_id: &str, key: &[u8; 32]) -> Result<()> {
        let mut accounts = self.accounts.write().unwrap();
        let account = accounts
            .get_mut(account_id)
            .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?;

        // Acquire per-account write lock
        {
            let _lock = account.lock_write();
        }

        let (segment_id, offset, data_size) =
            account.write_entry(*key, &[], 1, Codec::None)?;

        let record = IndexRecord::new(*key, segment_id, offset, data_size, 1);
        account.append_index(&record)?;

        // Update indexed_up_to_offset for incremental recovery
        let entry_end = offset + ENTRY_HEADER_SIZE as u64 + data_size as u64;
        account.mark_indexed(segment_id, entry_end)?;

        let bucket_id = bucket::bucket_id(key);
        self.cache.update_record(account_id, bucket_id, record);

        Ok(())
    }

    // ── Batch write ─────────────────────────────────────────────────────────

    /// Batch-write multiple entries with a single fsync.
    /// Each element is (key, value, codec).
    pub fn write_batch(&self, account_id: &str, entries: &[([u8; 32], Vec<u8>, Codec)]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut accounts = self.accounts.write().unwrap();
        let account = accounts
            .get_mut(account_id)
            .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?;

        {
            let _lock = account.lock_write();
        }

        // Phase 1: compress and append all entries without fsync
        let mut pending: Vec<(IndexRecord, u64)> = Vec::with_capacity(entries.len());
        for (key, value, codec) in entries {
            if value.len() > crate::types::MAX_VALUE_SIZE {
                return Err(Error::ValueTooLarge { size: value.len() });
            }
            let (data, actual_codec) =
                compress::compress(value, *codec, self.config.compress_threshold, self.config.compression_level);

            let (segment_id, offset, data_size) =
                account.append_entry(*key, &data, 0, actual_codec)?;

            let entry_end = offset + ENTRY_HEADER_SIZE as u64 + data_size as u64;
            let record = IndexRecord::new(*key, segment_id, offset, data_size, 0);
            pending.push((record, entry_end));
        }

        // Phase 2: single fsync
        account.flush_active()?;

        // Phase 3: append indices and update cache
        for (record, entry_end) in &pending {
            account.append_index(record)?;
            account.mark_indexed(record.segment_id, *entry_end)?;

            let bucket_id = bucket::bucket_id(&record.key);
            self.cache.update_record(account_id, bucket_id, record.clone());
        }

        Ok(())
    }

    // ── GC ──────────────────────────────────────────────────────────────────

    pub fn gc(&self, account_id: &str) -> Result<Option<GcStats>> {
        // Get account dir while holding read lock, then release before heavy I/O.
        let account_dir = {
            let accounts = self.accounts.read().unwrap();
            let account = accounts
                .get(account_id)
                .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?;
            account.dir().to_path_buf()
        }; // read lock released — GC runs without blocking reads

        let result = gc::gc_account(&account_dir, self.config.gc_deleted_ratio)?;

        // Invalidate all cached buckets for this account after GC rewrites them
        for bid in 0..crate::types::BUCKET_COUNT {
            self.cache.invalidate(account_id, bid);
        }

        Ok(result)
    }

    pub fn compact_buckets(&self, account_id: &str) -> Result<()> {
        let account_dir = {
            let accounts = self.accounts.read().unwrap();
            let account = accounts
                .get(account_id)
                .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?;
            account.dir().to_path_buf()
        };

        gc::compact_buckets(&account_dir)?;

        for bid in 0..crate::types::BUCKET_COUNT {
            self.cache.invalidate(account_id, bid);
        }

        Ok(())
    }

    // ── Stats / Shutdown ────────────────────────────────────────────────────

    pub fn stats(&self, account_id: &str) -> Result<AccountStats> {
        let accounts = self.accounts.read().unwrap();
        let account = accounts
            .get(account_id)
            .ok_or_else(|| Error::AccountNotFound(account_id.to_string()))?;

        let meta = account.meta();
        let mut total_bytes = 0u64;
        let mut deleted_bytes = 0u64;

        for seg in meta.segments.values() {
            total_bytes += seg.total_bytes;
            deleted_bytes += seg.deleted_bytes;
        }

        // Count total live keys from bucket indices
        let mut total_keys = 0u64;
        for bid in 0..crate::types::BUCKET_COUNT {
            if let Ok(records) =
                self.cache
                    .get_or_load(account_id, bid, account.dir())
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

    /// Gracefully shut down: fsync all active segments and persist meta.
    pub fn shutdown(&self) -> Result<()> {
        let mut accounts = self.accounts.write().unwrap();
        for (_, account) in accounts.iter_mut() {
            account.flush_active()?;
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
