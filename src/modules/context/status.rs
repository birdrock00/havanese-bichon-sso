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


use crate::modules::context::executors::BICHON_CONTEXT;
use chrono::Local;
use poem_openapi::Object;
use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;
use timeago::Formatter;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Object)]
pub struct BichonStatus {
    /// The service uptime in milliseconds since it started.
    pub uptime_ms: i64,
    /// A human-readable string indicating the time elapsed since the service started (e.g., "2 hours ago").
    pub timeago: String,
    /// The timezone in which the service is operating (e.g., "UTC" or "Asia/Tokyo").
    pub timezone: String,
    /// The version of the RustMailer service currently running.
    pub version: String,
}

impl BichonStatus {
    pub fn get() -> Self {
        Self {
            uptime_ms: BICHON_CONTEXT.uptime_ms(),
            timeago: Formatter::new()
                .convert(Duration::from_millis(BICHON_CONTEXT.uptime_ms() as u64)),
            timezone: Local::now().offset().to_string(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}
