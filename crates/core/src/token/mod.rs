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

use std::collections::HashMap;

use crate::database::manager::DB_MANAGER;
use crate::database::{
    MemDbModel, delete_impl, filter_impl, find_impl, insert_impl, list_all_impl, update_impl, with_transaction
};
use crate::error::code::ErrorCode;
use crate::raise_error;
use crate::settings::cli::SETTINGS;
use crate::token::view::AccessTokenResp;
use crate::users::UserModel;
use crate::{
    error::BichonResult, generate_token, token::payload::AccessTokenCreateRequest, utc_now,
};
//use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

pub mod payload;
pub mod view;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum TokenType {
    WebUI,
    Api,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AccessTokenModel {
    /// The ID of the user who owns this token
    pub user_id: u64,
    /// The unique token string used for authentication
    pub token: String,
    /// An optional name of the token.
    pub name: Option<String>,
    /// Token type: WebUI or API
    pub token_type: TokenType,
    /// The timestamp (in milliseconds since epoch) when the token was created.
    pub created_at: i64,
    /// The timestamp (in milliseconds since epoch) when the token was last updated.
    pub updated_at: i64,
    /// The timestamp (in milliseconds since epoch) when the token expires.
    /// None means the token does not expire (this applies only to API tokens).
    pub expire_at: Option<i64>,
    /// The timestamp (in milliseconds since epoch) when the token was last used.
    pub last_access_at: i64,
}

impl MemDbModel for AccessTokenModel {
    fn collection() -> &'static str {
        "tokens"
    }
    fn key(&self) -> String {
        self.token.clone()
    }
}

impl AccessTokenModel {
    pub fn new_api_token(
        token: String,
        user_id: u64,
        name: Option<String>,
        expire_at: Option<i64>,
    ) -> Self {
        Self {
            token,
            created_at: utc_now!(),
            updated_at: utc_now!(),
            last_access_at: Default::default(),
            name,
            user_id,
            token_type: TokenType::Api,
            expire_at,
        }
    }

    pub fn new_webui_token(user_id: u64) -> AccessTokenModel {
        let now = utc_now!();
        AccessTokenModel {
            token: generate_token!(128),
            created_at: now,
            updated_at: now,
            last_access_at: Default::default(),
            name: None,
            user_id,
            token_type: TokenType::WebUI,
            expire_at: None,
        }
    }

    pub fn reset_webui_token(user_id: u64) -> BichonResult<String> {
        let old_token = Self::get_user_webui_token(user_id)?;
        let new_token = Self::new_webui_token(user_id);
        let new_token_str = new_token.token.clone();

        match old_token {
            Some(old) => {
                with_transaction(DB_MANAGER.db(), move |txn| {
                    let txn = txn.delete(AccessTokenModel::collection(), old.token.clone());
                    txn.insert(AccessTokenModel::collection(), new_token.key(), &new_token)
                        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
                })?;
            }
            None => {
                insert_impl(DB_MANAGER.db(), new_token)?;
            }
        }

        Ok(new_token_str)
    }

    pub fn get_user_webui_token(user_id: u64) -> BichonResult<Option<AccessTokenModel>> {
        let tokens =
            filter_impl::<AccessTokenModel, _>(DB_MANAGER.db(), move |t| t.user_id == user_id)?;
        Ok(tokens
            .into_iter()
            .find(|t| t.token_type == TokenType::WebUI))
    }

    pub fn get_user_api_tokens(user_id: u64) -> BichonResult<Vec<AccessTokenModel>> {
        let tokens =
            filter_impl::<AccessTokenModel, _>(DB_MANAGER.db(), move |t| t.user_id == user_id)?;
        Ok(tokens
            .into_iter()
            .filter(|t| t.token_type == TokenType::Api)
            .collect())
    }

    pub fn resolve_user_from_token(token: &str) -> BichonResult<UserModel> {
        let token_str = token.to_string();
        let token_model = find_impl::<AccessTokenModel>(DB_MANAGER.db(), &token_str)?
            .ok_or_else(|| {
                raise_error!(
                    "Invalid access token provided. Please check your credentials.".into(),
                    ErrorCode::PermissionDenied
                )
            })?;

        if matches!(token_model.token_type, TokenType::WebUI) {
            let life = utc_now!() - token_model.created_at;
            let max_life = SETTINGS.bichon_webui_token_expiration_hours * 60 * 60 * 1000;

            if life > (max_life as i64) {
                return Err(raise_error!(
                    "Permission denied: the WebUI token has expired.".into(),
                    ErrorCode::PermissionDenied
                ));
            }
        }

        if matches!(token_model.token_type, TokenType::Api) {
            if let Some(expire_at) = token_model.expire_at {
                if utc_now!() > expire_at {
                    return Err(raise_error!(
                        "Your API token has expired and is no longer valid.".into(),
                        ErrorCode::PermissionDenied
                    ));
                }
            }
            update_impl(DB_MANAGER.db(), &token_str, |current: AccessTokenModel| {
                let mut updated = current.clone();
                updated.last_access_at = utc_now!();
                Ok(updated)
            })?;
        }

        let user = UserModel::find(token_model.user_id)
            ?
            .ok_or_else(|| raise_error!("The user associated with this access token does not exist or may have been deleted.".into(), ErrorCode::ResourceNotFound))?;
        Ok(user)
    }

    pub fn create_api_token(
        user_id: u64,
        request: AccessTokenCreateRequest,
    ) -> BichonResult<String> {
        // Validate request parameters first
        request.validate()?;
        let expire_at = request
            .expire_in
            .map(|hours| utc_now!() + (hours as i64) * 60 * 60 * 1000);
        let token = generate_token!(128);
        let access_token =
            AccessTokenModel::new_api_token(token.clone(), user_id, request.name, expire_at);
        insert_impl(DB_MANAGER.db(), access_token)?;
        Ok(token)
    }

    pub fn delete(token: &str) -> BichonResult<()> {
        delete_impl::<AccessTokenModel>(DB_MANAGER.db(), token)
    }

    pub fn get_token(token: &str) -> BichonResult<AccessTokenModel> {
        find_impl::<AccessTokenModel>(DB_MANAGER.db(), token)?.ok_or_else(|| {
            raise_error!(
                format!("Access token '{}' not found", token),
                ErrorCode::ResourceNotFound
            )
        })
    }

    pub fn list_all_api_tokens() -> BichonResult<Vec<AccessTokenResp>> {
        let users = UserModel::list_all()?;
        let all = list_all_impl::<AccessTokenModel>(DB_MANAGER.db())?;

        let user_map: HashMap<u64, UserModel> = users.into_iter().map(|u| (u.id, u)).collect();

        let resp = all
            .into_iter()
            .filter(|t| t.token_type == TokenType::Api)
            .map(|token| {
                let user = user_map.get(&token.user_id);
                AccessTokenResp {
                    user_name: user
                        .map(|u| u.username.clone())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    user_email: user
                        .map(|u| u.email.clone())
                        .unwrap_or_else(|| "N/A".to_string()),
                    user_id: token.user_id,
                    name: token.name,
                    token: token.token,
                    token_type: token.token_type,
                    created_at: token.created_at,
                    updated_at: token.updated_at,
                    expire_at: token.expire_at,
                    last_access_at: token.last_access_at,
                }
            })
            .collect();

        Ok(resp)
    }
}
