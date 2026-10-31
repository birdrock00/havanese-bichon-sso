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

//! Scheduled source-mailbox cleanup.
//!
//! Only the reversible move task is scheduled. Permanent purge lives behind an
//! explicit API call and has no scheduler entry by design.
//!
//! A scheduled run drains the backlog instead of moving one per-run cap and
//! waiting for the next cron slot: `max_messages_per_run` is the batch size,
//! and batches repeat until nothing more can move or the tick's time budget is
//! spent. Each batch is still a complete, safety-checked run (its own re-scan,
//! flag filter, UIDVALIDITY check and account lock), so stopping between
//! batches is always safe — a large backlog simply continues next time.

use std::str::FromStr;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

use chrono::{LocalResult, TimeZone, Utc};
use cron::Schedule;
use tracing::{info, warn};

use crate::account::migration::AccountModel;
use crate::account::state::{RemoteCleanupState, RemoteCleanupStatus};
use crate::archive::imap::cleanup::execute::{execute_move, MoveRequest};
use crate::common::periodic::{PeriodicTask, TaskHandle};
use crate::error::BichonResult;
use crate::utc_now;

const SCHEDULE_TICK: Duration = Duration::from_secs(60);
/// How long one scheduled tick may keep draining before yielding.
///
/// Every candidate costs several IMAP round trips and execution paces itself
/// (300 ms per message, 5 s per ten), so a fixed message count says little
/// about the load. A time budget bounds both the mail server's effort and how
/// long the account's sync slot stays taken; whatever is left continues on the
/// next scheduled run.
const SCHEDULED_DRAIN_BUDGET: Duration = Duration::from_secs(10 * 60);
/// Backstop on the number of batches within one budget: a tiny batch size
/// (e.g. 1) would otherwise spend the whole budget reconnecting and re-scanning.
const MAX_SCHEDULED_BATCHES: u32 = 50;

/// Slots that passed before this process started are not backfilled; the
/// first run waits for the next cron occurrence after startup.
static STARTED_AT: AtomicI64 = AtomicI64::new(0);

pub fn start_scheduler() -> TaskHandle {
    STARTED_AT.store(utc_now!(), Ordering::Release);
    info!(
        "remote-cleanup: scheduler started ({}s tick)",
        SCHEDULE_TICK.as_secs()
    );
    PeriodicTask::new("remote-cleanup-scheduler").start(
        |_param: Option<u64>| Box::pin(async { scheduler_tick().await }),
        None,
        SCHEDULE_TICK,
        false,
        false,
    )
}

async fn scheduler_tick() -> BichonResult<()> {
    // Sequential on purpose: one batch at a time, even across accounts. A
    // draining account therefore holds this tick until its backlog is empty,
    // its batch limit is reached, or it stops making progress.
    for account in AccountModel::list_all()? {
        let Some(config) = account.remote_cleanup.as_ref() else {
            continue;
        };
        if !account.enabled || account.deleting || !config.enabled || account.is_on_hold() {
            continue;
        }
        let Some(schedule) = config.schedule.as_ref() else {
            // No schedule means manual runs only.
            continue;
        };
        let last = RemoteCleanupState::get(account.id)?
            .and_then(|state| state.last_finished_at)
            .unwrap_or_else(|| STARTED_AT.load(Ordering::Acquire));
        if !should_trigger(schedule, last) {
            continue;
        }
        drain_scheduled(&account).await;
    }
    Ok(())
}

/// Run scheduled batches for one account until the backlog is drained.
///
/// A batch that moves nothing means the remaining candidates cannot be moved
/// (flagged, gone from the source, or failing verification), so repeating it
/// would only rewrite the same report. Any stop condition inside the batch
/// ends the drain as well.
async fn drain_scheduled(account: &AccountModel) {
    let started = Instant::now();
    for batch in 0..MAX_SCHEDULED_BATCHES {
        match execute_move(account, MoveRequest::scheduled()).await {
            Ok(run) => {
                info!(
                    account_id = account.id,
                    batch, moved = run.moved, status = ?run.status,
                    "remote-cleanup: scheduled batch finished"
                );
                if run.moved == 0 || run.status != RemoteCleanupStatus::Success {
                    return;
                }
            }
            Err(e) => {
                warn!(
                    "remote-cleanup: scheduled run for account {} failed: {e:?}",
                    account.id
                );
                return;
            }
        }

        // Between batches the account can be disabled, deleted, put on legal
        // hold or have its cleanup switched off; re-read before continuing.
        let still_eligible = AccountModel::get(account.id)
            .map(|current| {
                current.enabled
                    && !current.deleting
                    && !current.is_on_hold()
                    && current
                        .remote_cleanup
                        .as_ref()
                        .is_some_and(|config| config.enabled)
            })
            .unwrap_or(false);
        if !still_eligible {
            return;
        }
        if started.elapsed() >= SCHEDULED_DRAIN_BUDGET {
            info!(
                account_id = account.id,
                batches = batch + 1,
                elapsed_secs = started.elapsed().as_secs(),
                "remote-cleanup: scheduled drain reached its time budget; the next schedule continues"
            );
            return;
        }
    }
    info!(
        account_id = account.id,
        batches = MAX_SCHEDULED_BATCHES,
        "remote-cleanup: scheduled drain hit the per-tick batch limit; the next schedule continues"
    );
}

/// True when a scheduled instant falls strictly between `last` and now.
fn should_trigger(schedule_str: &str, last: i64) -> bool {
    if schedule_str.trim().is_empty() {
        return false;
    }
    let schedule = match Schedule::from_str(schedule_str) {
        Ok(schedule) => schedule,
        Err(e) => {
            warn!("remote-cleanup: invalid cron expression '{schedule_str}': {e}");
            return false;
        }
    };
    let last_utc = match Utc.timestamp_millis_opt(last) {
        LocalResult::Single(dt) => dt,
        _ => return false,
    };
    schedule
        .after(&last_utc)
        .next()
        .map_or(false, |next| next <= Utc::now())
}

#[cfg(test)]
mod tests {
    use super::should_trigger;

    #[test]
    fn a_passed_slot_triggers() {
        let last = chrono::Utc::now().timestamp_millis() - 90_000;
        assert!(should_trigger("0 * * * * *", last));
    }

    #[test]
    fn a_fresh_slot_does_not_trigger_twice() {
        let last = chrono::Utc::now().timestamp_millis();
        assert!(!should_trigger("0 * * * * *", last));
    }

    #[test]
    fn empty_or_invalid_schedule_never_triggers() {
        assert!(!should_trigger("", 0));
        assert!(!should_trigger("not a cron", 0));
    }
}
