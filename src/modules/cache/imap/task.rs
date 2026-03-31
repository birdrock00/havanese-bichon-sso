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

use crate::modules::account::entity::AuthType;
use crate::modules::cache::imap::sync::execute_imap_sync;
use crate::modules::common::periodic::{PeriodicTask, TaskHandle};
use crate::modules::oauth2::token::OAuth2AccessToken;
use crate::modules::{
    account::{dispatcher::STATUS_DISPATCHER, migration::AccountModel},
    error::BichonResult,
};
use crate::utc_now;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::{sync::LazyLock, time::Duration};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

static _DESCRIPTION: &str = "This task periodically synchronizes mailbox data for a specified account, ensuring that all local data is up-to-date.";
const TASK_INTERVAL: Duration = Duration::from_secs(10);
pub static SYNC_TASKS: LazyLock<AccountSyncTask> = LazyLock::new(AccountSyncTask::new);
static LAST_WARN_TIME: AtomicI64 = AtomicI64::new(0);
const WARN_INTERVAL_MS: i64 = 600_000;

pub struct AccountSyncTask {
    tasks: Mutex<Option<HashMap<u64, TaskHandle>>>,
}

impl AccountSyncTask {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(Some(HashMap::new())),
        }
    }

    pub async fn start_account_sync_task(&self, account_id: u64, email: String) {
        let task_name = format!("account-sync-task-{}-{}", account_id, &email);
        let periodic_task = PeriodicTask::new(&task_name);
        let task = move |param: Option<u64>| {
            let account_id = param.unwrap();
            Box::pin(async move {
                let account = AccountModel::async_get(account_id).await.ok();
                match account {
                    Some(account) => {
                        if !account.enabled {
                            let last = LAST_WARN_TIME.load(Ordering::Relaxed);
                            let now = utc_now!();
                            if now - last >= WARN_INTERVAL_MS {
                                LAST_WARN_TIME.store(now, Ordering::Relaxed);
                                warn!(
                                    "Account {}: Sync aborted. Account is currently disabled.",
                                    account_id
                                );
                            }
                        } else {
                            if let Some(imap) = &account.imap {
                                if let AuthType::OAuth2 = imap.auth.auth_type {
                                    if OAuth2AccessToken::get(account.id).await?.is_none() {
                                        if utc_now!() % 300_000 == 0 {
                                            warn!("Account {}: Sync aborted. OAuth2 authorization not completed. Please visit the rustmailer admin page to authorize this account.", account_id);
                                        }
                                        return Ok(());
                                    }
                                }
                            }
                            if let Err(e) = execute_imap_sync(&account).await {
                                STATUS_DISPATCHER
                                    .append_error(
                                        account_id,
                                        format!("error in account sync task: {:#?}", e),
                                    )
                                    .await;
                                error!(
                                    "Failed to synchronize mailbox data for '{}': {:?}",
                                    account_id, e
                                )
                            }
                        }
                    }
                    None => {
                        error!(
                            "Account {}: Sync aborted. Account entity not found.",
                            account_id
                        );
                    }
                }
                Ok(())
            })
        };
        let handler = periodic_task.start(task, Some(account_id), TASK_INTERVAL, true, true);
        self.add_task(account_id, handler).await;
    }

    pub async fn add_task(&self, account_id: u64, handler: TaskHandle) {
        let mut guard = self.tasks.lock().await;
        if let Some(map) = guard.as_mut() {
            map.insert(account_id, handler);
        } else {
            tracing::error!("Failed to add task: HashMap has been taken during shutdown.");
        }
    }

    pub async fn stop(&self, account_id: u64) -> BichonResult<()> {
        let mut guard = self.tasks.lock().await;
        if let Some(map) = guard.as_mut() {
            if let Some(handler) = map.remove(&account_id) {
                drop(guard);
                handler.cancel().await;
            } else {
                warn!("No sync task found for account: {}", account_id);
            }
        } else {
            warn!(
                "Stop called after global shutdown for account: {}",
                account_id
            );
        }

        Ok(())
    }

    pub async fn shutdown(&self) {
        let mut guard = self.tasks.lock().await;
        if let Some(map) = guard.take() {
            drop(guard);
            for (account_id, handler) in map {
                info!("Shutdown: Waiting for account {} to sync...", account_id);
                handler.stop().await;
            }
            info!("Shutdown: All sync tasks stopped successfully.");
        } else {
            warn!("Shutdown: Sync tasks were already shut down or never initialized.");
        }
    }
}
