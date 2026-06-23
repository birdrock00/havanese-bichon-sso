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
