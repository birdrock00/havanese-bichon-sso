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
use tantivy::doc;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct Envelope {
    pub id: String,
    pub message_id: String,
    pub account_id: u64,
    pub account_email: Option<String>,
    pub mailbox_id: u64,
    pub mailbox_name: Option<String>,
    pub uid: u32,
    pub subject: String,
    pub preview: String,
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub date: i64,
    pub internal_date: i64,
    pub ingest_at: i64,
    pub size: u32,
    pub thread_id: String,
    pub attachment_count: usize,
    pub regular_attachment_count: usize,
    pub tags: Option<Vec<String>>,
    pub content_hash: String,
}

impl Envelope {
    pub fn has_any_attachments(&self) -> bool {
        self.attachment_count > 0
    }
}
