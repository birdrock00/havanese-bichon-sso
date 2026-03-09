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


use crate::modules::{cache::imap::task::SYNC_TASKS, error::BichonResult};
use std::{sync::LazyLock, time::Duration};
use tokio::sync::mpsc;
use tracing::{error, info};

pub static SYNC_CONTROLLER: LazyLock<SyncController> = LazyLock::new(SyncController::new);

pub struct SyncController {
    channel: mpsc::Sender<(u64, String)>, // Channel to trigger account sync by account ID
}

impl SyncController {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel::<(u64, String)>(100);

        tokio::spawn(async move {
            while let Some((account_id, email)) = rx.recv().await {
                match Self::start_syncer(account_id, email.clone()).await {
                    Ok(Some(_)) => {}
                    Ok(None) => {}
                    Err(err) => {
                        error!(
                            "Failed to prepare and start syncer of account {{{}-{}}}, error: {:#?}",
                            &account_id, &email, err
                        );
                    }
                }
            }
        });

        SyncController { channel: tx }
    }

    /// Trigger synchronization for a specific account
    pub async fn trigger_start(&self, account_id: u64, email: String) {
        if let Err(e) = self.channel.send((account_id, email)).await {
            error!(
                "Failed to trigger synchronization for account={{{}}}, error: {:?}",
                account_id, e
            );
        }
    }

    async fn start_syncer(account_id: u64, email: String) -> BichonResult<Option<()>> {
        info!(
            "Account syncer starting for account: {}-{}.",
            account_id, email
        );
        SYNC_TASKS.start_account_sync_task(account_id, email).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(Some(()))
    }
}
