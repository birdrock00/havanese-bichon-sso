//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project

use poem::test::TestClient;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::{admin_token, build_api_route, setup};

fn api_client(route: impl poem::Endpoint) -> TestClient<impl poem::Endpoint> {
    TestClient::new(route).default_header("X-Forwarded-For", "127.0.0.1")
}

#[derive(Debug, Deserialize)]
struct UserRole {
    id: u64,
    name: String,
    is_builtin: bool,
    permissions: BTreeSet<String>,
    role_type: String,
}

#[derive(Debug, Serialize)]
struct CreateRolePayload {
    name: String,
    role_type: String,
    permissions: BTreeSet<String>,
}

#[derive(Debug, Serialize)]
struct UpdateRolePayload {
    name: Option<String>,
}

const ADMIN_ROLE_ID: u64 = 100_000_000_000_000;

#[tokio::test]
async fn role_crud() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    // List roles (5 built-in roles exist)
    let resp = cli
        .get("/api/v1/list-roles")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let roles: Vec<UserRole> = resp.json().await.value().deserialize();
    assert!(roles.len() >= 5, "should have at least 5 built-in roles");
    assert!(roles.iter().any(|r| r.name == "admin"));
    assert!(roles.iter().any(|r| r.name == "manager"));
    assert!(roles.iter().any(|r| r.name == "member"));

    // Create custom role
    let mut perms = BTreeSet::new();
    perms.insert("user:view".into());
    let create = CreateRolePayload {
        name: "test-role".into(),
        role_type: "Global".into(),
        permissions: perms,
    };
    let resp = cli
        .post("/api/v1/roles")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&create)
        .send()
        .await;
    resp.assert_status_is_ok();
    let role: UserRole = resp.json().await.value().deserialize();
    assert_eq!(role.name, "test-role");
    assert!(!role.is_builtin);
    let role_id = role.id;

    // Update custom role
    let update = UpdateRolePayload {
        name: Some("test-role-updated".into()),
    };
    let resp = cli
        .post(&format!("/api/v1/roles/{}", role_id))
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&update)
        .send()
        .await;
    resp.assert_status_is_ok();

    // Delete custom role
    let resp = cli
        .delete(&format!("/api/v1/roles/{}", role_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}

#[tokio::test]
async fn cannot_delete_builtin_role() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let resp = cli
        .delete(&format!("/api/v1/roles/{}", ADMIN_ROLE_ID))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "deleting builtin role should fail");
}
