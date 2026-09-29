use std::collections::BTreeMap;
use std::path::Path;

use crate::checksum;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// v3 added `SegmentStats::sha256` (computed at segment seal). v2 payloads
/// are migrated in [`GlobalMeta::load`] with a `None` hash — the first backup
/// after an upgrade backfills it.
const META_VERSION: u32 = 3;

// ── Helpers ────────────────────────────────────────────────────────────────

fn write_bin<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let payload =
        bincode::serde::encode_to_vec(value, bincode::config::standard()).map_err(|e| {
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

/// Decode a bincode payload with a CRC check already performed by the caller.
fn decode_bin<T: for<'de> Deserialize<'de>>(payload: &[u8], path: &Path) -> Result<T> {
    bincode::serde::decode_from_slice(payload, bincode::config::standard())
        .map(|(v, _)| v)
        .map_err(|e| {
            crate::error::Error::CorruptMeta(format!("{}: bincode decode: {}", path.display(), e))
        })
}

/// The v2 on-disk shape, for migrating existing stores. Kept separate because
/// bincode cannot skip a struct field that was absent in the old format.
#[derive(Serialize, Deserialize)]
struct GlobalMetaV2 {
    pub version: u32,
    pub active_segment_id: u32,
    pub segments: BTreeMap<u32, SegmentStatsV2>,
}

#[derive(Serialize, Deserialize)]
struct SegmentStatsV2 {
    pub segment_id: u32,
    pub total_bytes: u64,
    pub deleted_bytes: u64,
    pub deleted_ratio: f64,
    pub sealed: bool,
    pub indexed_up_to_offset: u64,
    pub bucket_compacted: u64,
}

impl From<GlobalMetaV2> for GlobalMeta {
    fn from(v2: GlobalMetaV2) -> Self {
        let segments = v2
            .segments
            .into_iter()
            .map(|(id, s)| {
                (
                    id,
                    SegmentStats {
                        segment_id: s.segment_id,
                        total_bytes: s.total_bytes,
                        deleted_bytes: s.deleted_bytes,
                        deleted_ratio: s.deleted_ratio,
                        sealed: s.sealed,
                        indexed_up_to_offset: s.indexed_up_to_offset,
                        bucket_compacted: s.bucket_compacted,
                        // Pre-hash metadata: the next backup backfills the hash.
                        sha256: None,
                    },
                )
            })
            .collect();
        Self {
            version: META_VERSION,
            active_segment_id: v2.active_segment_id,
            segments,
        }
    }
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
    /// SHA-256 of the sealed segment file, computed at seal time while the
    /// data is page-cache warm (design doc §7). Recomputed when GC compacts
    /// the segment. `None` for segments sealed before hashing existed —
    /// backfilled (and persisted) on the next backup.
    pub sha256: Option<[u8; 32]>,
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
            sha256: None,
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
        if !bin_path.exists() {
            return Ok(Self::new());
        }
        let data = std::fs::read(&bin_path)?;
        if data.len() < 8 {
            return Err(crate::error::Error::CorruptMeta(bin_path.display().to_string()));
        }
        let stored_crc = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let payload = &data[8..];
        let computed = checksum::crc32(payload);
        if stored_crc != computed {
            return Err(crate::error::Error::CorruptMeta(bin_path.display().to_string()));
        }
        match version {
            // Old stores migrate forward; their segments get hashes on the
            // next backup.
            2 => decode_bin::<GlobalMetaV2>(payload, &bin_path).map(GlobalMeta::from),
            META_VERSION => decode_bin(payload, &bin_path),
            other => Err(crate::error::Error::UnsupportedMetaVersion {
                path: bin_path,
                version: other,
            }),
        }
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
                sealed: true,
                indexed_up_to_offset: 500,
                bucket_compacted: 0,
                sha256: Some([0xAB; 32]),
            },
        );
        meta.save(dir.path()).unwrap();

        let loaded = GlobalMeta::load(dir.path()).unwrap();
        assert_eq!(loaded.active_segment_id, 1);
        assert_eq!(loaded.segments[&1].total_bytes, 1000);
        assert_eq!(loaded.segments[&1].indexed_up_to_offset, 500);
        assert_eq!(loaded.segments[&1].sha256, Some([0xAB; 32]));
    }

    #[test]
    fn test_v2_meta_migrates_with_null_hashes() {
        let dir = TempDir::new().unwrap();
        // Craft a v2-format meta.bin by hand: encode the v2 structs and write
        // the v2 header (crc + version 2), exactly like an old store.
        let v2 = GlobalMetaV2 {
            version: 2,
            active_segment_id: 3,
            segments: std::collections::BTreeMap::from([(
                1,
                SegmentStatsV2 {
                    segment_id: 1,
                    total_bytes: 100,
                    deleted_bytes: 0,
                    deleted_ratio: 0.0,
                    sealed: true,
                    indexed_up_to_offset: 100,
                    bucket_compacted: 0,
                },
            )]),
        };
        let payload = bincode::serde::encode_to_vec(&v2, bincode::config::standard()).unwrap();
        let crc = checksum::crc32(&payload);
        let mut buf = Vec::with_capacity(8 + payload.len());
        buf.extend_from_slice(&crc.to_le_bytes());
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&payload);
        std::fs::write(dir.path().join("meta.bin"), &buf).unwrap();

        let loaded = GlobalMeta::load(dir.path()).unwrap();
        assert_eq!(loaded.active_segment_id, 3, "migrated store keeps its layout");
        assert_eq!(loaded.segments[&1].total_bytes, 100);
        assert_eq!(
            loaded.segments[&1].sha256,
            None,
            "v2 segments carry no hash; the next backup backfills it"
        );
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
