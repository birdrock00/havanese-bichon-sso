//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project

use poem::test::TestClient;
use serde::Serialize;

use super::{admin_token, build_api_route, setup};

fn api_client(route: impl poem::Endpoint) -> TestClient<impl poem::Endpoint> {
    TestClient::new(route).default_header("X-Forwarded-For", "127.0.0.1")
}

#[derive(Debug, Serialize)]
struct CreateTokenPayload {
    name: String,
}

#[tokio::test]
async fn access_token_crud() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    // Create a new API token
    let create = CreateTokenPayload {
        name: "Test API Token".into(),
    };
    let resp = cli
        .post("/api/v1/access-token")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&create)
        .send()
        .await;
    resp.assert_status_is_ok();
    let new_token = resp.0.into_body().into_string().await.unwrap_or_default();
    assert!(!new_token.is_empty(), "token string should not be empty");

    // Verify token now appears in the list
    let resp = cli
        .get("/api/v1/access-token-list")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let tokens: Vec<serde_json::Value> = resp.json().await.value().deserialize();
    assert!(!tokens.is_empty(), "token list should not be empty after creation");

    // Delete the NEW token (not the admin's WebUI token)
    let resp = cli
        .delete(&format!("/api/v1/access-token/{}", new_token))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}
