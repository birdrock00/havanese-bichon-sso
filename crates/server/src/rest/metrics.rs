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

//! Prometheus / OpenMetrics endpoint.
//!
//! Registered as a plain poem route (not through poem-openapi) because the
//! response is the Prometheus text exposition format, not JSON. It is wrapped
//! in `ApiGuard`, so token validation, the IP whitelist and rate limiting all
//! apply exactly as on the OpenAPI tree.

use bichon_core::error::code::ErrorCode;
use bichon_core::metrics;
use bichon_core::users::permissions::Permission;
use poem::{handler, http::StatusCode, Response};

use crate::common::auth::WrappedContext;
use crate::common::create_api_error_response;

#[handler]
pub async fn get_metrics(context: WrappedContext) -> Result<Response, poem::Error> {
    // `system:metrics` is a global permission, so the account scope is None.
    if !context.has_permission(None, Permission::SYSTEM_METRICS) {
        // The token is valid but lacks the permission: 403, not 401 (which
        // ApiGuard already uses for a missing/invalid token).
        return Err(create_api_error_response(
            "Missing permission: system:metrics",
            ErrorCode::Forbidden,
        ));
    }

    let body = metrics::scrape().await.map_err(|e| {
        create_api_error_response(&format!("{:#?}", e), ErrorCode::InternalError)
    })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(body))
}
