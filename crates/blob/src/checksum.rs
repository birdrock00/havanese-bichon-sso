use std::io::Read;
use std::path::Path;

use crc32fast::Hasher;

use crate::error::Result;

pub fn crc32(data: &[u8]) -> u32 {
    let mut h = Hasher::new();
    h.update(data);
    h.finalize()
}

/// SHA-256 of a file, streamed in 1 MiB chunks so a multi-hundred-MB segment
/// never sits in memory. Called at seal time and after GC compaction, when
/// the file was just written and is page-cache warm (design doc §7).
pub fn sha256_file(path: &Path) -> Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().into())
}

pub struct CrcWriter {
    hasher: Hasher,
}

impl Default for CrcWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl CrcWriter {
    pub fn new() -> Self {
        Self {
            hasher: Hasher::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    pub fn finalize(self) -> u32 {
        self.hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_deterministic() {
        let a = crc32(b"hello");
        let b = crc32(b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn test_crc32_different() {
        let a = crc32(b"hello");
        let b = crc32(b"world");
        assert!(a != b);
    }

    #[test]
    fn test_crc_writer_matches_crc32() {
        let mut w = CrcWriter::new();
        w.update(b"hello");
        w.update(b" world");
        assert_eq!(w.finalize(), crc32(b"hello world"));
    }

    #[test]
    fn test_crc32_empty() {
        assert_eq!(crc32(b""), 0);
    }
}
