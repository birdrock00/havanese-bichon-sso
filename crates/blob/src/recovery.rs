use std::fs;
use std::path::Path;

use crate::bucket::IndexRecord;
use crate::error::Result;
use crate::meta::{GlobalMeta, SegmentStats};
use crate::segment::{self, SegmentReader};

/// Recover after a crash: scan any unindexed portions of segments,
/// update segment stats, and return newly discovered index records.
///
/// The caller is responsible for inserting the returned records into
/// the index store.
pub fn recover(store_root: &Path, meta: &mut GlobalMeta) -> Result<Vec<IndexRecord>> {
    scan_segments(store_root, meta, false)
}

/// Rebuild the entire index from scratch by scanning all segments from
/// offset 0.  Used when the index database is corrupted or lost.
///
/// Resets all segment stats and returns every entry found on disk.
/// The caller should replace the index database before calling this.
pub fn rebuild_index(store_root: &Path, meta: &mut GlobalMeta) -> Result<Vec<IndexRecord>> {
    // Reset all segment stats — they'll be recomputed during the scan.
    // Also reset indexed_up_to_offset so we scan from 0.
    for stats in meta.segments.values_mut() {
        stats.total_bytes = 0;
        stats.deleted_bytes = 0;
        stats.deleted_ratio = 0.0;
        stats.indexed_up_to_offset = 0;
    }
    scan_segments(store_root, meta, true)
}

/// Common implementation: scan segments and collect index records.
/// When `full_scan` is true, every segment is scanned from offset 0.
fn scan_segments(
    store_root: &Path,
    meta: &mut GlobalMeta,
    full_scan: bool,
) -> Result<Vec<IndexRecord>> {
    let seg_dir = store_root.join("segments");
    if !seg_dir.exists() {
        fs::create_dir_all(&seg_dir)?;
    }

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

    let mut all_records: Vec<IndexRecord> = Vec::new();

    // For each segment, scan unindexed portions
    for &seg_id in &disk_segments {
        let seg_path = seg_dir.join(segment::segment_filename(seg_id));
        let file_size = fs::metadata(&seg_path)?.len();

        let mut stats = meta
            .segments
            .remove(&seg_id)
            .unwrap_or_else(|| SegmentStats::new(seg_id));
        let is_sealed = seg_id != meta.active_segment_id;
        stats.sealed = is_sealed;

        // Determine scan range.
        let scan_start = if full_scan {
            0
        } else if stats.indexed_up_to_offset <= file_size {
            stats.indexed_up_to_offset
        } else {
            // Segment was replaced (interrupted GC) — full rescan needed.
            0
        };

        if scan_start >= file_size {
            meta.segments.insert(seg_id, stats);
            continue;
        }

        let reader = SegmentReader::open(seg_path.clone(), seg_id)?;

        let truncation_point = reader.scan_entries(scan_start, |entry, offset| {
            all_records.push(IndexRecord::new(
                entry.key,
                seg_id,
                offset,
                entry.data.len() as u32,
                entry.flags,
            ));

            stats.total_bytes += entry.data.len() as u64;
            if entry.is_tombstone() {
                stats.deleted_bytes += entry.raw_size as u64;
            }

            Ok(())
        })?;

        // Truncate if tail corruption found
        if truncation_point < file_size {
            segment::truncate_segment(&seg_path, truncation_point)?;
        }

        stats.indexed_up_to_offset = truncation_point;
        stats.recompute_ratio();
        meta.segments.insert(seg_id, stats);
    }

    meta.save(store_root)?;

    Ok(all_records)
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
    Ok(())
}
