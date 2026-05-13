//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project

pub mod access_token_tests;
pub mod account_tests;
pub mod oauth2_tests;
pub mod proxy_tests;
pub mod role_tests;
pub mod system_tests;
pub mod user_tests;

use std::{
    path::PathBuf,
    sync::{
        LazyLock,
        Mutex,
    },
};

use bichon_core::{
    common::signal::SignalManager,
    context::{executors::BichonContext, Initialize},
    settings::{
        cli::SETTINGS,
        dir::DataDirManager,
    },
    store::{
        blob::BLOB_MANAGER,
        tantivy::{attachment::ATTACHMENT_MANAGER, envelope::ENVELOPE_MANAGER},
    },
    users::manager::UserManager,
};
use poem::{EndpointExt, Route};
use serde::{Deserialize, Serialize};

use crate::{
    common::{
        auth::ApiGuard,
        error::ErrorCapture,
        log::Tracing,
        timeout::Timeout,
    },
    rest::api::create_openapi_service,
};

static INIT: Mutex<bool> = Mutex::new(false);

/// Initialize the test environment. Safe to call multiple times — only runs once.
pub async fn setup() {
    let mut initialized = INIT.lock().unwrap();
    if *initialized {
        drop(initialized);
        return;
    }

    let root = PathBuf::from(&SETTINGS.bichon_root_dir);
    if root.exists() {
        let _ = std::fs::remove_dir_all(&root);
    }

    SignalManager::initialize().await.unwrap();
    DataDirManager::initialize().await.unwrap();
    UserManager::initialize().await.unwrap();
    BichonContext::initialize().await.unwrap();
    LazyLock::force(&BLOB_MANAGER);
    LazyLock::force(&ENVELOPE_MANAGER);
    LazyLock::force(&ATTACHMENT_MANAGER);

    *initialized = true;
}

/// Build the full API route (same middleware stack as production).
pub fn build_api_route() -> impl poem::Endpoint {
    let api_service = create_openapi_service();
    Route::new()
        .nest_no_strip("/api/v1", api_service)
        .with(ApiGuard)
        .with(ErrorCapture)
        .with(Timeout)
        .with(Tracing)
}

// ── Shared types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct LoginPayload {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResult {
    success: bool,
    #[allow(dead_code)]
    error_message: Option<String>,
    access_token: Option<String>,
    #[allow(dead_code)]
    theme: Option<String>,
    #[allow(dead_code)]
    language: Option<String>,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

static TOKEN_CACHE: Mutex<Option<String>> = Mutex::new(None);

/// Login as admin and return the access token.  The token is cached so
/// multiple callers share the same token (each call to
/// `reset_webui_token` would invalidate the previous one).
pub async fn admin_token() -> String {
    let mut cache = TOKEN_CACHE.lock().unwrap();
    if let Some(ref token) = *cache {
        return token.clone();
    }

    let login_route = poem::Route::new()
        .at("/api/login", poem::post(crate::rest::public::login::login));

    let cli = poem::test::TestClient::new(login_route);
    let resp = cli
        .post("/api/login")
        .body_json(&LoginPayload {
            username: "admin".into(),
            password: "admin@bichon".into(),
        })
        .send()
        .await;

    resp.assert_status_is_ok();
    let result: LoginResult = resp.json().await.value().deserialize();
    assert!(result.success, "Admin login failed");
    let token = result.access_token.expect("access_token should be present");
    *cache = Some(token.clone());
    token
}

// ── Auth / Login Tests ─────────────────────────────────────────────────────

#[tokio::test]
async fn login_with_wrong_password_fails() {
    setup().await;
    let login_route = poem::Route::new()
        .at("/api/login", poem::post(crate::rest::public::login::login));

    let cli = poem::test::TestClient::new(login_route);
    let resp = cli
        .post("/api/login")
        .body_json(&LoginPayload {
            username: "admin".into(),
            password: "wrong-password".into(),
        })
        .send()
        .await;

    resp.assert_status_is_ok();
    let result: LoginResult = resp.json().await.value().deserialize();
    assert!(!result.success);
}

#[tokio::test]
async fn login_with_nonexistent_user_fails() {
    setup().await;
    let login_route = poem::Route::new()
        .at("/api/login", poem::post(crate::rest::public::login::login));

    let cli = poem::test::TestClient::new(login_route);
    let resp = cli
        .post("/api/login")
        .body_json(&LoginPayload {
            username: "nonexistent".into(),
            password: "whatever".into(),
        })
        .send()
        .await;

    resp.assert_status_is_ok();
    let result: LoginResult = resp.json().await.value().deserialize();
    assert!(!result.success);
}

#[tokio::test]
async fn protected_endpoint_requires_auth() {
    setup().await;
    let route = build_api_route();
    let cli = poem::test::TestClient::new(route)
        .default_header("X-Forwarded-For", "127.0.0.1");

    // Without auth header — should fail (4xx)
    let resp = cli.get("/api/v1/list-roles").send().await;
    assert!(
        resp.0.status().is_client_error(),
        "expected 4xx for missing auth"
    );

    // With invalid token — should fail (4xx)
    let resp = cli
        .get("/api/v1/list-roles")
        .header("Authorization", "Bearer invalid-token-here")
        .send()
        .await;
    assert!(
        resp.0.status().is_client_error(),
        "expected 4xx for invalid token"
    );

    // With valid admin token — should succeed
    let token = admin_token().await;
    let resp = cli
        .get("/api/v1/list-roles")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}
