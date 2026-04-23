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
use bichon_core::dashboard::DashboardStats;
use bichon_core::error::code::ErrorCode;
use bichon_core::raise_error;
use bichon_core::settings::cli::SETTINGS;
use bichon_core::settings::proxy::Proxy;
use bichon_core::settings::SystemConfigurations;
use bichon_core::users::permissions::Permission;
use bichon_core::version::{fetch_notifications, Notifications};
use poem_openapi::param::Path;
use poem_openapi::payload::{Json, PlainText};
use poem_openapi::OpenApi;

pub struct SystemApi;

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::System")]
impl SystemApi {
    /// Retrieves important system notifications for the Bichon service.
    ///
    /// This endpoint returns a consolidated view of all critical system notifications including:
    /// - Available version updates
    /// - License expiration warnings
    #[oai(
        method = "get",
        path = "/notifications",
        operation_id = "get_notifications"
    )]
    async fn get_notifications(&self) -> ApiResult<Json<Notifications>> {
        let notification = fetch_notifications()
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Ok(Json(notification))
    }

    /// Get overall dashboard statistics.
    ///
    /// Returns various aggregated metrics about the mail system, such as
    /// total email count, total storage size, index usage, top senders,
    /// recent activity histogram, and more.
    #[oai(
        method = "get",
        path = "/dashboard-stats",
        operation_id = "get_dashboard_stats"
    )]
    async fn get_dashboard_stats(
        &self,
        context: WrappedContext,
    ) -> ApiResult<Json<DashboardStats>> {
        let stats = DashboardStats::get(context.0).await?;
        Ok(Json(stats))
    }

    /// Get the full list of SOCKS5 proxy configurations.
    #[oai(method = "get", path = "/list-proxy", operation_id = "list_proxy")]
    async fn list_proxy(&self, _context: WrappedContext) -> ApiResult<Json<Vec<Proxy>>> {
        //The proxy list is visible to all users.
        let proxies = Proxy::list_all()
            .await
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Ok(Json(proxies))
    }

    /// Delete a specific proxy configuration by ID. Requires root permission.
    #[oai(path = "/proxy/:id", method = "delete", operation_id = "remove_proxy")]
    async fn remove_proxy(
        &self,
        /// The ID of the proxy configuration to delete.
        id: Path<u64>,
        context: WrappedContext,
    ) -> ApiResult<()> {
        context.require_permission(None, Permission::ROOT).await?;
        Ok(Proxy::delete(id.0).await?)
    }

    /// Retrieve a specific proxy configuration by ID. Requires root permission.
    #[oai(path = "/proxy/:id", method = "get", operation_id = "get_proxy")]
    async fn get_proxy(
        &self,
        /// The ID of the proxy configuration to retrieve.
        id: Path<u64>,
        context: WrappedContext,
    ) -> ApiResult<Json<Proxy>> {
        context.require_permission(None, Permission::ROOT).await?;
        Ok(Json(Proxy::get(id.0).await?))
    }

    /// Create a new proxy configuration. Requires root permission.
    #[oai(path = "/proxy", method = "post", operation_id = "create_proxy")]
    async fn create_proxy(&self, url: PlainText<String>, context: WrappedContext) -> ApiResult<()> {
        context.require_permission(None, Permission::ROOT).await?;
        let entity = Proxy::new(url.0);
        Ok(entity.save().await?)
    }

    /// Update the URL of a specific proxy by ID. Requires root permission.
    #[oai(path = "/proxy/:id", method = "post", operation_id = "update_proxy")]
    async fn update_proxy(
        &self,
        id: Path<u64>,
        url: PlainText<String>,
        context: WrappedContext,
    ) -> ApiResult<()> {
        context.require_permission(None, Permission::ROOT).await?;
        Ok(Proxy::update(id.0, url.0).await?)
    }
    /// Get system configurations.
    ///
    /// Returns a read-only snapshot of the server configuration
    /// resolved at startup. Sensitive values are not exposed.
    #[oai(
        method = "get",
        path = "/system-configurations",
        operation_id = "get_system_configurations"
    )]
    async fn get_system_configurations(
        &self,
        context: WrappedContext,
    ) -> ApiResult<Json<SystemConfigurations>> {
        context.require_permission(None, Permission::ROOT).await?;
        let config: SystemConfigurations = SystemConfigurations::from(&*SETTINGS);
        Ok(Json(config))
    }
}
