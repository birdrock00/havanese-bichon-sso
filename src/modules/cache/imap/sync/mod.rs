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
        account::{
            dispatcher::STATUS_DISPATCHER,
            migration::{AccountModel, AccountType},
            state::AccountRunningState,
        },
        cache::imap::mailbox::MailBox,
        error::BichonResult,
    },
    utc_now,
};
use flow::reconcile_mailboxes;
use rebuild::{rebuild_cache, rebuild_cache_since_date};
use std::time::Instant;
use sync_folders::get_sync_folders;
use sync_type::{determine_sync_type, SyncType};
use tracing::debug;

pub mod flow;
pub mod rebuild;
pub mod sync_folders;
pub mod sync_type;

pub async fn execute_imap_sync(account: &AccountModel) -> BichonResult<()> {
    assert_eq!(account.account_type, AccountType::IMAP);
    let start_time = Instant::now();
    let account_id = account.id;
    let sync_type = determine_sync_type(account).await?;
    if matches!(sync_type, SyncType::SkipSync) {
        return Ok(());
    }
    let remote_mailboxes = get_sync_folders(account).await?;
    if matches!(sync_type, SyncType::InitialSync) {
        AccountRunningState::add(account.id).await?;
        // AccountRunningState::set_initial_sync_start(account_id).await?;
        let result = match &account.date_since {
            Some(date_since) => {
                rebuild_cache_since_date(account, &remote_mailboxes, date_since).await
            }
            None => rebuild_cache(account, &remote_mailboxes).await,
        };
        match result {
            Ok(_) => {
                AccountRunningState::set_initial_sync_completed(account_id).await?;
            }
            Err(e) => {
                STATUS_DISPATCHER
                    .append_error(
                        account_id,
                        format!("Initial sync failed for the account, error: {:#?}", e),
                    )
                    .await;
                AccountRunningState::set_initial_sync_failed(account_id).await?;
            }
        }
        return Ok(());
    }

    if let Some(state) = AccountRunningState::get(account_id).await? {
        let now = utc_now!();
        const COOLDOWN_MS: i64 = 60 * 1000;
        let mut should_skip = false;
        if let Some(time) = state.initial_sync_end_time {
            if now - time < COOLDOWN_MS {
                should_skip = true;
            }
        }
        if let Some(time) = state.initial_sync_failed_time {
            if now - time < COOLDOWN_MS {
                should_skip = true;
            }
        }
        if should_skip {
            return Ok(());
        }
    }
    AccountRunningState::set_incremental_sync_start(account.id).await?;
    let local_mailboxes = MailBox::list_all(account_id).await?;
    reconcile_mailboxes(account, &remote_mailboxes, &local_mailboxes).await?;
    let elapsed_time = start_time.elapsed().as_secs();
    debug!(
        "Account{{{}}} Incremental sync completed: {} seconds elapsed.",
        account.email, elapsed_time
    );

    if let Some(state) = AccountRunningState::get(account.id).await? {
        if !state.is_initial_sync_completed {
            AccountRunningState::set_initial_sync_completed(account_id).await?;
        }
    }
    AccountRunningState::set_incremental_sync_end(account_id).await?;
    Ok(())
}
