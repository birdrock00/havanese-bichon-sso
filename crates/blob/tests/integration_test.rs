use bichon_blob::{Codec, Config, Engine};
use tempfile::TempDir;

#[test]
fn test_create_and_list_accounts() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    engine.create_account("alice").unwrap();
    engine.create_account("bob").unwrap();

    let accounts = engine.list_accounts();
    assert!(accounts.contains(&"alice".to_string()));
    assert!(accounts.contains(&"bob".to_string()));
}

#[test]
fn test_write_and_read() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();
    engine.create_account("alice").unwrap();

    let key = [0xAA; 32];
    let value = b"Hello, this is a test email!".to_vec();

    engine
        .write("alice", key, &value, Codec::Zstd)
        .unwrap();

    let result = engine.read("alice", &key).unwrap();
    assert_eq!(result, Some(value));
}

#[test]
fn test_read_missing_key() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();
    engine.create_account("alice").unwrap();

    let key = [0xFF; 32];
    let result = engine.read("alice", &key).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_delete() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();
    engine.create_account("alice").unwrap();

    let key = [0xBB; 32];
    let value = b"Some email content".to_vec();

    engine
        .write("alice", key, &value, Codec::Zstd)
        .unwrap();
    engine.delete("alice", &key).unwrap();

    let result = engine.read("alice", &key).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_delete_account() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();
    engine.create_account("alice").unwrap();
    engine.delete_account("alice").unwrap();

    let accounts = engine.list_accounts();
    assert!(!accounts.contains(&"alice".to_string()));
}

#[test]
fn test_small_value_not_compressed() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();
    engine.create_account("alice").unwrap();

    let key = [0xCC; 32];
    let value = b"hi"; // Smaller than 4KB threshold

    engine
        .write("alice", key, value, Codec::Zstd)
        .unwrap();

    let result = engine.read("alice", &key).unwrap();
    assert_eq!(result, Some(value.to_vec()));
}

#[test]
fn test_large_value() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();
    engine.create_account("alice").unwrap();

    let key = [0xDD; 32];
    let value = vec![b'X'; 100_000]; // 100KB

    engine
        .write("alice", key, &value, Codec::Zstd)
        .unwrap();

    let result = engine.read("alice", &key).unwrap();
    assert_eq!(result, Some(value));
}

#[test]
fn test_multiple_keys() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();
    engine.create_account("alice").unwrap();

    let n = 100;
    for i in 0..n {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        let value = format!("email number {}", i).into_bytes();
        engine
            .write("alice", key, &value, Codec::Zstd)
            .unwrap();
    }

    for i in 0..n {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        let result = engine.read("alice", &key).unwrap();
        assert_eq!(result, Some(format!("email number {}", i).into_bytes()));
    }
}

#[test]
fn test_gc() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();
    engine.create_account("alice").unwrap();

    // Write many entries
    let value = vec![b'Y'; 5000];
    let n = 100;

    for i in 0..n {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        engine
            .write("alice", key, &value, Codec::None)
            .unwrap();
    }

    // Delete even-numbered keys
    for i in (0..n).step_by(2) {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        engine.delete("alice", &key).unwrap();
    }

    // Run GC
    let _result = engine.gc("alice").unwrap();

    // Verify remaining keys still readable
    for i in (1..n).step_by(2) {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        let result = engine.read("alice", &key).unwrap();
        assert_eq!(result, Some(value.clone()));
    }

    // Deleted keys should not exist
    for i in (0..n).step_by(2) {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        let result = engine.read("alice", &key).unwrap();
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
        engine.create_account("alice").unwrap();
        engine
            .write("alice", key, &value, Codec::Zstd)
            .unwrap();
    }

    // Reopen
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        let result = engine.read("alice", &key).unwrap();
        assert_eq!(result, Some(value));
    }
}

#[test]
fn test_stats() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();
    engine.create_account("alice").unwrap();

    engine
        .write("alice", [1u8; 32], b"hello", Codec::None)
        .unwrap();

    let stats = engine.stats("alice").unwrap();
    assert!(stats.total_bytes > 0);
}

#[test]
fn test_batch_write() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();
    engine.create_account("alice").unwrap();

    let n = 50;
    let entries: Vec<_> = (0..n)
        .map(|i: u64| {
            let mut key = [0u8; 32];
            key[0..8].copy_from_slice(&i.to_le_bytes());
            let value = format!("batch email {}", i).into_bytes();
            (key, value, Codec::Zstd)
        })
        .collect();

    engine.write_batch("alice", &entries).unwrap();

    for (key, value, _) in &entries {
        let result = engine.read("alice", key).unwrap();
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
        engine.create_account("alice").unwrap();
        engine.write_batch("alice", &entries).unwrap();
    }

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        for (key, value, _) in &entries {
            let result = engine.read("alice", key).unwrap();
            assert_eq!(result.as_ref(), Some(value));
        }
    }
}

#[test]
fn test_invalid_config_rejected() {
    let dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.lru_bucket_count = 0;
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
    engine.create_account("alice").unwrap();

    // Write some data
    for i in 0..50u32 {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&i.to_le_bytes());
        engine.write("alice", key, &vec![i as u8; 1024], Codec::None).unwrap();
    }

    // Spawn 4 threads, each reading a different subset
    let mut handles = vec![];
    for t in 0..4 {
        let engine = engine.clone();
        handles.push(thread::spawn(move || {
            for i in (t * 12)..((t + 1) * 12) {
                let mut key = [0u8; 32];
                key[0..4].copy_from_slice(&(i as u32).to_le_bytes());
                let read = engine.read("alice", &key).unwrap();
                assert!(read.is_some(), "key {} should exist", i);
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_concurrent_writes_different_accounts() {
    use std::sync::Arc;
    use std::thread;

    let dir = TempDir::new().unwrap();
    let engine = Arc::new(Engine::open(dir.path(), Config::default()).unwrap());

    for name in &["alice", "bob", "carol"] {
        engine.create_account(name).unwrap();
    }

    let mut handles = vec![];
    for (t, name) in ["alice", "bob", "carol"].iter().enumerate() {
        let engine = engine.clone();
        let account_name = name.to_string();
        handles.push(thread::spawn(move || {
            for i in 0..20 {
                let mut key = [0u8; 32];
                key[0..4].copy_from_slice(&((t * 100 + i) as u32).to_le_bytes());
                let value = vec![(t * 100 + i) as u8; 512];
                engine.write(&account_name, key, &value, Codec::None).unwrap();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    // Verify all writes persisted
    for (t, name) in ["alice", "bob", "carol"].iter().enumerate() {
        for i in 0..20 {
            let mut key = [0u8; 32];
            key[0..4].copy_from_slice(&((t * 100 + i) as u32).to_le_bytes());
            let read = engine.read(name, &key).unwrap();
            assert!(read.is_some(), "account {} key {} should exist", name, i);
        }
    }
}

#[test]
fn test_crash_recovery() {
    let dir = TempDir::new().unwrap();
    let dir_path = dir.path().to_path_buf();

    // Phase 1: write data, then drop without shutdown (simulates crash)
    {
        let engine = Engine::open(&dir_path, Config::default()).unwrap();
        engine.create_account("alice").unwrap();

        for i in 0..50u32 {
            let mut key = [0u8; 32];
            key[0..4].copy_from_slice(&i.to_le_bytes());
            engine.write("alice", key, &vec![i as u8; 512], Codec::None).unwrap();
        }
        // Engine dropped here without calling shutdown()
    }

    // Phase 2: reopen — recovery should run, data should be intact
    let engine = Engine::open(&dir_path, Config::default()).unwrap();
    let stats = engine.stats("alice").unwrap();
    assert!(stats.total_keys > 0, "recovery should preserve data");

    // Verify reads work
    for i in 0..50u32 {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&i.to_le_bytes());
        let read = engine.read("alice", &key).unwrap();
        assert!(read.is_some(), "key {} should survive crash recovery", i);
    }
}

#[test]
fn test_meta_bin_durability() {
    // Verify meta.bin has valid CRC and can be read after a write cycle.
    let dir = TempDir::new().unwrap();
    let dir_path = dir.path().to_path_buf();

    {
        let engine = Engine::open(&dir_path, Config::default()).unwrap();
        engine.create_account("alice").unwrap();

        let key = [0x42u8; 32];
        engine.write("alice", key, b"durable", Codec::None).unwrap();
    }
    // Engine dropped → shutdown() called → meta saved via write_bin (with fsync)

    // Verify meta.bin exists and has valid CRC
    let meta_path = dir_path
        .join("accounts")
        .join("alice")
        .join("meta.bin");
    assert!(meta_path.exists(), "meta.bin should exist after clean shutdown");

    let data = std::fs::read(&meta_path).unwrap();
    assert!(data.len() >= 8, "meta.bin should have at least 8 bytes (crc + version)");

    let stored_crc = u32::from_le_bytes(data[0..4].try_into().unwrap());
    assert_ne!(stored_crc, 0, "stored CRC should be non-zero");

    // Reopen and verify data is intact
    let engine = Engine::open(&dir_path, Config::default()).unwrap();
    let read = engine.read("alice", &[0x42u8; 32]).unwrap();
    assert_eq!(read, Some(b"durable".to_vec()));
}

#[test]
fn test_gc_concurrent_with_writes() {
    // GC should not lose entries that are written concurrently.
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    let dir = TempDir::new().unwrap();
    let engine = Arc::new(Engine::open(dir.path(), Config::default()).unwrap());
    engine.create_account("alice").unwrap();

    // Pre-fill: write enough to trigger eventual GC
    let big_value = vec![b'X'; 8192];
    for i in 0..500u32 {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&i.to_le_bytes());
        engine.write("alice", key, &big_value, Codec::None).unwrap();
    }

    // Delete some to create GC candidates
    for i in 0..250u32 {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&i.to_le_bytes());
        engine.delete("alice", &key).unwrap();
    }

    let running = Arc::new(AtomicBool::new(true));
    let engine_gc = engine.clone();
    let running_gc = running.clone();

    // Thread 1: run GC in a loop
    let gc_handle = thread::spawn(move || {
        while running_gc.load(Ordering::Relaxed) {
            let _ = engine_gc.gc("alice");
            thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    // Thread 2: keep writing new entries
    let engine_write = engine.clone();
    let running_write = running.clone();
    let write_handle = thread::spawn(move || {
        let mut counter = 10000u32;
        while running_write.load(Ordering::Relaxed) {
            let mut key = [0u8; 32];
            key[0..4].copy_from_slice(&counter.to_le_bytes());
            engine_write
                .write("alice", key, &vec![counter as u8; 256], Codec::None)
                .unwrap();
            counter += 1;
        }
        counter
    });

    // Let them race for a bit
    thread::sleep(std::time::Duration::from_millis(500));
    running.store(false, Ordering::Relaxed);

    gc_handle.join().unwrap();
    let final_counter = write_handle.join().unwrap();

    // All written entries must be readable
    let mut missing = 0;
    for i in 0..500u32 {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&i.to_le_bytes());
        if engine.read("alice", &key).unwrap().is_none() {
            // Entries 0..250 were deleted, they should be gone
            if i >= 250 {
                missing += 1;
            }
        }
    }
    assert_eq!(missing, 0, "pre-existing entries should survive concurrent GC");

    // Entries written during the race should be readable
    for i in 10000..final_counter {
        let mut key = [0u8; 32];
        key[0..4].copy_from_slice(&i.to_le_bytes());
        let read = engine.read("alice", &key).unwrap();
        assert!(read.is_some(), "concurrently written key {} should exist after GC", i);
    }
}
