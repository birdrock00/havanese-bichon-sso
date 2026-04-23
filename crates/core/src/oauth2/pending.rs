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
    {
        database::{
            batch_delete_impl, delete_impl, async_find_impl, insert_impl, manager::DB_MANAGER,
        },
        error::{code::ErrorCode, BichonResult},
    },
    raise_error, utc_now,
};
use itertools::Itertools;
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};

const EXPIRATION_DURATION_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[native_model(id = 6, version = 1)]
#[native_db]
pub struct OAuth2PendingEntity {
    /// Unique identifier for the OAuth2 request record
    pub oauth2_id: u64,

    pub account_id: u64,
    /// CSRF protection state parameter used to verify the integrity of the authorization request
    #[primary_key]
    pub state: String,

    /// PKCE code verifier used in the authorization code exchange process to ensure security
    pub code_verifier: String,

    /// Timestamp when the OAuth2 request was created, used to determine request expiration
    pub created_at: i64,
}

impl OAuth2PendingEntity {
    pub fn new(
        oauth2_id: u64,
        account_id: u64,
        state: String,
        code_verifier: String,
    ) -> Self {
        Self {
            oauth2_id,
            account_id,
            state,
            code_verifier,
            created_at: utc_now!(),
        }
    }

    pub async fn save(&self) -> BichonResult<()> {
        insert_impl(DB_MANAGER.meta_db(), self.to_owned()).await
    }

    pub async fn delete(state: &str) -> BichonResult<()> {
        let state = state.to_string();
        delete_impl(DB_MANAGER.meta_db(), move |rw| {
            rw.get().primary::<OAuth2PendingEntity>(state.clone())
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
            .ok_or_else(|| raise_error!(format!(
                "The oauth2 pending entity with state={state} that you want to delete was not found."
            ), ErrorCode::ResourceNotFound))
        }).await
    }

    pub async fn clean() -> BichonResult<()> {
        batch_delete_impl(DB_MANAGER.meta_db(), |rw| {
            let all: Vec<OAuth2PendingEntity> = rw
                .scan()
                .primary()
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
                .all()
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
                .try_collect()
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

            let now = utc_now!();
            let to_delete: Vec<OAuth2PendingEntity> = all
                .into_iter()
                .filter(|e| now - e.created_at > EXPIRATION_DURATION_MS)
                .collect();
            Ok(to_delete)
        })
        .await?;
        Ok(())
    }

    pub async fn get(state: &str) -> BichonResult<Option<OAuth2PendingEntity>> {
        let entity =
            async_find_impl::<OAuth2PendingEntity>(DB_MANAGER.meta_db(), state.to_string())
                .await?;

        match entity {
            Some(entity) => {
                let state = state.to_string();
                if utc_now!() - entity.created_at > EXPIRATION_DURATION_MS {
                    delete_impl(DB_MANAGER.meta_db(), move |rw| {
                        rw.get()
                            .primary::<OAuth2PendingEntity>(state)
                            .map_err(|e| {
                                raise_error!(format!("{:#?}", e), ErrorCode::InternalError)
                            })?
                            .ok_or_else(|| {
                                raise_error!(
                                    "OAuth2 pending entity not found".into(),
                                    ErrorCode::ResourceNotFound
                                )
                            })
                    })
                    .await?;
                    return Ok(None);
                }
                Ok(Some(entity))
            }
            None => Ok(None),
        }
    }
}
