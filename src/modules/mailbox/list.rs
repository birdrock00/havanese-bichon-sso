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


use crate::modules::account::migration::{AccountModel, AccountType};
use crate::modules::cache::imap::mailbox::{Attribute, AttributeEnum, MailBox};
use crate::modules::context::executors::MAIL_CONTEXT;
use crate::modules::error::code::ErrorCode;
use crate::modules::error::{BichonError, BichonResult};
use crate::modules::utils::create_hash;
use crate::raise_error;
use async_imap::types::Name;

pub async fn get_account_mailboxes(account_id: u64, remote: bool) -> BichonResult<Vec<MailBox>> {
    let account = AccountModel::check_account_exists(account_id).await?;
    if remote {
        if matches!(account.account_type, AccountType::IMAP) {
            request_imap_all_mailbox_list(account_id).await
        } else {
            return Err(raise_error!(
                "The 'remote' option can only be used with IMAP accounts.".into(),
                ErrorCode::InvalidParameter
            ));
        }
    } else {
        MailBox::list_all(account_id).await
    }
}

pub async fn request_imap_all_mailbox_list(account_id: u64) -> BichonResult<Vec<MailBox>> {
    let executor = MAIL_CONTEXT.imap(account_id).await?;
    let names = executor.list_all_mailboxes().await?;
    convert_names_to_mailboxes(account_id, names.iter()).await
}

fn contains_no_select(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attr| attr.attr == AttributeEnum::NoSelect)
}

pub async fn convert_names_to_mailboxes(
    account_id: u64,
    names: impl IntoIterator<Item = &Name>,
) -> BichonResult<Vec<MailBox>> {
    // Preallocate enough space in the vector to avoid multiple reallocations
    let mut tasks = Vec::new();

    for name in names.into_iter() {
        // Convert the name into a MailBox structure
        let mailbox_name = name.name().to_string();
        let mut mailbox: MailBox = name.into();

        if contains_no_select(&mailbox.attributes) {
            continue;
        }
        mailbox.account_id = account_id;
        mailbox.id = create_hash(account_id, &mailbox.name);
        let task: tokio::task::JoinHandle<Result<MailBox, BichonError>> =
            tokio::spawn(async move {
                let executor = MAIL_CONTEXT.imap(account_id).await?;
                let mx = executor.examine_mailbox(mailbox_name.as_str()).await?;
                // Update the mailbox status information
                mailbox.exists = mx.exists; // Number of messages in the mailbox
                mailbox.unseen = mx.unseen; // Number of unseen messages
                mailbox.uid_next = mx.uid_next; // Next unique identifier to be assigned
                mailbox.uid_validity = mx.uid_validity; // Validity of the UIDs
                Ok(mailbox)
            });
        tasks.push(task);
    }

    let mut mailboxes = Vec::new();

    for task in tasks {
        match task.await {
            Ok(Ok(mailbox)) => mailboxes.push(mailbox),
            Ok(Err(err)) => return Err(err), // Handle mailbox-level errors
            Err(e) => return Err(raise_error!(format!("{:#?}", e), ErrorCode::InternalError)), // Handle task-level panics or errors
        }
    }

    Ok(mailboxes)
}
