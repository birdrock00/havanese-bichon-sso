use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bucket::{self, BucketFile, BucketIndex, IndexRecord};
use crate::error::Result;
#[cfg(test)]
use crate::meta::SegmentStats;
use crate::segment::{self, SegmentReader, SegmentWriter};

/// Result of a GC run.
#[derive(Debug)]
pub struct GcStats {
    pub segment_id: u32,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub entries_kept: usize,
    pub entries_skipped: usize,
}

/// Run GC on an account: pick the sealed segment with highest deleted_ratio,
/// rewrite it without deleted/overwritten entries, then rebuild all bucket files.
pub fn gc_account(
    account_dir: &Path,
    deleted_ratio_threshold: f64,
) -> Result<Option<GcStats>> {
    let meta = crate::meta::AccountMeta::load(account_dir)?;

    // Find the best candidate
    let candidate = meta
        .segments
        .values()
        .filter(|s| s.sealed && s.deleted_ratio >= deleted_ratio_threshold)
        .max_by(|a, b| a.deleted_ratio.partial_cmp(&b.deleted_ratio).unwrap());

    let target = match candidate {
        Some(s) => s.clone(),
        None => return Ok(None),
    };

    let seg_path = account_dir
        .join("segments")
        .join(segment::segment_filename(target.segment_id));
    let reader = SegmentReader::open(seg_path.clone(), target.segment_id)?;

    // Build a global view: for each key, which entry (segment_id + offset) is the latest?
    let mut latest_key: HashMap<[u8; 32], (u32, u64)> = HashMap::new();

    for &seg_id in meta.segments.keys() {
        let rpath = account_dir
            .join("segments")
            .join(segment::segment_filename(seg_id));
        if !rpath.exists() {
            continue;
        }
        let r = SegmentReader::open(rpath, seg_id)?;
        let _ = r.scan_entries(0, |entry, offset| {
            match latest_key.get(&entry.key) {
                Some((existing_seg, existing_off)) => {
                    if seg_id > *existing_seg
                        || (seg_id == *existing_seg && offset > *existing_off)
                    {
                        latest_key.insert(entry.key, (seg_id, offset));
                    }
                }
                None => {
                    latest_key.insert(entry.key, (seg_id, offset));
                }
            }
            Ok(())
        })?;
    }

    // Create temp segment with a unique name
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_name = format!("temp_{:016x}.seg", timestamp);
    let temp_path = account_dir.join("segments").join(&temp_name);
    let mut writer = SegmentWriter::create(temp_path.clone(), target.segment_id)?;

    let mut bytes_after: u64 = 0;
    let mut entries_kept: usize = 0;
    let mut entries_skipped: usize = 0;

    reader.scan_entries(0, |entry, offset| {
        // Skip tombstones
        if entry.is_tombstone() {
            entries_skipped += 1;
            return Ok(());
        }
        // Skip if this key has a newer entry in another segment
        if let Some((latest_seg, latest_off)) = latest_key.get(&entry.key) {
            if *latest_seg != target.segment_id || *latest_off != offset {
                entries_skipped += 1;
                return Ok(());
            }
        }
        // Keep this entry
        writer.append(entry)?;
        bytes_after += entry.data.len() as u64;
        entries_kept += 1;
        Ok(())
    })?;

    writer.fsync()?;

    // Atomic rename: replace old segment with new one
    fs::rename(&temp_path, &seg_path)?;

    // Rebuild all bucket files
    rebuild_buckets(account_dir, &meta)?;

    // Update meta
    let mut meta = crate::meta::AccountMeta::load(account_dir)?;
    if let Some(stats) = meta.segments.get_mut(&target.segment_id) {
        stats.total_bytes = bytes_after;
        stats.deleted_bytes = 0;
        stats.recompute_ratio();
    }
    meta.save(account_dir)?;

    Ok(Some(GcStats {
        segment_id: target.segment_id,
        bytes_before: target.total_bytes,
        bytes_after,
        entries_kept,
        entries_skipped,
    }))
}

/// Rebuild all 16 bucket files from scratch by scanning all segments.
fn rebuild_buckets(account_dir: &Path, meta: &crate::meta::AccountMeta) -> Result<()> {
    let mut bucket_records: HashMap<u16, Vec<IndexRecord>> = HashMap::new();
    for i in 0..crate::types::BUCKET_COUNT {
        bucket_records.insert(i, Vec::new());
    }

    for &seg_id in meta.segments.keys() {
        let seg_path = account_dir
            .join("segments")
            .join(segment::segment_filename(seg_id));
        if !seg_path.exists() {
            continue;
        }
        let reader = SegmentReader::open(seg_path, seg_id)?;
        reader.scan_entries(0, |entry, offset| {
            let bid = bucket::bucket_id(&entry.key);
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
        let index = BucketIndex::from_records(records.clone(), *bid);
        let bf = BucketFile::open(account_dir, *bid);
        bf.rewrite(&index.records)?;
    }

    Ok(())
}

/// Compact bucket files: load, dedup, rewrite.
pub fn compact_buckets(account_dir: &Path) -> Result<()> {
    for bid in 0..crate::types::BUCKET_COUNT {
        let bf = BucketFile::open(account_dir, bid);
        if bf.path().exists() {
            let index = bf.load_index()?;
            bf.rewrite(&index.records)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segment::Entry;
    use crate::types::Codec;
    use tempfile::TempDir;

    fn setup_account(dir: &Path) {
        fs::create_dir_all(dir.join("segments")).unwrap();
        crate::bucket::BucketFile::ensure_dir(dir).unwrap();

        let seg_path = dir
            .join("segments")
            .join(segment::segment_filename(1));
        let mut writer = SegmentWriter::create(seg_path, 1).unwrap();

        // Write 5 entries
        for i in 0..5u8 {
            let mut key = [0u8; 32];
            key[0] = i;
            let entry = Entry::new(key, &vec![i; 1000], 0, Codec::None);
            writer.append(&entry).unwrap();
        }

        // Tombstone entry 2
        let mut key2 = [0u8; 32];
        key2[0] = 2;
        let tomb = Entry::tombstone(key2);
        writer.append(&tomb).unwrap();

        writer.fsync().unwrap();

        // Save meta
        let mut meta = crate::meta::AccountMeta::new("test".into(), 2);
        meta.segments.insert(
            1,
            SegmentStats {
                segment_id: 1,
                total_bytes: 6000,
                deleted_bytes: 1000,
                deleted_ratio: 1000.0 / 6000.0,
                sealed: true,
                indexed_up_to_offset: 0,
            },
        );
        // Make segment 2 active so segment 1 is sealed
        let seg2_path = dir
            .join("segments")
            .join(segment::segment_filename(2));
        SegmentWriter::create(seg2_path, 2).unwrap();
        meta.save(dir).unwrap();
    }

    #[test]
    fn test_gc_removes_tombstones() {
        let dir = TempDir::new().unwrap();
        setup_account(dir.path());

        let result = gc_account(dir.path(), 0.01).unwrap();
        assert!(result.is_some());

        // Verify segment 1 no longer has the tombstone'd entry
        let seg_path = dir
            .path()
            .join("segments")
            .join(segment::segment_filename(1));
        let reader = SegmentReader::open(seg_path, 1).unwrap();
        let mut count = 0;
        reader.scan_entries(0, |entry, _offset| {
            count += 1;
            assert!(entry.key[0] != 2);
            Ok(())
        }).unwrap();
        assert_eq!(count, 4); // 5 original - 1 tombstoned
    }

    #[test]
    fn test_compact_buckets() {
        let dir = TempDir::new().unwrap();
        setup_account(dir.path());
        compact_buckets(dir.path()).unwrap();
        // Should not panic
    }
}
