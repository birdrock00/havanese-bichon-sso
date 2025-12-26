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


// use crate::modules::database::manager::DB_MANAGER;
// use crate::modules::database::{find_impl, upsert_impl};
// use crate::modules::error::BichonResult;
// use crate::utc_now;
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[native_model(id = 2, version = 1)]
#[native_db]
pub struct SystemSetting {
    #[primary_key]
    pub key: String,
    pub value: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl SystemSetting {
    // pub fn new(key: String, value: String) -> Self {
    //     Self {
    //         key,
    //         value,
    //         created_at: utc_now!(),
    //         updated_at: utc_now!(),
    //     }
    // }
    //overwrite
    // pub async fn set(&self) -> BichonResult<()> {
    //     upsert_impl(DB_MANAGER.meta_db(), self.to_owned()).await
    // }

    // pub fn get(key: &str) -> BichonResult<Option<SystemSetting>> {
    //     find_impl(DB_MANAGER.meta_db(), key)
    // }

    // pub async fn list() -> RustMailerResult<Vec<SystemSetting>> {
    //     list_all_impl(DB_MANAGER.metadata_db()).await
    // }

    // pub fn get_existing_value(key: &str) -> BichonResult<Option<String>> {
    //     let setting = Self::get(key)?;
    //     Ok(setting.map(|s| s.value))
    // }

    // pub async fn set_value(key: &str, value: String) -> BichonResult<()> {
    //     let setting = Self::new(key.to_string(), value);
    //     setting.set().await
    // }
}
