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
        account::migration::AccountModel,
        error::{code::ErrorCode, BichonResult},
        indexer::{envelope::Envelope, manager::ENVELOPE_INDEX_MANAGER},
        rest::response::DataPage,
    },
    raise_error,
};

pub async fn list_messages_impl(
    account_id: u64,
    mailbox_id: u64,
    page: u64,
    page_size: u64,
) -> BichonResult<DataPage<Envelope>> {
    AccountModel::check_account_exists(account_id).await?;
    validate_pagination_params(page, page_size)?;
    ENVELOPE_INDEX_MANAGER
        .list_mailbox_envelopes(account_id, mailbox_id, page, page_size, true)
        .await
}

fn validate_pagination_params(page: u64, page_size: u64) -> BichonResult<()> {
    if page == 0 || page_size == 0 {
        return Err(raise_error!(
            "Both page and page_size must be greater than 0.".into(),
            ErrorCode::InvalidParameter
        ));
    }
    if page_size > 500 {
        return Err(raise_error!(
            "The page_size exceeds the maximum allowed limit of 500.".into(),
            ErrorCode::InvalidParameter
        ));
    }
    Ok(())
}

pub async fn get_thread_messages(
    account_id: u64,
    thread_id: u64,
    page: u64,
    page_size: u64,
) -> BichonResult<DataPage<Envelope>> {
    AccountModel::check_account_exists(account_id).await?;
    ENVELOPE_INDEX_MANAGER
        .list_thread_envelopes(account_id, thread_id, page, page_size, true)
        .await
}
