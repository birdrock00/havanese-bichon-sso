use bichon_memdb::{DbError, MemDb, Page};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

// ─── Test models ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Account {
    id: String,
    email: String,
    status: String,
    #[serde(default)]
    role: String,
}

impl Account {
    fn new(id: &str, email: &str, status: &str) -> Self {
        Self {
            id: id.into(),
            email: email.into(),
            status: status.into(),
            role: "user".into(),
        }
    }
}

fn open_tmp() -> (MemDb, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db = MemDb::open(dir.path()).unwrap();
    (db, dir)
}

// ─── Basic CRUD ──────────────────────────────────────────────────────────

#[test]
fn test_insert_and_get() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    let acc = Account::new("1", "a@x.com", "active");
    col.insert("1", &acc).unwrap();
    let found: Option<Account> = col.get("1").unwrap();
    assert_eq!(found, Some(acc));
}

#[test]
fn test_get_missing_returns_none() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    let found: Option<Account> = col.get("nope").unwrap();
    assert!(found.is_none());
}

#[test]
fn test_get_required_missing_returns_error() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    let err = col.get_required::<Account>("nope").unwrap_err();
    assert!(matches!(err, DbError::NotFound(_, _)));
}

#[test]
fn test_insert_duplicate_returns_error() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    let acc = Account::new("1", "a@x.com", "active");
    col.insert("1", &acc).unwrap();
    let err = col.insert("1", &acc).unwrap_err();
    assert!(matches!(err, DbError::DuplicateKey(_, _)));
}

#[test]
fn test_upsert_creates_and_overwrites() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    let acc = Account::new("1", "a@x.com", "active");
    col.upsert("1", &acc).unwrap();
    let found: Account = col.get_required("1").unwrap();
    assert_eq!(found.status, "active");

    let updated = Account::new("1", "a@x.com", "disabled");
    col.upsert("1", &updated).unwrap();
    let found: Account = col.get_required("1").unwrap();
    assert_eq!(found.status, "disabled");
}

#[test]
fn test_update_modifies_record() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    col.insert("1", &Account::new("1", "a@x.com", "active"))
        .unwrap();
    let result: Account = col
        .update("1", |mut a: Account| {
            a.status = "disabled".into();
            a
        })
        .unwrap();
    assert_eq!(result.status, "disabled");
    let found: Account = col.get_required("1").unwrap();
    assert_eq!(found.status, "disabled");
}

#[test]
fn test_update_missing_returns_error() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    let err = col.update("nope", |a: Account| a).unwrap_err();
    assert!(matches!(err, DbError::NotFound(_, _)));
}

#[test]
fn test_delete_existing() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    col.insert("1", &Account::new("1", "a@x.com", "active"))
        .unwrap();
    let deleted = col.delete("1").unwrap();
    assert!(deleted);
    assert!(!col.exists("1"));
}

#[test]
fn test_delete_missing_returns_false() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    let deleted = col.delete("nope").unwrap();
    assert!(!deleted);
}

#[test]
fn test_exists() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    assert!(!col.exists("1"));
    col.insert("1", &Account::new("1", "a@x.com", "active"))
        .unwrap();
    assert!(col.exists("1"));
}

#[test]
fn test_count() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    assert_eq!(col.count(), 0);
    col.insert("1", &Account::new("1", "a@x.com", "active"))
        .unwrap();
    col.insert("2", &Account::new("2", "b@x.com", "active"))
        .unwrap();
    assert_eq!(col.count(), 2);
    col.delete("1").unwrap();
    assert_eq!(col.count(), 1);
}

// ─── Queries ─────────────────────────────────────────────────────────────

#[test]
fn test_list_all() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    col.insert("1", &Account::new("1", "a@x.com", "active"))
        .unwrap();
    col.insert("2", &Account::new("2", "b@x.com", "active"))
        .unwrap();
    let all: Vec<Account> = col.list_all().unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_list_all_empty_collection() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    let all: Vec<Account> = col.list_all().unwrap();
    assert!(all.is_empty());
}

#[test]
fn test_filter() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    col.insert("1", &Account::new("1", "a@x.com", "active"))
        .unwrap();
    col.insert("2", &Account::new("2", "b@x.com", "disabled"))
        .unwrap();
    col.insert("3", &Account::new("3", "c@x.com", "active"))
        .unwrap();

    let active: Vec<Account> = col.filter(|a: &Account| a.status == "active").unwrap();
    assert_eq!(active.len(), 2);

    let disabled: Vec<Account> = col.filter(|a: &Account| a.status == "disabled").unwrap();
    assert_eq!(disabled.len(), 1);
}

#[test]
fn test_filter_empty_collection() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    let result: Vec<Account> = col.filter(|_: &Account| true).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_scan_prefix() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    col.insert("2024_001", &Account::new("1", "a@x.com", "active"))
        .unwrap();
    col.insert("2024_002", &Account::new("2", "b@x.com", "active"))
        .unwrap();
    col.insert("2025_001", &Account::new("3", "c@x.com", "active"))
        .unwrap();

    let result: Vec<Account> = col.scan_prefix("2024_").unwrap();
    assert_eq!(result.len(), 2);

    let result: Vec<Account> = col.scan_prefix("2025_").unwrap();
    assert_eq!(result.len(), 1);

    let result: Vec<Account> = col.scan_prefix("9999_").unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_paginate_asc() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    for i in 1..=5 {
        col.insert(
            format!("{:03}", i),
            &Account::new(&i.to_string(), &format!("{}@x.com", i), "active"),
        )
        .unwrap();
    }
    let page = col.paginate::<Account>(&Page::new(1, 2), false).unwrap();
    assert_eq!(page.total, 5);
    assert_eq!(page.total_pages, 3);
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.items[0].id, "1");
    assert_eq!(page.items[1].id, "2");

    let page2 = col.paginate::<Account>(&Page::new(2, 2), false).unwrap();
    assert_eq!(page2.items.len(), 2);
    assert_eq!(page2.items[0].id, "3");

    let page3 = col.paginate::<Account>(&Page::new(3, 2), false).unwrap();
    assert_eq!(page3.items.len(), 1);
    assert_eq!(page3.items[0].id, "5");
}

#[test]
fn test_paginate_desc() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    for i in 1..=5 {
        col.insert(
            format!("{:03}", i),
            &Account::new(&i.to_string(), &format!("{}@x.com", i), "active"),
        )
        .unwrap();
    }
    let page = col.paginate::<Account>(&Page::new(1, 2), true).unwrap();
    assert_eq!(page.items[0].id, "5");
    assert_eq!(page.items[1].id, "4");
}

#[test]
fn test_paginate_out_of_range() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    col.insert("1", &Account::new("1", "a@x.com", "active"))
        .unwrap();
    let page = col.paginate::<Account>(&Page::new(99, 10), false).unwrap();
    assert_eq!(page.total, 1);
    assert!(page.items.is_empty());
}

#[test]
fn test_paginate_empty_collection() {
    let db = MemDb::in_memory();
    let col = db.collection("accounts");
    let page = col.paginate::<Account>(&Page::new(1, 10), false).unwrap();
    assert_eq!(page.total, 0);
    assert_eq!(page.total_pages, 0);
    assert!(page.items.is_empty());
}

// ─── Transactions ────────────────────────────────────────────────────────

#[test]
fn test_transaction_commit() {
    let db = MemDb::in_memory();
    let acc1 = Account::new("1", "a@x.com", "active");
    let acc2 = Account::new("2", "b@x.com", "active");

    db.transaction()
        .upsert("accounts", "1", &acc1)
        .unwrap()
        .upsert("accounts", "2", &acc2)
        .unwrap()
        .commit()
        .unwrap();

    let col = db.collection("accounts");
    assert!(col.exists("1"));
    assert!(col.exists("2"));
}

#[test]
fn test_transaction_delete_across_collections() {
    let db = MemDb::in_memory();
    let acc = Account::new("1", "a@x.com", "active");
    db.collection("accounts").insert("1", &acc).unwrap();
    db.collection("logs").insert("log-1", &acc).unwrap();

    db.transaction()
        .delete("accounts", "1")
        .delete("logs", "log-1")
        .commit()
        .unwrap();

    assert!(!db.collection("accounts").exists("1"));
    assert!(!db.collection("logs").exists("log-1"));
}

// ─── Collection isolation ────────────────────────────────────────────────

#[test]
fn test_collections_are_isolated() {
    let db = MemDb::in_memory();
    let acc = Account::new("1", "a@x.com", "active");
    db.collection("accounts").insert("1", &acc).unwrap();

    // Same key in different collections should not interfere.
    assert!(db.collection("accounts").exists("1"));
    assert!(!db.collection("users").exists("1"));
}

// ─── WAL + Snapshot persistence ──────────────────────────────────────────

#[test]
fn test_persist_and_recover() {
    let dir = tempfile::tempdir().unwrap();

    // Write data.
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        col.insert("1", &Account::new("1", "a@x.com", "active"))
            .unwrap();
        col.insert("2", &Account::new("2", "b@x.com", "disabled"))
            .unwrap();
    }

    // Recover after restart.
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        assert_eq!(col.count(), 2);
        let acc: Account = col.get_required("1").unwrap();
        assert_eq!(acc.email, "a@x.com");
    }
}

#[test]
fn test_recover_after_snapshot() {
    let dir = tempfile::tempdir().unwrap();

    // Write + snapshot.
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        col.insert("1", &Account::new("1", "a@x.com", "active"))
            .unwrap();
        db.snapshot().unwrap();
    }

    // Recover from snapshot.
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        assert_eq!(col.count(), 1);
        assert!(col.exists("1"));
    }
}

#[test]
fn test_recover_snapshot_plus_wal() {
    let dir = tempfile::tempdir().unwrap();

    // Write 3 records, snapshot, then write 2 more.
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        col.insert("1", &Account::new("1", "a@x.com", "active"))
            .unwrap();
        col.insert("2", &Account::new("2", "b@x.com", "active"))
            .unwrap();
        col.insert("3", &Account::new("3", "c@x.com", "active"))
            .unwrap();
        db.snapshot().unwrap(); // WAL truncated, last_seq=3
                                // Writes after snapshot — go into fresh WAL.
        col.insert("4", &Account::new("4", "d@x.com", "active"))
            .unwrap();
        col.insert("5", &Account::new("5", "e@x.com", "active"))
            .unwrap();
    }

    // Recover: snapshot(seq=3) + replay WAL(seq=4,5).
    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        assert_eq!(col.count(), 5);
        for i in 1..=5 {
            assert!(col.exists(&i.to_string()));
        }
    }
}

#[test]
fn test_snapshot_does_not_duplicate_on_recovery() {
    let dir = tempfile::tempdir().unwrap();

    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        col.insert("1", &Account::new("1", "a@x.com", "active"))
            .unwrap();
        col.insert("2", &Account::new("2", "b@x.com", "active"))
            .unwrap();
        db.snapshot().unwrap();
        // Multiple snapshots should not duplicate data.
        db.snapshot().unwrap();
    }

    {
        let db = MemDb::open(dir.path()).unwrap();
        assert_eq!(db.collection("accounts").count(), 2);
    }
}

#[test]
fn test_wal_seq_skips_already_snapshotted_entries() {
    let dir = tempfile::tempdir().unwrap();

    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        col.insert("1", &Account::new("1", "a@x.com", "active"))
            .unwrap(); // seq=1
        col.insert("2", &Account::new("2", "b@x.com", "active"))
            .unwrap(); // seq=2
        db.snapshot().unwrap(); // snapshot last_seq=2, WAL truncated

        // Write after snapshot.
        col.insert("3", &Account::new("3", "c@x.com", "active"))
            .unwrap(); // seq=3
    }

    // Verify WAL only contains seq=3.
    let wal_path = dir.path().join("wal.jsonl");
    let entries = bichon_memdb::wal::read_after(&wal_path, 2).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].seq, 3);

    // Data is complete after recovery.
    {
        let db = MemDb::open(dir.path()).unwrap();
        assert_eq!(db.collection("accounts").count(), 3);
    }
}

#[test]
fn test_delete_persisted_across_restart() {
    let dir = tempfile::tempdir().unwrap();

    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        col.insert("1", &Account::new("1", "a@x.com", "active"))
            .unwrap();
        col.insert("2", &Account::new("2", "b@x.com", "active"))
            .unwrap();
        col.delete("1").unwrap();
    }

    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        assert!(!col.exists("1"));
        assert!(col.exists("2"));
    }
}

#[test]
fn test_update_persisted_across_restart() {
    let dir = tempfile::tempdir().unwrap();

    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        col.insert("1", &Account::new("1", "a@x.com", "active"))
            .unwrap();
        col.update("1", |mut a: Account| {
            a.status = "disabled".into();
            a
        })
        .unwrap();
    }

    {
        let db = MemDb::open(dir.path()).unwrap();
        let acc: Account = db.collection("accounts").get_required("1").unwrap();
        assert_eq!(acc.status, "disabled");
    }
}

#[test]
fn test_transaction_persisted_across_restart() {
    let dir = tempfile::tempdir().unwrap();

    {
        let db = MemDb::open(dir.path()).unwrap();
        let acc1 = Account::new("1", "a@x.com", "active");
        let acc2 = Account::new("2", "b@x.com", "active");
        db.transaction()
            .upsert("accounts", "1", &acc1)
            .unwrap()
            .upsert("accounts", "2", &acc2)
            .unwrap()
            .delete("accounts", "nonexistent")
            .commit()
            .unwrap();
    }

    {
        let db = MemDb::open(dir.path()).unwrap();
        let col = db.collection("accounts");
        assert_eq!(col.count(), 2);
        assert!(col.exists("1"));
        assert!(col.exists("2"));
    }
}

// ─── Schema evolution ────────────────────────────────────────────────────

#[test]
fn test_schema_add_field_with_default() {
    let (db, dir) = open_tmp();

    // Write with old model.
    #[derive(Serialize, Deserialize)]
    struct AccountV1 {
        id: String,
        email: String,
    }
    db.collection("accounts")
        .insert(
            "1",
            &AccountV1 {
                id: "1".into(),
                email: "a@x.com".into(),
            },
        )
        .unwrap();
    drop(db);

    // New model adds a field with #[serde(default)].
    #[derive(Serialize, Deserialize, Debug)]
    struct AccountV2 {
        id: String,
        email: String,
        #[serde(default)]
        role: String,
    }
    let db = MemDb::open(dir.path()).unwrap();
    let acc: AccountV2 = db.collection("accounts").get_required("1").unwrap();
    assert_eq!(acc.email, "a@x.com");
    assert_eq!(acc.role, ""); // default fills empty string
}

#[test]
fn test_schema_remove_field() {
    let (db, dir) = open_tmp();

    // Write with old model that has a legacy field.
    #[derive(Serialize, Deserialize)]
    struct AccountWithLegacy {
        id: String,
        email: String,
        legacy_field: String,
    }
    db.collection("accounts")
        .insert(
            "1",
            &AccountWithLegacy {
                id: "1".into(),
                email: "a@x.com".into(),
                legacy_field: "old_value".into(),
            },
        )
        .unwrap();
    drop(db);

    // New model drops legacy_field — serde ignores unknown fields by default.
    #[derive(Serialize, Deserialize, Debug)]
    struct AccountV2 {
        id: String,
        email: String,
    }
    let db = MemDb::open(dir.path()).unwrap();
    let acc: AccountV2 = db.collection("accounts").get_required("1").unwrap();
    assert_eq!(acc.email, "a@x.com");
}

#[test]
fn test_schema_rename_field_with_alias() {
    let (db, dir) = open_tmp();

    #[derive(Serialize, Deserialize)]
    struct AccountOld {
        id: String,
        username: String,
    }
    db.collection("accounts")
        .insert(
            "1",
            &AccountOld {
                id: "1".into(),
                username: "alice".into(),
            },
        )
        .unwrap();
    drop(db);

    // Field renamed; alias keeps backward compatibility with old data.
    #[derive(Serialize, Deserialize, Debug)]
    struct AccountNew {
        id: String,
        #[serde(alias = "username")]
        display_name: String,
    }
    let db = MemDb::open(dir.path()).unwrap();
    let acc: AccountNew = db.collection("accounts").get_required("1").unwrap();
    assert_eq!(acc.display_name, "alice");
}

// ─── Concurrency safety ──────────────────────────────────────────────────

#[tokio::test]
async fn test_concurrent_writes_no_data_loss() {
    let db = MemDb::in_memory();
    let db = std::sync::Arc::new(db);
    let mut handles = vec![];

    for i in 0..100 {
        let db = db.clone();
        handles.push(tokio::spawn(async move {
            let col = db.collection("accounts");
            let acc = Account::new(&i.to_string(), &format!("{}@x.com", i), "active");
            // spawn_blocking because the Mutex may block briefly.
            tokio::task::spawn_blocking(move || col.upsert(i.to_string(), &acc))
                .await
                .unwrap()
                .unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert_eq!(db.collection("accounts").count(), 100);
}

#[tokio::test]
async fn test_concurrent_writes_wal_seq_monotonic() {
    let dir = tempfile::tempdir().unwrap();
    let db = std::sync::Arc::new(MemDb::open(dir.path()).unwrap());
    let mut handles = vec![];

    for i in 0..50 {
        let db = db.clone();
        handles.push(tokio::spawn(async move {
            let col = db.collection("accounts");
            let acc = Account::new(&i.to_string(), &format!("{}@x.com", i), "active");
            tokio::task::spawn_blocking(move || col.upsert(i.to_string(), &acc))
                .await
                .unwrap()
                .unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    // Verify WAL seq is strictly monotonic.
    let wal_path = dir.path().join("wal.jsonl");
    let entries = bichon_memdb::wal::read_after(&wal_path, 0).unwrap();
    assert_eq!(entries.len(), 50);
    let mut last = 0u64;
    for e in &entries {
        assert!(e.seq > last, "seq not monotonic: {} <= {}", e.seq, last);
        last = e.seq;
    }
}

#[tokio::test]
async fn test_concurrent_snapshot_and_writes() {
    let dir = tempfile::tempdir().unwrap();
    let db = std::sync::Arc::new(MemDb::open(dir.path()).unwrap());

    // Concurrent writes + snapshots.
    let db_write = db.clone();
    let write_handle = tokio::spawn(async move {
        for i in 0..100 {
            let col = db_write.collection("accounts");
            let acc = Account::new(&i.to_string(), &format!("{}@x.com", i), "active");
            tokio::task::spawn_blocking(move || col.upsert(i.to_string(), &acc))
                .await
                .unwrap()
                .unwrap();
        }
    });

    let db_snap = db.clone();
    let snap_handle = tokio::spawn(async move {
        for _ in 0..5 {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            db_snap.snapshot().unwrap();
        }
    });

    write_handle.await.unwrap();
    snap_handle.await.unwrap();

    // Final snapshot to ensure everything is on disk.
    db.snapshot().unwrap();

    // Data is complete after recovery.
    let db2 = MemDb::open(dir.path()).unwrap();
    assert_eq!(db2.collection("accounts").count(), 100);
}
