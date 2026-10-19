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

use crate::common::auth::WrappedContext;
use crate::rest::api::ApiTags;
use crate::rest::ApiResult;
use bichon_core::common::paginated::DataPage;
use bichon_core::error::code::ErrorCode;
use bichon_core::ext::event_bus::{emit, Event};
use bichon_core::message::attachment::AttachmentMetadata;
use bichon_core::message::search::search_attachment_scoped;
use bichon_core::message::search::AttachmentSearchRequest;
use bichon_core::message::tags::TagCount;
use bichon_core::message::tags::TagsRequest;
use bichon_core::raise_error;
use bichon_core::store::tantivy::attachment::ATTACHMENT_MANAGER;
use bichon_core::store::tantivy::model::AttachmentModel;
use bichon_core::store::tantivy::validate_facet;
use bichon_core::users::permissions::Permission;
use poem_openapi::param::Path;
use poem_openapi::payload::Json;
use poem_openapi::OpenApi;
use std::collections::HashSet;

pub struct AttachmentApi;

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::Attachment")]
impl AttachmentApi {
    /// Searches messages across all mailboxes using various filter criteria.
    /// The search filters are provided in the request body.
    #[oai(
        path = "/search-attachment",
        method = "post",
        operation_id = "search_attachment"
    )]
    async fn search_attachment(
        &self,
        payload: Json<AttachmentSearchRequest>,
        context: WrappedContext,
    ) -> ApiResult<Json<DataPage<AttachmentModel>>> {
        let search_text = payload
            .0
            .filter()
            .text
            .clone()
            .unwrap_or_default()
            .trim()
            .to_string();
        // Expand view accounts like the message search does. Only sources
        // without a participant filter can contribute: the attachment index
        // carries the parent's sender but not its recipients, so a filtered
        // source is excluded rather than guessed at.
        let scope = bichon_core::account::access::search_scope(
            &context,
            payload.0.filter().account_ids.as_ref(),
        )?;
        let result = search_attachment_scoped(scope, payload.0)?;
        if !search_text.is_empty() {
            emit(Event::SearchPerformed {
                query: search_text,
                user: context.user.username.clone(),
            });
        }
        Ok(Json(result))
    }

    /// Retrieves the attachment (metadata) of a specific message.
    #[oai(
        path = "/attachment/:account_id/:attachment_id",
        method = "get",
        operation_id = "get_attachment"
    )]
    async fn get_attachment(
        &self,
        /// The ID of the account.
        account_id: Path<u64>,
        /// The ID of the attachment.
        attachment_id: Path<String>,
        context: WrappedContext,
    ) -> ApiResult<Json<AttachmentModel>> {
        let account_id = account_id.0;
        context.require_permission(Some(account_id), Permission::DATA_READ)?;
        let attachment_id = attachment_id.0;
        let a = ATTACHMENT_MANAGER
            .get_attachment_by_id(account_id, &attachment_id)?
            .ok_or_else(|| {
                raise_error!(
                    format!(
                        "Attachment not found: account_id={} envelope_id={}",
                        account_id, &attachment_id
                    ),
                    ErrorCode::ResourceNotFound
                )
            })?;
        Ok(Json(a))
    }

    /// Returns all facets in the index along with their document counts.
    #[oai(
        path = "/all-attachment-tags",
        method = "get",
        operation_id = "get_all_attachment_tags"
    )]
    async fn get_all_attachment_tags(
        &self,
        context: WrappedContext,
    ) -> ApiResult<Json<Vec<TagCount>>> {
        // Views are expanded to their **unfiltered** sources: these aggregates
        // are account-level queries that cannot evaluate a source's participant
        // filter, so a filtered source contributes nothing rather than its
        // whole account (see `account::access::aggregate_accounts`).
        let authorized_ids = bichon_core::account::access::aggregate_accounts(&context)?;
        Ok(Json(ATTACHMENT_MANAGER.get_all_tags(authorized_ids)?))
    }

    /// Adds or removes facet tags for multiple emails across accounts.
    #[oai(
        path = "/update-attachment-tags",
        method = "post",
        operation_id = "update_attachment_tags"
    )]
    async fn update_attachment_tags(
        &self,
        req: Json<TagsRequest>,
        context: WrappedContext,
    ) -> ApiResult<()> {
        let req = req.0;
        for tag in &req.tags {
            validate_facet(tag)?;
        }

        for account_id in req.updates.keys() {
            context.require_permission(Some(*account_id), Permission::DATA_MANAGE)?;
        }

        let total_updates: u64 = req.updates.values().map(|ids| ids.len() as u64).sum();
        if total_updates > 0 {
            for account_id in req.updates.keys() {
                emit(Event::AttachmentTagged {
                    user: context.user.username.clone(),
                    account_id: *account_id,
                    count: req
                        .updates
                        .get(account_id)
                        .map(|ids| ids.len() as u64)
                        .unwrap_or(0),
                });
            }
        }

        ATTACHMENT_MANAGER.update_attachment_tags(req).await?;
        Ok(())
    }

    /// Retrieves a unique list of all contact email addresses across authorized accounts.
    #[oai(
        path = "/attachment-senders",
        method = "get",
        operation_id = "get_attachment_senders"
    )]
    async fn get_attachment_senders(
        &self,
        context: WrappedContext,
    ) -> ApiResult<Json<HashSet<String>>> {
        // Views are expanded to their **unfiltered** sources: these aggregates
        // are account-level queries that cannot evaluate a source's participant
        // filter, so a filtered source contributes nothing rather than its
        // whole account (see `account::access::aggregate_accounts`).
        let authorized_ids = bichon_core::account::access::aggregate_accounts(&context)?;
        Ok(Json(ATTACHMENT_MANAGER.get_all_senders(authorized_ids)?))
    }

    /// Retrieves unique metadata for all attachments across authorized accounts.
    #[oai(
        path = "/attachment_metadata",
        method = "get",
        operation_id = "get_attachment_metadata"
    )]
    async fn get_attachment_metadata(
        &self,
        context: WrappedContext,
    ) -> ApiResult<Json<AttachmentMetadata>> {
        // Views are expanded to their **unfiltered** sources: these aggregates
        // are account-level queries that cannot evaluate a source's participant
        // filter, so a filtered source contributes nothing rather than its
        // whole account (see `account::access::aggregate_accounts`).
        let authorized_ids = bichon_core::account::access::aggregate_accounts(&context)?;
        Ok(Json(
            ATTACHMENT_MANAGER.collect_attachment_metadata(authorized_ids)?,
        ))
    }
}
