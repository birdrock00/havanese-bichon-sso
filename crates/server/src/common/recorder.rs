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

//! HTTP request metrics middleware.
//!
//! Records only the method and the status class — never the concrete path.
//! Paths carry unbounded values (email ids, account ids, tokens) and would
//! explode the metric cardinality.

use bichon_core::metrics;
use poem::{Endpoint, IntoResponse, Middleware, Request, Response, Result};
use std::time::Instant;

pub struct Recorder;

pub struct RecorderEndpoint<E> {
    ep: E,
}

impl<E: Endpoint> Middleware<E> for Recorder {
    type Output = RecorderEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        RecorderEndpoint { ep }
    }
}

impl<E: Endpoint> Endpoint for RecorderEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let method = req.method().as_str().to_uppercase();
        let started = Instant::now();

        match self.ep.call(req).await {
            Ok(response) => {
                let response = response.into_response();
                let elapsed = started.elapsed().as_secs_f64();
                record(&method, response.status().as_u16(), elapsed);
                Ok(response)
            }
            Err(error) => {
                let elapsed = started.elapsed().as_secs_f64();
                record(&method, error.status().as_u16(), elapsed);
                Err(error)
            }
        }
    }
}

fn record(method: &str, status: u16, elapsed: f64) {
    let status_class = format!("{}xx", status / 100);
    metrics::count_http_request(method, &status_class, elapsed);
}
