//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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

use crate::{
    modules::{
        account::{migration::AccountModel, state::AccountRunningState},
        error::BichonResult,
    },
    utc_now,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyncType {
    /// Initial sync, used when fetching all messages for the first time.
    InitialSync,
    /// Incremental synchronization, typically used for updates or fetching new data since the last sync.
    IncrementalSync,
    /// Skip synchronization, used when it's not yet time to perform the next sync.
    SkipSync,
}

pub async fn determine_sync_type(account: &AccountModel) -> BichonResult<SyncType> {
    Ok(match AccountRunningState::get(account.id).await? {
        Some(info) => {
            let now = utc_now!();
            let incremental_sync = is_time_for_incremental_sync(
                now,
                info.last_incremental_sync_start,
                account.sync_interval_min.unwrap(),
            );

            if incremental_sync {
                SyncType::IncrementalSync
            } else {
                SyncType::SkipSync
            }
        }
        None => SyncType::InitialSync,
    })
}

/// Check if it's time for an incremental sync based on the provided interval.
fn is_time_for_incremental_sync(
    now: i64,
    last_incremental_sync_at: i64,
    sync_interval_min: i64,
) -> bool {
    now - last_incremental_sync_at > (sync_interval_min * 60 * 1000)
}
