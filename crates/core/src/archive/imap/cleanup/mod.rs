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

//! Source-mailbox cleanup (community).
//!
//! Moving archived mail out of the source IMAP folders into
//! [`crate::account::migration::REMOTE_CLEANUP_FOLDER`] is reversible and may
//! run on a schedule. Permanently emptying that folder is a separate, explicit
//! API operation and is never scheduled.
//!
//! Safety rules encoded here:
//! * A message is only ever moved after both the local archive (envelope +
//!   blob hash) and the remote copy (UIDVALIDITY + identity metadata) were
//!   proved in the same run.
//! * Any identity/post-condition failure stops the whole run; policy skips
//!   (age, flagged, missing local blob) do not.
//! * `MOVE` (RFC 6851) is the only way a message leaves a source folder.
//!   Bichon never issues `EXPUNGE`, plain or UID-scoped: a server that does
//!   not advertise `MOVE` cannot run cleanup at all, because copying would not
//!   free the source mailbox.
//! * Bichon never deletes from the server. Emptying
//!   `bichon_auto_created_trash` is left to the operator.

pub mod execute;
pub mod jobs;
pub mod plan;
pub mod preview;
pub mod scheduler;
pub mod verify;

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

pub use execute::{execute_move, MoveRequest};
pub use jobs::{
    account_jobs, cancel_job, get_job, start_move_preview, start_move_run, CleanupJob,
    CleanupJobKind, CleanupJobStatus, JOB_TTL_MS,
};
pub use preview::{preview_move, CleanupPreview, CleanupPreviewKind};

/// One cleanup operation per account at a time.
///
/// A plain (non-async) mutex on purpose: the critical section is a single set
/// lookup, and a synchronous [`release`] means the account lock is already free
/// when the operation returns — an executor that follows a finished preview can
/// never lose a race against the release.
static CLEANUP_ACCOUNTS: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// Policy skips (too young, flagged) are expected and do not stop a run.
pub const SKIP_FLAGGED: &str = "flagged";
pub const SKIP_LOCAL_BLOB: &str = "local_blob_missing";
pub const SKIP_LOCAL_HASH: &str = "local_hash_mismatch";
/// The source UID no longer exists on the server (deleted or moved by another
/// client since Bichon indexed it). Nothing to clean up; skip it.
pub const SKIP_SOURCE_UID_MISSING: &str = "source_uid_missing";

/// How long to pause between two single-message moves. Deliberately slow.
pub const MESSAGE_DELAY: Duration = Duration::from_millis(300);
/// Messages per re-validation batch.
pub const BATCH_SIZE: u64 = 10;
/// Pause between batches, after re-selecting and re-checking UIDVALIDITY.
pub const BATCH_DELAY: Duration = Duration::from_secs(5);
/// Consecutive local-integrity failures tolerated before the run stops.
pub const MAX_LOCAL_INTEGRITY_FAILURES: u32 = 3;
/// Preview tokens are short-lived and bound to the candidate set.
pub const PREVIEW_TTL_MS: i64 = 15 * 60 * 1000;
pub const DAY_MS: i64 = 24 * 60 * 60 * 1000;

pub async fn try_acquire(account_id: u64) -> bool {
    let mut guard = CLEANUP_ACCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if guard.contains(&account_id) {
        false
    } else {
        guard.insert(account_id);
        true
    }
}

/// Releases the account's cleanup lock. Synchronous so callers can drop it
/// from a scope guard (including a non-async `Drop` path) without spawning.
pub fn release(account_id: u64) {
    CLEANUP_ACCOUNTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&account_id);
}

/// IMAP SEARCH expression for an exact Message-ID value.
pub(crate) fn header_search(message_id: &str) -> String {
    let escaped = message_id.replace('\\', "\\\\").replace('"', "\\\"");
    format!("HEADER Message-ID \"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::header_search;

    #[test]
    fn header_search_escapes_quotes_and_backslashes() {
        assert_eq!(
            header_search("<a\"b\\c@example.com>"),
            "HEADER Message-ID \"<a\\\"b\\\\c@example.com>\""
        );
    }
}
