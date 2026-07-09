use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::bucket::{self, IndexRecord};
use crate::error::Result;
use crate::meta::{GlobalMeta, SegmentStats};
use crate::segment::{self, SegmentReader};

/// Recover after a crash: scan any unindexed portions of segments,
/// update bucket files, fix segment stats.
pub fn recover(store_root: &Path, meta: &mut GlobalMeta) -> Result<()> {
    let seg_dir = store_root.join("segments");
    if !seg_dir.exists() {
        fs::create_dir_all(&seg_dir)?;
    }

    let buckets_dir = store_root.join("buckets");
    fs::create_dir_all(&buckets_dir)?;

    // Discover all segment files on disk
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
        return Ok(());
    }

    // For each segment, scan unindexed portions and append to bucket files
    for &seg_id in &disk_segments {
        let seg_path = seg_dir.join(segment::segment_filename(seg_id));
        let file_size = fs::metadata(&seg_path)?.len();

        let mut stats = meta
            .segments
            .remove(&seg_id)
            .unwrap_or_else(|| SegmentStats::new(seg_id));
        let is_sealed = seg_id != meta.active_segment_id;
        stats.sealed = is_sealed;

        let scan_start = if stats.indexed_up_to_offset <= file_size {
            stats.indexed_up_to_offset
        } else {
            0
        };

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

        // Append new records to bucket files
        for (bid, records) in &new_records {
            let bf_path = buckets_dir.join(format!("{:02x}.idx", bid));
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&bf_path)?;
            for r in records {
                file.write_all(&r.encode())?;
            }
        }

        // Truncate if tail corruption found
        if truncation_point < file_size {
            segment::truncate_segment(&seg_path, truncation_point)?;
        }

        stats.indexed_up_to_offset = truncation_point;
        stats.recompute_ratio();
        meta.segments.insert(seg_id, stats);
    }

    meta.save(store_root)?;

    Ok(())
}

/// Clean up leftover temp files from interrupted GC.
pub fn cleanup_temp_files(store_root: &Path) -> Result<()> {
    let seg_dir = store_root.join("segments");
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
    let buckets_dir = store_root.join("buckets");
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
