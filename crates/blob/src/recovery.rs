use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::bucket::{self, BucketFile, IndexRecord};
use crate::error::Result;
use crate::meta::{AccountMeta, SegmentStats};
use crate::segment::{self, SegmentReader};

/// Recover an account after a crash: scan segments, repair indices, update stats.
pub fn recover_account(account_dir: &Path) -> Result<AccountMeta> {
    let meta_path = account_dir.join("meta.json");
    let mut meta = if meta_path.exists() {
        AccountMeta::load(account_dir).unwrap_or_else(|_| {
            AccountMeta::new(
                account_dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                1,
            )
        })
    } else {
        return Ok(AccountMeta::new(
            account_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            1,
        ));
    };

    // Discover all segment files on disk
    let seg_dir = account_dir.join("segments");
    if !seg_dir.exists() {
        fs::create_dir_all(&seg_dir)?;
    }

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

    if disk_segments.is_empty() {
        meta.active_segment_id = 1;
    } else {
        let max_id = *disk_segments.last().unwrap();
        meta.active_segment_id = max_id;
    }

    // Ensure buckets directory exists
    let buckets_dir = account_dir.join("buckets");
    fs::create_dir_all(&buckets_dir)?;

    // For each segment, scan only the unindexed tail and update stats incrementally
    for &seg_id in &disk_segments {
        let seg_path = seg_dir.join(segment::segment_filename(seg_id));
        let file_size = fs::metadata(&seg_path)?.len();

        // Preserve existing stats; start fresh if this is a newly discovered segment
        let mut stats = meta.segments.remove(&seg_id).unwrap_or_else(|| SegmentStats::new(seg_id));
        let is_sealed = seg_id != meta.active_segment_id;
        stats.sealed = is_sealed;

        // Scan start: from last indexed offset. Clamp defensively.
        let scan_start = if stats.indexed_up_to_offset <= file_size {
            stats.indexed_up_to_offset
        } else {
            0
        };

        // If fully indexed, skip scanning entirely
        if scan_start >= file_size {
            meta.segments.insert(seg_id, stats);
            continue;
        }

        let reader = SegmentReader::open(seg_path.clone(), seg_id)?;
        let mut new_records: HashMap<u16, Vec<IndexRecord>> = HashMap::new();

        let truncation_point = reader.scan_entries(scan_start, |entry, offset| {
            let bid = bucket::bucket_id(&entry.key);
            let rec = IndexRecord::new(
                entry.key,
                seg_id,
                offset,
                entry.data.len() as u32,
                entry.flags,
            );
            new_records.entry(bid).or_default().push(rec);

            stats.total_bytes += entry.data.len() as u64;
            if entry.is_tombstone() {
                stats.deleted_bytes += entry.raw_size as u64;
            }

            Ok(())
        })?;

        // Merge new records into bucket files (only the newly discovered ones)
        for (bid, records) in &new_records {
            let bf = BucketFile::open(account_dir, *bid);
            bf.append_batch(records)?;
        }

        // Truncate if tail corruption found
        if truncation_point < file_size {
            segment::truncate_segment(&seg_path, truncation_point)?;
        }

        stats.indexed_up_to_offset = truncation_point;
        stats.recompute_ratio();
        meta.segments.insert(seg_id, stats);
    }

    meta.save(account_dir)?;

    Ok(meta)
}

/// Clean up leftover temp files from interrupted GC.
pub fn cleanup_temp_files(account_dir: &Path) -> Result<()> {
    let seg_dir = account_dir.join("segments");
    if seg_dir.exists() {
        for entry in fs::read_dir(&seg_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("temp_") {
                let path = entry.path();
                tracing::warn!("Removing leftover temp file: {:?}", path);
                fs::remove_file(&path)?;
            }
        }
    }
    // Also cleanup temp bucket files
    let buckets_dir = account_dir.join("buckets");
    if buckets_dir.exists() {
        for entry in fs::read_dir(&buckets_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".tmp") {
                let path = entry.path();
                tracing::warn!("Removing leftover temp bucket file: {:?}", path);
                fs::remove_file(&path)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_recover_fresh_account() {
        let dir = TempDir::new().unwrap();
        let account_dir = dir.path().join("test");
        fs::create_dir_all(&account_dir).unwrap();

        let meta = recover_account(&account_dir).unwrap();
        assert_eq!(meta.active_segment_id, 1);
        assert!(meta.segments.is_empty());
    }

    #[test]
    fn test_cleanup_temp_files() {
        let dir = TempDir::new().unwrap();
        let account_dir = dir.path().join("test");
        fs::create_dir_all(account_dir.join("segments")).unwrap();
        fs::create_dir_all(account_dir.join("buckets")).unwrap();
        fs::write(
            account_dir.join("segments").join("temp_ABC123.seg"),
            b"garbage",
        )
        .unwrap();
        fs::write(account_dir.join("buckets").join("00.idx.tmp"), b"garbage").unwrap();

        cleanup_temp_files(&account_dir).unwrap();

        assert!(!account_dir.join("segments").join("temp_ABC123.seg").exists());
    }
}
