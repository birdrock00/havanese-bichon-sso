use std::{io, path::PathBuf};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("CRC32 mismatch at {path}:{offset}")]
    CrcMismatch { path: PathBuf, offset: u64 },

    #[error("Corrupt entry at {path}:{offset}: {reason}")]
    CorruptEntry {
        path: PathBuf,
        offset: u64,
        reason: String,
    },

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Account already exists: {0}")]
    AccountAlreadyExists(String),

    #[error("Segment not found: {0}")]
    SegmentNotFound(u32),

    #[error("Value too large: {size} bytes (max 100 MB)")]
    ValueTooLarge { size: usize },

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Disk full: {0}")]
    DiskFull(String),

    #[error("Invalid config: {0}")]
    InvalidConfig(String),

    #[error("Bucket index corrupt at {path}: {reason}")]
    BucketIndexCorrupt { path: PathBuf, reason: String },

    #[error("Segment file truncated at {path}: expected {expected}, got {actual}")]
    SegmentTruncated {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
}
