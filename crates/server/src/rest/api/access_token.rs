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

use crate::common::auth::WrappedContext;
use crate::rest::api::ApiTags;
use crate::rest::ApiResult;
use bichon_core::token::view::AccessTokenResp;
use bichon_core::users::permissions::Permission;
use bichon_core::{token::payload::AccessTokenCreateRequest, token::AccessTokenModel};
use poem_openapi::payload::PlainText;
use poem_openapi::{param::Path, payload::Json, OpenApi};

pub struct AccessTokenApi;

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::AccessToken")]
impl AccessTokenApi {
    #[oai(
        path = "/access-token-list",
        method = "get",
        operation_id = "list_access_tokens"
    )]
    async fn list_access_tokens(
        &self,
        context: WrappedContext,
    ) -> ApiResult<Json<Vec<AccessTokenResp>>> {
        context
            .require_permission(None, Permission::TOKEN_MANAGE)
            .await?;

        Ok(Json(AccessTokenModel::list_all_api_tokens().await?))
    }

    /// Deletes a specific access token.
    #[oai(
        path = "/access-token/:token",
        method = "delete",
        operation_id = "remove_access_token"
    )]
    async fn remove_access_token(
        &self,
        /// The access token to be deleted
        token: Path<String>,
        context: WrappedContext,
    ) -> ApiResult<()> {
        let token = token.0.trim();
        let token = AccessTokenModel::get_token(token).await?;
        if context.user.id != token.user_id {
            context
                .require_permission(None, Permission::TOKEN_MANAGE)
                .await?;
        }

        Ok(AccessTokenModel::delete(&token.token).await?)
    }

    /// Creates a new api token.
    #[oai(
        path = "/access-token",
        method = "post",
        operation_id = "create_access_token"
    )]
    async fn create_access_token(
        &self,
        context: WrappedContext,
        /// The request payload
        payload: Json<AccessTokenCreateRequest>,
    ) -> ApiResult<PlainText<String>> {
        let current_user_id = context.user.id;
        let target_user_id = payload.0.user_id.unwrap_or(current_user_id);
        if target_user_id != current_user_id {
            context
                .require_permission(None, Permission::USER_MANAGE)
                .await?;
        }

        let token_string = AccessTokenModel::create_api_token(target_user_id, payload.0).await?;
        Ok(PlainText(token_string))
    }
}
