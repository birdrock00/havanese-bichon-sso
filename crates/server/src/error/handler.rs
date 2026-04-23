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

use bichon_core::error::{code::ErrorCode, BichonError};
use poem::IntoResponse;
use poem_openapi::payload::Json;

use crate::error::{ApiError, ApiErrorResponse, code::IntoStatusCode};

pub async fn error_handler(error: poem::Error) -> impl poem::IntoResponse {
    if error.is::<BichonError>() {
        return error.into_response();
    }

    let error_mapping = [
        // Poem errors
        (
            error.is::<poem::error::NotFoundError>(),
            ErrorCode::ResourceNotFound,
        ),
        (
            error.is::<poem::error::ParsePathError>()
                || error.is::<poem::error::ParseTypedHeaderError>()
                || error.is::<poem::error::ParseQueryError>()
                || error.is::<poem::error::ParseJsonError>()
                || error.is::<poem_openapi::error::ParseRequestPayloadError>()
                || error.is::<poem_openapi::error::ContentTypeError>()
                || error.is::<poem_openapi::error::ParseParamError>()
                || error.is::<poem_openapi::error::ParsePathError>(),
            ErrorCode::InvalidParameter,
        ),
        (
            error.is::<poem::error::MethodNotAllowedError>(),
            ErrorCode::MethodNotAllowed,
        ),
        (
            error.is::<poem_openapi::error::AuthorizationError>(),
            ErrorCode::PermissionDenied,
        ),
    ];

    // Find the first matching error type
    if let Some((_, error_code)) = error_mapping.iter().find(|(condition, _)| *condition) {
        let api_error = ApiError::new_with_error_code(error.to_string(), *error_code as u32);
        let mut response =
            ApiErrorResponse::Generic(error_code.status(), Json(api_error)).into_response();
        response.set_status(error.status());
        return response;
    }
    // Handle other cases
    if error.has_source() {
        let api_error =
            ApiError::new_with_error_code(error.to_string(), ErrorCode::UnhandledPoemError as u32);
        let mut response =
            ApiErrorResponse::Generic(ErrorCode::UnhandledPoemError.status(), Json(api_error))
                .into_response();
        response.set_status(error.status());
        response
    } else {
        error.into_response()
    }
}
