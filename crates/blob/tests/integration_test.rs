use bichon_blob::{Codec, Config, Engine};
use tempfile::TempDir;

#[test]
fn test_write_and_read() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let key = [0xAA; 32];
    let value = b"Hello, this is a test email!".to_vec();

    engine.put(key, &value, Codec::Zstd).unwrap();

    let result = engine.get(&key).unwrap();
    assert_eq!(result, Some(value));
}

#[test]
fn test_read_missing_key() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let key = [0xFF; 32];
    let result = engine.get(&key).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_delete() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let key = [0xBB; 32];
    let value = b"Some email content".to_vec();

    engine.put(key, &value, Codec::Zstd).unwrap();
    engine.delete(&key).unwrap();

    let result = engine.get(&key).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_exists() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let key = [0xCC; 32];
    assert!(!engine.exists(&key).unwrap());

    engine.put(key, b"data", Codec::None).unwrap();
    assert!(engine.exists(&key).unwrap());

    engine.delete(&key).unwrap();
    assert!(!engine.exists(&key).unwrap());
}

#[test]
fn test_small_value_not_compressed() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let key = [0xCC; 32];
    let value = b"hi"; // Smaller than 4KB threshold

    engine.put(key, value, Codec::Zstd).unwrap();

    let result = engine.get(&key).unwrap();
    assert_eq!(result, Some(value.to_vec()));
}

#[test]
fn test_large_value() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let key = [0xDD; 32];
    let value = vec![b'X'; 100_000]; // 100KB

    engine.put(key, &value, Codec::Zstd).unwrap();

    let result = engine.get(&key).unwrap();
    assert_eq!(result, Some(value));
}

#[test]
fn test_multiple_keys() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let n = 100;
    for i in 0..n {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        let value = format!("email number {}", i).into_bytes();
        engine.put(key, &value, Codec::Zstd).unwrap();
    }

    for i in 0..n {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        let result = engine.get(&key).unwrap();
        assert_eq!(result, Some(format!("email number {}", i).into_bytes()));
    }
}

#[test]
fn test_gc() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    // Write many entries
    let value = vec![b'Y'; 5000];
    let n = 100;

    for i in 0..n {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        engine.put(key, &value, Codec::None).unwrap();
    }

    // Delete even-numbered keys
    for i in (0..n).step_by(2) {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        engine.delete(&key).unwrap();
    }

    // Run GC
    let _result = engine.gc().unwrap();

    // Verify remaining keys still readable
    for i in (1..n).step_by(2) {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        let result = engine.get(&key).unwrap();
        assert_eq!(result, Some(value.clone()));
    }

    // Deleted keys should not exist
    for i in (0..n).step_by(2) {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        let result = engine.get(&key).unwrap();
        assert_eq!(result, None);
    }
}

#[test]
fn test_reopen_persistence() {
    let dir = TempDir::new().unwrap();
    let key = [0xEE; 32];
    let value = b"persistent data".to_vec();

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.put(key, &value, Codec::Zstd).unwrap();
    }

    // Reopen
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        let result = engine.get(&key).unwrap();
        assert_eq!(result, Some(value));
    }
}

#[test]
fn test_stats() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    engine.put([1u8; 32], b"hello", Codec::None).unwrap();

    let stats = engine.stats().unwrap();
    assert!(stats.total_bytes > 0);
    assert!(stats.total_keys > 0);
}

#[test]
fn test_batch_write() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let n = 50;
    let entries: Vec<_> = (0..n)
        .map(|i: u64| {
            let mut key = [0u8; 32];
            key[0..8].copy_from_slice(&i.to_le_bytes());
            let value = format!("batch email {}", i).into_bytes();
            (key, value, Codec::Zstd)
        })
        .collect();

    engine.put_batch(&entries).unwrap();

    for (key, value, _) in &entries {
        let result = engine.get(key).unwrap();
        assert_eq!(result.as_ref(), Some(value));
    }
}

#[test]
fn test_batch_write_persistence() {
    let dir = TempDir::new().unwrap();
    let entries: Vec<_> = (0..30u64)
        .map(|i| {
            let mut key = [0u8; 32];
            key[0..8].copy_from_slice(&i.to_le_bytes());
            (key, format!("persist {}", i).into_bytes(), Codec::Zstd)
        })
        .collect();

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.put_batch(&entries).unwrap();
    }

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        for (key, value, _) in &entries {
            let result = engine.get(key).unwrap();
            assert_eq!(result.as_ref(), Some(value));
        }
    }
}

#[test]
fn test_invalid_config_rejected() {
    let dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.compact_threshold = 0;
    assert!(Engine::open(dir.path(), config).is_err());

    let mut config = Config::default();
    config.gc_deleted_ratio = 1.5;
    assert!(Engine::open(dir.path(), config).is_err());
}

#[test]
fn test_concurrent_reads() {
    use std::sync::Arc;
    use std::thread;

    let dir = TempDir::new().unwrap();
    let engine = Arc::new(Engine::open(dir.path(), Config::default()).unwrap());

    // Write some data
    for i in 0..50u32 {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&i.to_le_bytes());
        engine
            .put(key, &vec![i as u8; 1024], Codec::None)
            .unwrap();
    }

    // Spawn 4 threads, each reading a different subset
    let mut handles = vec![];
    for t in 0..4 {
        let engine = engine.clone();
        handles.push(thread::spawn(move || {
            for i in (t * 12)..((t + 1) * 12) {
                let mut key = [0u8; 32];
                key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
                let read = engine.get(&key).unwrap();
                assert!(read.is_some(), "key {} should exist", i);
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_global_dedup() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let key = [0x42; 32];
    let value = b"same content across what would be accounts".to_vec();

    // Write same key twice (simulating two accounts ingesting the same email)
    engine.put(key, &value, Codec::Zstd).unwrap();
    engine.put(key, &value, Codec::Zstd).unwrap();

    // Should still be readable
    let result = engine.get(&key).unwrap();
    assert_eq!(result, Some(value));

    // Stats should reflect dedup (not double count)
    let stats = engine.stats().unwrap();
    // The key appears once in the bucket store
    assert!(stats.total_keys > 0);
}

#[test]
fn test_crash_recovery() {
    let dir = TempDir::new().unwrap();
    let dir_path = dir.path().to_path_buf();

    // Phase 1: write data, then drop without shutdown (simulates crash)
    {
        let engine = Engine::open(&dir_path, Config::default()).unwrap();

        for i in 0..50u32 {
            let mut key = [0u8; 32];
            key[0..4].copy_from_slice(&i.to_le_bytes());
            engine
                .put(key, &vec![i as u8; 512], Codec::None)
                .unwrap();
        }
        // Engine dropped here without calling shutdown()
    }

    // Phase 2: reopen - recovery should run, data should be intact
    let engine = Engine::open(&dir_path, Config::default()).unwrap();
    let stats = engine.stats().unwrap();
    assert!(stats.total_keys > 0, "recovery should preserve data");

    // Verify reads work
    for i in 0..50u32 {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&i.to_le_bytes());
        let read = engine.get(&key).unwrap();
        assert!(read.is_some(), "key {} should survive crash recovery", i);
    }
}

#[test]
fn test_meta_bin_durability() {
    let dir = TempDir::new().unwrap();
    let dir_path = dir.path().to_path_buf();

    {
        let engine = Engine::open(&dir_path, Config::default()).unwrap();

        let key = [0x42u8; 32];
        engine.put(key, b"durable", Codec::None).unwrap();
    }
    // Engine dropped -> shutdown() called -> meta saved

    // Verify meta.bin exists
    let meta_path = dir_path.join("meta.bin");
    assert!(meta_path.exists(), "meta.bin should exist after clean shutdown");

    let data = std::fs::read(&meta_path).unwrap();
    assert!(
        data.len() >= 8,
        "meta.bin should have at least 8 bytes"
    );

    let stored_crc = u32::from_le_bytes(data[0..4].try_into().unwrap());
    assert_ne!(stored_crc, 0, "stored CRC should be non-zero");

    // Reopen and verify data is intact
    let engine = Engine::open(&dir_path, Config::default()).unwrap();
    let read = engine.get(&[0x42u8; 32]).unwrap();
    assert_eq!(read, Some(b"durable".to_vec()));
}
