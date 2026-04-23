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


use poem::{Endpoint, IntoResponse, Middleware, Request, Response, Result};

use crate::error::handler::error_handler;

pub struct ErrorCapture;

pub struct ErrorCaptureEndpoint<E> {
    ep: E,
}

impl<E: Endpoint> Middleware<E> for ErrorCapture {
    type Output = ErrorCaptureEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        ErrorCaptureEndpoint { ep }
    }
}

impl<E: Endpoint> Endpoint for ErrorCaptureEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        match self.ep.call(req).await {
            Ok(response) => Ok(response.into_response()),
            Err(error) => Ok(error_handler(error).await.into_response()),
        }
    }
}
