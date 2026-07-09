use std::collections::BTreeMap;
use std::path::Path;

use crate::checksum;
use crate::error::Result;
use serde::{Deserialize, Serialize};

const META_VERSION: u32 = 2;

// ── Helpers ────────────────────────────────────────────────────────────────

fn write_bin<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let payload = bincode::serde::encode_to_vec(value, bincode::config::standard()).map_err(|e| {
        crate::error::Error::CorruptMeta(format!("{}: bincode encode: {}", path.display(), e))
    })?;
    let crc = checksum::crc32(&payload);
    let mut buf = Vec::with_capacity(8 + payload.len());
    buf.extend_from_slice(&crc.to_le_bytes());
    buf.extend_from_slice(&META_VERSION.to_le_bytes());
    buf.extend_from_slice(&payload);

    crate::fs::create_atomic(path, &buf)?;
    Ok(())
}

fn read_bin<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let data = std::fs::read(path)?;
    if data.len() < 8 {
        return Err(crate::error::Error::CorruptMeta(path.display().to_string()));
    }
    let stored_crc = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if version != META_VERSION {
        return Err(crate::error::Error::UnsupportedMetaVersion {
            path: path.to_path_buf(),
            version,
        });
    }
    let computed = checksum::crc32(&data[8..]);
    if stored_crc != computed {
        return Err(crate::error::Error::CorruptMeta(path.display().to_string()));
    }
    bincode::serde::decode_from_slice(&data[8..], bincode::config::standard())
        .map(|(v, _)| v)
        .map_err(|e| {
            crate::error::Error::CorruptMeta(format!("{}: bincode decode: {}", path.display(), e))
    })
}

// ── SegmentStats ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentStats {
    pub segment_id: u32,
    pub total_bytes: u64,
    pub deleted_bytes: u64,
    pub deleted_ratio: f64,
    pub sealed: bool,
    /// Byte offset up to which entries have been indexed in bucket files.
    pub indexed_up_to_offset: u64,
    /// Number of compacted (sorted, deduped) records in each bucket file for this segment.
    /// Used by BucketStore on recovery to know where the clean portion ends.
    pub bucket_compacted: u64,
}

impl SegmentStats {
    pub fn new(segment_id: u32) -> Self {
        Self {
            segment_id,
            total_bytes: 0,
            deleted_bytes: 0,
            deleted_ratio: 0.0,
            sealed: false,
            indexed_up_to_offset: 0,
            bucket_compacted: 0,
        }
    }

    pub fn recompute_ratio(&mut self) {
        if self.total_bytes > 0 {
            self.deleted_ratio = self.deleted_bytes as f64 / self.total_bytes as f64;
        } else {
            self.deleted_ratio = 0.0;
        }
    }
}

// ── GlobalMeta ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMeta {
    pub version: u32,
    pub active_segment_id: u32,
    pub segments: BTreeMap<u32, SegmentStats>,
}

impl GlobalMeta {
    pub fn new() -> Self {
        Self {
            version: META_VERSION,
            active_segment_id: 1,
            segments: BTreeMap::new(),
        }
    }

    pub fn load(store_root: &Path) -> Result<Self> {
        let bin_path = store_root.join("meta.bin");
        if bin_path.exists() {
            return read_bin(&bin_path);
        }
        // Migration from old JSON format
        let json_path = store_root.join("meta.json");
        if json_path.exists() {
            let data = std::fs::read_to_string(&json_path)?;
            let meta: Self = serde_json::from_str(&data)?;
            write_bin(&bin_path, &meta)?;
            let _ = std::fs::remove_file(&json_path);
            return Ok(meta);
        }
        // Migration from old global_meta.bin (v1, only had accounts list)
        let old_path = store_root.join("global_meta.bin");
        if old_path.exists() {
            let _ = std::fs::remove_file(&old_path);
        }
        Ok(Self::new())
    }

    pub fn save(&self, store_root: &Path) -> Result<()> {
        let path = store_root.join("meta.bin");
        write_bin(&path, self)
    }
}

impl Default for GlobalMeta {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_global_meta_roundtrip() {
        let dir = TempDir::new().unwrap();
        let mut meta = GlobalMeta::new();
        meta.segments.insert(
            1,
            SegmentStats {
                segment_id: 1,
                total_bytes: 1000,
                deleted_bytes: 300,
                deleted_ratio: 0.3,
                sealed: false,
                indexed_up_to_offset: 500,
                bucket_compacted: 0,
            },
        );
        meta.save(dir.path()).unwrap();

        let loaded = GlobalMeta::load(dir.path()).unwrap();
        assert_eq!(loaded.active_segment_id, 1);
        assert_eq!(loaded.segments[&1].total_bytes, 1000);
        assert_eq!(loaded.segments[&1].indexed_up_to_offset, 500);
    }

    #[test]
    fn test_global_meta_default_when_missing() {
        let dir = TempDir::new().unwrap();
        let meta = GlobalMeta::load(dir.path()).unwrap();
        assert_eq!(meta.active_segment_id, 1);
        assert!(meta.segments.is_empty());
    }

    #[test]
    fn test_corrupt_bin_detected() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("meta.bin"), vec![0xFFu8; 100]).unwrap();
        let result = GlobalMeta::load(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_segment_stats_recompute() {
        let mut s = SegmentStats::new(1);
        s.total_bytes = 1000;
        s.deleted_bytes = 250;
        s.recompute_ratio();
        assert!((s.deleted_ratio - 0.25).abs() < 0.001);
    }
}
