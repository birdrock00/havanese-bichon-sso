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
    modules::error::{code::ErrorCode, BichonResult},
    raise_error,
};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize, Object)]
pub struct AccessTokenCreateRequest {
    /// An optional description of the token's purpose or usage.
    #[oai(validator(max_length = "32"))]
    pub name: Option<String>,
    /// The expiration interval for this token, in hours.
    /// None means the token does not expire (this applies only to API tokens).
    pub expire_in: Option<u64>,
    /// The ID of the user for whom the token is being created.
    /// If not specified, the token will be created for the current authenticated user.
    /// Accessing this for another user typically requires `USER_MANAGE` permissions.
    pub user_id: Option<u64>,
}

impl AccessTokenCreateRequest {
    pub async fn validate(&self) -> BichonResult<()> {
        if let Some(expire_in) = self.expire_in {
            if expire_in == 0 {
                return Err(raise_error!(
                    "expire_in must be a positive duration in hours; zero is not allowed.".into(),
                    ErrorCode::InvalidParameter
                ));
            }
        }
        Ok(())
    }
}
