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
use crate::modules::error::code::ErrorCode;
use crate::modules::error::BichonResult;
use crate::modules::imap::executor::ImapExecutor;
use crate::modules::imap::session::SessionStream;
use crate::modules::utils::create_hash;
use crate::raise_error;
use async_imap::types::Name;
use async_imap::Session;

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
    let mut session = ImapExecutor::create_connection(account_id).await?;
    let names = ImapExecutor::list_all_mailboxes(&mut session).await?;
    let result = convert_names_to_mailboxes(account_id, &mut session, names.iter()).await?;
    session.logout().await.ok();
    Ok(result)
}

fn contains_no_select(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attr| attr.attr == AttributeEnum::NoSelect)
}

pub async fn convert_names_to_mailboxes(
    account_id: u64,
    session: &mut Session<Box<dyn SessionStream>>,
    names: impl IntoIterator<Item = &Name>,
) -> BichonResult<Vec<MailBox>> {
    let mut mailboxes = Vec::new();

    for name in names {
        let mailbox_name = name.name().to_string();
        let mut mailbox: MailBox = name.into();

        if contains_no_select(&mailbox.attributes) {
            continue;
        }

        mailbox.account_id = account_id;
        mailbox.id = create_hash(account_id, &mailbox.name);
        let mx = session
            .examine(mailbox_name.as_str())
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))?;
        mailbox.exists = mx.exists;
        mailbox.unseen = mx.unseen;
        mailbox.uid_next = mx.uid_next;
        mailbox.uid_validity = mx.uid_validity;

        mailboxes.push(mailbox);
    }

    Ok(mailboxes)
}
