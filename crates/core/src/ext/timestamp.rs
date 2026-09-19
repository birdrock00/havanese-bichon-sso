// Per-email deletion tombstone extension point (Enterprise timestamp anchoring).
//
// Community edition: no tombstone hooks are registered — deleted email leaves
// are simply not recorded, and the feature is a no-op.
// Enterprise edition: registers a hook that records a deletion tombstone so the
// Merkle tree keeps a leaf for an email that was removed before the watermark
// scan ever saw it (append-only archive).
//
// Hooks run synchronously inside the deletion path. They must log their own
// errors and must not panic — email deletion never fails because of optional
// anchor bookkeeping.
//
// Used in: crates/core/src/store/tantivy/envelope.rs (delete paths)

use std::sync::{LazyLock, RwLock};

/// A single leaf record for the Merkle anchor tree.
#[derive(Debug, Clone)]
pub struct LeafRecord {
    pub envelope_id: String,
    pub content_hash: String,
    pub ingest_at: i64,
}

type TombstoneFn = dyn Fn(Vec<LeafRecord>, i64) + Send + Sync;

static TOMBSTONE_HOOKS: LazyLock<RwLock<Vec<Box<TombstoneFn>>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// Called by the Enterprise edition at startup to register the tombstone handler.
///
/// Hooks are fire-and-forget: they must log their own errors and must not
/// panic. Deletion never fails because of optional anchor bookkeeping.
pub fn register_tombstone_handler(handler: impl Fn(Vec<LeafRecord>, i64) + Send + Sync + 'static) {
    TOMBSTONE_HOOKS.write().unwrap().push(Box::new(handler));
}

/// Invoked by bichon-core whenever envelopes are deleted.
///
/// `records` carries the content hashes of the removed envelopes and
/// `deleted_at` is the deletion time (epoch millis). No-op when no hooks are
/// registered (community build).
pub fn record_deletions(records: Vec<LeafRecord>, deleted_at: i64) {
    if records.is_empty() {
        return;
    }
    let hooks = TOMBSTONE_HOOKS.read().unwrap();
    for hook in hooks.iter() {
        hook(records.clone(), deleted_at);
    }
}
