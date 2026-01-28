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

use crate::modules::error::code::ErrorCode;
use crate::modules::error::{BichonError, BichonResult};
use crate::modules::imap::{manager::ImapConnectionManager, session::SessionStream};
use crate::raise_error;
use async_imap::Session;
use bb8::Pool;
use std::time::Duration;

impl bb8::ManageConnection for ImapConnectionManager {
    type Connection = Session<Box<dyn SessionStream>>;

    type Error = BichonError;

    async fn connect(&self) -> BichonResult<Self::Connection> {
        self.build().await
    }
    // call this function before using the connection
    async fn is_valid(&self, conn: &mut Self::Connection) -> BichonResult<()> {
        conn.noop()
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}

pub async fn build_imap_pool(account_id: u64) -> BichonResult<Pool<ImapConnectionManager>> {
    let manager = ImapConnectionManager::new(account_id);
    let pool = Pool::builder()
        .connection_timeout(Duration::from_secs(60))
        //.idle_timeout(Duration::from_secs(120))
        .retry_connection(true)
        .queue_strategy(bb8::QueueStrategy::Fifo)
        .max_size(10)
        .test_on_check_out(true)
        .build(manager)
        .await?;

    Ok(pool)
}
