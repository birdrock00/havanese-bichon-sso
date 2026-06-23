use serde::{Deserialize, Serialize};

/// Magic number for entry identification
pub const ENTRY_MAGIC: u32 = 0xB3DB_0001;

/// Fixed header size: magic(4) + crc32(4) + flags(1) + codec(1) + key(32) + raw_size(4) + data_size(4)
pub const ENTRY_HEADER_SIZE: usize = 50;

/// Index record size: key(32) + segment_id(4) + offset(8) + data_size(4) + flags(1) + _pad(3)
pub const INDEX_RECORD_SIZE: usize = 52;

/// Maximum segment size (256 MB)
pub const SEGMENT_MAX_SIZE: u64 = 256 * 1024 * 1024;

/// Number of hash buckets per account
pub const BUCKET_COUNT: u16 = 16;

/// Maximum value size (100 MB)
pub const MAX_VALUE_SIZE: usize = 100 * 1024 * 1024;

/// Default compression threshold (4 KB)
pub const DEFAULT_COMPRESS_THRESHOLD: usize = 4096;

/// Default LRU bucket cache size
pub const DEFAULT_LRU_BUCKET_COUNT: usize = 256;

/// Default GC deleted ratio threshold
pub const DEFAULT_GC_DELETED_RATIO: f64 = 0.30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Codec {
    None = 0,
    Zstd = 1,
    Lz4 = 2,
}

impl Codec {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Codec::None),
            1 => Some(Codec::Zstd),
            2 => Some(Codec::Lz4),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub compress_threshold: usize,
    pub default_codec: Codec,
    pub compression_level: i32,
    pub lru_bucket_count: usize,
    pub gc_deleted_ratio: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            compress_threshold: DEFAULT_COMPRESS_THRESHOLD,
            default_codec: Codec::Zstd,
            compression_level: 0,
            lru_bucket_count: DEFAULT_LRU_BUCKET_COUNT,
            gc_deleted_ratio: DEFAULT_GC_DELETED_RATIO,
        }
    }
}

impl Config {
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.lru_bucket_count == 0 {
            return Err(crate::error::Error::InvalidConfig(
                "lru_bucket_count must be > 0".into(),
            ));
        }
        if self.gc_deleted_ratio <= 0.0 || self.gc_deleted_ratio >= 1.0 {
            return Err(crate::error::Error::InvalidConfig(
                "gc_deleted_ratio must be in (0.0, 1.0)".into(),
            ));
        }
        if self.compression_level < 0 {
            return Err(crate::error::Error::InvalidConfig(
                "compression_level must be >= 0".into(),
            ));
        }
        Ok(())
    }
}
