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
        delete_impl, filter_impl, find_impl, insert_impl, list_all_impl, manager::DB_MANAGER,
        update_impl, upsert_impl, MemDbModel,
    },
    decrypt, encrypt,
    error::{code::ErrorCode, BichonResult},
    oauth2::entity::OAuth2,
    raise_error, utc_now,
};
use serde::{Deserialize, Serialize};

pub const EXTERNAL_OAUTH_APP_ID: u64 = 0;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct OAuth2AccessToken {
    /// The ID of the account associated with this access token.
    pub account_id: u64,
    /// The id of the OAuth2 configuration associated with this access token.
    pub oauth2_id: u64,
    /// The OAuth2 access token used to authenticate requests to the provider.
    pub access_token: Option<String>,
    /// The OAuth2 refresh token used to obtain new access tokens.
    pub refresh_token: Option<String>,
    /// The timestamp when the token record was created, in milliseconds since the Unix epoch.
    pub created_at: i64,
    /// The timestamp when the token record was last updated, in milliseconds since the Unix epoch.
    pub updated_at: i64,
}

impl MemDbModel for OAuth2AccessToken {
    fn collection() -> &'static str {
        "oauth2_tokens"
    }
    fn key(&self) -> String {
        self.account_id.to_string()
    }
}

impl OAuth2AccessToken {
    pub fn create(
        account_id: u64,
        oauth2_id: u64,
        access_token: String,
        refresh_token: String,
    ) -> BichonResult<Self> {
        Ok(Self {
            account_id,
            oauth2_id,
            access_token: Some(encrypt!(&access_token)?),
            refresh_token: Some(encrypt!(&refresh_token)?),
            created_at: utc_now!(),
            updated_at: utc_now!(),
        })
    }

    pub fn upsert_external_oauth_token(
        account_id: u64,
        request: ExternalOAuth2Request,
    ) -> BichonResult<()> {
        let now = utc_now!();
        request.validate()?;

        let current = Self::get(account_id)?;
        match current {
            Some(mut current) => {
                // Update existing record
                if let Some(oauth2_id) = request.oauth2_id {
                    current.oauth2_id = oauth2_id;
                }
                if let Some(access_token) = request.access_token {
                    current.access_token = Some(encrypt!(&access_token)?);
                }
                if let Some(refresh_token) = request.refresh_token {
                    current.refresh_token = Some(encrypt!(&refresh_token)?);
                }

                current.updated_at = now;
                upsert_impl(DB_MANAGER.db(), current)?;
            }
            None => {
                // Insert new record
                let entity = Self {
                    account_id,
                    oauth2_id: request.oauth2_id.unwrap_or(EXTERNAL_OAUTH_APP_ID),
                    access_token: request
                        .access_token
                        .as_ref()
                        .map(|token| encrypt!(token))
                        .transpose()?,
                    refresh_token: request
                        .refresh_token
                        .as_ref()
                        .map(|token| encrypt!(token))
                        .transpose()?,
                    created_at: now,
                    updated_at: now,
                };
                insert_impl(DB_MANAGER.db(), entity)?;
            }
        }
        Ok(())
    }

    // This function may be called multiple times for one account, so we use upsert.
    pub fn save_or_update(&self) -> BichonResult<()> {
        upsert_impl(DB_MANAGER.db(), self.clone())
    }

    pub fn get(account_id: u64) -> BichonResult<Option<OAuth2AccessToken>> {
        find_impl::<OAuth2AccessToken>(DB_MANAGER.db(), &account_id.to_string())?
            .map(|mut token| {
                token.access_token = token.access_token.map(|t| decrypt!(&t)).transpose()?;
                token.refresh_token = token.refresh_token.map(|t| decrypt!(&t)).transpose()?;
                Ok(token)
            })
            .transpose()
    }

    pub fn list_all() -> BichonResult<Vec<OAuth2AccessToken>> {
        list_all_impl::<OAuth2AccessToken>(DB_MANAGER.db())?
            .into_iter()
            .map(|mut token| {
                token.access_token = token.access_token.map(|t| decrypt!(&t)).transpose()?;
                token.refresh_token = token.refresh_token.map(|t| decrypt!(&t)).transpose()?;
                Ok(token)
            })
            .collect()
    }

    pub fn try_delete(account_id: u64) -> BichonResult<()> {
        if Self::get(account_id)?.is_none() {
            return Ok(());
        }

        delete_impl::<OAuth2AccessToken>(DB_MANAGER.db(), &account_id.to_string())
    }

    pub fn delete_by_oauth2_id(oauth2_id: u64) -> BichonResult<()> {
        let tokens = filter_impl::<OAuth2AccessToken, _>(DB_MANAGER.db(), move |t| {
            t.oauth2_id == oauth2_id
        })?;
        if let Some(token) = tokens.first() {
            delete_impl::<OAuth2AccessToken>(DB_MANAGER.db(), &token.account_id.to_string())?;
        }
        Ok(())
    }

    pub fn set_access_token(
        account_id: u64,
        access_token: String,
        refresh_token: String,
    ) -> BichonResult<()> {
        update_impl(
            DB_MANAGER.db(),
            &account_id.to_string(),
            |current: OAuth2AccessToken| {
                let mut updated = current.clone();
                updated.access_token = Some(access_token);
                updated.refresh_token = Some(refresh_token);
                updated.updated_at = utc_now!();
                Ok(updated)
            },
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ExternalOAuth2Request {
    /// The id of the OAuth2 configuration associated with this access token.
    pub oauth2_id: Option<u64>,
    /// The OAuth2 access token used to authenticate requests to the provider.
    pub access_token: Option<String>,
    /// The OAuth2 refresh token used to obtain new access tokens.
    pub refresh_token: Option<String>,
}

impl ExternalOAuth2Request {
    /// Validates the request.
    ///
    /// Ensures mutual dependency between oauth2_id and refresh_token:
    /// - If `refresh_token` is provided, `oauth2_id` must also be present.
    /// - If `oauth2_id` is provided, `refresh_token` must also be present.
    pub fn validate(&self) -> BichonResult<()> {
        match (self.oauth2_id.is_some(), self.refresh_token.is_some()) {
            (true, false) => {
                return Err(raise_error!(
                    "refresh_token must be provided if oauth2_id is set".into(),
                    ErrorCode::InvalidParameter
                ));
            }
            (false, true) => {
                return Err(raise_error!(
                    "oauth2_id must be provided if refresh_token is set".into(),
                    ErrorCode::InvalidParameter
                ));
            }
            _ => {}
        }

        // Validate that oauth2_id exists in the database if provided
        if let Some(oauth2_id) = self.oauth2_id {
            let oauth2 = OAuth2::get(oauth2_id)?;
            if oauth2.is_none() {
                return Err(raise_error!(
                    format!("OAuth2 configuration with id {} does not exist", oauth2_id),
                    ErrorCode::InvalidParameter
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::oauth2::token::OAuth2AccessToken;

    #[tokio::test]
    async fn test1() {
        let token = OAuth2AccessToken::create(
            1000u64,
            1020u64,
            "access_token".into(),
            "refresh_token".into(),
        )
        .unwrap();
        token.save_or_update().unwrap();
        let token2 = OAuth2AccessToken::get(1000u64).unwrap().unwrap();
        assert_eq!(token2.access_token, Some("access_token".into()));
        assert_eq!(token2.refresh_token, Some("refresh_token".into()));

        let tokens = OAuth2AccessToken::list_all().unwrap();
        assert_eq!(tokens.len(), 1);

        let first = tokens.first().unwrap();
        assert_eq!(first.access_token, Some("access_token".into()));
        assert_eq!(first.refresh_token, Some("refresh_token".into()));
    }
}
