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
use crate::{
    modules::{
        account::migration::AccountModel, context::controller::SYNC_CONTROLLER, error::BichonResult,
    },
    utc_now,
};
use std::sync::LazyLock;
use tracing::info;

pub static BICHON_CONTEXT: LazyLock<BichonContext> = LazyLock::new(BichonContext::new);

pub struct BichonContext {
    start_at: i64,
}

impl Initialize for BichonContext {
    async fn initialize() -> BichonResult<()> {
        BICHON_CONTEXT.start_account_syncers().await
    }
}

impl BichonContext {
    pub fn new() -> Self {
        Self {
            start_at: utc_now!(),
        }
    }
    pub fn uptime_ms(&self) -> i64 {
        utc_now!() - self.start_at
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
