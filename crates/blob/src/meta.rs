use std::collections::HashMap;
use std::path::Path;

use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMeta {
    pub version: u32,
    pub accounts: Vec<String>,
}

impl Default for GlobalMeta {
    fn default() -> Self {
        Self {
            version: 1,
            accounts: Vec::new(),
        }
    }
}

impl GlobalMeta {
    pub fn load(store_root: &Path) -> Result<Self> {
        let path = store_root.join("global_meta.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save(&self, store_root: &Path) -> Result<()> {
        let path = store_root.join("global_meta.json");
        let tmp = path.with_extension("json.tmp");
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&tmp, &data)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMeta {
    pub account_id: String,
    pub active_segment_id: u32,
    pub segments: HashMap<u32, SegmentStats>,
}

impl AccountMeta {
    pub fn new(account_id: String, active_segment_id: u32) -> Self {
        Self {
            account_id,
            active_segment_id,
            segments: HashMap::new(),
        }
    }

    pub fn load(account_dir: &Path) -> Result<Self> {
        let path = account_dir.join("meta.json");
        if !path.exists() {
            return Err(crate::error::Error::AccountNotFound(
                account_dir.to_string_lossy().into(),
            ));
        }
        let data = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save(&self, account_dir: &Path) -> Result<()> {
        let path = account_dir.join("meta.json");
        let tmp = path.with_extension("json.tmp");
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&tmp, &data)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_global_meta_roundtrip() {
        let dir = TempDir::new().unwrap();
        let mut meta = GlobalMeta::default();
        meta.accounts.push("alice".into());
        meta.save(dir.path()).unwrap();

        let loaded = GlobalMeta::load(dir.path()).unwrap();
        assert_eq!(loaded.accounts, vec!["alice"]);
    }

    #[test]
    fn test_global_meta_default_when_missing() {
        let dir = TempDir::new().unwrap();
        let meta = GlobalMeta::load(dir.path()).unwrap();
        assert!(meta.accounts.is_empty());
    }

    #[test]
    fn test_account_meta_roundtrip() {
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
}
