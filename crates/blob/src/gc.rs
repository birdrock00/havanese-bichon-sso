use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::meta::GlobalMeta;
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

/// Prepared GC result — the compacted segment has been written to a temp file
/// but not yet renamed over the original.  `gc_finish` must be called to commit.
pub struct GcPrepare {
    pub segment_id: u32,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub entries_kept: usize,
    pub entries_skipped: usize,
    temp_path: PathBuf,
    seg_path: PathBuf,
}

/// Phase 1: pick the sealed segment with the highest deleted_ratio, scan all
/// segments to determine the latest entry for each key, then write a compacted
/// version of the target segment to a temp file.  Does NOT rename — the caller
/// should hold the write lock only during `gc_finish`.
pub fn gc_prepare(
    store_root: &Path,
    meta: &GlobalMeta,
    deleted_ratio_threshold: f64,
) -> Result<Option<GcPrepare>> {
    let candidate = meta
        .segments
        .values()
        .filter(|s| s.sealed && s.deleted_ratio >= deleted_ratio_threshold)
        .max_by(|a, b| a.deleted_ratio.partial_cmp(&b.deleted_ratio).unwrap());

    let target = match candidate {
        Some(s) => s.clone(),
        None => return Ok(None),
    };

    let seg_path = store_root
        .join("segments")
        .join(segment::segment_filename(target.segment_id));
    let reader = SegmentReader::open(seg_path.clone(), target.segment_id)?;

    // Build a global view: for each key, which entry (segment_id + offset) is the latest?
    let mut latest_key: HashMap<[u8; 32], (u32, u64)> = HashMap::new();

    for &seg_id in meta.segments.keys() {
        let rpath = store_root
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

    // Create temp segment (not renamed yet)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_name = format!("temp_{:016x}.seg", timestamp);
    let temp_path = store_root.join("segments").join(&temp_name);
    let mut writer = SegmentWriter::create(temp_path.clone(), target.segment_id)?;

    let mut bytes_after: u64 = 0;
    let mut entries_kept: usize = 0;
    let mut entries_skipped: usize = 0;

    reader.scan_entries(0, |entry, offset| {
        if entry.is_tombstone() {
            entries_skipped += 1;
            return Ok(());
        }
        if let Some((latest_seg, latest_off)) = latest_key.get(&entry.key) {
            if *latest_seg != target.segment_id || *latest_off != offset {
                entries_skipped += 1;
                return Ok(());
            }
        }
        writer.append(entry)?;
        bytes_after += entry.data.len() as u64;
        entries_kept += 1;
        Ok(())
    })?;

    writer.fsync()?;

    Ok(Some(GcPrepare {
        segment_id: target.segment_id,
        bytes_before: target.total_bytes,
        bytes_after,
        entries_kept,
        entries_skipped,
        temp_path,
        seg_path,
    }))
}

/// Phase 2: atomically replace the old segment with the compacted one.
pub fn gc_finish(prep: GcPrepare) -> Result<GcStats> {
    fs::rename(&prep.temp_path, &prep.seg_path)?;
    Ok(GcStats {
        segment_id: prep.segment_id,
        bytes_before: prep.bytes_before,
        bytes_after: prep.bytes_after,
        entries_kept: prep.entries_kept,
        entries_skipped: prep.entries_skipped,
    })
}
