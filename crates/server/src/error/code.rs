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

use bichon_core::error::code::ErrorCode;
use poem::http::StatusCode;

pub trait IntoStatusCode {
    fn status(&self) -> StatusCode;
}

impl IntoStatusCode for ErrorCode {
    fn status(&self) -> StatusCode {
        match self {
            ErrorCode::InvalidParameter
            | ErrorCode::MissingConfiguration
            | ErrorCode::Incompatible => StatusCode::BAD_REQUEST,
            ErrorCode::PermissionDenied => StatusCode::UNAUTHORIZED,
            ErrorCode::AccountDisabled | ErrorCode::OAuth2ItemDisabled | ErrorCode::Forbidden => {
                StatusCode::FORBIDDEN
            }
            ErrorCode::ResourceNotFound => StatusCode::NOT_FOUND,
            ErrorCode::RequestTimeout => StatusCode::REQUEST_TIMEOUT,
            ErrorCode::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            ErrorCode::TooManyRequest => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::AlreadyExists => StatusCode::CONFLICT,
            ErrorCode::InternalError
            | ErrorCode::AutoconfigFetchFailed
            | ErrorCode::ImapCommandFailed
            | ErrorCode::ImapUnexpectedResult
            | ErrorCode::HttpResponseError
            | ErrorCode::ImapAuthenticationFailed
            | ErrorCode::MissingRefreshToken
            | ErrorCode::NetworkError
            | ErrorCode::ConnectionTimeout
            | ErrorCode::ConnectionPoolTimeout
            | ErrorCode::UnhandledPoemError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
        }
    }
}
