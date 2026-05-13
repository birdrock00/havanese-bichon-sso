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

//use poem_openapi::Object;
use serde::{Deserialize, Serialize};

use crate::{
    database::{
        delete_impl, find_impl, insert_impl, list_all_impl, manager::DB_MANAGER, update_impl,
        MemDbModel,
    },
    error::{code::ErrorCode, BichonResult},
    id, raise_error, utc_now,
    utils::net::parse_proxy_addr,
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct Proxy {
    /// The unique identifier for this proxy configuration.
    pub id: u64,

    /// The proxy URL (e.g., socks5://127.0.0.1:1080) used to route network requests.
    pub url: String,

    /// The creation timestamp of this record, represented as milliseconds since the Unix epoch.
    pub created_at: i64,

    /// The last update timestamp of this record, represented as milliseconds since the Unix epoch.
    pub updated_at: i64,
}

impl MemDbModel for Proxy {
    fn collection() -> &'static str {
        "proxies"
    }
    fn key(&self) -> String {
        self.id.to_string()
    }
}

impl Proxy {
    /// Create a new Proxy instance with the given URL and timestamps.
    pub fn new(url: String) -> Self {
        Self {
            id: id!(64),
            url,
            created_at: utc_now!(),
            updated_at: utc_now!(),
        }
    }

    pub fn get(id: u64) -> BichonResult<Proxy> {
        let key = id.to_string();
        find_impl::<Proxy>(DB_MANAGER.db(), &key)?.ok_or_else(|| {
            raise_error!(
                format!("Proxy with id={} not found", id),
                ErrorCode::ResourceNotFound
            )
        })
    }

    pub fn list_all() -> BichonResult<Vec<Proxy>> {
        list_all_impl::<Proxy>(DB_MANAGER.db())
    }

    pub fn delete(id: u64) -> BichonResult<()> {
        delete_impl::<Proxy>(DB_MANAGER.db(), &id.to_string())
    }

    pub fn update(id: u64, url: String) -> BichonResult<()> {
        update_impl(DB_MANAGER.db(), &id.to_string(), move |current: Proxy| {
            let mut updated = current.clone();
            updated.url = url;
            updated.updated_at = utc_now!();
            Ok(updated)
        })?;
        Ok(())
    }

    pub fn save(&self) -> BichonResult<()> {
        self.validate()?;
        insert_impl(DB_MANAGER.db(), self.to_owned())
    }

    /// Validate that the URL is a valid SOCKS5 proxy URL.
    pub fn validate(&self) -> BichonResult<()> {
        parse_proxy_addr(&self.url)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_proxy_urls() {
        let urls = vec!["socks5://127.0.0.1:1080", "http://127.0.0.1:8080"];

        for url in urls {
            let proxy = Proxy::new(url.to_string());
            assert!(proxy.validate().is_ok(), "URL should be valid: {}", url);
        }
    }
}
