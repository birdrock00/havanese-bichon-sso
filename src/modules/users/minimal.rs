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

use poem_openapi::Object;
use serde::{Deserialize, Serialize};

use crate::modules::{
    database::{list_all_impl, manager::DB_MANAGER},
    error::BichonResult,
    users::UserModel,
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Object)]
pub struct MinimalUser {
    pub id: u64,
    pub username: String,
    pub email: String,
}

impl MinimalUser {
    pub async fn list_all() -> BichonResult<Vec<MinimalUser>> {
        let all_users = list_all_impl::<UserModel>(DB_MANAGER.meta_db()).await?;
        let minimal_list = all_users
            .into_iter()
            .map(|user| MinimalUser {
                id: user.id,
                username: user.username,
                email: user.email,
            })
            .collect();

        Ok(minimal_list)
    }
}
