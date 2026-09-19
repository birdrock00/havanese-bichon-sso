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
use bichon_core::ext::event_bus::{emit, Event};
use bichon_core::saved_search::{
    SavedSearchCreateRequest, SavedSearchKind, SavedSearchModel, SavedSearchRenameRequest,
};
use poem_openapi::param::{Path, Query};
use poem_openapi::payload::Json;
use poem_openapi::OpenApi;

pub struct SavedSearchApi;

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::SavedSearch")]
impl SavedSearchApi {
    /// Lists the current user's saved searches, newest first. Pass
    /// `kind=Email` / `kind=Attachment` to filter by search surface.
    #[oai(
        path = "/saved-searches",
        method = "get",
        operation_id = "list_saved_searches"
    )]
    async fn list_saved_searches(
        &self,
        context: WrappedContext,
        kind: Query<Option<SavedSearchKind>>,
    ) -> ApiResult<Json<Vec<SavedSearchModel>>> {
        let items = SavedSearchModel::list_for_user(context.user.id)?;
        let filtered = match kind.0 {
            Some(k) => items.into_iter().filter(|s| s.kind == k).collect(),
            None => items,
        };
        Ok(Json(filtered))
    }

    /// Saves the current search condition under a user-chosen name.
    #[oai(
        path = "/saved-searches",
        method = "post",
        operation_id = "create_saved_search"
    )]
    async fn create_saved_search(
        &self,
        context: WrappedContext,
        payload: Json<SavedSearchCreateRequest>,
    ) -> ApiResult<Json<SavedSearchModel>> {
        let model = SavedSearchModel::create(
            context.user.id,
            payload.0.name,
            payload.0.kind,
            payload.0.filter,
        )?;
        emit(Event::SavedSearchCreated {
            user: context.user.username.clone(),
            search_id: model.id.clone(),
            kind: format!("{:?}", model.kind).to_lowercase(),
            name: model.name.clone(),
        });
        Ok(Json(model))
    }

    /// Renames a saved search. The condition itself is immutable.
    #[oai(
        path = "/saved-searches/:id",
        method = "patch",
        operation_id = "rename_saved_search"
    )]
    async fn rename_saved_search(
        &self,
        id: Path<String>,
        context: WrappedContext,
        payload: Json<SavedSearchRenameRequest>,
    ) -> ApiResult<Json<SavedSearchModel>> {
        let model = SavedSearchModel::rename(context.user.id, &id.0, payload.0.name)?;
        emit(Event::SavedSearchRenamed {
            user: context.user.username.clone(),
            search_id: model.id.clone(),
            name: model.name.clone(),
        });
        Ok(Json(model))
    }

    /// Deletes a saved search owned by the caller.
    #[oai(
        path = "/saved-searches/:id",
        method = "delete",
        operation_id = "remove_saved_search"
    )]
    async fn remove_saved_search(
        &self,
        id: Path<String>,
        context: WrappedContext,
    ) -> ApiResult<()> {
        let model = SavedSearchModel::get_owned(context.user.id, &id.0)?;
        SavedSearchModel::delete(context.user.id, &id.0)?;
        emit(Event::SavedSearchDeleted {
            user: context.user.username.clone(),
            search_id: id.0,
            name: model.name,
        });
        Ok(())
    }
}