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

//! Data types shared between the server-side batch export API and its
//! clients (web UI and CLI). The export engine itself lives in the server
//! crate; this module only carries the wire format.

use serde::{Deserialize, Serialize};

/// Export file format for batch exports. Only `Mbox` is supported today;
/// the enum keeps the door open for additional formats.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum ExportFormat {
    #[default]
    Mbox,
}

impl ExportFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportFormat::Mbox => "mbox",
        }
    }
}

/// One account that will be (or was) included in an export scope.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ExportAccount {
    pub id: u64,
    pub email: String,
    pub name: Option<String>,
}

/// Request body for previewing an export before starting it.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ExportPreviewRequest {
    pub saved_search_id: String,
}

/// What an export would contain, computed before the job starts so the
/// caller can confirm the scope (account count, email count) first.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ExportPreviewView {
    pub saved_search_id: String,
    pub saved_search_name: String,
    pub format: ExportFormat,
    pub accounts: Vec<ExportAccount>,
    pub total_emails: u64,
    pub total_size: u64,
}

/// Request body for starting a batch export from a saved search.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ExportCreateRequest {
    pub saved_search_id: String,
    #[serde(default)]
    pub format: Option<ExportFormat>,
}

/// Snapshot of an export job, returned by the status and control endpoints.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ExportJobView {
    pub job_id: String,
    pub status: String,
    pub saved_search_id: String,
    pub saved_search_name: String,
    pub format: ExportFormat,
    pub accounts: Vec<ExportAccount>,
    pub total_emails: u64,
    pub total_size: u64,
    pub processed: u64,
    pub exported: u64,
    pub failed: u64,
    pub error: Option<String>,
    pub artifact_name: Option<String>,
    pub artifact_size: u64,
    pub created_at: i64,
    pub finished_at: Option<i64>,
}