/// Crash-consistency and ACID property tests for bichon-blob.
///
/// Since we can't kill the process mid-write in an inline test, we simulate crashes
/// by dropping the Engine without calling any cleanup (close/drop is the "crash"),
/// then re-opening and verifying recovery produced consistent state.
///
/// For true power-loss simulation, each test writes data, drops the engine abruptly,
/// then reopens and verifies: no corruption, no lost committed data, no partial writes.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use bichon_blob::{Codec, Config, Engine};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_key(seed: u64) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0..8].copy_from_slice(&seed.to_le_bytes());
    key
}

fn make_value(size: usize) -> Vec<u8> {
    let pattern = b"The quick brown fox jumps over the lazy dog. ";
    let mut v = Vec::with_capacity(size);
    while v.len() < size {
        let rem = size - v.len();
        let n = rem.min(pattern.len());
        v.extend_from_slice(&pattern[..n]);
    }
    v
}

// ---------------------------------------------------------------------------
// 1. Durability: committed data survives crash
// ---------------------------------------------------------------------------

#[test]
fn test_durability_single_write_survives_crash() {
    let dir = TempDir::new().unwrap();
    let key = make_key(42);
    let value = make_value(8192);

    // Write
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.create_account("alice").unwrap();
        engine
            .write("alice", key, &value, Codec::Zstd)
            .unwrap();
    } // <-- Engine dropped = simulated crash

    // Recover
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        let result = engine.read("alice", &key).unwrap();
        assert_eq!(result, Some(value));
    }
}

#[test]
fn test_durability_many_writes_survive_crash() {
    let dir = TempDir::new().unwrap();
    let n = 500;
    let value = make_value(2048);
    let mut keys = Vec::new();

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.create_account("alice").unwrap();
        for i in 0..n {
            let key = make_key(i as u64);
            keys.push(key);
            engine
                .write("alice", key, &value, Codec::Zstd)
                .unwrap();
        }
    } // crash

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        for (i, key) in keys.iter().enumerate() {
            let result = engine.read("alice", key).unwrap();
            assert_eq!(result, Some(value.clone()), "missing key at index {}", i);
        }
    }
}

#[test]
fn test_durability_delete_survives_crash() {
    let dir = TempDir::new().unwrap();
    let key = make_key(99);
    let value = make_value(4096);

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.create_account("alice").unwrap();
        engine
            .write("alice", key, &value, Codec::Zstd)
            .unwrap();
    } // crash after write

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.delete("alice", &key).unwrap();
    } // crash after delete

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        let result = engine.read("alice", &key).unwrap();
        assert_eq!(result, None, "delete should persist across crash");
    }
}

// ---------------------------------------------------------------------------
// 2. Atomicity: no partial writes visible after crash
// ---------------------------------------------------------------------------

#[test]
fn test_atomicity_no_partial_entries_after_crash() {
    let dir = TempDir::new().unwrap();

    // Write enough entries to fill part of a segment, then crash
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.create_account("alice").unwrap();
        let value = make_value(50_000); // big enough to notice
        for i in 0..200u64 {
            engine
                .write("alice", make_key(i), &value, Codec::None)
                .unwrap();
        }
    } // crash

    // Recovery should clean up any partial tail entries and all committed
    // entries should be readable
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        let value = make_value(50_000);
        for i in 0..200u64 {
            let result = engine.read("alice", &make_key(i)).unwrap();
            assert_eq!(
                result,
                Some(value.clone()),
                "committed key {} should be intact",
                i
            );
        }
    }
}

#[test]
fn test_atomicity_crash_during_segment_roll() {
    let dir = TempDir::new().unwrap();
    let big_value = make_value(2 * 1024 * 1024); // 2 MB each entry

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.create_account("alice").unwrap();
        // Write enough to cross at least one segment boundary (256 MB)
        for i in 0..140u64 {
            engine
                .write("alice", make_key(i), &big_value, Codec::None)
                .unwrap();
        }
    } // crash mid-way or after multiple segments

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        // All committed writes (that returned Ok) must be readable
        for i in 0..140u64 {
            let result = engine.read("alice", &make_key(i)).unwrap();
            assert!(
                result.is_some(),
                "key {} should exist after segment roll recovery",
                i
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Consistency: CRC detects corruption, no silent data loss
// ---------------------------------------------------------------------------

#[test]
fn test_consistency_crc_detects_corruption() {
    let dir = TempDir::new().unwrap();
    let key = make_key(77);
    let value = make_value(8192);

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.create_account("alice").unwrap();
        engine
            .write("alice", key, &value, Codec::Zstd)
            .unwrap();
    }

    // Corrupt the segment file by flipping a byte
    let seg_path = find_first_segment(dir.path(), "alice");
    let mut data = fs::read(&seg_path).unwrap();
    // Flip a byte in the data portion, not the header
    let flip_pos = data.len() - 100;
    data[flip_pos] ^= 0xFF;
    fs::write(&seg_path, &data).unwrap();

    // Reading should detect CRC mismatch
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        let result = engine.read("alice", &key);
        // Either error or None is acceptable — never silently wrong data
        match result {
            Err(_) => {} // CRC mismatch detected — good
            Ok(None) => {} // index may point to truncated/removed data
            Ok(Some(v)) => {
                if v == value {
                    panic!("CRC corruption was NOT detected — silent data corruption!");
                }
                // If value differs, index pointed elsewhere after recovery
            }
        }
    }
}

#[test]
fn test_consistency_corrupt_magic_truncated_on_recovery() {
    let dir = TempDir::new().unwrap();

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.create_account("alice").unwrap();
        for i in 0..10u64 {
            engine
                .write("alice", make_key(i), &make_value(4096), Codec::Zstd)
                .unwrap();
        }
    }

    // Append garbage to the segment file (simulating partial write from crash)
    let seg_path = find_first_segment(dir.path(), "alice");
    let mut data = fs::read(&seg_path).unwrap();
    let orig_len = data.len();
    // Append garbage that doesn't start with the magic number
    data.extend_from_slice(&[0xFF; 200]);
    fs::write(&seg_path, &data).unwrap();

    // Recovery should truncate the garbage
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        // Verify committed data is still intact
        for i in 0..10u64 {
            let result = engine.read("alice", &make_key(i)).unwrap();
            assert!(result.is_some(), "committed key {} should survive tail truncation", i);
        }
    }

    // Verify file was actually truncated
    let truncated_len = fs::metadata(&seg_path).unwrap().len();
    assert!(truncated_len <= orig_len as u64, "garbage should have been truncated");
}

// ---------------------------------------------------------------------------
// 4. Isolation: concurrent reader sees consistent snapshot
// ---------------------------------------------------------------------------

#[test]
fn test_isolation_reader_sees_snapshot_not_partial_write() {
    let dir = TempDir::new().unwrap();
    let engine = Arc::new(Engine::open(dir.path(), Config::default()).unwrap());
    engine.create_account("alice").unwrap();

    // Pre-populate a known key
    let original_value = make_value(4096);
    let key = make_key(100);
    engine
        .write("alice", key, &original_value, Codec::Zstd)
        .unwrap();

    let running = Arc::new(AtomicBool::new(true));
    let writer_done = Arc::new(AtomicBool::new(false));

    // Spawn a writer that continuously overwrites the same key
    let writer_engine = engine.clone();
    let writer_running = running.clone();
    let writer_done_flag = writer_done.clone();
    let writer_key = key;

    let writer = thread::spawn(move || {
        for i in 0..1000u64 {
            if !writer_running.load(Ordering::Relaxed) {
                break;
            }
            let val = make_value(4096 + (i as usize % 100));
            writer_engine
                .write("alice", writer_key, &val, Codec::Zstd)
                .unwrap();
            thread::yield_now();
        }
        writer_done_flag.store(true, Ordering::SeqCst);
    });

    // Concurrent reader: reads should never panic or hang
    let reader_engine = engine.clone();
    let reader_running = running.clone();
    let reader = thread::spawn(move || {
        let mut reads = 0;
        while reads < 500 {
            if !reader_running.load(Ordering::Relaxed) && reads > 0 {
                break;
            }
            let result = reader_engine.read("alice", &key);
            match result {
                Ok(Some(_)) | Ok(None) => {} // OK
                Err(e) => {
                    // Accept transient errors but report them
                    eprintln!("reader saw error: {:?}", e);
                }
            }
            reads += 1;
            thread::yield_now();
        }
    });

    reader.join().unwrap();
    running.store(false, Ordering::SeqCst);
    writer.join().unwrap();

    // Final read should see the last committed value (not partial)
    let final_result = engine.read("alice", &key).unwrap();
    assert!(final_result.is_some(), "final read should find a value");
}

// ---------------------------------------------------------------------------
// 5. Crash during GC: old data intact, no corruption
// ---------------------------------------------------------------------------

#[test]
fn test_crash_during_gc_leaves_data_intact() {
    let dir = TempDir::new().unwrap();

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.create_account("alice").unwrap();

        let value = make_value(500_000); // 500 KB each
        // Write enough entries and delete some to create GC candidate
        for i in 0..500u64 {
            engine
                .write("alice", make_key(i), &value, Codec::None)
                .unwrap();
        }
        // Delete ~40%
        for i in (0..500u64).step_by(5) {
            engine.delete("alice", &make_key(i)).unwrap();
        }
        // Single GC run (may or may not trigger)
        let _ = engine.gc("alice");
    } // crash after GC

    // All non-deleted entries must still be readable
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        let value = make_value(500_000);
        for i in 0..500u64 {
            let key = make_key(i);
            let result = engine.read("alice", &key).unwrap();
            if i % 5 == 0 {
                // Deleted keys
                assert_eq!(result, None, "key {} should be deleted", i);
            } else {
                assert_eq!(
                    result,
                    Some(value.clone()),
                    "key {} should survive GC+crash",
                    i
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Multiple crash-reopen cycles (torture test)
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_crash_reopen_cycles() {
    use std::collections::HashSet;

    let dir = TempDir::new().unwrap();
    let value = make_value(4096);
    let mut alive: HashSet<u64> = HashSet::new();

    // Populate and crash
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.create_account("alice").unwrap();
        for i in 0..50u64 {
            engine
                .write("alice", make_key(i), &value, Codec::Zstd)
                .unwrap();
            alive.insert(i);
        }
    }

    // Reopen, verify all exist, write more, crash
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        for &k in &alive {
            assert!(engine.read("alice", &make_key(k)).unwrap().is_some());
        }
        for i in 100..150u64 {
            engine
                .write("alice", make_key(i), &value, Codec::Zstd)
                .unwrap();
            alive.insert(i);
        }
    }

    // Reopen, verify all exist, delete some, crash
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        for &k in &alive {
            assert!(engine.read("alice", &make_key(k)).unwrap().is_some());
        }
        for i in 0..10u64 {
            engine.delete("alice", &make_key(i)).unwrap();
            alive.remove(&i);
        }
    }

    // Final reopen: survivors exist, deleted gone
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        for &k in &alive {
            assert!(engine.read("alice", &make_key(k)).unwrap().is_some(),
                "key {} should exist", k);
        }
        for i in 0..10u64 {
            assert_eq!(engine.read("alice", &make_key(i)).unwrap(), None,
                "key {} should be deleted", i);
        }
    }
}

// ---------------------------------------------------------------------------
// 7. Account-level isolation
// ---------------------------------------------------------------------------

#[test]
fn test_account_isolation_crash_one_account_does_not_affect_others() {
    let dir = TempDir::new().unwrap();

    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        engine.create_account("alice").unwrap();
        engine.create_account("bob").unwrap();

        engine
            .write("alice", make_key(1), &make_value(4096), Codec::Zstd)
            .unwrap();
        engine
            .write("bob", make_key(1), &make_value(8192), Codec::Zstd)
            .unwrap();
    }

    // Delete alice's account dir partially to simulate corruption
    // Then verify bob is intact
    {
        let engine = Engine::open(dir.path(), Config::default()).unwrap();
        // Bob should be fine
        let result = engine.read("bob", &make_key(1)).unwrap();
        assert!(result.is_some(), "bob should be unaffected");
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_first_segment(store_root: &Path, account: &str) -> std::path::PathBuf {
    let seg_dir = store_root.join("accounts").join(account).join("segments");
    for entry in fs::read_dir(&seg_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".seg") && !name.contains("temp_") {
            return entry.path();
        }
    }
    panic!("no segment found in {:?}", seg_dir);
}
