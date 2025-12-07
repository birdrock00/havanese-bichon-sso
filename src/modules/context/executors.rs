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

use crate::modules::account::migration::AccountType;
use crate::modules::context::Initialize;
use crate::modules::error::code::ErrorCode;
use crate::raise_error;
use crate::{
    modules::{
        account::migration::AccountModel,
        context::controller::SYNC_CONTROLLER,
        error::BichonResult,
        imap::{executor::ImapExecutor, pool::build_imap_pool},
    },
    utc_now,
};
use dashmap::DashMap;
use std::sync::{Arc, LazyLock};
use tracing::info;

pub static MAIL_CONTEXT: LazyLock<EmailClientExecutors> = LazyLock::new(EmailClientExecutors::new);

pub struct EmailClientExecutors {
    start_at: i64,
    imap: DashMap<u64, Arc<ImapExecutor>>,
}

impl Initialize for EmailClientExecutors {
    async fn initialize() -> BichonResult<()> {
        MAIL_CONTEXT.start_account_syncers().await
    }
}

impl EmailClientExecutors {
    pub fn new() -> Self {
        Self {
            start_at: utc_now!(),
            imap: DashMap::new(),
        }
    }
    pub fn uptime_ms(&self) -> i64 {
        utc_now!() - self.start_at
    }

    pub async fn imap(&self, account_id: u64) -> BichonResult<Arc<ImapExecutor>> {
        if let Some(executor) = self.imap.get(&account_id) {
            return Ok(executor.value().clone());
        }

        let pool = build_imap_pool(account_id).await?;
        let new_executor = Arc::new(ImapExecutor::new(account_id, pool));

        match self.imap.try_entry(account_id) {
            Some(dashmap::mapref::entry::Entry::Occupied(entry)) => Ok(entry.get().clone()),
            Some(dashmap::mapref::entry::Entry::Vacant(entry)) => {
                entry.insert(new_executor.clone());
                Ok(new_executor)
            }
            None => Err(raise_error!(
                "DashMap locked".into(),
                ErrorCode::InternalError
            )),
        }
    }

    pub async fn clean_account(&self, account_id: u64) -> BichonResult<()> {
        if self.imap.remove(&account_id).is_some() {
            info!(account_id, "Closed IMAP pool for account");
        }
        Ok(())
    }

    pub async fn start_account_syncers(&self) -> BichonResult<()> {
        let accounts = AccountModel::list_all().await?;
        let active_accounts: Vec<AccountModel> = accounts
            .into_iter()
            .filter(|a| a.enabled && matches!(a.account_type, AccountType::IMAP))
            .collect();

        if active_accounts.is_empty() {
            info!("No active accounts found for account initialization.");
            return Ok(());
        }
        info!(
            "System has {} active IMAP accounts to initialize.",
            active_accounts.len()
        );
        for account in active_accounts {
            SYNC_CONTROLLER
                .trigger_start(account.id, account.email)
                .await
        }

        Ok(())
    }
}
