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
use crate::database::{delete_impl, async_find_impl, upsert_impl};
use crate::error::code::ErrorCode;
use crate::raise_error;
use crate::{
    autoconfig::entity::MailServerConfig, error::BichonResult, utc_now,
};
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};

pub mod entity;
pub mod load;
#[cfg(test)]
mod tests;

const EXPIRE_TIME_MS: i64 = 30 * 24 * 60 * 60 * 1000;

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[native_model(id = 3, version = 1)]
#[native_db]
pub struct CachedMailSettings {
    #[primary_key]
    pub domain: String,
    pub config: MailServerConfig,
    pub created_at: i64,
}

impl CachedMailSettings {
    pub async fn add(domain: String, config: MailServerConfig) -> BichonResult<()> {
        Self {
            domain,
            config,
            created_at: utc_now!(),
        }
        .save()
        .await
    }

    async fn save(&self) -> BichonResult<()> {
        upsert_impl(DB_MANAGER.meta_db(), self.to_owned()).await
    }

    pub async fn get(domain: &str) -> BichonResult<Option<CachedMailSettings>> {
        if let Some(found) =
            async_find_impl::<CachedMailSettings>(DB_MANAGER.meta_db(), domain.to_string()).await?
        {
            if (utc_now!() - found.created_at) > EXPIRE_TIME_MS {
                let domain = domain.to_string();
                delete_impl(DB_MANAGER.meta_db(), |rw| {
                    rw.get()
                        .primary::<CachedMailSettings>(domain)
                        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
                        .ok_or_else(|| {
                            raise_error!("auto config cache miss".into(), ErrorCode::InternalError)
                        })
                })
                .await?;
                Ok(None)
            } else {
                Ok(Some(found))
            }
        } else {
            Ok(None)
        }
    }
}
