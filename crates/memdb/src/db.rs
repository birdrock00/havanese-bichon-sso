use crate::error::{DbError, Result};
use crate::query::{Page, Paginated};
use crate::wal::{self, WalEntry, WalOp};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ─── Durability ──────────────────────────────────────────────────────────

/// Controls when WAL data is fsynced to disk.
#[derive(Clone, Debug)]
pub enum Durability {
    /// fsync every write — safest, ~250 ops/s.
    Full,
    /// Buffer up to `max_ops` writes, then fsync once.
    /// Call `MemDb::flush()` before shutdown to commit any remaining
    /// buffered writes.
    Batch { max_ops: usize },
    /// Never fsync — fastest (~10k+ ops/s), zero durability.
    /// Useful for ephemeral caches or when `snapshot()` handles persistence.
    Off,
}

impl Default for Durability {
    fn default() -> Self {
        Durability::Full
    }
}

impl Durability {
    /// Convenience: batch up to `max_ops` writes per fsync.
    pub fn batch(max_ops: usize) -> Self {
        assert!(max_ops > 0);
        Durability::Batch { max_ops }
    }
}

// ─── In-memory state ─────────────────────────────────────────────────────

/// Complete in-memory state, used for deserializing snapshot.json.
#[derive(Deserialize, Default)]
struct Snapshot {
    /// The highest WAL seq covered by this snapshot.
    last_seq: u64,
    /// All collection data.
    data: BTreeMap<String, BTreeMap<String, Value>>,
}

/// Borrowed snapshot for zero-copy serialization — avoids cloning the
/// entire dataset when writing a snapshot file.
#[derive(Serialize)]
struct SnapshotRef<'a> {
    last_seq: u64,
    data: &'a BTreeMap<String, BTreeMap<String, Value>>,
}

/// Runtime state. All writes are serialized under this lock.
struct Inner {
    last_seq: u64,
    data: BTreeMap<String, BTreeMap<String, Value>>,
    /// Open WAL file handle, reused across writes.
    wal_file: Option<File>,
    /// WAL file path, used when snapshot truncation needs to reopen the handle.
    wal_path: PathBuf,
    snapshot_path: PathBuf,
    durability: Durability,
    /// Buffered WAL entries not yet flushed to disk (Batch mode).
    pending: Vec<WalEntry>,
    /// When the first entry was added to the current batch.
    pending_since: Option<Instant>,
}

impl Inner {
    /// Execute a batch of ops under the lock: allocate seq → apply to memory
    /// → write WAL (fsync behaviour depends on Durability).
    fn commit(&mut self, ops: Vec<WalOp>) -> Result<u64> {
        if ops.is_empty() {
            return Ok(0);
        }
        self.last_seq += 1;
        let seq = self.last_seq;
        let entry = WalEntry {
            seq,
            ops,
        };
        // Always apply to memory first — clients can read their own writes
        // immediately regardless of durability mode.
        for op in &entry.ops {
            apply_op(&mut self.data, op.clone());
        }
        // WAL path depends on Durability.
        match self.durability {
            Durability::Full => {
                if let Some(ref mut f) = self.wal_file {
                    wal::write_entry(f, &entry)?;
                    wal::sync_wal(f)?;
                }
            }
            Durability::Batch { .. } => {
                self.push_pending(entry);
                if self.pending.len() >= self.batch_threshold() {
                    self.flush_pending()?;
                }
            }
            Durability::Off => {
                if let Some(ref mut f) = self.wal_file {
                    wal::write_entry(f, &entry)?;
                }
            }
        }
        Ok(seq)
    }

    fn batch_threshold(&self) -> usize {
        match self.durability {
            Durability::Batch { max_ops } => max_ops,
            _ => 0,
        }
    }

    fn push_pending(&mut self, entry: WalEntry) {
        if self.pending.is_empty() {
            self.pending_since = Some(Instant::now());
        }
        self.pending.push(entry);
    }

    /// Write all buffered entries to WAL and fsync once.
    fn flush_pending(&mut self) -> Result<usize> {
        let count = self.pending.len();
        if count == 0 {
            return Ok(0);
        }
        if let Some(ref mut f) = self.wal_file {
            for entry in &self.pending {
                wal::write_entry(f, entry)?;
            }
            wal::sync_wal(f)?;
        }
        self.pending.clear();
        self.pending_since = None;
        Ok(count)
    }

}

fn apply_op(data: &mut BTreeMap<String, BTreeMap<String, Value>>, op: WalOp) {
    match op {
        WalOp::Insert { collection, key, value } => {
            data.entry(collection).or_default().insert(key, value);
        }
        WalOp::Upsert { collection, key, value } => {
            data.entry(collection).or_default().insert(key, value);
        }
        WalOp::Delete { collection, key } => {
            if let Some(col) = data.get_mut(&collection) {
                col.remove(&key);
            }
        }
    }
}

// ─── MemDb ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MemDb {
    /// All writes (WAL + memory) are serialized under this lock.
    inner: Arc<Mutex<Inner>>,
}

impl MemDb {
    /// Open the database with `Durability::Full` (backward-compatible).
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Self> {
        Self::open_with(data_dir, Durability::Full)
    }

    /// Open the database with a specific durability policy.
    pub fn open_with(data_dir: impl AsRef<Path>, durability: Durability) -> Result<Self> {
        let dir = data_dir.as_ref();
        std::fs::create_dir_all(dir)?;

        let snapshot_path = dir.join("snapshot.json");
        let wal_path = dir.join("wal.jsonl");

        // 1. Read snapshot.
        let mut snapshot = if snapshot_path.exists() {
            let bytes = std::fs::read(&snapshot_path)?;
            serde_json::from_slice::<Snapshot>(&bytes)?
        } else {
            Snapshot::default()
        };

        let after_seq = snapshot.last_seq;

        // 2. Replay WAL entries with seq > last_seq.
        let entries = wal::read_after(&wal_path, after_seq)?;
        let replayed = entries.len();
        for entry in entries {
            for op in entry.ops {
                apply_op(&mut snapshot.data, op);
            }
            snapshot.last_seq = snapshot.last_seq.max(entry.seq);
        }

        if replayed > 0 {
            eprintln!("[memdb] replayed {replayed} WAL entries after seq={after_seq}");
        }

        // 3. Open WAL file handle for subsequent writes.
        let wal_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)?;

        Ok(Self {
            inner: Arc::new(Mutex::new(Inner {
                last_seq: snapshot.last_seq,
                data: snapshot.data,
                wal_file: Some(wal_file),
                wal_path,
                snapshot_path,
                durability,
                pending: vec![],
                pending_since: None,
            })),
        })
    }

    /// Pure in-memory mode (for tests, no persistence).
    pub fn in_memory() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                last_seq: 0,
                data: BTreeMap::new(),
                wal_file: None,
                wal_path: PathBuf::from("/dev/null"),
                snapshot_path: PathBuf::from("/dev/null"),
                durability: Durability::Off,
                pending: vec![],
                pending_since: None,
            })),
        }
    }

    /// Flush any buffered WAL entries to disk.
    /// Important in `Durability::Batch` mode before shutdown — without this
    /// call the last buffered batch may be lost on crash.
    pub fn flush(&self) -> Result<usize> {
        self.inner.lock().unwrap().flush_pending()
    }

    /// Return the number of buffered entries not yet flushed to disk.
    pub fn pending_writes(&self) -> usize {
        self.inner.lock().unwrap().pending.len()
    }

    /// Trigger a manual snapshot:
    /// 1. Flush pending WAL entries (so crash recovery sees them).
    /// 2. Read (last_seq, data) atomically under the lock.
    /// 3. Write snapshot file outside the lock (non-blocking for writers).
    /// 4. Atomic rename ensures snapshot file is never partial.
    /// 5. Re-lock and truncate the WAL only when no writes raced in between.
    pub fn snapshot(&self) -> Result<()> {
        // Flush pending so every committed write is in the WAL before we
        // potentially truncate it.
        self.flush()?;

        // Serialize inside the lock — borrows data directly (zero-copy),
        // then write to disk outside the lock so writers aren't blocked.
        let (bytes, last_seq, snapshot_path) = {
            let inner = self.inner.lock().unwrap();
            let path = inner.snapshot_path.clone();
            if path == Path::new("/dev/null") {
                return Ok(());
            }
            let snap = SnapshotRef {
                last_seq: inner.last_seq,
                data: &inner.data,
            };
            (serde_json::to_vec_pretty(&snap)?, inner.last_seq, path)
        };

        // Write snapshot outside lock so writers are not blocked.
        let tmp = snapshot_path.with_extension("tmp");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &snapshot_path)?;

        // Re-lock. Only truncate the WAL if no writes have committed since
        // the snapshot was taken.
        let mut inner = self.inner.lock().unwrap();
        if inner.last_seq == last_seq {
            if let Some(f) = inner.wal_file.take() {
                drop(f);
                // Truncate the file to zero.
                std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&inner.wal_path)?;
                // Reopen in append mode for future writes.
                inner.wal_file = Some(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&inner.wal_path)?,
                );
            }
        }

        eprintln!("[memdb] snapshot saved at seq={last_seq}");
        Ok(())
    }

    /// Start a background snapshot worker that fires at the given interval.
    pub fn start_snapshot_worker(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let db = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await; // skip immediate first tick
            loop {
                ticker.tick().await;
                if let Err(e) = db.snapshot() {
                    eprintln!("[memdb] snapshot error: {e}");
                }
            }
        })
    }

    /// Start a background flush worker for Batch durability mode.
    /// Guarantees that buffered writes are flushed at least every `interval`.
    pub fn start_flush_worker(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let db = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await;
            loop {
                ticker.tick().await;
                if let Err(e) = db.flush() {
                    eprintln!("[memdb] flush error: {e}");
                }
            }
        })
    }

    /// Get a handle to the named collection.
    pub fn collection(&self, name: &'static str) -> Collection {
        Collection {
            db: self.clone(),
            name,
        }
    }

    /// Begin a cross-collection atomic transaction.
    pub fn transaction(&self) -> Transaction {
        Transaction {
            db: self.clone(),
            ops: vec![],
        }
    }

    fn commit(&self, ops: Vec<WalOp>) -> Result<()> {
        self.inner.lock().unwrap().commit(ops)?;
        Ok(())
    }
}

// ─── Collection ──────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Collection {
    db: MemDb,
    name: &'static str,
}

impl Collection {
    // ── Write operations ──────────────────────────────────────────────────

    /// Insert a record. Returns `DuplicateKey` if the key already exists.
    pub fn insert<T: Serialize>(&self, key: impl Into<String>, value: &T) -> Result<()> {
        let key = key.into();
        let value = serde_json::to_value(value)?;
        let mut inner = self.db.inner.lock().unwrap();
        if inner.data.get(self.name).and_then(|c| c.get(&key)).is_some() {
            return Err(DbError::DuplicateKey(self.name.to_string(), key));
        }
        inner.commit(vec![WalOp::Insert {
            collection: self.name.to_string(),
            key,
            value,
        }])?;
        Ok(())
    }

    /// Insert or overwrite a record.
    pub fn upsert<T: Serialize>(&self, key: impl Into<String>, value: &T) -> Result<()> {
        let key = key.into();
        let value = serde_json::to_value(value)?;
        self.db.commit(vec![WalOp::Upsert {
            collection: self.name.to_string(),
            key,
            value,
        }])
    }

    /// Read-modify-write, performed atomically under the lock.
    /// Returns the updated value.
    pub fn update<T>(&self, key: impl Into<String>, f: impl FnOnce(T) -> T) -> Result<T>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let key = key.into();
        let mut inner = self.db.inner.lock().unwrap();
        let old_val = inner
            .data
            .get(self.name)
            .and_then(|c| c.get(&key))
            .ok_or_else(|| DbError::NotFound(self.name.to_string(), key.clone()))?
            .clone();
        let old: T = serde_json::from_value(old_val)?;
        let updated = f(old);
        let new_val = serde_json::to_value(&updated)?;
        inner.commit(vec![WalOp::Upsert {
            collection: self.name.to_string(),
            key,
            value: new_val,
        }])?;
        Ok(updated)
    }

    /// Delete a record. Returns whether the record existed.
    pub fn delete(&self, key: impl Into<String>) -> Result<bool> {
        let key = key.into();
        let mut inner = self.db.inner.lock().unwrap();
        let existed = inner.data.get(self.name).and_then(|c| c.get(&key)).is_some();
        if existed {
            inner.commit(vec![WalOp::Delete {
                collection: self.name.to_string(),
                key,
            }])?;
        }
        Ok(existed)
    }

    // ── Read operations (in-memory, no WAL involvement) ───────────────────

    /// Look up a record by primary key.
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let inner = self.db.inner.lock().unwrap();
        match inner.data.get(self.name).and_then(|c| c.get(key)) {
            Some(v) => Ok(Some(serde_json::from_value(v.clone())?)),
            None => Ok(None),
        }
    }

    /// Look up a record by primary key; return an error if not found.
    pub fn get_required<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T> {
        self.get(key)?
            .ok_or_else(|| DbError::NotFound(self.name.to_string(), key.to_string()))
    }

    /// Full scan with a predicate filter.
    pub fn filter<T, F>(&self, predicate: F) -> Result<Vec<T>>
    where
        T: for<'de> Deserialize<'de>,
        F: Fn(&T) -> bool,
    {
        let inner = self.db.inner.lock().unwrap();
        let Some(col) = inner.data.get(self.name) else {
            return Ok(vec![]);
        };
        let mut results = vec![];
        for v in col.values() {
            let item: T = serde_json::from_value(v.clone())?;
            if predicate(&item) {
                results.push(item);
            }
        }
        Ok(results)
    }

    /// List all records in key order.
    pub fn list_all<T: for<'de> Deserialize<'de>>(&self) -> Result<Vec<T>> {
        let inner = self.db.inner.lock().unwrap();
        let Some(col) = inner.data.get(self.name) else {
            return Ok(vec![]);
        };
        col.values()
            .map(|v| serde_json::from_value(v.clone()).map_err(DbError::from))
            .collect()
    }

    /// Prefix scan — returns records whose key starts with the given prefix,
    /// in key order.
    pub fn scan_prefix<T: for<'de> Deserialize<'de>>(&self, prefix: &str) -> Result<Vec<T>> {
        let inner = self.db.inner.lock().unwrap();
        let Some(col) = inner.data.get(self.name) else {
            return Ok(vec![]);
        };
        col.range(prefix.to_string()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(_, v)| serde_json::from_value(v.clone()).map_err(DbError::from))
            .collect()
    }

    /// Paginated query in key order.
    pub fn paginate<T: for<'de> Deserialize<'de>>(
        &self,
        page: &Page,
        desc: bool,
    ) -> Result<Paginated<T>> {
        let inner = self.db.inner.lock().unwrap();
        let Some(col) = inner.data.get(self.name) else {
            return Ok(Paginated::empty(page));
        };
        let total = col.len() as u64;
        let offset = page.offset();
        let items: Vec<T> = if desc {
            col.values()
                .rev()
                .skip(offset)
                .take(page.page_size as usize)
                .map(|v| serde_json::from_value(v.clone()).map_err(DbError::from))
                .collect::<Result<_>>()?
        } else {
            col.values()
                .skip(offset)
                .take(page.page_size as usize)
                .map(|v| serde_json::from_value(v.clone()).map_err(DbError::from))
                .collect::<Result<_>>()?
        };
        Ok(Paginated::new(page, total, items))
    }

    /// Return the number of records in this collection.
    pub fn count(&self) -> usize {
        let inner = self.db.inner.lock().unwrap();
        inner.data.get(self.name).map(|c| c.len()).unwrap_or(0)
    }

    /// Check whether a key exists.
    pub fn exists(&self, key: &str) -> bool {
        let inner = self.db.inner.lock().unwrap();
        inner.data.get(self.name).and_then(|c| c.get(key)).is_some()
    }
}

// ─── Transaction ─────────────────────────────────────────────────────────

pub struct Transaction {
    db: MemDb,
    ops: Vec<WalOp>,
}

impl Transaction {
    pub fn insert<T: Serialize>(
        mut self,
        collection: &str,
        key: impl Into<String>,
        value: &T,
    ) -> Result<Self> {
        self.ops.push(WalOp::Insert {
            collection: collection.to_string(),
            key: key.into(),
            value: serde_json::to_value(value)?,
        });
        Ok(self)
    }

    pub fn upsert<T: Serialize>(
        mut self,
        collection: &str,
        key: impl Into<String>,
        value: &T,
    ) -> Result<Self> {
        self.ops.push(WalOp::Upsert {
            collection: collection.to_string(),
            key: key.into(),
            value: serde_json::to_value(value)?,
        });
        Ok(self)
    }

    pub fn delete(mut self, collection: &str, key: impl Into<String>) -> Self {
        self.ops.push(WalOp::Delete {
            collection: collection.to_string(),
            key: key.into(),
        });
        self
    }

    /// Atomically commit all buffered ops as a single WAL entry.
    pub fn commit(self) -> Result<()> {
        self.db.commit(self.ops)
    }
}
