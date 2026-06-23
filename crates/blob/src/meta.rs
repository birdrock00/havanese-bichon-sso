use std::collections::BTreeMap;
use std::path::Path;

use crate::checksum;
use crate::error::Result;
use serde::{Deserialize, Serialize};

const META_VERSION: u32 = 1;

// ── Helpers ────────────────────────────────────────────────────────────────

fn write_bin<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let payload = bincode::serialize(value).map_err(|e| {
        crate::error::Error::CorruptMeta(format!("{}: bincode encode: {}", path.display(), e))
    })?;
    let crc = checksum::crc32(&payload);
    let mut buf = Vec::with_capacity(8 + payload.len());
    buf.extend_from_slice(&crc.to_le_bytes());
    buf.extend_from_slice(&META_VERSION.to_le_bytes());
    buf.extend_from_slice(&payload);

    let tmp = path.with_extension("bin.tmp");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&buf)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
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
    bincode::deserialize(&data[8..]).map_err(|e| {
        crate::error::Error::CorruptMeta(format!("{}: bincode decode: {}", path.display(), e))
    })
}

// ── GlobalMeta ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMeta {
    pub version: u32,
    pub accounts: Vec<String>,
}

impl Default for GlobalMeta {
    fn default() -> Self {
        Self {
            version: META_VERSION,
            accounts: Vec::new(),
        }
    }
}

impl GlobalMeta {
    pub fn load(store_root: &Path) -> Result<Self> {
        let bin_path = store_root.join("global_meta.bin");
        if bin_path.exists() {
            return read_bin(&bin_path);
        }
        // Migration from JSON
        let json_path = store_root.join("global_meta.json");
        if json_path.exists() {
            let data = std::fs::read_to_string(&json_path)?;
            let mut meta: Self = serde_json::from_str(&data)?;
            meta.accounts.sort();
            write_bin(&bin_path, &meta)?;
            let _ = std::fs::remove_file(&json_path);
            return Ok(meta);
        }
        Ok(Self::default())
    }

    pub fn save(&self, store_root: &Path) -> Result<()> {
        let path = store_root.join("global_meta.bin");
        let mut meta = self.clone();
        meta.accounts.sort();
        write_bin(&path, &meta)
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
    /// Recovery starts scanning from here instead of 0.
    pub indexed_up_to_offset: u64,
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

// ── AccountMeta ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMeta {
    pub account_id: String,
    pub active_segment_id: u32,
    pub segments: BTreeMap<u32, SegmentStats>,
}

impl AccountMeta {
    pub fn new(account_id: String, active_segment_id: u32) -> Self {
        Self {
            account_id,
            active_segment_id,
            segments: BTreeMap::new(),
        }
    }

    pub fn load(account_dir: &Path) -> Result<Self> {
        let bin_path = account_dir.join("meta.bin");
        if bin_path.exists() {
            return read_bin(&bin_path);
        }
        // Migration from JSON
        let json_path = account_dir.join("meta.json");
        if json_path.exists() {
            let data = std::fs::read_to_string(&json_path)?;
            let meta: Self = serde_json::from_str(&data)?;
            write_bin(&bin_path, &meta)?;
            let _ = std::fs::remove_file(&json_path);
            return Ok(meta);
        }
        Err(crate::error::Error::AccountNotFound(
            account_dir.to_string_lossy().into(),
        ))
    }

    pub fn save(&self, account_dir: &Path) -> Result<()> {
        write_bin(&account_dir.join("meta.bin"), self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_global_meta_bin_roundtrip() {
        let dir = TempDir::new().unwrap();
        let mut meta = GlobalMeta::default();
        meta.accounts.push("alice".into());
        meta.save(dir.path()).unwrap();

        let loaded = GlobalMeta::load(dir.path()).unwrap();
        assert_eq!(loaded.accounts, vec!["alice"]);
        assert!(!dir.path().join("global_meta.json").exists());
        assert!(dir.path().join("global_meta.bin").exists());
    }

    #[test]
    fn test_global_meta_default_when_missing() {
        let dir = TempDir::new().unwrap();
        let meta = GlobalMeta::load(dir.path()).unwrap();
        assert!(meta.accounts.is_empty());
    }

    #[test]
    fn test_json_migration() {
        let dir = TempDir::new().unwrap();
        // Write old JSON format
        let json = r#"{"version":1,"accounts":["bob","alice"]}"#;
        std::fs::write(dir.path().join("global_meta.json"), json).unwrap();

        let meta = GlobalMeta::load(dir.path()).unwrap();
        // Should be sorted
        assert_eq!(meta.accounts, vec!["alice", "bob"]);
        // JSON should be removed
        assert!(!dir.path().join("global_meta.json").exists());
        // BIN should exist
        assert!(dir.path().join("global_meta.bin").exists());
    }

    #[test]
    fn test_account_meta_bin_roundtrip() {
        let dir = TempDir::new().unwrap();
        let mut meta = AccountMeta::new("alice".into(), 1);
        meta.segments.insert(
            1,
            SegmentStats {
                segment_id: 1,
                total_bytes: 1000,
                deleted_bytes: 300,
                deleted_ratio: 0.3,
                sealed: false,
                indexed_up_to_offset: 0,
            },
        );
        meta.save(dir.path()).unwrap();

        let loaded = AccountMeta::load(dir.path()).unwrap();
        assert_eq!(loaded.active_segment_id, 1);
        assert_eq!(loaded.segments[&1].total_bytes, 1000);
    }

    #[test]
    fn test_corrupt_bin_detected() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("meta.bin"), vec![0xFFu8; 100]).unwrap();
        let result = AccountMeta::load(dir.path());
        assert!(result.is_err());
        // 0xFFFFFFFF version triggers UnsupportedMetaVersion
        assert!(matches!(result.unwrap_err(), crate::error::Error::UnsupportedMetaVersion { .. }));
    }

    #[test]
    fn test_crc_corruption_detected() {
        let dir = TempDir::new().unwrap();
        // Write a well-formed header (version=1) but with wrong CRC bytes
        let mut buf = Vec::new();
        buf.extend_from_slice(&0xDEADBEEFu32.to_le_bytes()); // wrong CRC
        buf.extend_from_slice(&1u32.to_le_bytes());           // version = 1 (OK)
        buf.extend_from_slice(b"some payload bytes");         // payload
        std::fs::write(dir.path().join("meta.bin"), &buf).unwrap();
        let result = AccountMeta::load(dir.path());
        assert!(matches!(result.unwrap_err(), crate::error::Error::CorruptMeta(_)));
    }

    #[test]
    fn test_account_json_migration() {
        let dir = TempDir::new().unwrap();
        // Write old JSON format for AccountMeta
        let json = r#"{"account_id":"alice","active_segment_id":5,"segments":{}}"#;
        std::fs::write(dir.path().join("meta.json"), json).unwrap();

        let meta = AccountMeta::load(dir.path()).unwrap();
        assert_eq!(meta.account_id, "alice");
        assert_eq!(meta.active_segment_id, 5);
        // JSON should be removed
        assert!(!dir.path().join("meta.json").exists());
        // BIN should exist
        assert!(dir.path().join("meta.bin").exists());
    }

    #[test]
    fn test_bin_sorted_keys() {
        let dir = TempDir::new().unwrap();
        let mut meta = AccountMeta::new("test".into(), 1);
        meta.segments.insert(3, SegmentStats::new(3));
        meta.segments.insert(1, SegmentStats::new(1));
        meta.segments.insert(2, SegmentStats::new(2));
        meta.save(dir.path()).unwrap();

        let loaded = AccountMeta::load(dir.path()).unwrap();
        let keys: Vec<u32> = loaded.segments.keys().copied().collect();
        assert_eq!(keys, vec![1, 2, 3]);
    }
}
