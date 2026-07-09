use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::bucket::{BucketStore, IndexRecord};
use crate::compress;
use crate::error::{Error, Result};
use crate::file_pool::FilePool;
use crate::gc::{self, GcStats};
use crate::meta::{GlobalMeta, SegmentStats};
use crate::segment::{self, SegmentReader, SegmentWriter};
use crate::types::{Codec, Config, ENTRY_HEADER_SIZE};

/// Global content-addressable blob store.
///
/// All data is keyed by a 32-byte content hash.  Identical content is stored
/// only once.  The caller is responsible for tracking which entities reference
/// which content hashes — this crate is a pure content-addressable KV engine.
pub struct Engine {
    shared: Arc<EngineShared>,
    flush_handle: Mutex<Option<FlushHandle>>,
}

struct EngineShared {
    config: Config,
    inner: RwLock<EngineInner>,
    bucket_store: BucketStore,
    write_mutex: Mutex<()>,
    file_pool: FilePool,
}

struct FlushHandle {
    handle: JoinHandle<()>,
    stop: Arc<AtomicBool>,
}

struct EngineInner {
    root: PathBuf,
    meta: GlobalMeta,
    active_writer: SegmentWriter,
    readers: HashMap<u32, SegmentReader>,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub total_keys: u64,
    pub total_bytes: u64,
    pub deleted_bytes: u64,
    pub segment_count: usize,
}

impl Engine {
    pub fn open(path: &Path, config: Config) -> Result<Self> {
        config.validate()?;

        fs::create_dir_all(path)?;
        fs::create_dir_all(path.join("segments"))?;

        let bucket_dir = path.join("buckets");
        let bucket_store = BucketStore::open(&bucket_dir, config.compact_threshold)?;

        let mut meta = GlobalMeta::load(path)?;

        // Discover existing segments on disk
        let seg_dir = path.join("segments");
        let mut disk_segments: Vec<u32> = Vec::new();
        if seg_dir.exists() {
            for entry in fs::read_dir(&seg_dir)? {
                let entry = entry?;
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.ends_with(".seg") && !name_str.contains("temp_") {
                    if let Some(id_str) = name_str.strip_suffix(".seg") {
                        if let Ok(id) = id_str.parse::<u32>() {
                            disk_segments.push(id);
                        }
                    }
                }
            }
        }
        disk_segments.sort_unstable();

        for &seg_id in &disk_segments {
            if !meta.segments.contains_key(&seg_id) {
                meta.segments.insert(seg_id, SegmentStats::new(seg_id));
            }
        }

        let max_disk_id = disk_segments.last().copied().unwrap_or(0);
        if max_disk_id > meta.active_segment_id {
            meta.active_segment_id = max_disk_id;
        }
        if meta.active_segment_id == 0 {
            meta.active_segment_id = 1;
        }

        crate::recovery::cleanup_temp_files(path)?;
        crate::recovery::recover(path, &mut meta)?;

        let seg_path = path
            .join("segments")
            .join(segment::segment_filename(meta.active_segment_id));
        let active_writer = if seg_path.exists() {
            SegmentWriter::open_append(seg_path, meta.active_segment_id)?
        } else {
            SegmentWriter::create(seg_path, meta.active_segment_id)?
        };

        let mut readers = HashMap::new();
        for (&seg_id, stats) in &meta.segments {
            if stats.sealed {
                let seg_path = path
                    .join("segments")
                    .join(segment::segment_filename(seg_id));
                if seg_path.exists() {
                    readers.insert(seg_id, SegmentReader::open(seg_path, seg_id)?);
                }
            }
        }

        meta.save(path)?;

        let shared = Arc::new(EngineShared {
            config: config.clone(),
            inner: RwLock::new(EngineInner {
                root: path.to_path_buf(),
                meta,
                active_writer,
                readers,
            }),
            bucket_store,
            write_mutex: Mutex::new(()),
            file_pool: FilePool::new(8),
        });

        let flush_handle = if config.flush_interval_secs > 0 {
            let shared2 = Arc::clone(&shared);
            let stop = Arc::new(AtomicBool::new(false));
            let stop2 = Arc::clone(&stop);
            let interval = Duration::from_secs(config.flush_interval_secs);

            let handle = thread::Builder::new()
                .name("blob-flush".into())
                .spawn(move || {
                    while !stop2.load(Ordering::Acquire) {
                        thread::park_timeout(interval);
                        if stop2.load(Ordering::Acquire) {
                            break;
                        }
                        let _lock = shared2.write_mutex.lock().unwrap();
                        let mut inner = shared2.inner.write().unwrap();
                        if let Err(e) = inner.flush_active() {
                            tracing::error!("background flush failed: {}", e);
                        }
                    }
                })
                .expect("failed to spawn blob-flush thread");

            Some(FlushHandle { handle, stop })
        } else {
            None
        };

        Ok(Self {
            shared,
            flush_handle: Mutex::new(flush_handle),
        })
    }

    // ── Read / Write / Delete ───────────────────────────────────────────

    pub fn put(&self, key: [u8; 32], value: &[u8], codec: Codec) -> Result<()> {
        if value.len() > crate::types::MAX_VALUE_SIZE {
            return Err(Error::ValueTooLarge { size: value.len() });
        }

        let _write_lock = self.shared.write_mutex.lock().unwrap();
        let mut inner = self.shared.inner.write().unwrap();

        let (data, actual_codec) = compress::compress(
            value,
            codec,
            self.shared.config.compress_threshold,
            self.shared.config.compression_level,
        );

        let original_len = value.len() as u32;
        let (segment_id, offset, data_size) =
            inner.append_entry(key, &data, original_len, 0, actual_codec)?;

        let record = IndexRecord::new(key, segment_id, offset, data_size, 0);
        self.shared.bucket_store.insert(record)?;

        let entry_end = offset + ENTRY_HEADER_SIZE as u64 + data_size as u64;
        inner.mark_indexed(segment_id, entry_end)?;

        Ok(())
    }

    pub fn get(&self, key: &[u8; 32]) -> Result<Option<Vec<u8>>> {
        let record = match self.shared.bucket_store.get(key)? {
            Some(r) => r,
            None => return Ok(None),
        };

        let inner = self.shared.inner.read().unwrap();
        let seg_path = inner.segment_path(record.segment_id)?;

        if !seg_path.exists() {
            return Err(Error::SegmentNotFound(record.segment_id));
        }

        let reader = SegmentReader::open(seg_path.clone(), record.segment_id)?;
        let file = self.shared.file_pool.get(record.segment_id, &seg_path)?;
        let (entry, _) = reader.read_entry_at_file(record.offset, &file)?;

        let value = compress::decompress(&entry.data, entry.codec, entry.raw_size as usize)?;
        Ok(Some(value))
    }

    pub fn delete(&self, key: &[u8; 32]) -> Result<()> {
        let _write_lock = self.shared.write_mutex.lock().unwrap();
        let mut inner = self.shared.inner.write().unwrap();

        let (segment_id, offset, data_size) =
            inner.append_entry(*key, &[], 0, 1, Codec::None)?;

        let record = IndexRecord::new(*key, segment_id, offset, data_size, 1);
        self.shared.bucket_store.insert(record)?;

        let entry_end = offset + ENTRY_HEADER_SIZE as u64 + data_size as u64;
        inner.mark_indexed(segment_id, entry_end)?;

        Ok(())
    }

    pub fn exists(&self, key: &[u8; 32]) -> Result<bool> {
        self.shared.bucket_store.exists(key)
    }

    // ── Batch delete ─────────────────────────────────────────────────────

    pub fn delete_batch(&self, keys: &[[u8; 32]]) -> Result<()> {
        if keys.is_empty() {
            return Ok(());
        }

        let _write_lock = self.shared.write_mutex.lock().unwrap();
        let mut inner = self.shared.inner.write().unwrap();

        let mut records: Vec<IndexRecord> = Vec::with_capacity(keys.len());
        let mut ends: Vec<(u32, u64)> = Vec::with_capacity(keys.len());

        for key in keys {
            let (segment_id, offset, data_size) =
                inner.append_entry(*key, &[], 0, 1, Codec::None)?;

            let entry_end = offset + ENTRY_HEADER_SIZE as u64 + data_size as u64;
            records.push(IndexRecord::new(*key, segment_id, offset, data_size, 1));
            ends.push((segment_id, entry_end));
        }

        inner.flush_active()?;

        self.shared.bucket_store.insert_batch(&records)?;

        for (segment_id, entry_end) in &ends {
            inner.mark_indexed(*segment_id, *entry_end)?;
        }

        Ok(())
    }

    // ── Batch write ─────────────────────────────────────────────────────

    pub fn put_batch(&self, entries: &[([u8; 32], Vec<u8>, Codec)]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let _write_lock = self.shared.write_mutex.lock().unwrap();
        let mut inner = self.shared.inner.write().unwrap();

        let mut records: Vec<IndexRecord> = Vec::with_capacity(entries.len());
        let mut ends: Vec<(u32, u64)> = Vec::with_capacity(entries.len());

        for (key, value, codec) in entries {
            if value.len() > crate::types::MAX_VALUE_SIZE {
                return Err(Error::ValueTooLarge { size: value.len() });
            }
            let (data, actual_codec) = compress::compress(
                value,
                *codec,
                self.shared.config.compress_threshold,
                self.shared.config.compression_level,
            );

            let original_len = value.len() as u32;
            let (segment_id, offset, data_size) =
                inner.append_entry(*key, &data, original_len, 0, actual_codec)?;

            let entry_end = offset + ENTRY_HEADER_SIZE as u64 + data_size as u64;
            records.push(IndexRecord::new(*key, segment_id, offset, data_size, 0));
            ends.push((segment_id, entry_end));
        }

        inner.flush_active()?;

        self.shared.bucket_store.insert_batch(&records)?;

        for (segment_id, entry_end) in &ends {
            inner.mark_indexed(*segment_id, *entry_end)?;
        }

        Ok(())
    }

    // ── GC ──────────────────────────────────────────────────────────────

    pub fn gc(&self) -> Result<Option<GcStats>> {
        // Phase 1: scan segments and write compacted temp file.
        // Read-only with respect to Engine state — no write_mutex needed.
        let prep = {
            let inner = self.shared.inner.read().unwrap();
            gc::gc_prepare(
                &inner.root,
                &inner.meta,
                self.shared.config.gc_deleted_ratio,
            )?
        };

        let prep = match prep {
            Some(p) => p,
            None => return Ok(None),
        };

        // Phase 2: rename temp file + rebuild bucket indices.
        // This is the only part that requires exclusive access.
        let _write_lock = self.shared.write_mutex.lock().unwrap();

        let stats = gc::gc_finish(prep)?;
        self.shared.file_pool.invalidate(stats.segment_id);

        // Rebuild bucket indices from the updated segment files
        let inner = self.shared.inner.write().unwrap();
        let seg_ids: Vec<u32> = inner.meta.segments.keys().copied().collect();
        let mut seg_refs: Vec<(u32, PathBuf)> = Vec::with_capacity(seg_ids.len());
        for &id in &seg_ids {
            let p = inner.root.join("segments").join(segment::segment_filename(id));
            seg_refs.push((id, p));
        }
        let paths: Vec<(u32, &Path)> =
            seg_refs.iter().map(|(id, p)| (*id, p.as_path())).collect();

        let bucket_dir = inner.root.join("buckets");
        BucketStore::rebuild_from_segments(&bucket_dir, &paths)?;

        // Update meta (segment stats changed after GC compaction)
        let mut meta = crate::meta::GlobalMeta::load(&inner.root)?;
        meta.active_segment_id = inner.meta.active_segment_id;
        meta.save(&inner.root)?;

        // Reload bucket store mmaps after GC rewrites
        self.shared.bucket_store.reload_all()?;

        Ok(Some(stats))
    }

    /// Run GC only if some segment exceeds the configured deleted-ratio threshold.
    /// Returns `Ok(None)` immediately without acquiring the write lock when no
    /// segment qualifies.
    pub fn gc_if_needed(&self) -> Result<Option<GcStats>> {
        let inner = self.shared.inner.read().unwrap();
        let needs_gc = inner
            .meta
            .segments
            .values()
            .any(|s| s.sealed && s.deleted_ratio >= self.shared.config.gc_deleted_ratio);
        drop(inner);

        if needs_gc {
            self.gc()
        } else {
            Ok(None)
        }
    }

    // ── Flush / Stats / Shutdown ────────────────────────────────────────

    /// Fsync the active segment and save metadata without compacting buckets.
    /// Lightweight checkpoint suitable for periodic calls from the background
    /// flush thread or external schedulers.
    pub fn flush(&self) -> Result<()> {
        let _write_lock = self.shared.write_mutex.lock().unwrap();
        let mut inner = self.shared.inner.write().unwrap();
        inner.flush_active()
    }

    pub fn stats(&self) -> Result<Stats> {
        let inner = self.shared.inner.read().unwrap();
        let meta = &inner.meta;

        let mut total_bytes = 0u64;
        let mut deleted_bytes = 0u64;
        for seg in meta.segments.values() {
            total_bytes += seg.total_bytes;
            deleted_bytes += seg.deleted_bytes;
        }

        let total_keys = self.shared.bucket_store.total_keys() as u64;

        Ok(Stats {
            total_keys,
            total_bytes,
            deleted_bytes,
            segment_count: meta.segments.len(),
        })
    }

    pub fn shutdown(&self) -> Result<()> {
        // Stop background flush thread first
        if let Some(fh) = self.flush_handle.lock().unwrap().take() {
            fh.stop.store(true, Ordering::Release);
            fh.handle.thread().unpark();
            let _ = fh.handle.join();
        }

        let mut inner = self.shared.inner.write().unwrap();
        inner.flush_active()?;

        self.shared.bucket_store.compact_all()?;

        inner.meta.save(&inner.root)?;
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

// ── EngineInner ───────────────────────────────────────────────────────────

impl EngineInner {
    fn append_entry(
        &mut self,
        key: [u8; 32],
        data: &[u8],
        raw_size: u32,
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
            Entry::new(key, data, raw_size, flags, codec)
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

    fn flush_active(&mut self) -> Result<()> {
        self.active_writer.fsync()?;
        self.meta.save(&self.root)
    }

    fn mark_indexed(&mut self, segment_id: u32, offset: u64) -> Result<()> {
        if let Some(stats) = self.meta.segments.get_mut(&segment_id) {
            if offset > stats.indexed_up_to_offset {
                stats.indexed_up_to_offset = offset;
            }
        }
        self.meta.save(&self.root)
    }

    fn seal_active(&mut self) -> Result<()> {
        let old_id = self.active_writer.id();
        let old_stats = self
            .meta
            .segments
            .entry(old_id)
            .or_insert_with(|| SegmentStats::new(old_id));
        old_stats.sealed = true;

        let seg_path = self
            .root
            .join("segments")
            .join(segment::segment_filename(old_id));
        self.readers
            .insert(old_id, SegmentReader::open(seg_path, old_id)?);

        let new_id = old_id + 1;
        self.meta.active_segment_id = new_id;
        let new_path = self
            .root
            .join("segments")
            .join(segment::segment_filename(new_id));
        self.active_writer = SegmentWriter::create(new_path, new_id)?;
        self.meta.save(&self.root)?;

        Ok(())
    }

    fn segment_path(&self, segment_id: u32) -> Result<PathBuf> {
        let path = self
            .root
            .join("segments")
            .join(segment::segment_filename(segment_id));
        if path.exists() {
            Ok(path)
        } else {
            Err(Error::SegmentNotFound(segment_id))
        }
    }
}
