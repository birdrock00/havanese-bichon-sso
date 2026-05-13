//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project

use poem::test::TestClient;
use serde::Deserialize;

use super::{admin_token, build_api_route, setup};

fn api_client(route: impl poem::Endpoint) -> TestClient<impl poem::Endpoint> {
    TestClient::new(route).default_header("X-Forwarded-For", "127.0.0.1")
}

#[derive(Debug, Deserialize)]
struct Proxy {
    id: u64,
    url: String,
}

#[tokio::test]
async fn proxy_crud() {
    setup().await;
    let token = admin_token().await;
    let route = build_api_route();
    let cli = api_client(route);

    // Create proxy
    let resp = cli
        .post("/api/v1/proxy")
        .header("Authorization", &format!("Bearer {}", token))
        .content_type("text/plain")
        .body("socks5://127.0.0.1:1080")
        .send()
        .await;
    resp.assert_status_is_ok();

    // List proxies
    let resp = cli
        .get("/api/v1/list-proxy")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let proxies: Vec<Proxy> = resp.json().await.value().deserialize();
    assert!(!proxies.is_empty(), "should have at least one proxy");
    let proxy_id = proxies[0].id;

    // Get single proxy
    let resp = cli
        .get(&format!("/api/v1/proxy/{}", proxy_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();

    // Update proxy
    let resp = cli
        .post(&format!("/api/v1/proxy/{}", proxy_id))
        .header("Authorization", &format!("Bearer {}", token))
        .content_type("text/plain")
        .body("socks5://192.168.1.1:1080")
        .send()
        .await;
    resp.assert_status_is_ok();

    // Delete proxy
    let resp = cli
        .delete(&format!("/api/v1/proxy/{}", proxy_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}
