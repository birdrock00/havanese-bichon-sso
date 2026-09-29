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

//! Global write pause for backup windows.
//!
//! The backup subsystem takes a fully quiescent snapshot: for the duration of
//! the backup window **no** content write may land on disk, so every storage
//! layer can be flushed and copied in a consistent state.
//!
//! `WRITE_GATE` is the single process-wide primitive that makes that possible.
//! Every content-write path must either
//!
//! * [`WriteGate::acquire`] a [`WriteGuard`] that lives for the whole write
//!   (async paths — envelope extraction, tantivy deletes/tag updates), or
//! * call [`WriteGate::check`] and hold the returned [`WriteGuard`] for the
//!   whole write (synchronous helpers — the memdb collection write helpers).
//!
//! When a backup starts, the [`crate::backup::manager`](super::manager)
//! sets the gate to *paused*, waits for every in-flight guard to drop
//! ([`WriteGate::drain`]), and finally resumes the gate when the window ends —
//! unconditionally, even on failure.

use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::raise_error;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::LazyLock;
use std::time::Duration;

/// How long a writer waits for the backup window to end before giving up.
/// Used by the non-configurable call sites (extractor, tantivy delete paths).
pub const BACKUP_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);

/// How often a blocked acquirer/drainer re-checks the gate state on its own.
/// Wakeups on `notify` can be stolen between competing waiters (a guard drop
/// wakes exactly one of them), so every waiter also self-heals on this tick.
const RECHECK_INTERVAL: Duration = Duration::from_millis(100);

pub static WRITE_GATE: LazyLock<WriteGate> = LazyLock::new(WriteGate::new);

/// The process-wide pause/drain/resume gate described in the module docs.
pub struct WriteGate {
    paused: AtomicBool,
    in_flight: AtomicUsize,
    notify: tokio::sync::Notify,
}

/// RAII guard returned by [`WriteGate::acquire`]. Holds the write counted as
/// in-flight until it is dropped; dropping it is what lets [`WriteGate::drain`]
/// make progress.
#[must_use]
pub struct WriteGuard<'a>(&'a WriteGate);

impl std::fmt::Debug for WriteGuard<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriteGuard").finish_non_exhaustive()
    }
}

impl Drop for WriteGuard<'_> {
    fn drop(&mut self) {
        self.0.in_flight.fetch_sub(1, Ordering::AcqRel);
        // Wake every waiter registered on `notify` — a drain waiting for this
        // count to reach zero, and any acquirer queued behind a paused gate.
        // Waiters not polled yet self-heal via `RECHECK_INTERVAL`.
        self.0.notify.notify_waiters();
    }
}

impl WriteGate {
    pub fn new() -> Self {
        Self {
            paused: AtomicBool::new(false),
            in_flight: AtomicUsize::new(0),
            notify: tokio::sync::Notify::new(),
        }
    }

    /// Acquire the gate for a write. Returns immediately when not paused;
    /// otherwise blocks until the backup window ends or `timeout` elapses.
    pub async fn acquire(&self, timeout: Duration) -> BichonResult<WriteGuard<'_>> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if self.try_admit() {
                return Ok(WriteGuard(self));
            }
            let now = tokio::time::Instant::now();
            if now >= deadline {
                return Err(raise_error!(
                    "server is currently backing up, writes are temporarily paused".to_string(),
                    ErrorCode::TooManyRequest
                ));
            }
            // Wake on resume() (or a guard drop), but re-check on our own
            // every `RECHECK_INTERVAL` in case the wakeup was delivered to
            // another waiter competing on the same `Notify`.
            let slice = (deadline - now).min(RECHECK_INTERVAL);
            let _ = tokio::time::timeout(slice, self.notify.notified()).await;
        }
    }

    /// Atomically join the in-flight write set, unless the gate is paused.
    ///
    /// The check-then-act must be atomic with respect to [`WriteGate::pause`]:
    /// incrementing *first* means a concurrent [`WriteGate::drain`] either
    /// observes this write (and keeps waiting) or the re-check below sees
    /// `paused` and backs out. Checking first would let a write slip into
    /// the capture window after drain already observed zero in-flight writes.
    fn try_admit(&self) -> bool {
        self.in_flight.fetch_add(1, Ordering::AcqRel);
        if !self.paused.load(Ordering::Acquire) {
            return true;
        }
        self.in_flight.fetch_sub(1, Ordering::AcqRel);
        // A drainer may be waiting on this count reaching zero.
        self.notify.notify_waiters();
        false
    }

    /// Synchronous fast-fail for callers that cannot await (the memdb write
    /// helpers). Returns a guard that holds the write counted as in-flight
    /// for the duration of the write; returns an error immediately while a
    /// backup window is open.
    pub fn check(&self) -> BichonResult<WriteGuard<'_>> {
        if self.try_admit() {
            Ok(WriteGuard(self))
        } else {
            Err(raise_error!(
                "server is currently backing up, writes are temporarily paused".to_string(),
                ErrorCode::TooManyRequest
            ))
        }
    }

    /// True while a backup window is open. Long-lived background tasks (dedup,
    /// retention, IMAP sync) use this to skip a whole round instead of failing
    /// midway through it.
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    /// Re-open the gate for writes and wake every blocked acquirer.
    pub fn resume(&self) {
        self.paused.store(false, Ordering::Release);
        self.notify.notify_waiters();
    }

    /// Block until every in-flight write has completed. Must be called after
    /// [`WriteGate::pause`]; new acquirers queue on `notify` and never join
    /// `in_flight` while paused.
    pub async fn drain(&self) {
        while self.in_flight.load(Ordering::Acquire) != 0 {
            // A guard drop wakes one round of waiters on `notify`; a blocked
            // acquirer may win it instead of this drain. Re-check `in_flight`
            // on a timer so drain can never park forever on a stolen wakeup.
            let _ = tokio::time::timeout(RECHECK_INTERVAL, self.notify.notified()).await;
        }
    }

    /// Number of in-flight writes, for status reporting.
    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Acquire)
    }

    /// Close the gate for new writes. `pub(crate)`: only the backup manager
    /// opens a window; everyone else observes it via `acquire`/`check`.
    pub(crate) fn pause(&self) {
        self.paused.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Barrier;

    fn err_is_busy(r: &crate::error::BichonError) -> bool {
        matches!(
            r,
            crate::error::BichonError::Generic { code: ErrorCode::TooManyRequest, .. }
        )
    }

    // Tests use fresh local gates rather than the process-wide WRITE_GATE so
    // they cannot interfere with each other (or with the running server) when
    // executed in parallel.

    #[test]
    fn check_passes_when_open_and_fails_when_paused() {
        let gate = WriteGate::new();
        assert!(gate.check().is_ok());
        gate.pause();
        let e = gate.check().unwrap_err();
        assert!(err_is_busy(&e), "expected busy error, got {e:?}");
        // A failed admission must not leave a phantom in-flight count behind.
        assert_eq!(gate.in_flight(), 0);
        gate.resume();
        let _g = gate.check().unwrap();
        assert_eq!(gate.in_flight(), 1);
        drop(_g);
        assert_eq!(gate.in_flight(), 0);
    }

    #[test]
    fn try_admit_backs_out_when_pause_lands_after_increment() {
        let gate = WriteGate::new();
        // Simulate the interleaving: admission counted, then the gate closed
        // before the re-check. try_admit must back out and leave the count
        // at zero — otherwise drain would wait on a write that never runs.
        gate.in_flight.fetch_add(1, Ordering::AcqRel);
        gate.pause();
        assert!(!gate.try_admit());
        assert_eq!(gate.in_flight(), 1, "failed admission must undo its increment");
        gate.in_flight.fetch_sub(1, Ordering::AcqRel);
        assert_eq!(gate.in_flight(), 0);
    }

    #[tokio::test]
    async fn acquire_counts_in_flight_and_guard_drops_release_it() {
        let gate = WriteGate::new();
        let g = gate.acquire(Duration::from_millis(50)).await.unwrap();
        assert_eq!(gate.in_flight(), 1);
        drop(g);
        assert_eq!(gate.in_flight(), 0);
    }

    #[tokio::test]
    async fn acquire_blocks_while_paused_and_proceeds_after_resume() {
        let gate = Arc::new(WriteGate::new());
        gate.pause();
        assert!(gate.is_paused());

        let barrier = Arc::new(Barrier::new(2));
        let gate2 = Arc::clone(&gate);
        let b2 = Arc::clone(&barrier);
        let t = tokio::spawn(async move {
            barrier_wait(&b2).await;
            let _g = gate2
                .acquire(Duration::from_secs(5))
                .await
                .expect("acquire should succeed after resume");
            // Holding the guard counts as in-flight.
            assert_eq!(gate2.in_flight(), 1);
        });

        barrier_wait(&barrier).await;
        assert_eq!(gate.in_flight(), 0, "blocked acquirer must not count yet");
        gate.resume();
        t.await.unwrap();
        assert_eq!(gate.in_flight(), 0);
    }

    #[tokio::test]
    async fn acquire_times_out_when_paused_longer_than_timeout() {
        let gate = WriteGate::new();
        gate.pause();
        let e = gate.acquire(Duration::from_millis(50)).await.unwrap_err();
        assert!(err_is_busy(&e), "expected busy error, got {e:?}");
        gate.resume();
    }

    #[tokio::test]
    async fn drain_waits_for_in_flight_guards() {
        let gate = Arc::new(WriteGate::new());
        let g = gate.acquire(Duration::from_millis(50)).await.unwrap();
        gate.pause();

        let gate2 = Arc::clone(&gate);
        let task = tokio::spawn(async move { gate2.drain().await });
        // Give drain a moment to observe in_flight != 0.
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        drop(g); // releases the guard → drain should unblock
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect("drain should return once the in-flight guard is dropped")
            .expect("drain task panicked");
        assert_eq!(gate.in_flight(), 0);
        gate.resume();
    }

    async fn barrier_wait(b: &Barrier) {
        b.wait().await;
    }

    // Regression: a blocked acquire waiter competes with drain for the
    // wakeup of the last guard drop. Drain must still complete (via its
    // re-check timer) instead of parking forever on the stolen wakeup.
    #[tokio::test]
    async fn drain_completes_when_acquire_waiter_steals_the_wakeup() {
        let gate = Arc::new(WriteGate::new());
        let g = gate.acquire(Duration::from_millis(50)).await.unwrap();
        gate.pause();

        // A writer queues on `notify` before drain does; it signals success
        // by taking the barrier after resume.
        let barrier = Arc::new(Barrier::new(2));
        let gate2 = Arc::clone(&gate);
        let b2 = Arc::clone(&barrier);
        let writer = tokio::spawn(async move {
            let _g = gate2
                .acquire(Duration::from_secs(5))
                .await
                .expect("writer should acquire after resume");
            barrier_wait(&b2).await;
        });
        tokio::task::yield_now().await;

        let gate3 = Arc::clone(&gate);
        let drainer = tokio::spawn(async move { gate3.drain().await });
        tokio::task::yield_now().await;

        drop(g); // its wakeup may be delivered to either waiter…
        // …but drain must finish regardless, well within its re-check window.
        tokio::time::timeout(Duration::from_secs(2), drainer)
            .await
            .expect("drain should not park on a stolen wakeup")
            .expect("drain task panicked");
        assert_eq!(gate.in_flight(), 0);

        // The blocked writer proceeds only after resume().
        gate.resume();
        barrier_wait(&barrier).await;
        writer.await.unwrap();
    }
}
