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
use bichon_core::autoconfig::entity::MailServerConfig;
use bichon_core::autoconfig::load::resolve_autoconfig;
use bichon_core::error::code::ErrorCode;
use bichon_core::raise_error;
use bichon_core::users::permissions::Permission;
use poem_openapi::param::Path;
use poem_openapi::payload::Json;
use poem_openapi::OpenApi;

pub struct AutoConfigApi;

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::AutoConfig")]
impl AutoConfigApi {
    /// Retrieve mail server configuration for a given email address
    #[oai(
        path = "/autoconfig/:email_address",
        method = "get",
        operation_id = "autoconfig"
    )]
    async fn autoconfig(
        &self,
        /// The email address to lookup configuration for
        email_address: Path<String>,
        context: WrappedContext,
    ) -> ApiResult<Json<MailServerConfig>> {
        context
            .require_permission(None, Permission::ACCOUNT_CREATE)
            .await?;

        let result = resolve_autoconfig(email_address.0.trim())
            .await?
            .ok_or_else(|| {
                raise_error!(
                    "Unable to find account configuration information in the backend.".into(),
                    ErrorCode::ResourceNotFound
                )
            })?;
        Ok(Json(result))
    }
}
