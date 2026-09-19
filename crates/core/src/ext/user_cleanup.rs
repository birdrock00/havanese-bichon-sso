// Per-user data cleanup extension point.
//
// Community edition: no cleanup hooks are registered (user-owned data is
// already removed together with the user record).
// Pro edition: registers hooks that delete edition-specific per-user data
// (e.g. saved analytics views) when a user account is removed.
//
// Hooks run synchronously inside the user-deletion path so no orphaned
// per-user records survive a user removal.
//
// Used in: crates/core/src/users/mod.rs (BichonUserV2::remove)

use std::sync::{LazyLock, RwLock};

type CleanupFn = dyn Fn(u64) + Send + Sync;

static CLEANUP_HOOKS: LazyLock<RwLock<Vec<Box<CleanupFn>>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// Called by Pro/Enterprise at startup to register per-user cleanup logic.
///
/// Hooks are fire-and-forget: they must log their own errors and must not
/// panic. User deletion never fails because of optional cleanup work.
pub fn register_cleanup(hook: impl Fn(u64) + Send + Sync + 'static) {
    CLEANUP_HOOKS.write().unwrap().push(Box::new(hook));
}

/// Invoked by bichon-core whenever a user record is removed.
pub fn run_cleanups(user_id: u64) {
    let hooks = CLEANUP_HOOKS.read().unwrap();
    for hook in hooks.iter() {
        hook(user_id);
    }
}