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

use std::collections::{BTreeMap, BTreeSet};

use poem::test::TestClient;
use poem::{get, EndpointExt, Route};
use serde::{Deserialize, Serialize};

use crate::common::{auth::ApiGuard, error::ErrorCapture};
use crate::rest::metrics::get_metrics;

use super::{admin_token, build_api_route, setup};

fn metrics_route() -> impl poem::Endpoint {
    Route::new()
        .at("/api/v1/metrics", get(get_metrics).with(ApiGuard))
        .with(ErrorCapture)
}

fn api_client(route: impl poem::Endpoint) -> TestClient<impl poem::Endpoint> {
    TestClient::new(route).default_header("X-Forwarded-For", "127.0.0.1")
}

#[derive(Debug, Serialize)]
struct LoginPayload {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResult {
    success: bool,
    access_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateRolePayload {
    name: String,
    role_type: String,
    permissions: BTreeSet<String>,
}

#[derive(Debug, Deserialize)]
struct RoleView {
    id: u64,
}

#[derive(Debug, Serialize)]
struct CreateUserPayload {
    username: String,
    email: String,
    password: String,
    global_roles: Vec<u64>,
    account_access_map: BTreeMap<u64, u64>,
}

#[derive(Debug, Deserialize)]
struct UserView {
    id: u64,
}

/// Log in with a username/password and return the access token.
async fn login(username: &str, password: &str) -> String {
    let route = Route::new().at("/api/login", poem::post(crate::rest::public::login::login));
    let cli = TestClient::new(route);
    let resp = cli
        .post("/api/login")
        .body_json(&LoginPayload {
            username: username.into(),
            password: password.into(),
        })
        .send()
        .await;
    resp.assert_status_is_ok();
    let result: LoginResult = resp.json().await.value().deserialize();
    assert!(result.success, "login failed for {username}");
    result.access_token.expect("access_token should be present")
}

#[tokio::test]
async fn metrics_endpoint_requires_a_token() {
    setup().await;
    let cli = api_client(metrics_route());

    let resp = cli.get("/api/v1/metrics").send().await;
    assert!(
        resp.0.status().is_client_error(),
        "metrics without a token must be rejected"
    );
}

#[tokio::test]
async fn admin_can_scrape_metrics() {
    setup().await;
    let token = admin_token().await;
    let cli = api_client(metrics_route());

    let resp = cli
        .get("/api/v1/metrics")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();

    let content_type = resp
        .0
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        content_type.starts_with("text/plain"),
        "unexpected content type: {content_type}"
    );

    let body = resp.0.into_body().into_string().await.unwrap_or_default();
    assert!(body.contains("# HELP bichon_build_info"));
    assert!(body.contains("# TYPE bichon_uptime_seconds gauge"));
    assert!(body.contains("bichon_accounts_total{type=\"imap\"}"));

    // Community purity: edition-specific families must never leak.
    assert!(!body.contains("bichon_edition_info"));
    assert!(!body.contains("bichon_audit_events_total"));
    assert!(!body.contains("bichon_legal_holds_total"));
}

#[tokio::test]
async fn token_without_system_metrics_is_forbidden() {
    setup().await;
    let admin = admin_token().await;
    let cli = api_client(build_api_route());

    // A global role with only the basic platform permission.
    let mut permissions = BTreeSet::new();
    permissions.insert("system:access".to_string());
    let resp = cli
        .post("/api/v1/roles")
        .header("Authorization", &format!("Bearer {}", admin))
        .body_json(&CreateRolePayload {
            name: "metrics-test-role".into(),
            role_type: "Global".into(),
            permissions,
        })
        .send()
        .await;
    resp.assert_status_is_ok();
    let role: RoleView = resp.json().await.value().deserialize();

    let username = "metrics_test_user";
    let resp = cli
        .post("/api/v1/users")
        .header("Authorization", &format!("Bearer {}", admin))
        .body_json(&CreateUserPayload {
            username: username.into(),
            email: "metrics-test-user@example.com".into(),
            password: "metrics-test-pass".into(),
            global_roles: vec![role.id],
            account_access_map: BTreeMap::new(),
        })
        .send()
        .await;
    resp.assert_status_is_ok();
    let user: UserView = resp.json().await.value().deserialize();

    let token = login(username, "metrics-test-pass").await;
    let metrics = api_client(metrics_route());
    let resp = metrics
        .get("/api/v1/metrics")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    assert_eq!(
        resp.0.status(),
        poem::http::StatusCode::FORBIDDEN,
        "a valid token without system:metrics must be forbidden"
    );

    // Clean up so a rerun against the same test root starts clean.
    let resp = cli
        .delete(&format!("/api/v1/users/{}", user.id))
        .header("Authorization", &format!("Bearer {}", admin))
        .send()
        .await;
    resp.assert_status_is_ok();
    let resp = cli
        .delete(&format!("/api/v1/roles/{}", role.id))
        .header("Authorization", &format!("Bearer {}", admin))
        .send()
        .await;
    resp.assert_status_is_ok();
}

#[derive(Debug, Serialize)]
struct CreateTokenPayload {
    user_id: u64,
    name: String,
    metrics_only: bool,
}

/// A metrics-only token keeps the API-token lifecycle, is accepted on the
/// metrics endpoint, and is rejected everywhere else — even on endpoints its
/// owner could otherwise call.
#[tokio::test]
async fn metrics_only_token_is_scoped_to_the_metrics_endpoint() {
    setup().await;
    let admin = admin_token().await;
    let admin_cli = api_client(build_api_route());

    let mut good_permissions = BTreeSet::new();
    good_permissions.insert("system:access".to_string());
    good_permissions.insert("system:metrics".to_string());
    let resp = admin_cli
        .post("/api/v1/roles")
        .header("Authorization", &format!("Bearer {}", admin))
        .body_json(&CreateRolePayload {
            name: "metrics-scope-role".into(),
            role_type: "Global".into(),
            permissions: good_permissions,
        })
        .send()
        .await;
    resp.assert_status_is_ok();
    let good_role: RoleView = resp.json().await.value().deserialize();

    let mut bad_permissions = BTreeSet::new();
    bad_permissions.insert("system:access".to_string());
    let resp = admin_cli
        .post("/api/v1/roles")
        .header("Authorization", &format!("Bearer {}", admin))
        .body_json(&CreateRolePayload {
            name: "metrics-scope-role-bad".into(),
            role_type: "Global".into(),
            permissions: bad_permissions,
        })
        .send()
        .await;
    resp.assert_status_is_ok();
    let bad_role: RoleView = resp.json().await.value().deserialize();

    let resp = admin_cli
        .post("/api/v1/users")
        .header("Authorization", &format!("Bearer {}", admin))
        .body_json(&CreateUserPayload {
            username: "metrics_scope_user".into(),
            email: "metrics-scope-user@example.com".into(),
            password: "metrics-scope-pass".into(),
            global_roles: vec![good_role.id],
            account_access_map: BTreeMap::new(),
        })
        .send()
        .await;
    resp.assert_status_is_ok();
    let good_user: UserView = resp.json().await.value().deserialize();

    let resp = admin_cli
        .post("/api/v1/users")
        .header("Authorization", &format!("Bearer {}", admin))
        .body_json(&CreateUserPayload {
            username: "metrics_scope_user_bad".into(),
            email: "metrics-scope-user-bad@example.com".into(),
            password: "metrics-scope-pass".into(),
            global_roles: vec![bad_role.id],
            account_access_map: BTreeMap::new(),
        })
        .send()
        .await;
    resp.assert_status_is_ok();
    let bad_user: UserView = resp.json().await.value().deserialize();

    // Creating a metrics-only token for a user without system:metrics fails.
    let resp = admin_cli
        .post("/api/v1/access-token")
        .header("Authorization", &format!("Bearer {}", admin))
        .body_json(&CreateTokenPayload {
            user_id: bad_user.id,
            name: "forbidden-metrics-token".into(),
            metrics_only: true,
        })
        .send()
        .await;
    assert_eq!(
        resp.0.status(),
        poem::http::StatusCode::FORBIDDEN,
        "metrics-only tokens require the owner to hold system:metrics"
    );

    // The permitted owner gets a working metrics token...
    let resp = admin_cli
        .post("/api/v1/access-token")
        .header("Authorization", &format!("Bearer {}", admin))
        .body_json(&CreateTokenPayload {
            user_id: good_user.id,
            name: "prometheus".into(),
            metrics_only: true,
        })
        .send()
        .await;
    resp.assert_status_is_ok();
    let scoped_token = resp.0.into_body().into_string().await.unwrap_or_default();
    assert!(!scoped_token.is_empty(), "token creation must return the token");

    // ...accepted on the metrics endpoint...
    let metrics = api_client(metrics_route());
    let resp = metrics
        .get("/api/v1/metrics")
        .header("Authorization", &format!("Bearer {}", scoped_token))
        .send()
        .await;
    resp.assert_status_is_ok();

    // ...and rejected on a normal endpoint, even one requiring no permission.
    let api = api_client(build_api_route());
    let resp = api
        .get("/api/v1/current-user")
        .header("Authorization", &format!("Bearer {}", scoped_token))
        .send()
        .await;
    assert!(
        resp.0.status().is_client_error(),
        "a metrics-only token must not authenticate on other endpoints"
    );

    // Clean up.
    for user in [&good_user, &bad_user] {
        let resp = admin_cli
            .delete(&format!("/api/v1/users/{}", user.id))
            .header("Authorization", &format!("Bearer {}", admin))
            .send()
            .await;
        resp.assert_status_is_ok();
    }
    for role in [&good_role, &bad_role] {
        let resp = admin_cli
            .delete(&format!("/api/v1/roles/{}", role.id))
            .header("Authorization", &format!("Bearer {}", admin))
            .send()
            .await;
        resp.assert_status_is_ok();
    }
}
