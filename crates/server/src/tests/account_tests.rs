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

// ── Payloads ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct CreateAccountPayload {
    email: String,
    enabled: bool,
    account_type: String,
    use_dangerous: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AccountResp {
    id: u64,
    email: String,
    enabled: bool,
    account_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DataPage<T> {
    items: Vec<T>,
    total_items: u64,
}

#[derive(Debug, Serialize)]
struct UpdateAccountPayload {
    enabled: Option<bool>,
    account_name: Option<String>,
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn account_crud() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    // ── Create ──────────────────────────────────────────────────────────
    let create_payload = CreateAccountPayload {
        email: "test-crud@example.com".into(),
        enabled: false,
        account_type: "NoSync".into(),
        use_dangerous: false,
        account_name: Some("CRUD Test Account".into()),
    };

    let resp = cli
        .post("/api/v1/account")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&create_payload)
        .send()
        .await;

    resp.assert_status_is_ok();
    let account: AccountResp = resp.json().await.value().deserialize();
    assert_eq!(account.email, "test-crud@example.com");
    assert!(!account.enabled);
    let account_id = account.id;

    // ── Read ────────────────────────────────────────────────────────────
    let resp = cli
        .get(&format!("/api/v1/account/{}", account_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let account: AccountResp = resp.json().await.value().deserialize();
    assert_eq!(account.id, account_id);

    // ── List ────────────────────────────────────────────────────────────
    let resp = cli
        .get("/api/v1/accounts")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();

    // ── Update ──────────────────────────────────────────────────────────
    let update_payload = UpdateAccountPayload {
        enabled: Some(true),
        account_name: Some("Updated Name".into()),
    };
    let resp = cli
        .post(&format!("/api/v1/account/{}", account_id))
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&update_payload)
        .send()
        .await;
    resp.assert_status_is_ok();

    // ── Delete ──────────────────────────────────────────────────────────
    let resp = cli
        .delete(&format!("/api/v1/account/{}", account_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();

    // ── Verify deleted ──────────────────────────────────────────────────
    let resp = cli
        .get(&format!("/api/v1/account/{}", account_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "should be 4xx after delete");
}

#[tokio::test]
async fn create_account_with_invalid_email_fails() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let payload = CreateAccountPayload {
        email: "not-an-email".into(),
        enabled: false,
        account_type: "NoSync".into(),
        use_dangerous: false,
        account_name: None,
    };

    let resp = cli
        .post("/api/v1/account")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&payload)
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "invalid email should fail");
}

#[tokio::test]
async fn create_account_with_empty_email_fails() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let payload = CreateAccountPayload {
        email: "".into(),
        enabled: false,
        account_type: "NoSync".into(),
        use_dangerous: false,
        account_name: None,
    };

    let resp = cli
        .post("/api/v1/account")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&payload)
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "empty email should fail");
}

#[tokio::test]
async fn get_nonexistent_account_returns_error() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let resp = cli
        .get("/api/v1/account/99999999")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "nonexistent account should 4xx");
}

#[tokio::test]
async fn delete_nonexistent_account_returns_error() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    let resp = cli
        .delete("/api/v1/account/99999999")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    assert!(resp.0.status().is_client_error(), "delete nonexistent should 4xx");
}
