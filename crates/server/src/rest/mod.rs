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

use crate::common::error::ErrorCapture;
use crate::common::log::Tracing;
use crate::common::tls::rustls_config;
use crate::error::handler::error_handler;
use crate::rest::public::login::login;
use crate::rest::public::status::get_status;
use bichon_core::common::signal::SIGNAL_MANAGER;
use bichon_core::error::code::ErrorCode;
use bichon_core::error::BichonResult;
use bichon_core::settings::cli::SETTINGS;

use super::error::ApiErrorResponse;
use crate::common::auth::ApiGuard;
use crate::common::timeout::{Timeout, TIMEOUT_HEADER};
use api::create_openapi_service;
use assets::FrontEndAssets;
use bichon_core::raise_error;
use http::{HeaderValue, Method};
use poem::endpoint::EmbeddedFilesEndpoint;
use poem::listener::{Listener, TcpListener};
use poem::middleware::{CatchPanic, Compression, SetHeader};
use poem::{get, handler, post, IntoResponse};
use poem::{middleware::Cors, EndpointExt, Route, Server};
use public::oauth2::oauth2_callback;
use std::collections::HashSet;
use std::time::Duration;

pub mod api;
pub mod assets;
pub mod public;

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
        .summary("A lightweight, high-performance Rust email archiver with WebUI");

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

    let cors_origins: Option<HashSet<String>> = SETTINGS.bichon_cors_origins.clone();

    let cors_origins: Vec<String> = cors_origins.unwrap_or_default().into_iter().collect();

    let cors = Cors::new()
        .allow_origins_fn(move |origin| {
            tracing::debug!("CORS: Incoming Origin = {:?}", origin);
            tracing::debug!("CORS: Configured origins = {:?}", cors_origins);
            if cors_origins.is_empty() {
                tracing::debug!("CORS: No origins configured, allowing all");
                return true;
            }
            cors_origins.iter().any(|o| o == origin)
        })
        //.allow_origins(cors_origins)
        .allow_credentials(true)
        .allow_methods(&[
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::HEAD,
            Method::PATCH,
        ])
        .allow_headers(vec!["Content-Type", "Authorization", TIMEOUT_HEADER])
        .expose_headers(vec!["Accept"])
        .max_age(SETTINGS.bichon_cors_max_age);

    let cache_static = || {
        SetHeader::new().overriding(
            http::header::CACHE_CONTROL,
            HeaderValue::from_static("max-age=86400"),
        )
    };

    let app_logic = Route::new()
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
        .at("/*", serve_index_with_base);

    let route = Route::new()
        .nest(&SETTINGS.bichon_base_url, app_logic)
        .with(cors)
        .with_if(SETTINGS.bichon_http_compression_enabled, Compression::new())
        .with(CatchPanic::new());

    let mut rx = SIGNAL_MANAGER.subscribe();
    let shutdown_fut = async move {
        let _ = rx.recv().await;
    };
    let server = Server::new(listener)
        .name("Bichon Service")
        .idle_timeout(Duration::from_secs(60))
        .run_with_graceful_shutdown(
            route.catch_all_error(error_handler),
            shutdown_fut,
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

#[handler]
async fn serve_index_with_base() -> impl IntoResponse {
    let mut html =
        String::from_utf8_lossy(&FrontEndAssets::get("index.html").unwrap().data).to_string();

    let raw_base = &SETTINGS.bichon_base_url;
    let base_href = if raw_base.ends_with('/') {
        raw_base.clone()
    } else {
        format!("{}/", raw_base)
    };

    let inject_content = format!(
        r#"<base href="{}"><script>window.__BICHON_BASE__ = '{}';</script>"#,
        base_href, raw_base
    );

    html = html.replace("<head>", &format!("<head>{}", inject_content));
    poem::Response::builder()
        .content_type("text/html; charset=utf-8")
        .body(html)
}
