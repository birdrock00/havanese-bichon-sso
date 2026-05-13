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

use crate::{
    utc_now,
    {
        account::{
            migration::AccountModel,
            state::{DownloadState, TriggerType},
        },
        error::BichonResult,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DownloadTask {
    FullFetch,
    TraceFetch,
    Idle,
}

pub async fn decide_next_download_task(
    account: &AccountModel,
    trigger_type: TriggerType,
) -> BichonResult<DownloadTask> {
    let state = match DownloadState::get(account.id)? {
        None => {
            DownloadState::init(account.id).await?;
            return Ok(DownloadTask::FullFetch);
        }
        Some(s) => s,
    };

    let should_start = match trigger_type {
        TriggerType::Manual => true,
        TriggerType::Scheduled => should_trigger_next_download(
            state.last_trigger_at,
            state.last_finished_at.unwrap_or(0),
            account.download_interval_min.unwrap_or(60),
        ),
    };

    if should_start {
        DownloadState::start_new_session(account.id, trigger_type)?;
        Ok(DownloadTask::TraceFetch)
    } else {
        Ok(DownloadTask::Idle)
    }
}

fn should_trigger_next_download(
    last_trigger_at: i64,
    last_finished_at: i64,
    sync_interval_min: i64,
) -> bool {
    let now = utc_now!();
    now - last_trigger_at > (sync_interval_min * 60 * 1000) && now - last_finished_at > 60 * 1000
}
