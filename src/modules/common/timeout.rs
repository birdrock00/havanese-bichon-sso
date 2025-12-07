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


use poem::{Endpoint, Middleware, Request, Result};
use std::time::Duration;
use tracing::error;

use crate::modules::error::code::ErrorCode;

use super::create_api_error_response;

pub const TIMEOUT_HEADER: &str = "X-Bichon-Timeout-Seconds";

pub struct Timeout;

impl<E: Endpoint> Middleware<E> for Timeout {
    type Output = TimeoutEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        TimeoutEndpoint { ep }
    }
}

pub struct TimeoutEndpoint<E> {
    ep: E,
}

#[inline]
fn extract_timeout(req: &Request) -> Option<u64> {
    if let Some(v) = req.header(TIMEOUT_HEADER) {
        v.parse::<u64>().ok()
    } else {
        None
    }
}

impl<E: Endpoint> Endpoint for TimeoutEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let timeout = extract_timeout(&req);
        let seconds = timeout.unwrap_or(30).min(600);
        match tokio::time::timeout(Duration::from_secs(seconds), self.ep.call(req)).await {
            Ok(Ok(response)) => Ok(response), // If the request completes successfully
            Ok(Err(e)) => Err(e),             // If the request returns an error
            Err(_) => {
                error!("Request timed out after {} seconds", seconds);
                Err(create_api_error_response(
                    &format!(
                        "Request timed out after {} seconds (timeout set via X-Bichon-Timeout-Seconds header, max allowed: 600 seconds)",
                        seconds
                    ),
                    ErrorCode::RequestTimeout,
                ))
            }
        }
    }
}
