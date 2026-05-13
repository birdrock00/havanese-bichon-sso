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

use crate::{
    database::{
        batch_delete_impl, delete_impl, find_impl, insert_impl, list_all_impl, manager::DB_MANAGER,
        MemDbModel,
    },
    error::BichonResult,
    utc_now,
};
use serde::{Deserialize, Serialize};

const EXPIRATION_DURATION_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OAuth2PendingEntity {
    /// Unique identifier for the OAuth2 request record
    pub oauth2_id: u64,

    pub account_id: u64,
    /// CSRF protection state parameter used to verify the integrity of the authorization request
    pub state: String,

    /// PKCE code verifier used in the authorization code exchange process to ensure security
    pub code_verifier: String,

    /// Timestamp when the OAuth2 request was created, used to determine request expiration
    pub created_at: i64,
}

impl MemDbModel for OAuth2PendingEntity {
    fn collection() -> &'static str {
        "oauth2_pending"
    }
    fn key(&self) -> String {
        self.state.clone()
    }
}

impl OAuth2PendingEntity {
    pub fn new(oauth2_id: u64, account_id: u64, state: String, code_verifier: String) -> Self {
        Self {
            oauth2_id,
            account_id,
            state,
            code_verifier,
            created_at: utc_now!(),
        }
    }

    pub fn save(&self) -> BichonResult<()> {
        insert_impl(DB_MANAGER.db(), self.to_owned())
    }

    pub fn delete(state: &str) -> BichonResult<()> {
        delete_impl::<OAuth2PendingEntity>(DB_MANAGER.db(), state)
    }

    pub fn clean() -> BichonResult<()> {
        let all = list_all_impl::<OAuth2PendingEntity>(DB_MANAGER.db())?;
        let now = utc_now!();
        let to_delete: Vec<String> = all
            .into_iter()
            .filter(|e| now - e.created_at > EXPIRATION_DURATION_MS)
            .map(|e| e.state)
            .collect();
        if !to_delete.is_empty() {
            batch_delete_impl::<OAuth2PendingEntity>(DB_MANAGER.db(), to_delete)?;
        }
        Ok(())
    }

    pub fn get(state: &str) -> BichonResult<Option<OAuth2PendingEntity>> {
        let entity = find_impl::<OAuth2PendingEntity>(DB_MANAGER.db(), state)?;

        match entity {
            Some(entity) => {
                if utc_now!() - entity.created_at > EXPIRATION_DURATION_MS {
                    delete_impl::<OAuth2PendingEntity>(DB_MANAGER.db(), state)?;
                    return Ok(None);
                }
                Ok(Some(entity))
            }
            None => Ok(None),
        }
    }
}
