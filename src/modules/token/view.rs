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


use crate::modules::token::TokenType;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Object)]
pub struct AccessTokenResp {
    pub user_id: u64,
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
    
    pub user_name: String,
    pub user_email: String,
}
