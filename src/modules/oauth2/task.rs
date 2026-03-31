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

use crate::modules::{
    common::periodic::{PeriodicTask, TaskHandle},
    context::BichonTask,
    oauth2::pending::OAuth2PendingEntity,
};
use std::time::Duration;

const TASK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

///This task cleans up expired OAuth2 pending authorizations that haven't been completed by users in a timely manner.
pub struct OAuth2CleanTask;

impl BichonTask for OAuth2CleanTask {
    fn start() -> TaskHandle {
        let periodic_task = PeriodicTask::new("oauth2-pending-task-cleaner");

        let task = move |_: Option<u64>| {
            Box::pin(async move {
                OAuth2PendingEntity::clean().await?;
                Ok(())
            })
        };

        periodic_task.start(task, None, TASK_INTERVAL, false, false)
    }
}
