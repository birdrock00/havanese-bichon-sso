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

use crate::account::migration::{AccountModel, AccountType};
use crate::cache::imap::mailbox::{Attribute, AttributeEnum, MailBox};
use crate::cache::imap::mailbox_cache::{self, FetchStatus};
use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::imap::executor::ImapExecutor;
use crate::imap::session::SessionStream;
use crate::raise_error;
use crate::utils::create_hash;
use async_imap::types::Name;
use async_imap::Session;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct MailboxListResponse {
    pub mailboxes: Vec<MailBox>,
    /// "ready" | "fetching" | "error"
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub examined: Option<usize>,
    pub total: Option<usize>,
}

pub async fn get_account_mailboxes(
    account_id: u64,
    remote: bool,
) -> BichonResult<MailboxListResponse> {
    let account = AccountModel::check_account_exists(account_id)?;
    if remote {
        if matches!(account.account_type, AccountType::IMAP) {
            return Ok(remote_mailboxes(account_id).await);
        } else {
            return Err(raise_error!(
                "The 'remote' option can only be used with IMAP accounts.".into(),
                ErrorCode::InvalidParameter
            ));
        }
    } else {
        let mailboxes = MailBox::list_all(account_id)?;
        return Ok(MailboxListResponse {
            mailboxes,
            status: "ready".into(),
            error: None,
            examined: None,
            total: None,
        });
    }
}

fn make_pending_response(status: &FetchStatus, error: Option<String>) -> MailboxListResponse {
    let (examined, total) = match status {
        FetchStatus::Fetching { examined, total } => (Some(*examined), Some(*total)),
        _ => (None, None),
    };
    MailboxListResponse {
        mailboxes: vec![],
        status: match status {
            FetchStatus::Ready => "ready".into(),
            FetchStatus::Fetching { .. } => "fetching".into(),
            FetchStatus::Error(_) => "error".into(),
        },
        error,
        examined,
        total,
    }
}

async fn remote_mailboxes(account_id: u64) -> MailboxListResponse {
    // Cache hit
    if let Some(cached) = mailbox_cache::get(account_id).await {
        return MailboxListResponse {
            mailboxes: cached,
            status: "ready".into(),
            error: None,
            examined: None,
            total: None,
        };
    }

    match mailbox_cache::fetch_status(account_id).await {
        Some(status @ FetchStatus::Fetching { .. }) => {
            return make_pending_response(&status, None);
        }
        Some(FetchStatus::Error(err)) => {
            mailbox_cache::clear_fetch_state(account_id).await;
            return MailboxListResponse {
                mailboxes: vec![],
                status: "error".into(),
                error: Some(err),
                examined: None,
                total: None,
            };
        }
        _ => {}
    }

    // No cache, no fetch in progress — start background fetch
    mailbox_cache::set_fetching(account_id).await;
    spawn_fetch_task(account_id);
    MailboxListResponse {
        mailboxes: vec![],
        status: "fetching".into(),
        error: None,
        examined: Some(0),
        total: Some(0),
    }
}

fn spawn_fetch_task(account_id: u64) {
    tokio::spawn(async move {
        match fetch_remote_with_progress(account_id).await {
            Ok(mailboxes) => {
                mailbox_cache::set(account_id, mailboxes).await;
                mailbox_cache::set_fetch_ready(account_id).await;
            }
            Err(e) => {
                mailbox_cache::set_fetch_error(account_id, format!("{:#?}", e)).await;
            }
        }
    });
}

async fn fetch_remote_with_progress(account_id: u64) -> BichonResult<Vec<MailBox>> {
    let mut session = ImapExecutor::create_connection(account_id).await?;
    let names = ImapExecutor::list_all_mailboxes(&mut session).await?;
    let total = names.len();
    mailbox_cache::update_fetch_progress(account_id, 0, total).await;

    let mut mailboxes = Vec::new();
    for (i, name) in names.iter().enumerate() {
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
        mailbox_cache::update_fetch_progress(account_id, i + 1, total).await;
    }

    session.logout().await.ok();
    Ok(mailboxes)
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
