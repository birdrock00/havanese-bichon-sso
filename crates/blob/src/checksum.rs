use crc32fast::Hasher;

pub fn crc32(data: &[u8]) -> u32 {
    let mut h = Hasher::new();
    h.update(data);
    h.finalize()
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
