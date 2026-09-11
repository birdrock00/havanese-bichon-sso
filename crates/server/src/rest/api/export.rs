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
use crate::export::{
    cancel_export, create_export, delete_export, download_artifact, get_export, list_exports,
    preview,
};
use crate::rest::api::ApiTags;
use crate::rest::ApiResult;
use bichon_core::error::code::ErrorCode;
use bichon_core::export::{
    ExportCreateRequest, ExportJobView, ExportPreviewRequest, ExportPreviewView,
};
use bichon_core::ext::event_bus::{emit, Event};
use bichon_core::raise_error;
use bichon_core::users::permissions::Permission;
use poem::Body;
use poem_openapi::param::Path;
use poem_openapi::payload::{Attachment, AttachmentType, Json};
use poem_openapi::OpenApi;
use std::collections::HashSet;

pub struct ExportApi;

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::Export")]
impl ExportApi {
    /// Previews what an export from a saved search would contain (accounts,
    /// email count, estimated size), restricted to the caller's RBAC scope.
    #[oai(
        path = "/exports/preview",
        method = "post",
        operation_id = "preview_export"
    )]
    async fn preview_export(
        &self,
        context: WrappedContext,
        payload: Json<ExportPreviewRequest>,
    ) -> ApiResult<Json<ExportPreviewView>> {
        let scope = export_scope(&context)?;
        let view = preview(context.user.id, &payload.0.saved_search_id, scope)?;
        Ok(Json(view))
    }

    /// Starts a background batch export from a saved search.
    #[oai(
        path = "/exports",
        method = "post",
        operation_id = "create_export"
    )]
    async fn create_export_handler(
        &self,
        context: WrappedContext,
        payload: Json<ExportCreateRequest>,
    ) -> ApiResult<Json<ExportJobView>> {
        let scope = export_scope(&context)?;
        let job = create_export(
            context.user.id,
            context.user.username.clone(),
            &payload.0.saved_search_id,
            payload.0.format.unwrap_or_default(),
            scope,
        )?;
        Ok(Json(job))
    }

    /// Lists the caller's export jobs, newest first.
    #[oai(
        path = "/exports",
        method = "get",
        operation_id = "list_exports"
    )]
    async fn list_exports_handler(
        &self,
        context: WrappedContext,
    ) -> ApiResult<Json<Vec<ExportJobView>>> {
        Ok(Json(list_exports(context.user.id)))
    }

    /// Snapshot of an export job.
    #[oai(
        path = "/exports/:job_id",
        method = "get",
        operation_id = "get_export"
    )]
    async fn get_export_handler(
        &self,
        job_id: Path<String>,
        context: WrappedContext,
    ) -> ApiResult<Json<ExportJobView>> {
        Ok(Json(get_export(context.user.id, &job_id.0)?))
    }

    /// Requests cancellation of a running export job.
    #[oai(
        path = "/exports/:job_id/cancel",
        method = "post",
        operation_id = "cancel_export"
    )]
    async fn cancel_export_handler(
        &self,
        job_id: Path<String>,
        context: WrappedContext,
    ) -> ApiResult<Json<ExportJobView>> {
        Ok(Json(cancel_export(context.user.id, &job_id.0)?))
    }

    /// Deletes an export job and its on-disk artifact.
    #[oai(
        path = "/exports/:job_id",
        method = "delete",
        operation_id = "delete_export"
    )]
    async fn delete_export_handler(
        &self,
        job_id: Path<String>,
        context: WrappedContext,
    ) -> ApiResult<()> {
        Ok(delete_export(context.user.id, &job_id.0)?)
    }

    /// Downloads the finished mbox artifact.
    #[oai(
        path = "/exports/:job_id/download",
        method = "get",
        operation_id = "download_export"
    )]
    async fn download_export_handler(
        &self,
        job_id: Path<String>,
        context: WrappedContext,
    ) -> ApiResult<Attachment<Body>> {
        let job_id = job_id.0;
        let (path, name) = download_artifact(context.user.id, &job_id)?;
        let view = get_export(context.user.id, &job_id)?;
        emit(Event::ExportDownloaded {
            user: context.user.username.clone(),
            export_id: job_id.clone(),
            email_count: view.exported,
            artifact_size: view.artifact_size,
        });
        let reader = tokio::fs::File::open(&path).await.map_err(|e| {
            raise_error!(
                format!("Failed to open export artifact: {e}"),
                ErrorCode::InternalError
            )
        })?;
        let body = Body::from_async_read(reader);
        let attachment = Attachment::new(body)
            .attachment_type(AttachmentType::Attachment)
            .filename(name);
        Ok(attachment)
    }
}

/// Computes the account scope the caller may export: all accounts when the
/// caller holds the global `data:export:batch` permission (or is an admin),
/// otherwise the subset of their accessible accounts whose role grants
/// `data:export:batch`. Returns `Forbidden` when nothing is exportable.
fn export_scope(
    context: &WrappedContext,
) -> bichon_core::error::BichonResult<Option<HashSet<u64>>> {
    if context.has_permission(None, Permission::DATA_EXPORT_BATCH) {
        return Ok(None);
    }
    let scoped: HashSet<u64> = context
        .user
        .account_access_map
        .keys()
        .cloned()
        .filter(|account_id| {
            context.has_permission(Some(*account_id), Permission::DATA_EXPORT_BATCH)
        })
        .collect();
    if scoped.is_empty() {
        return Err(raise_error!(
            "Access Denied: Missing permission 'data:export:batch'".into(),
            ErrorCode::Forbidden
        ));
    }
    Ok(Some(scoped))
}