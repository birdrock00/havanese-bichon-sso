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
    /// SHA-256 of the finished mbox artifact, recorded at export time.
    /// Present only for finished exports.
    pub artifact_hash: Option<String>,
    pub created_at: i64,
    pub finished_at: Option<i64>,
    /// Compliance verification status: idle, running, finished or failed.
    /// idle means the export has never been verified.
    #[serde(default)]
    pub verify_status: String,
    /// Messages checked so far and total when finished.
    #[serde(default)]
    pub verify_checked: u64,
    #[serde(default)]
    pub verify_matched: u64,
    #[serde(default)]
    pub verify_mismatched: u64,
    /// Verification error when the verification failed.
    #[serde(default)]
    pub verify_error: Option<String>,
    #[serde(default)]
    pub verify_started_at: Option<i64>,
    #[serde(default)]
    pub verify_finished_at: Option<i64>,
}

/// One per-message mismatch found by a compliance verification. The export
/// manifest records each message's content hash; a mismatch means the
/// archived message no longer matches what was exported (or the archive was
/// modified since the export ran).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ExportVerifyMismatch {
    pub envelope_id: String,
    pub message_id: Option<String>,
    pub subject: Option<String>,
    /// Content hash recorded in the export manifest.
    pub expected_hash: Option<String>,
    /// Content hash recomputed from the archive at verify time.
    pub actual_hash: Option<String>,
    /// Why the message did not match. One of `blob_missing`,
    /// `attachment_missing` or `content_changed`.
    pub reason: String,
}

/// Result of a compliance verification of a finished export: the artifact's
/// recorded SHA-256 is recomputed and compared, and each exported message's
/// content hash is cross-checked against the live archive.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ExportVerifyView {
    pub job_id: String,
    pub status: String,
    pub artifact_name: Option<String>,
    /// SHA-256 recorded in the manifest at export time.
    pub expected_artifact_hash: Option<String>,
    /// SHA-256 recomputed from the on-disk artifact.
    pub actual_artifact_hash: Option<String>,
    pub artifact_hash_match: bool,
    /// Messages whose content hash still matches the archive.
    pub checked: u64,
    pub matched: u64,
    pub mismatched: u64,
    pub verified_at: i64,
    pub mismatches: Vec<ExportVerifyMismatch>,
}

/// Live progress of a compliance verification of a finished export. The
/// verification runs in the background so large archives do not block the
/// API; poll this until `status` is `finished` or `failed`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ExportVerifyProgressView {
    pub job_id: String,
    /// `idle`, `running`, `finished` or `failed`.
    pub status: String,
    /// Messages checked so far.
    pub checked: u64,
    /// Total messages from the manifest.
    pub total: u64,
    pub matched: u64,
    pub mismatched: u64,
    pub error: Option<String>,
    /// Present when `status` is `finished`.
    pub result: Option<ExportVerifyView>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}
