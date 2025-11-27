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

use crate::modules::common::auth::ClientContext;
use crate::modules::import::BatchEmlResult;
use crate::modules::import::{BatchEmlRequest, ImportEmls};
use crate::modules::rest::api::ApiTags;
use crate::modules::rest::ApiResult;
use poem_openapi::payload::Json;
use poem_openapi::OpenApi;

pub struct ImportApi;

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::Import")]
impl ImportApi {
    /// Batch import one or more EML files into a specified account and mail folder.
    ///
    /// This endpoint accepts a JSON payload containing:
    /// - `account_id`: the target account to import emails into
    /// - `mail_folder`: the mailbox/folder name
    /// - `emls`: a list of base64-encoded .eml files
    ///
    /// Returns a summary of the import result, including total processed, successful, and failed emails.
    #[oai(path = "/import", method = "post", operation_id = "do_batch_import")]
    async fn do_batch_import(
        &self,
        /// JSON payload with account info and EML files to import
        payload: Json<BatchEmlRequest>,
        context: ClientContext,
    ) -> ApiResult<Json<BatchEmlResult>> {
        context.require_root()?;
        Ok(Json(ImportEmls::do_import(payload.0).await?))
    }
}
