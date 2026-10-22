//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Persisted records of blob repairs.
//!
//! Stored in the memdb `blob_repair_reports` collection as a bounded
//! **diagnostic ring**: the [`BlobRepairRecord::HISTORY_LIMIT`] most recent
//! entries, pruned on every write. It is deliberately not an audit log —
//! repairs that found nothing wrong are not recorded at all (see
//! [`BlobRepairRecord::persist`]) and anything older than the ring is dropped.
//! The audit trail for this operation belongs on the event bus.
//!
//! [`MemDbModel::BACKUP_GATE_EXEMPT`] is set because the outcome an operator
//! most needs to see — `skipped_due_to_backup` — is produced *while a backup
//! capture window is open*, so the gated write helpers would reject exactly
//! that write. The exemption is safe because the sole writer is the
//! `repair_message` REST handler, and during a capture window that handler
//! mutates nothing but memdb: its blob write is itself rejected by the gate, so
//! the record here is the request's only change. Any future writer must
//! preserve that property.

use serde::{Deserialize, Serialize};

use crate::database::{batch_delete_impl, list_all_impl, manager::DB_MANAGER, upsert_impl, MemDbModel};
use crate::envelope::extractor::{BlobRepairReport, STATUS_COMPLETE};
use crate::error::BichonResult;

/// One blob-repair attempt persisted in the memdb.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct BlobRepairRecord {
    pub id: String,
    /// UTC millis when the repair finished.
    pub created_at: i64,
    /// Stable id of the user who triggered the repair.
    pub operator_id: u64,
    /// Display name of the user who triggered the repair.
    pub operator: String,
    /// Account that owns the repaired record.
    pub account_id: u64,
    pub mailbox_id: u64,
    pub subject: String,
    pub message_id: String,
    pub report: BlobRepairReport,
}

impl MemDbModel for BlobRepairRecord {
    fn collection() -> &'static str {
        "blob_repair_reports"
    }

    /// Time-ordered key: a 13-digit zero-padded millis prefix makes memdb's
    /// lexicographic `list_all` order equal chronological order (good for
    /// ~250 years), so [`BlobRepairRecord::list_recent`] needs no sort and
    /// [`BlobRepairRecord::prune_beyond`] evicts oldest-first exactly.
    ///
    /// Records written within the *same* millisecond tie on the prefix and are
    /// then ordered by `id`, which is a random uuid — total and deterministic
    /// for a given set of records, but not chronological. Two repairs landing
    /// in one millisecond may therefore be evicted in either order. That is
    /// acceptable for a diagnostic ring and is not worth a monotonic counter.
    fn key(&self) -> String {
        format!("{:013}-{}", self.created_at, self.id)
    }

    /// See the module docs for why this is exempt.
    const BACKUP_GATE_EXEMPT: bool = true;
}

impl BlobRepairRecord {
    /// Cap on persisted records. Every write prunes beyond the most recent
    /// [`Self::HISTORY_LIMIT`], so the ring rolls without a cleanup task.
    pub const HISTORY_LIMIT: usize = 200;

    /// Persists one repair attempt and prunes the ring. Returns `false` —
    /// writing nothing — when the record turned out to be healthy
    /// (`status: "complete"`): a no-op click would otherwise evict a real
    /// `partial`/`failed` entry from a ring this small, and the caller already
    /// sees the `complete` result in the response.
    pub fn persist(operator_id: u64, operator: &str, report: &BlobRepairReport) -> BichonResult<bool> {
        if report.status == STATUS_COMPLETE {
            return Ok(false);
        }
        let record = Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: crate::utc_now!(),
            operator_id,
            operator: operator.to_string(),
            account_id: report.account_id,
            mailbox_id: report.mailbox_id,
            subject: report.subject.clone(),
            message_id: report.message_id.clone(),
            report: report.clone(),
        };
        upsert_impl::<Self>(DB_MANAGER.db(), record)?;
        Self::prune_beyond(Self::HISTORY_LIMIT);
        Ok(true)
    }

    /// Newest first. `list_all` returns key order and the key is time-ordered,
    /// so a reverse is enough — no sort, and no ambiguity between records
    /// written in the same millisecond.
    pub fn list_recent(limit: usize) -> Vec<BlobRepairRecord> {
        let mut all: Vec<BlobRepairRecord> = list_all_impl(DB_MANAGER.db()).unwrap_or_default();
        all.reverse();
        all.truncate(limit);
        all
    }

    /// Deletes records beyond the most recent `limit`, oldest first. Returns
    /// how many were removed. Uses one batched transaction rather than a
    /// delete per key, which is also safe inside a backup window because the
    /// model is gate-exempt.
    pub fn prune_beyond(limit: usize) -> usize {
        let all: Vec<BlobRepairRecord> = list_all_impl(DB_MANAGER.db()).unwrap_or_default();
        if all.len() <= limit {
            return 0;
        }
        let excess = all.len() - limit;
        let keys: Vec<String> = all.into_iter().take(excess).map(|r| r.key()).collect();
        batch_delete_impl::<Self>(DB_MANAGER.db(), keys).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::cli::SETTINGS;
    use std::sync::{Mutex, Once};

    /// Same shared temp-root convention as `backup::manager::tests`: whichever
    /// module initializes the process globals first wins, so this only has to
    /// happen once and must happen before anything dereferences `SETTINGS`
    /// (which would otherwise fall through to clap and exit the test process).
    static TEST_ENV: Once = Once::new();
    fn init_test_env() {
        TEST_ENV.call_once(|| {
            let root = crate::test_support::unique_test_root("bichon-core-test");
            std::env::set_var("BICHON_ROOT_DIR", &root);
            std::env::set_var("BICHON_ENCRYPT_PASSWORD", "test-password");
            let _ = &*SETTINGS;
            let _ = &*DB_MANAGER;
        });
    }

    /// The DB-backed tests share one collection, so they must not interleave.
    /// Scoped to this module: no other module touches `blob_repair_reports`.
    /// A panicking test would otherwise poison the lock and turn one real
    /// failure into a cascade of confusing `PoisonError`s.
    static DB_TESTS: Mutex<()> = Mutex::new(());

    fn db_lock() -> std::sync::MutexGuard<'static, ()> {
        DB_TESTS.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn report(status: &str) -> BlobRepairReport {
        BlobRepairReport {
            envelope_id: "env-1".into(),
            account_id: 7,
            mailbox_id: 3,
            subject: "Quarterly report".into(),
            message_id: "<abc@example.com>".into(),
            status: status.into(),
            ..Default::default()
        }
    }

    /// A healthy record must not consume a slot in the ring.
    #[test]
    fn persist_skips_a_complete_report() {
        // Returns before touching the DB, so no test env is needed.
        assert!(!BlobRepairRecord::persist(1, "admin", &report(STATUS_COMPLETE)).unwrap());
    }

    #[tokio::test]
    async fn persist_records_every_actionable_status() {
        let _guard = db_lock();
        init_test_env();
        for status in ["repaired", "partial", "failed"] {
            assert!(
                BlobRepairRecord::persist(1, "admin", &report(status)).unwrap(),
                "{status} should be recorded"
            );
        }
        // The outcome an operator most needs to see lands in `partial`.
        let skipped = BlobRepairReport {
            skipped_due_to_backup: true,
            ..report("partial")
        };
        assert!(BlobRepairRecord::persist(1, "admin", &skipped).unwrap());
    }

    #[tokio::test]
    async fn persist_copies_identity_from_the_report_and_operator_from_the_arguments() {
        let _guard = db_lock();
        init_test_env();
        BlobRepairRecord::persist(42, "alice", &report("repaired")).unwrap();
        let latest = BlobRepairRecord::list_recent(1).remove(0);
        assert_eq!(latest.account_id, 7);
        assert_eq!(latest.mailbox_id, 3);
        assert_eq!(latest.subject, "Quarterly report");
        assert_eq!(latest.message_id, "<abc@example.com>");
        assert_eq!(latest.operator_id, 42);
        assert_eq!(latest.operator, "alice");
        assert_eq!(latest.report.status, "repaired");
    }

    /// The whole ordering scheme rests on this: lexicographic == chronological.
    #[test]
    fn key_is_time_ordered() {
        let make = |created_at: i64, id: &str| BlobRepairRecord {
            id: id.into(),
            created_at,
            operator_id: 1,
            operator: "admin".into(),
            account_id: 1,
            mailbox_id: 1,
            subject: String::new(),
            message_id: String::new(),
            report: report("repaired"),
        };
        // Two records in the same millisecond still order deterministically.
        let mut keys = vec![
            make(1_700_000_000_000, "zzz").key(),
            make(1_700_000_000_000, "aaa").key(),
            make(999, "aaa").key(),
            make(2_000_000_000_000, "aaa").key(),
        ];
        let expected = vec![
            make(999, "aaa").key(),
            make(1_700_000_000_000, "aaa").key(),
            make(1_700_000_000_000, "zzz").key(),
            make(2_000_000_000_000, "aaa").key(),
        ];
        keys.sort();
        assert_eq!(keys, expected);
    }

    /// Inserts a record with an explicit timestamp and id, bypassing `persist`
    /// so the ordering tests do not depend on how many records happen to land
    /// in one millisecond (within a millisecond the key's tiebreak is a random
    /// uuid, so `persist` cannot express a chosen order).
    fn insert(id: &str, created_at: i64, subject: &str) -> BlobRepairRecord {
        let record = BlobRepairRecord {
            id: id.into(),
            created_at,
            operator_id: 1,
            operator: "admin".into(),
            account_id: 7,
            mailbox_id: 3,
            subject: subject.into(),
            message_id: String::new(),
            report: report("repaired"),
        };
        upsert_impl::<BlobRepairRecord>(DB_MANAGER.db(), record.clone()).unwrap();
        record
    }

    #[tokio::test]
    async fn list_recent_is_newest_first() {
        let _guard = db_lock();
        init_test_env();
        BlobRepairRecord::prune_beyond(0);
        for i in 0..5 {
            insert(&format!("id-{i}"), 1_000 + i, &format!("s{i}"));
        }
        let recent = BlobRepairRecord::list_recent(usize::MAX);
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].subject, "s4");
        assert_eq!(recent[4].subject, "s0");
        // Truncation keeps the newest.
        let two = BlobRepairRecord::list_recent(2);
        assert_eq!(two.len(), 2);
        assert_eq!(two[0].subject, "s4");
        assert_eq!(two[1].subject, "s3");
    }

    #[tokio::test]
    async fn prune_beyond_evicts_oldest_first() {
        let _guard = db_lock();
        init_test_env();
        BlobRepairRecord::prune_beyond(0);
        assert_eq!(BlobRepairRecord::list_recent(usize::MAX).len(), 0);

        for i in 0..5 {
            insert(&format!("id-{i}"), 1_000 + i, &format!("p{i}"));
        }
        assert_eq!(BlobRepairRecord::list_recent(usize::MAX).len(), 5);

        assert_eq!(BlobRepairRecord::prune_beyond(2), 3);
        let survivors = BlobRepairRecord::list_recent(usize::MAX);
        assert_eq!(survivors.len(), 2);
        assert_eq!(survivors[0].subject, "p4");
        assert_eq!(survivors[1].subject, "p3");

        // Pruning at or below the cap is a no-op.
        assert_eq!(BlobRepairRecord::prune_beyond(2), 0);
        assert_eq!(BlobRepairRecord::prune_beyond(10), 0);
    }

    /// The ring is capped at exactly `HISTORY_LIMIT`, oldest evicted first.
    #[tokio::test]
    async fn prune_enforces_the_history_limit() {
        let _guard = db_lock();
        init_test_env();
        BlobRepairRecord::prune_beyond(0);

        let over = BlobRepairRecord::HISTORY_LIMIT + 5;
        for i in 0..over {
            insert(&format!("id-{i}"), 1_000 + i as i64, &format!("h{i}"));
        }
        assert_eq!(BlobRepairRecord::list_recent(usize::MAX).len(), over);

        assert_eq!(
            BlobRepairRecord::prune_beyond(BlobRepairRecord::HISTORY_LIMIT),
            5
        );
        let survivors = BlobRepairRecord::list_recent(usize::MAX);
        assert_eq!(survivors.len(), BlobRepairRecord::HISTORY_LIMIT);
        // The five oldest are gone; the newest is still at the head.
        assert_eq!(survivors[0].subject, format!("h{}", over - 1));
        assert_eq!(survivors.last().unwrap().subject, "h5");
    }

    /// Pins the documented same-millisecond behaviour: the ordering is total
    /// and deterministic (by id) but not chronological.
    #[tokio::test]
    async fn same_millisecond_records_order_by_id() {
        let _guard = db_lock();
        init_test_env();
        BlobRepairRecord::prune_beyond(0);
        for id in ["aaa", "bbb", "ccc"] {
            insert(id, 1_000, id);
        }
        let recent = BlobRepairRecord::list_recent(usize::MAX);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].subject, "ccc");
        assert_eq!(recent[1].subject, "bbb");
        assert_eq!(recent[2].subject, "aaa");
    }
}
