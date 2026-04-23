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
    {
        account::{
            migration::AccountModel,
            state::{DownloadState, TriggerType},
        },
        error::BichonResult,
    },
    utc_now,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DownloadTask {
    FullFetch,
    TraceFetch,
    Idle,
}

pub async fn decide_next_download_task(account: &AccountModel) -> BichonResult<DownloadTask> {
    Ok(match DownloadState::get(account.id).await? {
        Some(state) => {
            let should_trigger = should_trigger_next_download(
                state.last_trigger_at,
                state.last_finished_at.unwrap_or(0),
                account.download_interval_min.unwrap(),
            );

            if should_trigger {
                DownloadState::start_new_session(account.id, TriggerType::Scheduled).await?;
                DownloadTask::TraceFetch
            } else {
                DownloadTask::Idle
            }
        }
        None => {
            DownloadState::init(account.id).await?;
            DownloadTask::FullFetch
        }
    })
}

fn should_trigger_next_download(
    last_trigger_at: i64,
    last_finished_at: i64,
    sync_interval_min: i64,
) -> bool {
    let now = utc_now!();
    now - last_trigger_at > (sync_interval_min * 60 * 1000) && now - last_finished_at > 60 * 1000
}
