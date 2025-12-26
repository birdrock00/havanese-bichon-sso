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

use std::{collections::BTreeSet, net::IpAddr};

use poem_openapi::Object;
use serde::{Deserialize, Serialize};

use crate::{modules::error::{BichonResult, code::ErrorCode}, raise_error};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Object)]
pub struct RateLimit {
    /// The time window in seconds for the rate limit.
    pub interval: u64,
    /// The maximum number of allowed requests within the time window.
    pub quota: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Object)]
pub struct AccessControl {
    /// An optional set of valid IPv4 or IPv6 addresses allowed to use the access token.
    pub ip_whitelist: Option<BTreeSet<String>>,
    /// An optional rate limit configuration for the access token.
    pub rate_limit: Option<RateLimit>,
}

impl AccessControl {
    pub fn validate(&self) -> BichonResult<()> {
        if let Some(ip_whitelist) = &self.ip_whitelist {
            for ip in ip_whitelist {
                if ip.parse::<IpAddr>().is_err() {
                    return Err(raise_error!(
                        format!("Invalid IP address: {}", ip),
                        ErrorCode::InvalidParameter
                    ));
                }
            }
        }

        // Validate rate limit
        if let Some(rate_limit) = &self.rate_limit {
            if rate_limit.interval < 1 {
                return Err(raise_error!(
                    "Rate limit interval must be at least 1 second".into(),
                    ErrorCode::InvalidParameter
                ));
            }
            if rate_limit.quota < 1 {
                return Err(raise_error!(
                    "Rate limit quota must be at least 1".into(),
                    ErrorCode::InvalidParameter
                ));
            }
        }

        Ok(())
    }
}
