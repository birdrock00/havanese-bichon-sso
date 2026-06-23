use bichon_memdb::{Durability, MemDb, Page};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ─── Test models ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Record {
    id: String,
    name: String,
    amount: u64,
    tags: Vec<String>,
    metadata: String,
}

impl Record {
    fn new(id: usize) -> Self {
        let tag_count = (id % 5) as usize + 1;
        Self {
            id: format!("rec_{:06}", id),
            name: format!("record_{}", id),
            amount: (id * 7 % 1_000_000) as u64 + 1,
            tags: (0..tag_count)
                .map(|t| format!("tag_{:02}", (id + t) % 20))
                .collect(),
            metadata: format!("meta data blob for record {}", id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Counter {
    value: u64,
    updates: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimpleVal {
    val: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Blob {
    id: usize,
    data: Vec<u8>,
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn report(name: &str, count: u64, elapsed_ms: u64) {
    let ops_per_sec = if elapsed_ms > 0 {
        count * 1000 / elapsed_ms
    } else {
        count
    };
    println!("  [{name}] {count} ops in {elapsed_ms}ms → {ops_per_sec} ops/sec");
}

// ─── 1. Bulk insert performance ──────────────────────────────────────────

#[test]
fn stress_bulk_insert_performance() {
    let db = MemDb::in_memory();
    let col = db.collection("records");
    let n = 10_000u64;

    let start = Instant::now();
    for i in 0..n {
        col.insert(format!("{:06}", i), &Record::new(i as usize))
            .unwrap();
    }
    let elapsed = start.elapsed().as_millis() as u64;
    assert_eq!(col.count(), n as usize);
    report("bulk insert", n, elapsed);
}

// ─── 2. Bulk read performance ────────────────────────────────────────────

#[test]
fn stress_bulk_read_performance() {
    let db = MemDb::in_memory();
    let col = db.collection("records");
    let n = 10_000u64;

    for i in 0..n {
        col.insert(format!("{:06}", i), &Record::new(i as usize))
            .unwrap();
    }

    let start = Instant::now();
    for i in 0..n {
        let _found: Option<Record> = col.get(&format!("{:06}", i)).unwrap();
    }
    let elapsed = start.elapsed().as_millis() as u64;
    report("bulk read (by key)", n, elapsed);
}

// ─── 3. Bulk update performance ──────────────────────────────────────────

#[test]
fn stress_bulk_update_performance() {
    let db = MemDb::in_memory();
    let col = db.collection("records");
    let n = 5_000u64;

    for i in 0..n {
        col.insert(format!("{:06}", i), &Record::new(i as usize))
            .unwrap();
    }

    let start = Instant::now();
    for i in 0..n {
        let key = format!("{:06}", i);
        col.update(&key, |mut r: Record| {
            r.amount += 1;
            r
        })
        .unwrap();
    }
    let elapsed = start.elapsed().as_millis() as u64;
    report("bulk update", n, elapsed);

    let r: Record = col.get_required("000000").unwrap();
    assert!(r.amount > 0);
}

// ─── 4. Prefix scan performance ──────────────────────────────────────────

#[test]
fn stress_prefix_scan_performance() {
    let db = MemDb::in_memory();
    let col = db.collection("events");
    for day in 1..=100 {
        for seq in 1..=200 {
            let id = format!("2026_{:03}_{:05}", day, seq);
            let rec = Record::new((day * 1000 + seq) as usize);
            col.insert(id, &rec).unwrap();
        }
    }

    let start = Instant::now();
    let results: Vec<Record> = col.scan_prefix("2026_050_").unwrap();
    let elapsed = start.elapsed().as_millis() as u64;
    assert_eq!(results.len(), 200);
    report("prefix scan (200 of 20k)", 200, elapsed);

    let start = Instant::now();
    let results: Vec<Record> = col.scan_prefix("9999_").unwrap();
    let empty_ms = start.elapsed().as_millis() as u64;
    assert!(results.is_empty());
    println!("  [prefix scan empty] in {empty_ms}ms");
}

// ─── 5. Pagination performance ───────────────────────────────────────────

#[test]
fn stress_pagination_performance() {
    let db = MemDb::in_memory();
    let col = db.collection("records");
    let n = 10_000u64;

    for i in 0..n {
        col.insert(format!("{:06}", i), &Record::new(i as usize))
            .unwrap();
    }

    let start = Instant::now();
    let page_size = 50u64;
    let total_pages = (n + page_size - 1) / page_size;
    let mut total_items = 0u64;
    for p in 1..=total_pages {
        let page = col
            .paginate::<Record>(&Page::new(p, page_size), false)
            .unwrap();
        total_items += page.items.len() as u64;
    }
    let elapsed = start.elapsed().as_millis() as u64;
    assert_eq!(total_items, n);
    report(&format!("paginate {} pages", total_pages), n, elapsed);
}

// ─── 6. Filter performance ───────────────────────────────────────────────

#[test]
fn stress_filter_performance() {
    let db = MemDb::in_memory();
    let col = db.collection("records");
    let n = 10_000u64;

    for i in 0..n {
        col.insert(format!("{:06}", i), &Record::new(i as usize))
            .unwrap();
    }

    let start = Instant::now();
    let filtered: Vec<Record> = col
        .filter(|r: &Record| r.amount > 500_000 && r.tags.contains(&"tag_05".to_string()))
        .unwrap();
    let elapsed = start.elapsed().as_millis() as u64;
    println!(
        "  [filter] matched {}/{} in {}ms",
        filtered.len(),
        n,
        elapsed
    );
}

// ─── 7. Concurrent writes ────────────────────────────────────────────────

#[tokio::test]
async fn stress_concurrent_writes_no_data_loss() {
    let db = Arc::new(MemDb::in_memory());
    let n_writers = 8u64;
    let n_per_writer = 1_250u64;
    let total = n_writers * n_per_writer;

    let start = Instant::now();
    let mut handles = vec![];
    for w in 0..n_writers {
        let db = db.clone();
        handles.push(tokio::task::spawn_blocking(move || {
            for i in 0..n_per_writer {
                let id = format!("w{:02}_{:06}", w, i);
                let rec = Record::new((w * 10000 + i) as usize);
                db.collection("records").upsert(id, &rec).unwrap();
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    let elapsed = start.elapsed().as_millis() as u64;

    let count = db.collection("records").count() as u64;
    assert_eq!(count, total);
    report("concurrent writes (8×1250)", total, elapsed);
}

// ─── 8. Concurrent reads and writes ──────────────────────────────────────

#[tokio::test]
async fn stress_concurrent_read_write() {
    let db = Arc::new(MemDb::in_memory());
    let n = 5_000u64;

    for i in 0..n {
        db.collection("records")
            .insert(format!("{:06}", i), &Record::new(i as usize))
            .unwrap();
    }

    let start = Instant::now();

    let db_w = db.clone();
    let write_handle = tokio::task::spawn_blocking(move || {
        for i in n..n + 2_000 {
            let id = format!("{:06}", i);
            let rec = Record::new(i as usize);
            db_w.collection("records").upsert(id, &rec).unwrap();
        }
    });

    let db_r = db.clone();
    let read_handle = tokio::task::spawn_blocking(move || {
        for i in 0..2_000u64 {
            let key = format!("{:06}", i % n);
            let _: Option<Record> = db_r.collection("records").get(&key).unwrap();
        }
    });

    let (wr, rr) = tokio::join!(write_handle, read_handle);
    wr.unwrap();
    rr.unwrap();
    let elapsed = start.elapsed().as_millis() as u64;

    let count = db.collection("records").count() as u64;
    assert_eq!(count, n + 2_000);
    report("concurrent r/w (2k+2k)", 4_000, elapsed);
}

// ─── 9. Snapshot with concurrent writes (data integrity) ─────────────────

#[tokio::test]
async fn stress_snapshot_with_concurrent_writes() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().to_path_buf();
    let n_writes = 500u64;

    {
        let db = Arc::new(MemDb::open(&db_path).unwrap());

        let db_w = db.clone();
        let write_handle = tokio::task::spawn_blocking(move || {
            for i in 0..n_writes {
                let rec = Record::new(i as usize);
                db_w
                    .collection("records")
                    .upsert(format!("{:06}", i), &rec)
                    .unwrap();
            }
        });

        let db_s = db.clone();
        let snap_handle = tokio::spawn(async move {
            for _ in 0..10 {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                db_s.snapshot().unwrap();
            }
        });

        write_handle.await.unwrap();
        snap_handle.await.unwrap();
        db.snapshot().unwrap();
    }

    // Recover and verify.
    {
        let db = MemDb::open(&db_path).unwrap();
        let col = db.collection("records");
        let count = col.count() as u64;
        println!("  [snapshot stress] wrote {n_writes}, recovered {count}");
        assert_eq!(
            count, n_writes,
            "data loss: wrote {n_writes} but recovered {count}"
        );

        for i in 0..n_writes {
            let key = format!("{:06}", i);
            assert!(col.exists(&key), "missing record: {key}");
        }
    }
}

// ─── 10. Large dataset recovery ─────────────────────────────────────────

#[test]
fn stress_recover_large_dataset() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().to_path_buf();
    let n = 3_000u64;

    {
        let db = MemDb::open(&db_path).unwrap();
        let col = db.collection("records");
        let start = Instant::now();
        for i in 0..n {
            col.insert(format!("{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
        let elapsed = start.elapsed().as_millis() as u64;
        report("persistent insert", n, elapsed);
        db.snapshot().unwrap();
    }

    {
        let start = Instant::now();
        let db = MemDb::open(&db_path).unwrap();
        let elapsed = start.elapsed().as_millis() as u64;
        println!("  [recovery] {n} records loaded in {elapsed}ms");

        let col = db.collection("records");
        assert_eq!(col.count() as u64, n);

        for i in 0..n {
            let key = format!("{:06}", i);
            assert!(col.exists(&key), "missing after recovery: {key}");
        }
    }
}

// ─── 11. Transaction batch performance ───────────────────────────────────

#[test]
fn stress_transaction_batch_performance() {
    let db = MemDb::in_memory();
    let batch_size = 100usize;
    let batches = 100usize;

    let start = Instant::now();
    for b in 0..batches {
        let mut txn = db.transaction();
        for i in 0..batch_size {
            let id = format!("b{:03}_{:04}", b, i);
            let rec = Record::new(b * batch_size + i);
            txn = txn.upsert("records", &id, &rec).unwrap();
        }
        txn.commit().unwrap();
    }
    let elapsed = start.elapsed().as_millis() as u64;
    let total = (batches * batch_size) as u64;

    let count = db.collection("records").count() as u64;
    assert_eq!(count, total);
    report("txn batch (100×100)", total, elapsed);
}

// ─── 12. Cross-collection transactions ───────────────────────────────────

#[test]
fn stress_cross_collection_transaction() {
    let db = MemDb::in_memory();
    let n = 1_000u64;

    let start = Instant::now();
    for i in 0..n {
        let rec = Record::new(i as usize);
        db.transaction()
            .upsert("records", format!("{:06}", i), &rec)
            .unwrap()
            .upsert("audit_log", format!("log_{:06}", i), &rec)
            .unwrap()
            .commit()
            .unwrap();
    }
    let elapsed = start.elapsed().as_millis() as u64;

    assert_eq!(db.collection("records").count() as u64, n);
    assert_eq!(db.collection("audit_log").count() as u64, n);
    report("cross-collection txn", n, elapsed);
}

// ─── 13. Update-heavy workload ───────────────────────────────────────────

#[test]
fn stress_update_heavy_workload() {
    let db = MemDb::in_memory();
    let col = db.collection("counters");
    let n = 1_000u64;

    for i in 0..n {
        col.insert(
            format!("counter_{:04}", i),
            &Counter {
                value: 0,
                updates: 0,
            },
        )
        .unwrap();
    }

    let start = Instant::now();
    let rounds = 10u64;
    for _ in 0..rounds {
        for i in 0..n {
            let key = format!("counter_{:04}", i);
            let _ = col
                .update::<Counter>(&key, |mut c| {
                    c.value += 1;
                    c.updates += 1;
                    c
                })
                .unwrap();
        }
    }
    let elapsed = start.elapsed().as_millis() as u64;
    let total_ops = rounds * n;

    let all: Vec<Counter> = col.list_all().unwrap();
    let total_value: u64 = all.iter().map(|c| c.value).sum();
    let total_updates: u64 = all.iter().map(|c| c.updates).sum();
    assert_eq!(total_value, total_updates);
    report("update-heavy (10×1000)", total_ops, elapsed);
    println!("  [update] total value={total_value}, total updates={total_updates}");
}

// ─── 14. Mass delete ─────────────────────────────────────────────────────

#[test]
fn stress_mass_delete() {
    let db = MemDb::in_memory();
    let col = db.collection("records");
    let n = 5_000u64;

    for i in 0..n {
        col.insert(format!("{:06}", i), &Record::new(i as usize))
            .unwrap();
    }
    assert_eq!(col.count() as u64, n);

    let start = Instant::now();
    for i in 0..n / 2 {
        let deleted = col.delete(format!("{:06}", i)).unwrap();
        assert!(deleted);
    }
    let elapsed = start.elapsed().as_millis() as u64;
    assert_eq!(col.count() as u64, n / 2);
    report("delete 2500 of 5000", n / 2, elapsed);
}

// ─── 15. Pagination edge cases ───────────────────────────────────────────

#[test]
fn stress_pagination_edge_cases() {
    let db = MemDb::in_memory();
    let col = db.collection("records");
    let n = 100u64;

    for i in 0..n {
        col.insert(format!("{:06}", i), &SimpleVal { val: i }).unwrap();
    }

    // Single-item page.
    let p1 = col
        .paginate::<serde_json::Value>(&Page::new(1, 1), false)
        .unwrap();
    assert_eq!(p1.items.len(), 1);
    assert_eq!(p1.total_pages, 100);

    // Out of range.
    let p2 = col
        .paginate::<serde_json::Value>(&Page::new(999, 50), false)
        .unwrap();
    assert!(p2.items.is_empty());
    assert_eq!(p2.total, 100);

    // Out of range (descending).
    let p3 = col
        .paginate::<serde_json::Value>(&Page::new(999, 50), true)
        .unwrap();
    assert!(p3.items.is_empty());

    // First page descending.
    let p4 = col
        .paginate::<serde_json::Value>(&Page::new(1, 3), true)
        .unwrap();
    assert_eq!(p4.items.len(), 3);

    // Last page ascending.
    let p5 = col
        .paginate::<serde_json::Value>(&Page::new(34, 3), false)
        .unwrap();
    assert_eq!(p5.items.len(), 1);
    assert_eq!(p5.total_pages, 34);
}

// ─── 16. WAL seq monotonicity under load ─────────────────────────────────

#[test]
fn stress_wal_seq_monotonic_under_load() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemDb::open(dir.path()).unwrap();
    let col = db.collection("records");
    let n = 2_000;

    for i in 0..n {
        col.upsert(format!("{:06}", i), &Record::new(i)).unwrap();
    }

    let wal_path = dir.path().join("wal.jsonl");
    let entries = bichon_memdb::wal::read_after(&wal_path, 0).unwrap();
    assert_eq!(entries.len(), n);
    let mut last = 0u64;
    for e in &entries {
        assert!(e.seq > last, "seq not monotonic: {} <= {}", e.seq, last);
        last = e.seq;
    }
    println!("  [wal seq] {n} entries, seq monotonic verified");
}

// ─── 17. Snapshot truncation safety ─────────────────────────────────────

#[test]
fn stress_snapshot_truncate_safety() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemDb::open(dir.path()).unwrap();
    let col = db.collection("records");

    col.insert("a", &Record::new(1)).unwrap();
    db.snapshot().unwrap(); // seq=1 snapshotted, WAL truncated

    col.insert("b", &Record::new(2)).unwrap(); // seq=2 must survive

    assert_eq!(col.count(), 2);

    drop(db);
    let db = MemDb::open(dir.path()).unwrap();
    assert_eq!(
        db.collection("records").count(),
        2,
        "data loss: record b missing after snapshot+truncate+recovery"
    );
    println!("  [truncate safety] both records survived snapshot+truncate+recovery");
}

// ─── 18. Multi-collection isolation ──────────────────────────────────────

#[test]
fn stress_multi_collection_isolation() {
    let db = MemDb::in_memory();
    let n = 2_000;

    let collections = ["users", "orders", "products", "sessions", "audit_log"];
    for (i, col_name) in collections.iter().enumerate() {
        let col = db.collection(col_name);
        let base_id = i * 10_000;
        for j in 0..n {
            let id = format!("{:06}", base_id + j);
            col.insert(&id, &Record::new(j)).unwrap();
        }
    }

    for col_name in collections {
        assert_eq!(
            db.collection(col_name).count(),
            n,
            "collection {col_name} count mismatch"
        );
    }
    println!("  [isolation] 5×2000 records, all counts verified");
}

// ─── 19. Large value read/write ──────────────────────────────────────────

#[test]
fn stress_large_value_read_write() {
    let db = MemDb::in_memory();
    let col = db.collection("blobs");

    let blob_size = 64 * 1024; // 64 KiB
    let n = 100u64;

    let start = Instant::now();
    for i in 0..n {
        let blob = Blob {
            id: i as usize,
            data: vec![(i % 256) as u8; blob_size],
        };
        col.insert(format!("blob_{:04}", i), &blob).unwrap();
    }
    let write_ms = start.elapsed().as_millis() as u64;
    report(&format!("large value write ({}×64KB)", n), n, write_ms);

    let start = Instant::now();
    for i in 0..n {
        let blob: Blob = col.get_required(&format!("blob_{:04}", i)).unwrap();
        assert_eq!(blob.id, i as usize);
        assert_eq!(blob.data.len(), blob_size);
    }
    let read_ms = start.elapsed().as_millis() as u64;
    report(&format!("large value read ({}×64KB)", n), n, read_ms);
}

// ─── 20. Realistic scenario: chat messages ───────────────────────────────

#[test]
fn stress_realistic_chat_messages() {
    let db = MemDb::in_memory();

    #[derive(Serialize, Deserialize)]
    struct Message {
        room_id: String,
        sender: String,
        text: String,
        ts: u64,
    }

    let users = ["alice", "bob", "charlie", "diana", "eve"];
    let rooms = ["general", "random", "dev", "ops"];
    let n_messages = 5_000u64;

    let start = Instant::now();
    for i in 0..n_messages {
        let room = rooms[i as usize % rooms.len()];
        let user = users[i as usize % users.len()];
        let msg = Message {
            room_id: room.to_string(),
            sender: user.to_string(),
            text: format!("message number {} from {} in {}", i, user, room),
            ts: 1700000000 + i,
        };

        // One collection per chat room.
        let col = db.collection(room);
        col.insert(format!("msg_{:06}", i), &msg).unwrap();
    }
    let write_elapsed = start.elapsed().as_millis() as u64;
    report("chat insert (5k msgs)", n_messages, write_elapsed);

    // Each room should have messages.
    for room in rooms {
        let col = db.collection(room);
        assert!(col.count() > 0, "room {room} should have messages");
    }

    // Prefix-scan messages in one room.
    let start = Instant::now();
    let general_msgs: Vec<serde_json::Value> =
        db.collection("general").scan_prefix("msg_000").unwrap();
    let scan_elapsed = start.elapsed().as_millis() as u64;
    assert!(!general_msgs.is_empty());
    report("chat prefix scan", general_msgs.len() as u64, scan_elapsed);
}

// ═══════════════════════════════════════════════════════════════════════════
// File-backed WAL stress tests
// These exercise real disk I/O — fsync, recovery, WAL growth, snapshot
// interleaving — at scale.  Every test below uses MemDb::open(), not
// in_memory().
// ═══════════════════════════════════════════════════════════════════════════

// ─── 21. Bulk persistent write (WAL + fsync cost) ────────────────────────

#[test]
fn wal_bulk_insert_fsync_cost() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemDb::open(dir.path()).unwrap();
    let col = db.collection("records");
    let n = 3_000u64;

    let start = Instant::now();
    for i in 0..n {
        col.insert(format!("{:06}", i), &Record::new(i as usize))
            .unwrap();
    }
    let elapsed = start.elapsed().as_millis() as u64;

    assert_eq!(col.count(), n as usize);
    report("WAL insert (fsync per op)", n, elapsed);

    // Verify WAL file exists and has content.
    let wal_path = dir.path().join("wal.jsonl");
    let wal_size = std::fs::metadata(&wal_path).unwrap().len();
    println!(
        "  [wal file] {} ops → {} KiB ({:.1} bytes/op)",
        n,
        wal_size / 1024,
        wal_size as f64 / n as f64
    );

    // Recover and verify.
    drop(db);
    let db = MemDb::open(dir.path()).unwrap();
    assert_eq!(db.collection("records").count(), n as usize);
}

// ─── 22. WAL recovery from large dataset (no snapshot) ───────────────────

#[test]
fn wal_recover_pure_wal_no_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let n = 5_000u64;

    // Write everything — never snapshot, so recovery must replay every entry.
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("records");
        let start = Instant::now();
        for i in 0..n {
            col.insert(format!("{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
        let elapsed = start.elapsed().as_millis() as u64;
        report("WAL write (no snapshot)", n, elapsed);
    }

    // Recover from pure WAL replay.
    {
        let start = Instant::now();
        let db = MemDb::open(dir.path()).unwrap();
        let elapsed = start.elapsed().as_millis() as u64;
        report("WAL replay recovery", n, elapsed);

        let col = db.collection("records");
        assert_eq!(col.count(), n as usize);

        // Spot-check random keys across the whole range.
        for i in (0..n).step_by(500) {
            let key = format!("{:06}", i);
            let rec: Record = col.get_required(&key).unwrap();
            assert_eq!(rec.id, format!("rec_{:06}", i as usize));
        }
    }
}

// ─── 23. Sustained write throughput over time ────────────────────────────

#[test]
fn wal_sustained_write_throughput() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemDb::open(dir.path()).unwrap();
    let col = db.collection("records");
    let rounds = 5u64;
    let per_round = 1_000u64;

    for r in 0..rounds {
        let base = r * per_round;
        let start = Instant::now();
        for i in 0..per_round {
            let idx = base + i;
            col.upsert(format!("{:06}", idx), &Record::new(idx as usize))
                .unwrap();
        }
        let elapsed = start.elapsed().as_millis() as u64;
        let wal_size = std::fs::metadata(dir.path().join("wal.jsonl"))
            .unwrap()
            .len();
        println!(
            "  [round {}] {} ops in {}ms → {} ops/sec | WAL {} KiB",
            r + 1,
            per_round,
            elapsed,
            per_round * 1000 / elapsed.max(1),
            wal_size / 1024,
        );
    }

    assert_eq!(col.count(), (rounds * per_round) as usize);
}

// ─── 24. Concurrent persistent writes ────────────────────────────────────

#[tokio::test]
async fn wal_concurrent_persistent_writes() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().to_path_buf();
    let db = Arc::new(MemDb::open(&db_path).unwrap());
    let n_writers = 8u64;
    let per_writer = 500u64;
    let total = n_writers * per_writer;

    let start = Instant::now();
    let mut handles = vec![];
    for w in 0..n_writers {
        let db = db.clone();
        handles.push(tokio::task::spawn_blocking(move || {
            for i in 0..per_writer {
                let id = format!("w{:02}_{:06}", w, i);
                let rec = Record::new((w * 10000 + i) as usize);
                db.collection("records").upsert(id, &rec).unwrap();
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    let elapsed = start.elapsed().as_millis() as u64;

    let count = db.collection("records").count() as u64;
    assert_eq!(count, total);
    report("WAL concurrent writes (8×500)", total, elapsed);

    // Verify strict seq ordering in WAL under concurrent load.
    let wal_path = db_path.join("wal.jsonl");
    let entries = bichon_memdb::wal::read_after(&wal_path, 0).unwrap();
    assert_eq!(entries.len(), total as usize);
    let mut last = 0u64;
    for e in &entries {
        assert!(e.seq > last, "seq not monotonic under concurrency");
        last = e.seq;
    }

    // Recover.
    drop(db);
    let db = MemDb::open(&db_path).unwrap();
    assert_eq!(db.collection("records").count() as u64, total);
}

// ─── 25. Snapshot interleaved with sustained writes ──────────────────────

#[tokio::test]
async fn wal_snapshot_interleaved_heavy_writes() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().to_path_buf();
    let db = Arc::new(MemDb::open(&db_path).unwrap());
    let n = 2_000u64;

    let start = Instant::now();
    let db_w = db.clone();
    let write_handle = tokio::task::spawn_blocking(move || {
        for i in 0..n {
            let rec = Record::new(i as usize);
            db_w
                .collection("records")
                .upsert(format!("{:06}", i), &rec)
                .unwrap();
        }
    });

    let db_s = db.clone();
    let snap_handle = tokio::spawn(async move {
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            if let Err(e) = db_s.snapshot() {
                eprintln!("  [snapshot] error: {e}");
            }
        }
    });

    write_handle.await.unwrap();
    snap_handle.await.unwrap();
    db.snapshot().unwrap();
    let elapsed = start.elapsed().as_millis() as u64;

    let count = db.collection("records").count() as u64;
    assert_eq!(count, n);
    report("WAL snapshot+write interleaved", n, elapsed);

    // Recover and verify ALL records survived every snapshot+truncate cycle.
    drop(db);
    let db = MemDb::open(&db_path).unwrap();
    let col = db.collection("records");
    assert_eq!(col.count() as u64, n, "data loss during snapshot interleaving");
    for i in 0..n {
        assert!(col.exists(&format!("{:06}", i)), "missing record {i}");
    }
}

// ─── 26. Crash recovery simulation: kill without snapshot ────────────────

#[test]
fn wal_crash_recovery_no_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let n = 2_000u64;

    // Simulate normal operation, then "crash" (just drop the db).
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("records");
        for i in 0..n {
            col.insert(format!("{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
        // No snapshot — crash! (db dropped without clean shutdown)
    }

    // Recover — all data must be intact from WAL alone.
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("records");
        assert_eq!(
            col.count() as u64,
            n,
            "crash recovery lost data (no snapshot)"
        );
        // Verify a mix of early, middle, and late records.
        for &i in &[0, 1, n / 2, n - 2, n - 1] {
            let rec: Record = col.get_required(&format!("{:06}", i)).unwrap();
            assert_eq!(rec.name, format!("record_{}", i as usize));
        }
    }
}

// ─── 27. Mixed read/write with WAL ───────────────────────────────────────

#[tokio::test]
async fn wal_mixed_read_write_workload() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().to_path_buf();

    // Pre-populate.
    {
        let db = MemDb::open(&db_path).unwrap();
        let col = db.collection("records");
        for i in 0..2_000u64 {
            col.insert(format!("{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
    }

    let db = Arc::new(MemDb::open(&db_path).unwrap());
    let start = Instant::now();

    // Writer: append new records.
    let db_w = db.clone();
    let write_handle = tokio::task::spawn_blocking(move || {
        for i in 2_000..4_000u64 {
            let rec = Record::new(i as usize);
            db_w
                .collection("records")
                .upsert(format!("{:06}", i), &rec)
                .unwrap();
        }
    });

    // Reader: random reads across existing range.
    let db_r = db.clone();
    let read_handle = tokio::task::spawn_blocking(move || {
        for i in 0..5_000u64 {
            let key = format!("{:06}", i % 2500);
            let _: Option<Record> = db_r.collection("records").get(&key).unwrap();
        }
    });

    // Snapshotter: periodic snapshots during the workload.
    let db_s = db.clone();
    let snap_handle = tokio::spawn(async move {
        for _ in 0..8 {
            tokio::time::sleep(std::time::Duration::from_millis(3)).await;
            let _ = db_s.snapshot();
        }
    });

    let (wr, rr, sr) = tokio::join!(write_handle, read_handle, snap_handle);
    wr.unwrap();
    rr.unwrap();
    sr.unwrap();
    db.snapshot().unwrap();
    let elapsed = start.elapsed().as_millis() as u64;

    let count = db.collection("records").count() as u64;
    assert_eq!(count, 4_000);
    report("WAL mixed r/w/snapshot", 7_000, elapsed);

    // Final recovery check.
    drop(db);
    let db = MemDb::open(&db_path).unwrap();
    assert_eq!(db.collection("records").count() as u64, 4_000);
}

// ─── 28. WAL behaviour: delete + insert same key ─────────────────────────

#[test]
fn wal_delete_insert_same_key_replay() {
    let dir = tempfile::tempdir().unwrap();

    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("records");

        col.insert("x", &Record::new(1)).unwrap(); // seq=1: insert
        col.delete("x").unwrap(); // seq=2: delete
        col.insert("x", &Record::new(3)).unwrap(); // seq=3: insert again
        // Final state: key "x" exists with record_3 data.
        let rec: Record = col.get_required("x").unwrap();
        assert_eq!(rec.name, "record_3");
    }

    // Recover and verify final state is preserved.
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("records");
        assert_eq!(col.count(), 1);
        let rec: Record = col.get_required("x").unwrap();
        assert_eq!(rec.name, "record_3");
    }
}

// ─── 29. WAL with cross-collection transactions ──────────────────────────

#[test]
fn wal_cross_collection_txn_replay() {
    let dir = tempfile::tempdir().unwrap();
    let n = 500u64;

    {
        let db = MemDb::open(dir.path()).unwrap();
        for i in 0..n {
            let rec = Record::new(i as usize);
            db.transaction()
                .upsert("alpha", format!("a_{:06}", i), &rec)
                .unwrap()
                .upsert("beta", format!("b_{:06}", i), &rec)
                .unwrap()
                .commit()
                .unwrap();
        }
        assert_eq!(db.collection("alpha").count(), n as usize);
        assert_eq!(db.collection("beta").count(), n as usize);
    }

    // Recover and verify both collections.
    {
        let db = MemDb::open(dir.path()).unwrap();
        assert_eq!(db.collection("alpha").count(), n as usize);
        assert_eq!(db.collection("beta").count(), n as usize);

        // Spot-check: each collection's records should match.
        let a: Record = db.collection("alpha").get_required("a_000123").unwrap();
        let b: Record = db.collection("beta").get_required("b_000123").unwrap();
        assert_eq!(a.name, "record_123");
        assert_eq!(b.name, "record_123");
    }
}

// ─── 30. Snapshot then immediate crash — verify no data loss ─────────────

#[test]
fn wal_snapshot_then_crash_recovery() {
    let dir = tempfile::tempdir().unwrap();

    // Phase 1: write batch A, snapshot, write batch B, snapshot, crash.
    let snapshot_at: u64;
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("records");

        // Batch A.
        for i in 0..500u64 {
            col.insert(format!("a_{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
        db.snapshot().unwrap();

        // Batch B.
        for i in 0..500u64 {
            col.insert(format!("b_{:06}", i), &Record::new(500 + i as usize))
                .unwrap();
        }
        snapshot_at = col.count() as u64; // 1000
        db.snapshot().unwrap();

        // Batch C — no snapshot after this (simulates crash).
        for i in 0..500u64 {
            col.insert(format!("c_{:06}", i), &Record::new(1000 + i as usize))
                .unwrap();
        }
    } // crash

    // Phase 2: recover. Batches A+B must survive (snapshotted).
    // Batch C must also survive (in WAL).
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("records");
        let count = col.count() as u64;
        println!(
            "  [snapshot+cold crash] before crash={snapshot_at}, recovered={count}"
        );
        assert_eq!(count, 1_500, "data loss across snapshot boundaries");

        // Verify records from all three batches.
        assert!(col.exists("a_000000"));
        assert!(col.exists("a_000499"));
        assert!(col.exists("b_000000"));
        assert!(col.exists("b_000499"));
        assert!(col.exists("c_000000"));
        assert!(col.exists("c_000499"));
    }
}

// ─── 31. Large batch transaction with WAL ────────────────────────────────

#[test]
fn wal_large_transaction_batch() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemDb::open(dir.path()).unwrap();
    let n = 1_000u64;

    // Build one big transaction.
    let start = Instant::now();
    let mut txn = db.transaction();
    for i in 0..n {
        let rec = Record::new(i as usize);
        txn = txn
            .upsert("records", format!("{:06}", i), &rec)
            .unwrap();
    }
    txn.commit().unwrap();
    let elapsed = start.elapsed().as_millis() as u64;

    assert_eq!(db.collection("records").count(), n as usize);
    report(&format!("WAL large txn ({} ops)", n), n, elapsed);

    // The entire transaction should be a single WAL entry.
    let wal_path = dir.path().join("wal.jsonl");
    let entries = bichon_memdb::wal::read_after(&wal_path, 0).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].ops.len(), n as usize);

    // Recover and verify.
    drop(db);
    let db = MemDb::open(dir.path()).unwrap();
    assert_eq!(db.collection("records").count(), n as usize);
}

// ─── 32. Update-heavy persistent workload ────────────────────────────────

#[test]
fn wal_update_heavy_persistent() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemDb::open(dir.path()).unwrap();
    let col = db.collection("counters");
    let n = 500u64;

    // Seed counters.
    for i in 0..n {
        col.insert(
            format!("c_{:04}", i),
            &Counter {
                value: 0,
                updates: 0,
            },
        )
        .unwrap();
    }

    let rounds = 5u64;
    let start = Instant::now();
    for _ in 0..rounds {
        for i in 0..n {
            col.update::<Counter>(&format!("c_{:04}", i), |mut c| {
                c.value += 1;
                c.updates += 1;
                c
            })
            .unwrap();
        }
    }
    let elapsed = start.elapsed().as_millis() as u64;
    let total = rounds * n;
    report("WAL update-heavy", total, elapsed);

    // Verify in-memory state.
    let all: Vec<Counter> = col.list_all().unwrap();
    let sum_v: u64 = all.iter().map(|c| c.value).sum();
    let sum_u: u64 = all.iter().map(|c| c.updates).sum();
    assert_eq!(sum_v, sum_u);
    assert_eq!(sum_v, n * rounds);

    // Recover and verify again.
    drop(db);
    let db = MemDb::open(dir.path()).unwrap();
    let all: Vec<Counter> = db.collection("counters").list_all().unwrap();
    assert_eq!(all.len(), n as usize);
    let sum_v: u64 = all.iter().map(|c| c.value).sum();
    assert_eq!(sum_v, n * rounds);
}

// ─── 33. WAL seq gaps do not affect recovery ─────────────────────────────

#[test]
fn wal_seq_gaps_on_recovery() {
    let dir = tempfile::tempdir().unwrap();

    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("records");
        // Insert some records.
        for i in 1..=5u64 {
            col.insert(format!("k{}", i), &Record::new(i as usize))
                .unwrap();
        }
        // Snapshot captures seq=5.
        db.snapshot().unwrap();
        // More writes after snapshot — seq continues 6, 7, 8...
        for i in 6..=10u64 {
            col.insert(format!("k{}", i), &Record::new(i as usize))
                .unwrap();
        }
        assert_eq!(col.count(), 10);
    }

    // Recover. Seq 6-10 must replay from WAL on top of snapshot (seq=5).
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("records");
        assert_eq!(col.count(), 10);
        for i in 1..=10u64 {
            let rec: Record = col.get_required(&format!("k{}", i)).unwrap();
            assert_eq!(rec.name, format!("record_{}", i as usize));
        }
    }
}

// ─── 34. Stress: many small snapshots during continuous writes ───────────

#[tokio::test]
async fn wal_many_small_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().to_path_buf();
    let db = Arc::new(MemDb::open(&db_path).unwrap());
    let n = 1_000u64;

    let start = Instant::now();
    let db_w = db.clone();
    let write_handle = tokio::task::spawn_blocking(move || {
        for i in 0..n {
            let rec = Record::new(i as usize);
            db_w.collection("ticks")
                .upsert(format!("{:06}", i), &rec)
                .unwrap();
        }
    });

    let db_s = db.clone();
    let snap_handle = tokio::spawn(async move {
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            let _ = db_s.snapshot();
        }
    });

    let (wr, sr) = tokio::join!(write_handle, snap_handle);
    wr.unwrap();
    sr.unwrap();
    db.snapshot().unwrap();
    let elapsed = start.elapsed().as_millis() as u64;

    let count = db.collection("ticks").count() as u64;
    assert_eq!(count, n);
    report("WAL 50 snapshots + 1k writes", n, elapsed);

    // Recover.
    drop(db);
    let db = MemDb::open(&db_path).unwrap();
    assert_eq!(db.collection("ticks").count() as u64, n);
}

// ═══════════════════════════════════════════════════════════════════════════
// Durability mode comparison: Full vs Batch vs Off
// ═══════════════════════════════════════════════════════════════════════════

// ─── 35. Batch mode bulk insert performance ──────────────────────────────

#[test]
fn durability_batch_bulk_insert_perf() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemDb::open_with(dir.path(), Durability::batch(100)).unwrap();
    let col = db.collection("records");
    let n = 3_000u64;

    let start = Instant::now();
    for i in 0..n {
        col.insert(format!("{:06}", i), &Record::new(i as usize))
            .unwrap();
    }
    // Flush the final partial batch.
    let _flushed = db.flush().unwrap();
    let elapsed = start.elapsed().as_millis() as u64;

    let wal_size = std::fs::metadata(dir.path().join("wal.jsonl"))
        .unwrap()
        .len();
    // 3000 ops / 100 batch = ~30 fsyncs (vs 3000 in Full mode)
    let expected_syncs = (n + 99) / 100 + 1; // +1 for final flush
    assert_eq!(col.count(), n as usize);
    report("Batch-100 insert", n, elapsed);
    println!(
        "  WAL {:.0} KiB, ~{expected_syncs} fsyncs (vs {n} in Full mode)",
        wal_size as f64 / 1024.0,
    );

    // Recover — flush ensures everything is on disk.
    drop(db);
    let db = MemDb::open(dir.path()).unwrap();
    assert_eq!(db.collection("records").count(), n as usize);
}

// ─── 36. Batch mode: concurrent writes ───────────────────────────────────

#[tokio::test]
async fn durability_batch_concurrent_writes() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().to_path_buf();
    let db = Arc::new(MemDb::open_with(&db_path, Durability::batch(50)).unwrap());
    let n_writers = 4u64;
    let per_writer = 500u64;
    let total = n_writers * per_writer;

    let start = Instant::now();
    let mut handles = vec![];
    for w in 0..n_writers {
        let db = db.clone();
        handles.push(tokio::task::spawn_blocking(move || {
            for i in 0..per_writer {
                let id = format!("w{:02}_{:06}", w, i);
                let rec = Record::new((w * 10000 + i) as usize);
                db.collection("records").upsert(id, &rec).unwrap();
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    db.flush().unwrap();
    let elapsed = start.elapsed().as_millis() as u64;

    assert_eq!(db.collection("records").count() as u64, total);
    report("Batch-50 concurrent (4×500)", total, elapsed);

    drop(db);
    let db = MemDb::open(&db_path).unwrap();
    assert_eq!(db.collection("records").count() as u64, total);
}

// ─── 37. Batch mode: crash recovery of batched writes ────────────────────

#[test]
fn durability_batch_crash_recovery() {
    let dir = tempfile::tempdir().unwrap();

    {
        let db = MemDb::open_with(dir.path(), Durability::batch(20)).unwrap();
        let col = db.collection("records");

        // Write 95 records — triggers 4 batches of 20 + 15 buffered.
        for i in 0..95u64 {
            col.insert(format!("{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
        // Do NOT flush — last 15 are buffered, not yet on disk.
    } // "crash"

    // Recover: 80 flushed records (4 batches × 20) should survive.
    // The 15 buffered records are lost (expected behaviour).
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("records");
        let count = col.count() as u64;
        println!("  [batch crash] wrote 95, no flush, recovered {count}");
        assert!(count >= 80, "at least 4 batches should survive");
        assert!(count < 95, "unflushed records should be lost on crash");
    }
}

// ─── 38. Batch mode: flush worker ensures eventual durability ────────────

#[tokio::test]
async fn durability_batch_flush_worker() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().to_path_buf();
    let db = Arc::new(MemDb::open_with(&db_path, Durability::batch(200)).unwrap());

    // Start a flush worker that fires every 50ms.
    let _flush_handle = db.start_flush_worker(Duration::from_millis(50));

    let db_w = db.clone();
    let handle = tokio::task::spawn_blocking(move || {
        for i in 0..500u64 {
            db_w.collection("records")
                .upsert(format!("{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
    });

    // Meanwhile, the flush worker periodically commits buffered writes.
    handle.await.unwrap();
    // Give the flush worker a moment to catch up.
    tokio::time::sleep(Duration::from_millis(100)).await;
    db.flush().unwrap();

    assert_eq!(db.collection("records").count(), 500);
    drop(db);

    // All records should survive because the flush worker (and final flush)
    // pushed them to disk.
    let db = MemDb::open(&db_path).unwrap();
    assert_eq!(db.collection("records").count(), 500);
}

// ─── 39. Durability::Off mode — maximum throughput, zero fsync ───────────

#[test]
fn durability_off_bulk_insert_perf() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemDb::open_with(dir.path(), Durability::Off).unwrap();
    let col = db.collection("records");
    let n = 5_000u64;

    let start = Instant::now();
    for i in 0..n {
        col.insert(format!("{:06}", i), &Record::new(i as usize))
            .unwrap();
    }
    let elapsed = start.elapsed().as_millis() as u64;

    assert_eq!(col.count(), n as usize);
    report("Off (no fsync) insert", n, elapsed);

    // Data is written but never synced.  On crash, recovery may lose data.
    drop(db);
    let db = MemDb::open(dir.path()).unwrap();
    // OS may have flushed some pages — count what survived.
    let recovered = db.collection("records").count();
    println!("  [no fsync] wrote {n}, OS flushed {recovered} (may be 0 on crash)");
}

// ─── 40. Full vs Batch vs Off side-by-side comparison ────────────────────

#[test]
fn durability_full_vs_batch_vs_off() {
    let n = 1_000u64;

    // Full.
    {
        let dir = tempfile::tempdir().unwrap();
        let db = MemDb::open_with(dir.path(), Durability::Full).unwrap();
        let col = db.collection("records");
        let start = Instant::now();
        for i in 0..n {
            col.insert(format!("{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
        let elapsed = start.elapsed().as_millis() as u64;
        report("Full (fsync every op)", n, elapsed);
    }

    // Batch 100.
    {
        let dir = tempfile::tempdir().unwrap();
        let db = MemDb::open_with(dir.path(), Durability::batch(100)).unwrap();
        let col = db.collection("records");
        let start = Instant::now();
        for i in 0..n {
            col.insert(format!("{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
        db.flush().unwrap();
        let elapsed = start.elapsed().as_millis() as u64;
        report("Batch-100 (1 fsync/100)", n, elapsed);
    }

    // Batch 10.
    {
        let dir = tempfile::tempdir().unwrap();
        let db = MemDb::open_with(dir.path(), Durability::batch(10)).unwrap();
        let col = db.collection("records");
        let start = Instant::now();
        for i in 0..n {
            col.insert(format!("{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
        db.flush().unwrap();
        let elapsed = start.elapsed().as_millis() as u64;
        report("Batch-10 (1 fsync/10)", n, elapsed);
    }

    // Off.
    {
        let dir = tempfile::tempdir().unwrap();
        let db = MemDb::open_with(dir.path(), Durability::Off).unwrap();
        let col = db.collection("records");
        let start = Instant::now();
        for i in 0..n {
            col.insert(format!("{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
        let elapsed = start.elapsed().as_millis() as u64;
        report("Off (never fsync)", n, elapsed);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Concurrent read scalability — how well does the Mutex hold up under
// pure read load at various thread counts?
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn bench_concurrent_read_scaling() {
    use rand::Rng;

    let db = Arc::new(MemDb::in_memory());
    let n_records = 50_000u64;
    let reads_per_thread = 20_000u64;

    // Pre-populate.
    {
        let col = db.collection("records");
        for i in 0..n_records {
            col.insert(format!("{:06}", i), &Record::new(i as usize))
                .unwrap();
        }
    }
    println!(
        "\n  dataset: {n_records} records, each thread does {reads_per_thread} random get() calls\n"
    );

    // Baseline: single-threaded.
    let start = Instant::now();
    let mut rng = rand::rng();
    let col = db.collection("records");
    for _ in 0..reads_per_thread {
        let key = format!("{:06}", rng.random_range(0..n_records));
        let _: Option<Record> = col.get(&key).unwrap();
    }
    let single_ms = start.elapsed().as_millis() as u64;
    let single_ops = reads_per_thread * 1000 / single_ms.max(1);
    println!("  [1  thread ] {reads_per_thread} reads in {single_ms}ms → {single_ops} ops/sec");

    // Multi-threaded: 2, 4, 8, 16 threads.
    for &n_threads in &[2, 4, 8, 16] {
        let start = Instant::now();
        let mut handles = vec![];
        for _t in 0..n_threads {
            let db = db.clone();
            handles.push(tokio::task::spawn_blocking(move || {
                let col = db.collection("records");
                let mut rng = rand::rng();
                let mut found = 0u64;
                for _ in 0..reads_per_thread {
                    let key = format!("{:06}", rng.random_range(0..n_records));
                    let _: Option<Record> = col.get(&key).unwrap();
                    found += 1;
                }
                found
            }));
        }

        let mut total_found = 0u64;
        for h in handles {
            total_found += h.await.unwrap();
        }
        let elapsed_ms = start.elapsed().as_millis() as u64;
        let total_reads = n_threads * reads_per_thread;
        let total_ops = total_reads * 1000 / elapsed_ms.max(1);
        let speedup = total_ops as f64 / single_ops as f64;
        println!(
            "  [{n_threads:>2} threads] {total_reads} reads in {elapsed_ms}ms → {total_ops} ops/sec  (×{speedup:.2})"
        );
        assert_eq!(total_found, total_reads);
    }
}
