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


use crate::modules::common::error::ErrorCapture;
use crate::modules::common::log::Tracing;
use crate::modules::common::tls::rustls_config;
use crate::modules::error::code::ErrorCode;
use crate::modules::error::handler::error_handler;
use crate::modules::error::BichonResult;
use crate::modules::rest::public::login::login;
use crate::modules::rest::public::status::get_status;
use crate::modules::{settings::cli::SETTINGS, utils::shutdown::shutdown_signal};

use super::error::ApiErrorResponse;
use crate::modules::common::auth::ApiGuard;
use crate::modules::common::timeout::{Timeout, TIMEOUT_HEADER};
use crate::raise_error;
use api::create_openapi_service;
use assets::FrontEndAssets;
use http::HeaderValue;
use poem::endpoint::EmbeddedFilesEndpoint;
use poem::listener::{Listener, TcpListener};
use poem::middleware::{CatchPanic, Compression, SetHeader};
use poem::{endpoint::EmbeddedFileEndpoint, middleware::Cors, EndpointExt, Route, Server};
use poem::{get, post};
use public::oauth2::oauth2_callback;
use std::time::Duration;

pub mod api;
pub mod assets;
pub mod public;
pub mod response;

pub type ApiResult<T, E = ApiErrorResponse> = std::result::Result<T, E>;

pub async fn start_http_server() -> BichonResult<()> {
    let listener = TcpListener::bind((
        SETTINGS.bichon_bind_ip.clone().unwrap_or("0.0.0.0".into()),
        SETTINGS.bichon_http_port as u16,
    ));

    let listener = if SETTINGS.bichon_enable_rest_https {
        listener.rustls(rustls_config()?).boxed()
    } else {
        listener.boxed()
    };

    let api_service = create_openapi_service()
        .summary("A self-hosted IMAP/SMTP middleware designed for developers");

    let swagger = api_service.swagger_ui();
    let redoc = api_service.redoc();
    let scalar = api_service.scalar();
    let spec_json = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();
    let openapi_explorer = api_service.openapi_explorer();

    let open_api_route = Route::new()
        .nest_no_strip("/api/v1", api_service)
        .with(ApiGuard)
        .with(ErrorCapture)
        .with(Timeout)
        .with(Tracing);

    let mut cors_origins = SETTINGS.bichon_cors_origins.clone();
    if cors_origins.is_empty() {
        cors_origins = ["*".to_string()].into_iter().collect();
    }

    let cache_static = || {
        SetHeader::new().overriding(
            http::header::CACHE_CONTROL,
            HeaderValue::from_static("max-age=86400"),
        )
    };

    let cors = Cors::new()
        .allow_origins(cors_origins)
        .allow_credentials(true)
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS", "HEAD"])
        .allow_headers(vec!["Content-Type", "Authorization", TIMEOUT_HEADER])
        .expose_headers(vec!["Accept"])
        .max_age(SETTINGS.bichon_cors_max_age);

    let route = Route::new()
        .nest("/api-docs/swagger", swagger)
        .nest("/api-docs/redoc", redoc)
        .nest("/api-docs/explorer", openapi_explorer)
        .nest("/api-docs/scalar", scalar)
        .nest("/api-docs/spec.json", spec_json)
        .nest("/api-docs/spec.yaml", spec_yaml)
        .nest("/oauth2/callback", get(oauth2_callback))
        .nest("/api/status", get(get_status))
        .nest("/api/login", post(login))
        .nest_no_strip("/api/v1", open_api_route)
        .nest_no_strip(
            "/assets",
            EmbeddedFilesEndpoint::<FrontEndAssets>::new().with(cache_static()),
        )
        .at(
            "/*",
            EmbeddedFileEndpoint::<FrontEndAssets>::new("index.html"),
        )
        .with(cors)
        .with_if(SETTINGS.bichon_http_compression_enabled, Compression::new())
        .with(CatchPanic::new());

    let server = Server::new(listener)
        .name("Bichon Service")
        .idle_timeout(Duration::from_secs(60))
        .run_with_graceful_shutdown(
            route.catch_all_error(error_handler),
            shutdown_signal(),
            Some(Duration::from_secs(5)),
        );
    println!(
        "Bichon Service is now running on port {}.",
        SETTINGS.bichon_http_port
    );
    server
        .await
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
}
