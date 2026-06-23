use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::bucket::{self, BucketFile, IndexRecord};
use crate::error::{Error, Result};
use crate::meta::{AccountMeta, SegmentStats};
use crate::segment::{self, SegmentReader, SegmentWriter};
use crate::types::Codec;

pub struct Account {
    pub id: String,
    dir: PathBuf,
    meta: AccountMeta,
    active_writer: SegmentWriter,
    readers: HashMap<u32, SegmentReader>,
    write_lock: Mutex<()>,
}

impl Account {
    /// Open an existing account.
    pub fn open(store_root: &Path, account_id: &str) -> Result<Self> {
        let dir = store_root.join("accounts").join(account_id);
        if !dir.exists() {
            return Err(Error::AccountNotFound(account_id.to_string()));
        }

        let meta = AccountMeta::load(&dir)?;

        // Open active writer
        let seg_path = dir
            .join("segments")
            .join(segment::segment_filename(meta.active_segment_id));
        let active_writer = if seg_path.exists() {
            SegmentWriter::open_append(seg_path, meta.active_segment_id)?
        } else {
            fs::create_dir_all(dir.join("segments"))?;
            SegmentWriter::create(seg_path, meta.active_segment_id)?
        };

        // Open readers for existing sealed segments
        let mut readers = HashMap::new();
        for (&seg_id, stats) in &meta.segments {
            if stats.sealed {
                let seg_path = dir
                    .join("segments")
                    .join(segment::segment_filename(seg_id));
                if seg_path.exists() {
                    readers.insert(seg_id, SegmentReader::open(seg_path, seg_id)?);
                }
            }
        }

        Ok(Self {
            id: account_id.to_string(),
            dir,
            meta,
            active_writer,
            readers,
            write_lock: Mutex::new(()),
        })
    }

    /// Create a new account.
    pub fn create(store_root: &Path, account_id: &str) -> Result<Self> {
        let dir = store_root.join("accounts").join(account_id);
        if dir.exists() {
            return Err(Error::AccountAlreadyExists(account_id.to_string()));
        }

        fs::create_dir_all(dir.join("segments"))?;
        BucketFile::ensure_dir(&dir)?;

        let meta = AccountMeta::new(account_id.to_string(), 1);

        let seg_path = dir
            .join("segments")
            .join(segment::segment_filename(1));
        let active_writer = SegmentWriter::create(seg_path, 1)?;

        meta.save(&dir)?;

        Ok(Self {
            id: account_id.to_string(),
            dir,
            meta,
            active_writer,
            readers: HashMap::new(),
            write_lock: Mutex::new(()),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn meta(&self) -> &AccountMeta {
        &self.meta
    }

    /// Lock for writing. Returns a guard.
    pub fn lock_write(&self) -> std::sync::MutexGuard<'_, ()> {
        self.write_lock.lock().unwrap()
    }

    /// Mark the segment as indexed up to the given offset and persist meta.
    /// Called after append_index to enable incremental recovery.
    pub fn mark_indexed(&mut self, segment_id: u32, indexed_up_to_offset: u64) -> Result<()> {
        if let Some(stats) = self.meta.segments.get_mut(&segment_id) {
            if indexed_up_to_offset > stats.indexed_up_to_offset {
                stats.indexed_up_to_offset = indexed_up_to_offset;
            }
        }
        self.meta.save(&self.dir)
    }

    /// Append an entry without fsync. Returns (segment_id, offset, data_size).
    /// Caller must hold the write lock and should call `flush_active()` after.
    pub fn append_entry(
        &mut self,
        key: [u8; 32],
        data: &[u8],
        flags: u8,
        codec: Codec,
    ) -> Result<(u32, u64, u32)> {
        if self.active_writer.is_full() {
            self.seal_active()?;
        }

        use crate::segment::Entry;
        let entry = if flags == 1 {
            Entry::tombstone(key)
        } else {
            Entry::new(key, data, flags, codec)
        };

        let data_size = entry.data.len() as u32;
        let segment_id = self.active_writer.id();
        let offset = self.active_writer.append(&entry)?;

        let stats = self
            .meta
            .segments
            .entry(segment_id)
            .or_insert_with(|| SegmentStats::new(segment_id));
        stats.total_bytes += data_size as u64;
        if flags == 1 {
            stats.deleted_bytes += entry.raw_size as u64;
        }
        stats.recompute_ratio();

        Ok((segment_id, offset, data_size))
    }

    /// Fsync the active segment and persist meta.
    pub fn flush_active(&mut self) -> Result<()> {
        self.active_writer.fsync()?;
        self.meta.save(&self.dir)
    }

    /// Write an entry with fsync. Convenience wrapper for single writes.
    pub fn write_entry(
        &mut self,
        key: [u8; 32],
        data: &[u8],
        flags: u8,
        codec: Codec,
    ) -> Result<(u32, u64, u32)> {
        let result = self.append_entry(key, data, flags, codec)?;
        self.flush_active()?;
        Ok(result)
    }

    fn seal_active(&mut self) -> Result<()> {
        let old_id = self.active_writer.id();
        let old_stats = self
            .meta
            .segments
            .entry(old_id)
            .or_insert_with(|| SegmentStats::new(old_id));
        old_stats.sealed = true;

        // Open reader for the old segment
        let seg_path = self
            .dir
            .join("segments")
            .join(segment::segment_filename(old_id));
        self.readers
            .insert(old_id, SegmentReader::open(seg_path, old_id)?);

        // Create new segment
        let new_id = old_id + 1;
        self.meta.active_segment_id = new_id;
        let new_path = self
            .dir
            .join("segments")
            .join(segment::segment_filename(new_id));
        self.active_writer = SegmentWriter::create(new_path, new_id)?;
        self.meta.save(&self.dir)?;

        Ok(())
    }

    /// Get the on-disk path for a segment.
    pub fn segment_path(&self, segment_id: u32) -> Result<PathBuf> {
        let filename = segment::segment_filename(segment_id);
        let path = self.dir.join("segments").join(&filename);
        if path.exists() {
            Ok(path)
        } else {
            Err(Error::SegmentNotFound(segment_id))
        }
    }

    /// Append index record to the appropriate bucket file. No fsync.
    pub fn append_index(&self, record: &IndexRecord) -> Result<()> {
        let bucket_id = bucket::bucket_id(&record.key);
        let bf = BucketFile::open(&self.dir, bucket_id);
        bf.append(record)
    }

    /// Return list of sealed segment IDs for GC consideration.
    pub fn sealed_segments(&self) -> Vec<u32> {
        self.meta
            .segments
            .iter()
            .filter(|(_, s)| s.sealed)
            .map(|(id, _)| *id)
            .collect()
    }

    /// All segment IDs (including active).
    pub fn all_segment_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.meta.segments.keys().copied().collect();
        if !ids.contains(&self.meta.active_segment_id) {
            ids.push(self.meta.active_segment_id);
        }
        ids.sort_unstable();
        ids
    }
}
