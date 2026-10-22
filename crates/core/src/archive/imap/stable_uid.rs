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

//! Stable archive-side IMAP UID allocation.
//!
//! Invariants:
//! 1. A UID is only handed out from a range that was persisted (fsync'd)
//!    beforehand; crashes may waste numbers, never reissue them.
//! 2. UIDs are write-once: assigned at envelope creation, carried (never
//!    recomputed) by every later document-reconstruction path.
//! 3. The per-account counter only moves forward. A counter found BEHIND the
//!    index means an inconsistent restore: bump the account's UIDVALIDITY
//!    generation (clients drop their cache) and move the counter above the
//!    observed maximum.
//!
//! The counter lives on the `Account` row rather than on `MailBox` because
//! every account write is a read-modify-write, while mailbox rows are rewritten
//! wholesale from remote listings (`download/flow.rs`), which would silently
//! reset a counter stored there.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use bichon_memdb::MemDb;
use tantivy::collector::TopDocs;
use tantivy::query::TermQuery;
use tantivy::schema::IndexRecordOption;
use tantivy::{DocAddress, Order, Term};

use crate::{
    account::migration::Account,
    database::{manager::DB_MANAGER, update_atomic_impl, MemDbModel},
    error::{code::ErrorCode, BichonError, BichonResult},
    raise_error,
    store::tantivy::{envelope::ENVELOPE_MANAGER, fields::F_IMAP_UID, schema::SchemaTools},
};

/// How many UIDs one reservation hands out before the counter is advanced (and
/// fsync'd) again. A crash can waste at most this many numbers per account;
/// 1024 keeps a million-message import at ~1000 fsyncs.
pub const UID_BLOCK: u64 = 1024;

/// Highest UID the IMAP protocol can carry (RFC 3501: 32-bit non-zero).
const UID_MAX: u64 = u32::MAX as u64;

/// In-process block cache: account_id -> (next_free, block_end).
///
/// All allocation goes through this mutex, so concurrent ingest paths
/// (downloader / import / journaling) cannot receive overlapping ranges within
/// one process; the atomic read-modify-write below covers the cross-process and
/// stale-read cases.
static BLOCKS: LazyLock<Mutex<HashMap<u64, (u64, u64)>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// The block cache, tolerating a poisoned mutex.
///
/// Poisoning means some thread panicked while holding it. The map is a cache,
/// not the source of truth — the persisted counter is — so the worst case of
/// continuing with it is a wasted range, never a reissued number. Panicking
/// here instead would turn one unrelated panic into "no mail can be archived
/// any more", which is a far worse failure.
fn blocks() -> std::sync::MutexGuard<'static, HashMap<u64, (u64, u64)>> {
    BLOCKS.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Next stable UID for one new envelope of `account_id`.
///
/// Call this only at the ingest choke point: after the dedup decision, before
/// building the `Envelope`.
pub fn next_uid(account_id: u64) -> BichonResult<u32> {
    let mut blocks = blocks();

    if let Some((next_free, block_end)) = blocks.get_mut(&account_id) {
        if next_free < block_end {
            let uid = *next_free;
            *next_free += 1;
            return to_u32(uid, account_id);
        }
    }

    // No cached block left: reserve a fresh one (this is the fsync-ing step).
    let (start, end) = reserve_block(account_id)?;
    blocks.insert(account_id, (start + 1, end));
    to_u32(start, account_id)
}

/// Reserve one block for `account_id`, returning `(start, end)` with `end`
/// exclusive. Holds no lock: the caller owns the `BLOCKS` critical section.
fn reserve_block(account_id: u64) -> BichonResult<(u64, u64)> {
    // Optimistic read, only used to decide whether the index has to be probed.
    // A zero counter must NOT be read as "start at 1": it also means "the
    // memdb snapshot is older than the index" (an inconsistent restore), where
    // starting at 1 would reissue numbers that clients have already seen.
    let looks_unset = Account::get(account_id)?.imap_uid_next == 0;
    let floor = if looks_unset {
        max_assigned_uid(account_id)?.saturating_add(1)
    } else {
        0
    };

    // The read-modify-write runs under the memdb lock, so the `max` below sees
    // the current stored value no matter how many racers computed the same
    // `floor`: the second one finds the counter already advanced and continues
    // after it instead of overlapping.
    let mut assigned: Option<(u64, u64)> = None;
    let account_key = account_id.to_string();
    let row = update_atomic_impl::<Account>(DB_MANAGER.db(), &account_key, |mut account| {
        let start = block_start(account.imap_uid_next, floor);
        let end = start.saturating_add(UID_BLOCK).min(UID_MAX + 1);
        account.imap_uid_next = end;
        assigned = Some((start, end));
        account
    })?;
    debug_assert_eq!(row.imap_uid_next, assigned.map(|(_, end)| end).unwrap_or(0));

    let (start, end) = assigned.expect("update closure always runs");
    if start > UID_MAX {
        return Err(uid_space_exhausted(account_id));
    }

    // The range must be durable before any number is handed out. A failed flush
    // means the range is not persisted, so nothing may be issued from it:
    // propagate the error and let the caller retry rather than caching a block
    // that a crash would roll back.
    DB_MANAGER.db().flush().map_err(|e| {
        raise_error!(
            format!("Failed to flush the IMAP UID reservation for account {account_id}: {e}"),
            ErrorCode::InternalError
        )
    })?;

    Ok((start, end))
}

/// First UID of the next block: never below the stored counter, never below
/// `floor` (the index probe, when the counter looks unset), and never 0.
///
/// This is where "the counter is 0" is deliberately *not* read as "start at 1":
/// a zero counter also means "the memdb snapshot is older than the index", and
/// restarting at 1 would reissue numbers clients have already seen.
fn block_start(stored: u64, floor: u64) -> u64 {
    stored.max(floor).max(1)
}

/// Largest `imap_uid` currently stored for `account_id` (0 when it has none).
///
/// Used by the unset-counter guard above, by the startup reconcile, and by the
/// bulk assigners. Reads the `imap_uid` fast field in descending order, so it
/// costs one document regardless of the archive size.
pub fn max_assigned_uid(account_id: u64) -> BichonResult<u64> {
    let fields = SchemaTools::email_fields();
    let reader = ENVELOPE_MANAGER.create_reader()?;
    let searcher = reader.searcher();
    let query = TermQuery::new(
        Term::from_field_u64(fields.f_account_id, account_id),
        IndexRecordOption::Basic,
    );
    let top: Vec<(Option<u64>, DocAddress)> = searcher
        .search(
            &query,
            &TopDocs::with_limit(1).order_by_fast_field::<u64>(F_IMAP_UID, Order::Desc),
        )
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    Ok(top.first().and_then(|(uid, _)| *uid).unwrap_or(0))
}

/// Invalidate every client cache for one account: used after a restore to an
/// earlier backup point (where the counter and the index are mutually
/// consistent but both older than what clients have cached) and when a counter
/// is found behind the index.
pub fn bump_generation(account_id: u64) -> BichonResult<u32> {
    let account_key = account_id.to_string();
    let row = update_atomic_impl::<Account>(DB_MANAGER.db(), &account_key, |mut account| {
        account.imap_uid_generation = account.imap_uid_generation.wrapping_add(1);
        account
    })?;
    DB_MANAGER.db().flush().map_err(|e| {
        raise_error!(
            format!("Failed to flush the UIDVALIDITY generation of account {account_id}: {e}"),
            ErrorCode::InternalError
        )
    })?;
    Ok(row.imap_uid_generation)
}

/// Repair a counter found behind the index (invariant 3), in one atomic write:
/// the counter moves above `observed_max` and the generation is bumped so every
/// client drops its cache.
///
/// The bump is not optional. A counter below the index means the counter was
/// reset at some point, and nothing proves the numbers handed out during that
/// window were not already reissued — clients may hold a mapping that is now
/// wrong. Only a UIDVALIDITY change makes them resync.
///
/// Returns the new generation.
pub fn repair_counter_behind(account_id: u64, observed_max: u64) -> BichonResult<u32> {
    let account_key = account_id.to_string();
    let row = update_atomic_impl::<Account>(DB_MANAGER.db(), &account_key, move |mut account| {
        // Re-check under the lock: a concurrent reservation may already have
        // moved the counter past the observed maximum, and it must not be
        // pulled back (or have its generation bumped needlessly).
        if account.imap_uid_next <= observed_max {
            account.imap_uid_next = observed_max.saturating_add(1);
            account.imap_uid_generation = account.imap_uid_generation.wrapping_add(1);
        }
        account
    })?;
    DB_MANAGER.db().flush().map_err(|e| {
        raise_error!(
            format!("Failed to flush the repaired UID counter of account {account_id}: {e}"),
            ErrorCode::InternalError
        )
    })?;
    Ok(row.imap_uid_generation)
}

/// Bump the UIDVALIDITY generation of every mail-storing account in `db`.
///
/// Restore-time invalidation, and the only thing that can express it: a restore
/// point is *self-consistent* — its counter and its index agree with each
/// other — so nothing inside the restored data reveals that clients are holding
/// UIDs from a later generation. Without this bump a client would keep its
/// cache, and messages that the archive re-creates under the same numbers would
/// silently never be fetched.
///
/// Operates on an explicit database because the restore target is not the
/// configured data directory. Returns how many accounts were bumped.
pub fn bump_all_generations(db: &MemDb) -> BichonResult<usize> {
    let accounts: Vec<Account> = db
        .collection(Account::collection())
        .list_all()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

    let mut bumped = 0usize;
    for account in accounts {
        // Views hold no mail, so they have no UID space to invalidate.
        if !account.stores_mail() {
            continue;
        }
        let account_key = account.id.to_string();
        update_atomic_impl::<Account>(db, &account_key, |mut account| {
            account.imap_uid_generation = account.imap_uid_generation.wrapping_add(1);
            account
        })?;
        bumped += 1;
    }

    db.flush().map(|_| ()).map_err(|e| {
        raise_error!(
            format!("Failed to flush the restored UIDVALIDITY generations: {e}"),
            ErrorCode::InternalError
        )
    })?;
    Ok(bumped)
}

/// UIDVALIDITY served to clients for one mailbox.
///
/// Derived, so nothing has to be stored per mailbox: the per-account counter is
/// monotonic, which means a mailbox's UID space is never reset and its validity
/// never has to change. The generation component is what lets a restore
/// invalidate every client cache at once. Never 0 (RFC 3501).
pub fn uid_validity(account_id: u64, mailbox_id: u64, generation: u32) -> u32 {
    let mixed = account_id
        .wrapping_mul(31)
        .wrapping_add(mailbox_id.wrapping_mul(17))
        .wrapping_add((generation as u64).wrapping_mul(1_000_003)) as u32;
    if mixed == 0 {
        1
    } else {
        mixed
    }
}

/// Streaming per-account UID assigner for bulk rebuilds (reindex, legacy
/// migration, lazy backfill). Keeps counters in memory while documents stream
/// through, then `finish` persists them to the account rows.
#[derive(Default)]
pub struct AccountUidAssigner {
    /// account_id -> next free uid.
    counters: HashMap<u64, u64>,
    /// Accounts whose documents were not uniformly carried or assigned, i.e.
    /// the old index was already inconsistent. Reported, never fatal.
    mixed: Vec<u64>,
    /// account_id -> whether every document so far carried a UID.
    carried: HashMap<u64, bool>,
}

impl AccountUidAssigner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Carry a UID that the document already had, so later fresh assignments
    /// stay above it.
    pub fn carry(&mut self, account_id: u64, uid: u32) {
        let next = self.counters.entry(account_id).or_insert(1);
        *next = (*next).max(uid as u64 + 1);
        match self.carried.get(&account_id) {
            None => {
                self.carried.insert(account_id, true);
            }
            Some(false) if !self.mixed.contains(&account_id) => self.mixed.push(account_id),
            _ => {}
        }
    }

    /// Assign a fresh UID for a document that had none.
    pub fn assign(&mut self, account_id: u64) -> BichonResult<u32> {
        let next = self.counters.entry(account_id).or_insert(1);
        let uid = *next;
        *next += 1;
        if self.carried.insert(account_id, false) == Some(true)
            && !self.mixed.contains(&account_id)
        {
            self.mixed.push(account_id);
        }
        to_u32(uid, account_id)
    }

    /// Accounts that saw both carried and freshly assigned UIDs.
    pub fn mixed_accounts(&self) -> &[u64] {
        &self.mixed
    }

    /// Persist the counters (max assigned/carried + 1 per account) to the
    /// account rows of `db`.
    ///
    /// `db` is explicit rather than `DB_MANAGER` because the bulk callers are
    /// `bichon-admin` commands that operate on a root given on the command line
    /// (see `core::admin::meta::open_database`, the same handle `reset` uses),
    /// which need not be the configured data directory.
    ///
    /// Must be called only once the index build succeeded, and its failure must
    /// fail the caller: an index ahead of the counter is exactly the "behind"
    /// state, even though the startup reconcile repairs it.
    pub fn finish(&mut self, db: &MemDb) -> BichonResult<()> {
        for (account_id, next) in std::mem::take(&mut self.counters) {
            let account_key = account_id.to_string();
            // An index can name an account `db` has no row for (a root whose
            // accounts were never migrated, or a fixture). There is then nowhere
            // to persist the counter — and that is safe: the next reservation
            // finds an unset counter, probes the index and continues above it
            // (see `block_start`). Creating a partial account row here would be
            // far worse. The lookup must go through `db`, not `Account::find`,
            // which reads the *configured* database rather than this one.
            let present = db
                .collection(Account::collection())
                .get::<Account>(&account_key)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
                .is_some();
            if !present {
                tracing::warn!(
                    account_id,
                    "rebuilt IMAP UID counter has no account row to live on; skipping"
                );
                continue;
            }
            update_atomic_impl::<Account>(db, &account_key, move |mut account| {
                // Never regress: a counter that is already ahead of what this
                // rebuild saw must stay ahead.
                account.imap_uid_next = account.imap_uid_next.max(next);
                account
            })?;
        }
        db.flush().map(|_| ()).map_err(|e| {
            raise_error!(
                format!("Failed to flush the rebuilt IMAP UID counters: {e}"),
                ErrorCode::InternalError
            )
        })
    }
}

fn to_u32(uid: u64, account_id: u64) -> BichonResult<u32> {
    u32::try_from(uid).map_err(|_| uid_space_exhausted(account_id))
}

fn uid_space_exhausted(account_id: u64) -> BichonError {
    raise_error!(
        format!(
            "UID space exhausted for account {account_id}: the 32-bit IMAP UID pool is full. \
             Bump the account's UIDVALIDITY generation to start a new generation."
        ),
        ErrorCode::InternalError
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use super::*;
    use crate::backup::BACKUP_STATE_TESTS;
    use crate::database::upsert_impl;
    use crate::settings::{cli::SETTINGS, dir::DATA_DIR_MANAGER};

    /// The data root must be seeded before anything derefs `SETTINGS`, and every
    /// module that does this uses the same pid-derived path so the whole test
    /// binary agrees on one root. Copied from the `retention` / `envelope` tests
    /// for exactly that reason.
    static TEST_ENV: Once = Once::new();

    fn init_test_env() {
        TEST_ENV.call_once(|| {
            let root = crate::test_support::unique_test_root("bichon-core-test");
            std::env::set_var("BICHON_ROOT_DIR", &root);
            std::env::set_var("BICHON_ENCRYPT_PASSWORD", "test-password");
            let _ = &*SETTINGS;
            let _ = &*DATA_DIR_MANAGER;
            let _ = &*DB_MANAGER;
        });
    }

    /// The block cache and the accounts collection are process-wide, so every
    /// test uses its own account id rather than sharing one. The counters are
    /// seeded non-zero wherever the test does not specifically exercise the
    /// unset-counter path, which keeps the envelope index out of it.
    fn seed_account(account_id: u64, imap_uid_next: u64) {
        let account = Account {
            id: account_id,
            imap_uid_next,
            ..Default::default()
        };
        upsert_impl(DB_MANAGER.db(), account).unwrap();
    }

    fn stored_counter(account_id: u64) -> u64 {
        Account::get(account_id).unwrap().imap_uid_next
    }

    /// The regression this whole design exists for: a counter of 0 must not be
    /// read as "start at 1" when the index already holds UIDs, because that
    /// reissues numbers clients have already cached.
    #[test]
    fn t13_a_zero_counter_never_restarts_below_the_index() {
        assert_eq!(block_start(0, 0), 1, "a genuinely fresh account starts at 1");
        assert_eq!(
            block_start(0, 101),
            101,
            "an unset counter with 100 stored UIDs must continue at 101"
        );
        assert_eq!(
            block_start(500, 101),
            500,
            "the stored counter wins once it is ahead"
        );
        assert_eq!(block_start(0, u64::MAX), u64::MAX);
    }

    /// A UID only ever comes from a range that was persisted first, so losing
    /// the in-process cache (a restart) must skip the reserved range instead of
    /// reissuing it.
    #[tokio::test]
    async fn t1_a_dropped_cache_never_reissues() {
        init_test_env();
        // Every write here goes through the backup write gate, so serialize
        // with the tests that pause it (see `backup::BACKUP_STATE_TESTS`).
        let _gate = BACKUP_STATE_TESTS.lock().await;
        let account_id = 9_100_001;
        seed_account(account_id, 0);

        let first = next_uid(account_id).unwrap();
        assert_eq!(first, 1, "a fresh account starts at 1");
        let reserved_end = stored_counter(account_id);
        assert!(
            reserved_end > first as u64,
            "the reservation must run ahead of what was handed out"
        );

        blocks().remove(&account_id);

        let after_restart = next_uid(account_id).unwrap();
        assert!(
            after_restart as u64 >= reserved_end,
            "after losing the cache the next UID ({after_restart}) must be at or above the \
             persisted range end ({reserved_end}); below it would be a reissue"
        );
    }

    /// Concurrent ingest paths must never receive the same number.
    #[tokio::test]
    async fn t3_concurrent_allocation_is_unique() {
        init_test_env();
        // Every write here goes through the backup write gate, so serialize
        // with the tests that pause it (see `backup::BACKUP_STATE_TESTS`).
        let _gate = BACKUP_STATE_TESTS.lock().await;
        let account_id = 9_100_003;
        seed_account(account_id, 1_000);

        let threads = 8;
        let per_thread = 400;
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                std::thread::spawn(move || {
                    (0..per_thread)
                        .map(|_| next_uid(account_id).unwrap())
                        .collect::<Vec<u32>>()
                })
            })
            .collect();

        let mut all: Vec<u32> = handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect();
        let total = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), total, "every allocation must be distinct");
    }

    /// Within one reservation the numbers are contiguous; crossing into the
    /// next one keeps going up. (Holes between blocks are permitted by the
    /// design — reuse is not.)
    #[tokio::test]
    async fn t4_a_block_is_handed_out_in_order() {
        init_test_env();
        // Every write here goes through the backup write gate, so serialize
        // with the tests that pause it (see `backup::BACKUP_STATE_TESTS`).
        let _gate = BACKUP_STATE_TESTS.lock().await;
        let account_id = 9_100_004;
        seed_account(account_id, 700);

        let first = next_uid(account_id).unwrap();
        for expected in (first + 1)..(first + UID_BLOCK as u32) {
            assert_eq!(next_uid(account_id).unwrap(), expected);
        }

        let across_the_boundary = next_uid(account_id).unwrap();
        assert!(
            across_the_boundary >= first + UID_BLOCK as u32,
            "the next block must continue above the previous one"
        );
    }

    /// The pool must report exhaustion rather than wrap around into numbers
    /// clients have already seen.
    #[tokio::test]
    async fn t14_exhaustion_is_an_error_not_a_wraparound() {
        init_test_env();
        // Every write here goes through the backup write gate, so serialize
        // with the tests that pause it (see `backup::BACKUP_STATE_TESTS`).
        let _gate = BACKUP_STATE_TESTS.lock().await;
        let account_id = 9_100_014;
        seed_account(account_id, u32::MAX as u64);

        assert_eq!(next_uid(account_id).unwrap(), u32::MAX);
        assert!(
            next_uid(account_id).is_err(),
            "a full pool must error instead of restarting at 1"
        );
    }

    /// Bumping the generation changes UIDVALIDITY for every mailbox of the
    /// account at once, which is what makes clients drop their cache after a
    /// restore.
    #[tokio::test]
    async fn t12_a_generation_bump_invalidates_every_mailbox() {
        init_test_env();
        // Every write here goes through the backup write gate, so serialize
        // with the tests that pause it (see `backup::BACKUP_STATE_TESTS`).
        let _gate = BACKUP_STATE_TESTS.lock().await;
        let account_id = 9_100_012;
        seed_account(account_id, 0);

        let before = uid_validity(account_id, 42, 0);
        assert_ne!(before, 0, "UIDVALIDITY must never be 0 (RFC 3501)");

        let generation = bump_generation(account_id).unwrap();
        assert_eq!(generation, 1);

        let after = uid_validity(account_id, 42, generation);
        assert_ne!(before, after, "clients must observe a new UIDVALIDITY");
        assert_ne!(
            after,
            uid_validity(account_id, 43, generation),
            "different mailboxes must not collide within one generation"
        );
    }

    /// The account row is rewritten by many paths (folder lists, capabilities,
    /// the account editor). None of them may disturb the counter — that is the
    /// whole reason it lives here rather than on `MailBox`.
    #[tokio::test]
    async fn t16_account_updates_leave_the_counter_alone() {
        init_test_env();
        // Every write here goes through the backup write gate, so serialize
        // with the tests that pause it (see `backup::BACKUP_STATE_TESTS`).
        let _gate = BACKUP_STATE_TESTS.lock().await;
        let account_id = 9_100_016;
        seed_account(account_id, 7_000);

        Account::update_known_folders(
            account_id,
            ["INBOX".to_string()].into_iter().collect(),
        )
        .unwrap();
        Account::update_capabilities(account_id, vec!["IMAP4rev1".to_string()]).unwrap();

        let account = Account::get(account_id).unwrap();
        assert_eq!(account.imap_uid_next, 7_000);
        assert_eq!(account.imap_uid_generation, 0);
        assert!(account.known_folders.is_some(), "the update itself landed");
        assert!(account.capabilities.is_some(), "the update itself landed");
    }

    /// A rebuild carries the UIDs it finds and assigns above them, so `finish`
    /// lands on max+1 — and never below a counter that is already ahead.
    #[tokio::test]
    async fn t7_assigner_persists_max_plus_one_and_never_regresses() {
        init_test_env();
        // Every write here goes through the backup write gate, so serialize
        // with the tests that pause it (see `backup::BACKUP_STATE_TESTS`).
        let _gate = BACKUP_STATE_TESTS.lock().await;
        let account_id = 9_100_007;
        seed_account(account_id, 0);

        let mut uids = AccountUidAssigner::new();
        uids.carry(account_id, 40);
        uids.carry(account_id, 41);
        assert_eq!(uids.assign(account_id).unwrap(), 42);
        uids.finish(DB_MANAGER.db()).unwrap();
        assert_eq!(
            stored_counter(account_id),
            43,
            "the counter must end above every carried and assigned UID"
        );

        // An account whose counter is already ahead keeps it: a rebuild of a
        // stale copy must never pull it back down.
        let ahead_id = 9_100_017;
        seed_account(ahead_id, 5_000);
        let mut uids = AccountUidAssigner::new();
        uids.assign(ahead_id).unwrap();
        uids.finish(DB_MANAGER.db()).unwrap();
        assert_eq!(stored_counter(ahead_id), 5_000);
    }

    /// A restore point is self-consistent, so the restored counter cannot
    /// reveal that clients hold UIDs from a later generation: the restore path
    /// has to invalidate them explicitly. Views hold no mail and are left alone.
    #[test]
    fn t17_restore_invalidation_bumps_only_mail_accounts() {
        use crate::account::migration::AccountType;

        let dir = std::env::temp_dir().join(format!("bichon-uid-bump-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = MemDb::open_with(&dir, bichon_memdb::Durability::Full).unwrap();

        let mail = Account {
            id: 1,
            ..Default::default() // AccountType::IMAP
        };
        let view = Account {
            id: 2,
            account_type: AccountType::View,
            ..Default::default()
        };
        let collection = db.collection(Account::collection());
        collection.upsert("1", &mail).unwrap();
        collection.upsert("2", &view).unwrap();

        assert_eq!(bump_all_generations(&db).unwrap(), 1);

        let generation = |key: &str| {
            db.collection(Account::collection())
                .get::<Account>(key)
                .unwrap()
                .unwrap()
                .imap_uid_generation
        };
        assert_eq!(generation("1"), 1, "the mail account is invalidated");
        assert_eq!(generation("2"), 0, "a view has no UID space to invalidate");

        // What clients are actually served has to change.
        assert_ne!(uid_validity(1, 10, 0), uid_validity(1, 10, 1));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Mixing carried and freshly assigned UIDs within one account means the old
    /// index was already inconsistent; it is reported, not fatal.
    #[test]
    fn t8_mixed_uid_state_is_reported() {
        let mut uids = AccountUidAssigner::new();
        uids.carry(1, 10);
        assert!(uids.mixed_accounts().is_empty());
        assert_eq!(uids.assign(1).unwrap(), 11);
        assert_eq!(uids.mixed_accounts().to_vec(), vec![1u64]);

        let mut clean = AccountUidAssigner::new();
        clean.assign(2).unwrap();
        clean.assign(2).unwrap();
        clean.carry(3, 5);
        clean.carry(3, 6);
        assert!(
            clean.mixed_accounts().is_empty(),
            "all-assigned and all-carried accounts are not mixed"
        );
    }
}
