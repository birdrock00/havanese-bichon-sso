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

//! Scheduled backups.
//!
//! A [`PeriodicTask`] ticks every 60s and fires a run when the cron
//! expression in the configured schedule (`backup.schedule`) matches. The same "next occurrence
//! after the last trigger, and it is already in the past" test used by the
//! IMAP download scheduler ([`crate::archive::imap::download`]) drives the
//! decision, so a slot missed while the process was busy (e.g. during a long
//! backup window) is caught up on once the window closes.
//!
//! `LAST_TRIGGER_AT` starts at process start rather than epoch, so a server
//! that comes up *after* today's slot does not fire a backup immediately on
//! the first tick; slots that already passed while the process was down are
//! not backfilled.

use std::str::FromStr;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use chrono::{TimeZone, Utc};
use cron::Schedule;

use crate::backup::config;
use crate::backup::gate::WRITE_GATE;
use crate::backup::manager::{self, BackupTrigger};
use crate::common::periodic::PeriodicTask;
use crate::error::BichonResult;
use crate::utc_now;
use tracing::{info, warn};

const SCHEDULE_TICK: Duration = Duration::from_secs(60);

/// UTC millis of the last scheduled instant we fired for. See module docs for
/// the process-start initialization rationale.
static LAST_TRIGGER_AT: AtomicI64 = AtomicI64::new(0);

/// Start the scheduled backup task (a no-op when backups are disabled — the
/// tick checks the setting and simply does nothing). The returned handle is
/// dropped intentionally: the loop self-terminates on the shared shutdown
/// signal, matching the retention scheduler.
pub fn start_scheduler() {
    LAST_TRIGGER_AT.store(utc_now!(), Ordering::Release);
    info!("backup: scheduler started ({}s tick)", SCHEDULE_TICK.as_secs());
    let task = PeriodicTask::new("backup-scheduler");
    task.start(
        |_param: Option<u64>| Box::pin(async move { scheduler_tick().await }),
        None,
        SCHEDULE_TICK,
        false,
        false,
    );
}

async fn scheduler_tick() -> BichonResult<()> {
    if !config::enabled() {
        return Ok(());
    }
    // A run is already underway (gate paused) — leave the slot for the next
    // tick after the window closes, which the catch-up logic will pick up.
    if WRITE_GATE.is_paused() {
        return Ok(());
    }
    let schedule_str = config::schedule();
    let last = LAST_TRIGGER_AT.load(Ordering::Acquire);
    if should_trigger_schedule(&schedule_str, last) {
        LAST_TRIGGER_AT.store(utc_now!(), Ordering::Release);
        if let Err(e) = manager::request_run(BackupTrigger::Schedule) {
            warn!("backup: scheduled run rejected: {e:?}");
        }
    }
    Ok(())
}

/// True when a scheduled instant falls strictly between `last_trigger_at`
/// (UTC millis) and now — i.e. the slot came due and has not been fired for.
fn should_trigger_schedule(schedule_str: &str, last_trigger_at: i64) -> bool {
    let schedule = match Schedule::from_str(schedule_str) {
        Ok(s) => s,
        Err(e) => {
            warn!(
                "backup: invalid cron expression '{schedule_str}', no scheduled backups: {e}"
            );
            return false;
        }
    };
    let last_utc = match Utc.timestamp_millis_opt(last_trigger_at) {
        chrono::LocalResult::Single(dt) => dt,
        _ => {
            warn!("backup: invalid last_trigger_at timestamp: {last_trigger_at}");
            return false;
        }
    };
    let now = Utc::now();
    schedule.after(&last_utc).next().map_or(false, |next| next <= now)
}

/// Next occurrence of the cron schedule after now, as an RFC 3339 string.
/// `None` when the expression is invalid. Used by the status API / UI to show
/// when the next backup is due.
pub fn next_run(schedule_str: &str) -> Option<String> {
    let schedule = Schedule::from_str(schedule_str).ok()?;
    schedule.after(&Utc::now()).next().map(|dt| dt.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn cron_every_minute_triggers_after_a_minute_passes() {
        // "0 * * * * *" = every minute at second 0. last trigger 90s ago → the
        // minute boundary has passed → trigger.
        let now = Utc::now();
        let last = now.timestamp_millis() - 90_000;
        assert!(should_trigger_schedule("0 * * * * *", last));
    }

    #[test]
    fn cron_daily_triggers_when_the_slot_was_missed_while_running() {
        // Last trigger 25h ago → today's slot has already passed → trigger
        // (catch-up semantics, matching the IMAP scheduler).
        let now = Utc::now();
        let last = now.timestamp_millis() - 25 * 60 * 60 * 1000;
        assert!(should_trigger_schedule("0 0 3 * * *", last));
    }

    #[test]
    fn cron_does_not_trigger_twice_for_the_same_slot() {
        // Last trigger *after* the slot (e.g. set to "now" when the run
        // started) → the next occurrence is in the future → no trigger.
        let now = Utc::now();
        assert!(!should_trigger_schedule("0 0 3 * * *", now.timestamp_millis()));
    }

    #[test]
    fn cron_future_slot_does_not_trigger() {
        // A slot that is 2h in the future (whatever the clock says) must not
        // trigger when the last trigger was "now".
        let now = Utc::now();
        let future = now + chrono::Duration::hours(2);
        let spec = format!("{} {} * * *", future.minute(), future.hour());
        assert!(!should_trigger_schedule(&spec, now.timestamp_millis()));
    }

    #[test]
    fn cron_invalid_expression_never_triggers() {
        assert!(!should_trigger_schedule("not a cron", utc_now!()));
        assert!(!should_trigger_schedule("", utc_now!()));
    }
}
