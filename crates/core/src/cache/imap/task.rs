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

use crate::account::entity::AuthType;
use crate::account::state::DownloadState;
use crate::cache::imap::download::process_imap_download;
use crate::common::periodic::{PeriodicTask, TaskHandle};
use crate::oauth2::token::OAuth2AccessToken;
use crate::{account::migration::AccountModel, error::BichonResult};
use crate::utc_now;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::{sync::LazyLock, time::Duration};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

static _DESCRIPTION: &str = "This task periodically synchronizes mailbox data for a specified account, ensuring that all local data is up-to-date.";
const TASK_INTERVAL: Duration = Duration::from_secs(10);
pub static SYNC_TASKS: LazyLock<AccountSyncTask> = LazyLock::new(AccountSyncTask::new);
static LAST_WARN_TIME: AtomicI64 = AtomicI64::new(0);
const WARN_INTERVAL_MS: i64 = 600_000;

pub struct AccountSyncTask {
    tasks: Mutex<Option<HashMap<u64, (TaskHandle, CancellationToken)>>>,
}

impl AccountSyncTask {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(Some(HashMap::new())),
        }
    }

    pub async fn start_account_download_task(&self, account_id: u64, email: String) {
        let task_name = format!("account-download-task-{}-{}", account_id, &email);
        let periodic_task = PeriodicTask::new(&task_name);

        let cancel_token = CancellationToken::new();
        let task_token = cancel_token.clone();

        let task = move |param: Option<u64>| {
            let account_id = param.unwrap();
            let internal_token = task_token.clone();
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
                                    "Account {}: download aborted. Account is currently disabled.",
                                    account_id
                                );
                            }
                        } else {
                            if let Some(imap) = &account.imap {
                                if let AuthType::OAuth2 = imap.auth.auth_type {
                                    if OAuth2AccessToken::get(account.id).await?.is_none() {
                                        if utc_now!() % 300_000 == 0 {
                                            warn!("Account {}: download aborted. OAuth2 authorization not completed. Please visit the rustmailer admin page to authorize this account.", account_id);
                                        }
                                        return Ok(());
                                    }
                                }
                            }
                            if let Err(e) = process_imap_download(&account, internal_token).await {
                                DownloadState::append_global_error_message(
                                    account.id,
                                    format!("error in account download task: {:#?}", e),
                                )
                                .await?;
                                error!(
                                    "Failed to download mailbox data for '{}': {:?}",
                                    account_id, e
                                )
                            }
                        }
                    }
                    None => {
                        error!(
                            "Account {}: download aborted. Account entity not found.",
                            account_id
                        );
                    }
                }
                Ok(())
            })
        };
        let handler = periodic_task.start(task, Some(account_id), TASK_INTERVAL, true, true);
        self.add_task(account_id, (handler, cancel_token)).await;
    }

    pub async fn add_task(&self, account_id: u64, handler: (TaskHandle, CancellationToken)) {
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
            if let Some((handler, token)) = map.remove(&account_id) {
                drop(guard);
                token.cancel();
                handler.cancel().await;
            }
        }
        Ok(())
    }

    pub async fn shutdown(&self) {
        let mut guard = self.tasks.lock().await;
        if let Some(map) = guard.take() {
            drop(guard);
            for (account_id, (handler, token)) in map {
                info!(
                    "Shutdown: Sending cancel signal to account {}...",
                    account_id
                );
                token.cancel();
                if let Err(_) = tokio::time::timeout(Duration::from_secs(5), handler.stop()).await
                {
                    error!(
                        "Shutdown: Account {} download task forced timeout.",
                        account_id
                    );
                }
            }
            info!("Shutdown: All download tasks processed.");
        }
    }
}
