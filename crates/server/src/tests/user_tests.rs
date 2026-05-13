//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project

use poem::test::TestClient;
use serde::{Deserialize, Serialize};

use super::{admin_token, build_api_route, setup};

fn api_client(route: impl poem::Endpoint) -> TestClient<impl poem::Endpoint> {
    TestClient::new(route).default_header("X-Forwarded-For", "127.0.0.1")
}

#[derive(Debug, Serialize)]
struct CreateUserPayload {
    username: String,
    email: String,
    password: String,
    global_roles: Vec<u64>,
    account_access_map: std::collections::BTreeMap<u64, u64>,
}

#[derive(Debug, Deserialize)]
struct UserView {
    id: u64,
    username: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct UpdateUserPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

const ADMIN_ROLE_ID: u64 = 100_000_000_000_000;

#[tokio::test]
async fn user_crud() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    // Create
    let create_payload = CreateUserPayload {
        username: "testuser1".into(),
        email: "testuser1@example.com".into(),
        password: "testpass123".into(),
        global_roles: vec![ADMIN_ROLE_ID],
        account_access_map: Default::default(),
    };
    let resp = cli
        .post("/api/v1/users")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&create_payload)
        .send()
        .await;
    resp.assert_status_is_ok();
    let user: UserView = resp.json().await.value().deserialize();
    assert_eq!(user.username, "testuser1");
    let user_id = user.id;

    // List users
    let resp = cli
        .get("/api/v1/list-users")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();

    // Get current user
    let resp = cli
        .get("/api/v1/current-user")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();

    // Update
    let update = UpdateUserPayload {
        description: Some("Test description".into()),
    };
    let resp = cli
        .post(&format!("/api/v1/users/{}", user_id))
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&update)
        .send()
        .await;
    resp.assert_status_is_ok();

    // Delete
    let resp = cli
        .delete(&format!("/api/v1/users/{}", user_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}

#[tokio::test]
async fn create_user_with_invalid_data_fails() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    // Username too short (min 3)
    let payload = CreateUserPayload {
        username: "ab".into(),
        email: "valid@example.com".into(),
        password: "testpass123".into(),
        global_roles: vec![ADMIN_ROLE_ID],
        account_access_map: Default::default(),
    };
    let resp = cli
        .post("/api/v1/users")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&payload)
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "short username should fail");

    // Password too short (min 8)
    let payload = CreateUserPayload {
        username: "validuser".into(),
        email: "valid@example.com".into(),
        password: "short".into(),
        global_roles: vec![ADMIN_ROLE_ID],
        account_access_map: Default::default(),
    };
    let resp = cli
        .post("/api/v1/users")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&payload)
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "short password should fail");
}

#[tokio::test]
async fn cannot_delete_default_admin() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let resp = cli
        .delete("/api/v1/users/100000000000000")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "deleting admin should fail");
}
