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

use tantivy::schema::Field;

pub const F_ACCOUNT_ID: &str = "account_id";
pub const F_MAILBOX_ID: &str = "mailbox_id";

pub const F_ID: &str = "id";
pub const F_BLOB: &str = "blob";

pub struct BlobFields {
    pub f_id: Field,
    pub f_account_id: Field,
    pub f_mailbox_id: Field,
    pub f_blob: Field,
}
