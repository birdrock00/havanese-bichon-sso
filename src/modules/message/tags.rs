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


use std::collections::HashMap;

use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize, Object)]
pub struct UpdateTagsRequest {
    pub updates: HashMap<u64, Vec<String>>, // account_id -> envelope_ids
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize, Object)]
pub struct TagCount {
    pub tag: String,
    pub count: u64,
}
