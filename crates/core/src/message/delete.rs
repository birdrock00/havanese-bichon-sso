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

use crate::account::migration::AccountModel;
use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::raise_error;
use crate::store::tantivy::attachment::ATTACHMENT_MANAGER;
use crate::store::tantivy::envelope::ENVELOPE_MANAGER;
use std::collections::HashMap;

pub async fn delete_messages_impl(request: HashMap<u64, Vec<String>>) -> BichonResult<()> {
    // Legal hold: an account under a hold is frozen — no message may be
    // deleted from it, including manual single/batch deletes from the web UI
    // (retention and auto-expunge already skip held accounts at their own call
    // sites; this closes the manual-delete path). The check refuses the whole
    // request if any target account is held, so a batch can never partially
    // delete. In the community edition `legal_hold` is always false, so this
    // guard is a no-op there.
    for account_id in request.keys() {
        let account = AccountModel::get(*account_id)?;
        if account.is_on_hold() {
            return Err(raise_error!(
                format!(
                    "Account {} ({}) is under a legal hold; deleting its messages is disabled while the hold is active",
                    account.id, account.email
                ),
                ErrorCode::Forbidden
            ));
        }
    }
    ENVELOPE_MANAGER
        .delete_envelopes_multi_account(request.clone())
        .await?;
    ATTACHMENT_MANAGER
        .delete_attachments_multi_account(request)
        .await
}
