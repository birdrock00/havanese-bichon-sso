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

use crate::modules::common::auth::ClientContext;
use crate::modules::message::attachment::AttachmentMetadata;
use crate::modules::message::search::search_attachment_impl;
use crate::modules::message::search::AttachmentSearchRequest;
use crate::modules::message::tags::TagCount;
use crate::modules::message::tags::TagsRequest;
use crate::modules::rest::api::ApiTags;
use crate::modules::rest::response::DataPage;
use crate::modules::rest::ApiResult;
use crate::modules::rest::ErrorCode;
use crate::modules::store::tantivy::attachment::ATTACHMENT_MANAGER;
use crate::modules::store::tantivy::model::AttachmentModel;
use crate::modules::users::permissions::Permission;
use crate::raise_error;
use poem_openapi::param::Path;
use poem_openapi::payload::Json;
use poem_openapi::OpenApi;
use tantivy::schema::Facet;
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
        context: ClientContext,
    ) -> ApiResult<Json<DataPage<AttachmentModel>>> {
        let authorized_ids: Option<HashSet<u64>> = if context
            .has_permission(None, Permission::DATA_READ_ALL)
            .await
        {
            None
        } else {
            Some(context.user.account_access_map.keys().cloned().collect())
        };
        Ok(Json(
            search_attachment_impl(authorized_ids, payload.0).await?,
        ))
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
        context: ClientContext,
    ) -> ApiResult<Json<AttachmentModel>> {
        let account_id = account_id.0;
        context
            .require_permission(Some(account_id), Permission::DATA_READ)
            .await?;
        let attachment_id = attachment_id.0;
        let a = ATTACHMENT_MANAGER
            .get_attachment_by_id(account_id, &attachment_id)
            .await?
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
        context: ClientContext,
    ) -> ApiResult<Json<Vec<TagCount>>> {
        let authorized_ids: Option<HashSet<u64>> = if context
            .has_permission(None, Permission::DATA_READ_ALL)
            .await
        {
            None
        } else {
            Some(context.user.account_access_map.keys().cloned().collect())
        };
        Ok(Json(ATTACHMENT_MANAGER.get_all_tags(authorized_ids).await?))
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
        context: ClientContext,
    ) -> ApiResult<()> {
        let req = req.0;
        for tag in &req.tags {
            Facet::from_text(tag)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter))?;
        }

        for account_id in req.updates.keys() {
            context
                .require_permission(Some(*account_id), Permission::DATA_MANAGE)
                .await?;
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
        context: ClientContext,
    ) -> ApiResult<Json<HashSet<String>>> {
        let authorized_ids: Option<HashSet<u64>> = if context
            .has_permission(None, Permission::DATA_READ_ALL)
            .await
        {
            None
        } else {
            Some(context.user.account_access_map.keys().cloned().collect())
        };
        Ok(Json(
            ATTACHMENT_MANAGER.get_all_senders(authorized_ids).await?,
        ))
    }

    /// Retrieves unique metadata for all attachments across authorized accounts.
    #[oai(
        path = "/attachment_metadata",
        method = "get",
        operation_id = "get_attachment_metadata"
    )]
    async fn get_attachment_metadata(
        &self,
        context: ClientContext,
    ) -> ApiResult<Json<AttachmentMetadata>> {
        let authorized_ids: Option<HashSet<u64>> = if context
            .has_permission(None, Permission::DATA_READ_ALL)
            .await
        {
            None
        } else {
            Some(context.user.account_access_map.keys().cloned().collect())
        };
        Ok(Json(
            ATTACHMENT_MANAGER.collect_attachment_metadata(authorized_ids)?,
        ))
    }
}
