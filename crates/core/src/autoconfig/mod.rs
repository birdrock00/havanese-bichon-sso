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

use crate::database::manager::DB_MANAGER;
use crate::database::{delete_impl, upsert_impl};
use crate::database::{find_impl, MemDbModel};
use crate::{autoconfig::entity::MailServerConfig, error::BichonResult, utc_now};
use serde::{Deserialize, Serialize};

pub mod client;
pub mod entity;
pub mod guess;
pub mod load;
mod oauth2_providers;
#[cfg(test)]
mod tests;

const EXPIRE_TIME_MS: i64 = 30 * 24 * 60 * 60 * 1000;

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct CachedMailSettings {
    pub domain: String,
    pub config: MailServerConfig,
    pub created_at: i64,
}

impl MemDbModel for CachedMailSettings {
    fn collection() -> &'static str {
        "autoconfig"
    }
    fn key(&self) -> String {
        self.domain.clone()
    }
}

impl CachedMailSettings {
    pub fn add(domain: String, config: MailServerConfig) -> BichonResult<()> {
        Self {
            domain,
            config,
            created_at: utc_now!(),
        }
        .save()
    }

    fn save(&self) -> BichonResult<()> {
        upsert_impl(DB_MANAGER.db(), self.to_owned())
    }

    pub fn get(domain: &str) -> BichonResult<Option<CachedMailSettings>> {
        if let Some(found) = find_impl::<CachedMailSettings>(DB_MANAGER.db(), domain)? {
            if (utc_now!() - found.created_at) > EXPIRE_TIME_MS {
                delete_impl::<CachedMailSettings>(DB_MANAGER.db(), domain)?;
                Ok(None)
            } else {
                Ok(Some(found))
            }
        } else {
            Ok(None)
        }
    }
}
