use crate::types::Codec;

pub fn compress(data: &[u8], codec: Codec, threshold: usize, level: i32) -> (Vec<u8>, Codec) {
    if data.len() < threshold {
        return (data.to_vec(), Codec::None);
    }
    let (compressed, actual_codec) = match codec {
        Codec::Zstd => {
            match zstd::encode_all(data, level) {
                Ok(out) => (out, Codec::Zstd),
                Err(e) => {
                    tracing::warn!("zstd compression failed, storing uncompressed: {}", e);
                    (data.to_vec(), Codec::None)
                }
            }
        }
        Codec::Lz4 => {
            let out = lz4_flex::compress(data);
            (out, Codec::Lz4)
        }
        Codec::None => (data.to_vec(), Codec::None),
    };
    // If compression made it larger, store uncompressed
    if compressed.len() >= data.len() {
        (data.to_vec(), Codec::None)
    } else {
        (compressed, actual_codec)
    }
}

pub fn decompress(data: &[u8], codec: Codec, raw_size: usize) -> crate::error::Result<Vec<u8>> {
    match codec {
        Codec::None => Ok(data.to_vec()),
        Codec::Zstd => {
            zstd::decode_all(data)
                .map_err(|e| crate::error::Error::Compression(format!("zstd decompress: {}", e)))
        }
        Codec::Lz4 => {
            lz4_flex::decompress(data, raw_size)
                .map_err(|e| crate::error::Error::Compression(format!("lz4 decompress: {}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_data_not_compressed() {
        let data = b"hi";
        let (out, codec) = compress(data, Codec::Zstd, 4096, 0);
        assert_eq!(out, b"hi");
        assert_eq!(codec, Codec::None);
    }

    #[test]
    fn test_large_data_compressed_zstd() {
        let data = vec![b'A'; 5000];
        let (out, codec) = compress(&data, Codec::Zstd, 4096, 0);
        assert_eq!(codec, Codec::Zstd);
        assert!(out.len() < data.len());
    }

    #[test]
    fn test_roundtrip_zstd() {
        let data = vec![b'B'; 10000];
        let (compressed, codec) = compress(&data, Codec::Zstd, 4096, 0);
        let decompressed = decompress(&compressed, codec, data.len()).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_roundtrip_lz4() {
        let data = vec![b'C'; 10000];
        let (compressed, codec) = compress(&data, Codec::Lz4, 4096, 0);
        let decompressed = decompress(&compressed, codec, data.len()).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_roundtrip_none() {
        let data = vec![b'D'; 100];
        let (compressed, codec) = compress(&data, Codec::None, 4096, 0);
        assert_eq!(codec, Codec::None);
        let decompressed = decompress(&compressed, codec, data.len()).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_threshold_zero_always_compresses() {
        let data = vec![b'E'; 100];
        let (out, codec) = compress(&data, Codec::Zstd, 0, 0);
        assert_eq!(codec, Codec::Zstd);
        assert!(out.len() < data.len());
    }
}
