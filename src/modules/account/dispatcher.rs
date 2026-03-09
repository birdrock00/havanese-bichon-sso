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


use crate::modules::account::state::AccountRunningState;
use std::sync::LazyLock;
use tokio::sync::mpsc;
use tracing::error;

pub static STATUS_DISPATCHER: LazyLock<ErrorDispatcher> = LazyLock::new(ErrorDispatcher::new);

pub struct ErrorDispatcher {
    channel: mpsc::Sender<(u64, String)>,
}

impl ErrorDispatcher {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel::<(u64, String)>(100);

        tokio::spawn(async move {
            while let Some((account_id, error)) = rx.recv().await {
                match AccountRunningState::append_error_message(account_id, error).await {
                    Ok(()) => {}
                    Err(error) => {
                        error!(
                            "Failed to append error for account: {}. Error: {:#?}",
                            &account_id, error
                        );
                    }
                }
            }
        });

        ErrorDispatcher { channel: tx }
    }

    pub async fn append_error(&self, account_id: u64, error: String) {
        if let Err(e) = self.channel.send((account_id, error.clone())).await {
            error!(
                "Failed to dispatch status update for account: {}, Error: {}. Channel error: {:?}",
                &account_id, error, e
            );
        }
    }
}
