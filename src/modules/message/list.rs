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
    account::migration::AccountModel,
    error::BichonResult,
    rest::response::DataPage,
    store::{envelope::Envelope, tantivy::manager::INDEX_MANAGER},
};

pub async fn get_thread_messages(
    account_id: u64,
    thread_id: &str,
    page: u64,
    page_size: u64,
) -> BichonResult<DataPage<Envelope>> {
    AccountModel::check_account_exists(account_id).await?;
    INDEX_MANAGER
        .list_thread_envelopes(account_id, thread_id, page, page_size, true)
        .await
}
