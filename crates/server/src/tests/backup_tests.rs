//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// These tests exercise the backup REST surface. The test environment has
// backups disabled (no `backup.enabled` in system_config → false), so the
// backup-facing endpoints assert the clean "disabled" 400 rather than trying
// to run a real backup. Access control (admin / `backup:manage` /
// no permission) is exercised against the always-available `status` endpoint.

use poem::test::TestClient;
use poem::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use super::{admin_token, build_api_route, setup};

fn api_client(route: impl poem::Endpoint) -> TestClient<impl poem::Endpoint> {
    TestClient::new(route).default_header("X-Forwarded-For", "127.0.0.1")
}

// ── Payloads / views ────────────────────────────────────────────────────────

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

const MEMBER_ROLE_ID: u64 = 100_200_000_000_000;

// ── Helpers ─────────────────────────────────────────────────────────────────

async fn login(username: &str, password: &str) -> String {
    let login_route = poem::Route::new()
        .at("/api/login", poem::post(crate::rest::public::login::login));
    let cli = poem::test::TestClient::new(login_route);
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

async fn create_role_with_backup_manage(token: &str) -> u64 {
    let cli = api_client(build_api_route());
    let mut perms = BTreeSet::new();
    perms.insert("backup:manage".into());
    let payload = CreateRolePayload {
        name: "backup-operator".into(),
        role_type: "Global".into(),
        permissions: perms,
    };
    let resp = cli
        .post("/api/v1/roles")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&payload)
        .send()
        .await;
    resp.assert_status_is_ok();
    let role: RoleView = resp.json().await.value().deserialize();
    role.id
}

async fn create_user(token: &str, username: &str, global_roles: Vec<u64>) {
    let cli = api_client(build_api_route());
    let payload = CreateUserPayload {
        username: username.into(),
        email: format!("{username}@example.com"),
        password: "testpass123".into(),
        global_roles,
        account_access_map: Default::default(),
    };
    let resp = cli
        .post("/api/v1/users")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(&payload)
        .send()
        .await;
    resp.assert_status_is_ok();
}

// ── Access control ──────────────────────────────────────────────────────────

#[tokio::test]
async fn backup_status_ok_for_admin() {
    setup().await;
    let token = admin_token().await;
    let cli = api_client(build_api_route());
    let resp = cli
        .get("/api/v1/backup/status")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
}

#[tokio::test]
async fn backup_status_forbidden_without_backup_permission() {
    setup().await;
    let token = admin_token().await;
    create_user(&token, "backup-denied-user", vec![MEMBER_ROLE_ID]).await;
    let member_token = login("backup-denied-user", "testpass123").await;

    let cli = api_client(build_api_route());
    let resp = cli
        .get("/api/v1/backup/status")
        .header("Authorization", &format!("Bearer {}", member_token))
        .send()
        .await;
    assert_eq!(
        resp.0.status(),
        StatusCode::FORBIDDEN,
        "a member without backup:manage must be rejected"
    );
}

#[tokio::test]
async fn backup_status_allowed_with_backup_manage_permission() {
    setup().await;
    let token = admin_token().await;
    let role_id = create_role_with_backup_manage(&token).await;
    create_user(&token, "backup-operator-user", vec![role_id]).await;
    let op_token = login("backup-operator-user", "testpass123").await;

    let cli = api_client(build_api_route());
    let resp = cli
        .get("/api/v1/backup/status")
        .header("Authorization", &format!("Bearer {}", op_token))
        .send()
        .await;
    resp.assert_status_is_ok();
}

// ── Management endpoints ────────────────────────────────────────────────────

#[tokio::test]
async fn backup_records_returns_empty_array_when_none_exist() {
    setup().await;
    let token = admin_token().await;
    let cli = api_client(build_api_route());
    let resp = cli
        .get("/api/v1/backup/records")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
    let records: Vec<serde_json::Value> = resp.json().await.value().deserialize();
    assert!(
        records.is_empty(),
        "a fresh test DB must have no backup records"
    );
}

#[tokio::test]
async fn backup_run_rejected_when_disabled() {
    setup().await;
    let token = admin_token().await;
    let cli = api_client(build_api_route());
    let resp = cli
        .post("/api/v1/backup/run")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    assert_eq!(
        resp.0.status(),
        StatusCode::BAD_REQUEST,
        "backups are disabled by default in the test environment"
    );
}

// ── Manifest-browser endpoints (disabled in tests) ──────────────────────────

#[tokio::test]
async fn backup_manifests_rejected_when_disabled() {
    setup().await;
    let token = admin_token().await;
    let cli = api_client(build_api_route());
    let resp = cli
        .get("/api/v1/backup/manifests")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    assert_eq!(
        resp.0.status(),
        StatusCode::BAD_REQUEST,
        "manifests require the backup feature to be enabled"
    );
}

#[tokio::test]
async fn backup_manifest_by_id_rejected_when_disabled() {
    setup().await;
    let token = admin_token().await;
    let cli = api_client(build_api_route());
    let resp = cli
        .get("/api/v1/backup/manifests/m-whatever")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    assert_eq!(
        resp.0.status(),
        StatusCode::BAD_REQUEST,
        "manifest detail requires the backup feature to be enabled"
    );
}

// ── WebUI-configurable backup configuration ────────────────────────────────

#[derive(Debug, Default, Serialize)]
struct BackupConfigUpdatePayload {
    enabled: Option<bool>,
    schedule: Option<String>,
    prefix: Option<String>,
    retention: Option<serde_json::Value>,
    s3_endpoint: Option<String>,
    s3_region: Option<String>,
    s3_bucket: Option<String>,
    s3_access_key: Option<String>,
    s3_secret_key: Option<String>,
}

/// The config tests write the same process-global config document + cache
/// (and never flip `backup.enabled`, which would race `backup_run_rejected_
/// when_disabled`), so they must not run concurrently with each other.
static CONFIG_TEST_LOCK: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));

/// Restore the store to its code-default baseline after a config test: drop
/// the config document and the in-memory cache.
fn cleanup_backup_config() {
    let _ = bichon_core::backup::config::clear();
}

async fn get_backup_config_raw(token: &str) -> serde_json::Value {
    let cli = api_client(build_api_route());
    let resp = cli
        .get("/api/v1/backup/config")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.json().await.value().deserialize()
}

async fn post_backup_config(
    token: &str,
    payload: &BackupConfigUpdatePayload,
) -> poem::http::StatusCode {
    let cli = api_client(build_api_route());
    let resp = cli
        .post("/api/v1/backup/config")
        .header("Authorization", &format!("Bearer {}", token))
        .body_json(payload)
        .send()
        .await;
    resp.0.status()
}

#[tokio::test]
async fn backup_config_get_ok_for_admin_and_masks_secrets() {
    setup().await;
    let _guard = CONFIG_TEST_LOCK.lock().await;
    let token = admin_token().await;
    cleanup_backup_config();

    let raw = get_backup_config_raw(&token).await;
    assert_eq!(raw["prefix"], "bichon-backup", "unconfigured prefix is the default");
    assert_eq!(raw["retention"]["keep_last"], 7, "default retention policy");
    assert!(
        raw.get("s3_secret_key").is_none(),
        "secrets must never be serialized: {raw}"
    );
    assert!(
        raw.get("password").is_none() && raw.get("local_dir").is_none(),
        "unexpected fields in the S3-only form: {raw}"
    );

    cleanup_backup_config();
}

#[tokio::test]
async fn backup_config_roundtrips_s3_target_with_secret_semantics() {
    setup().await;
    let _guard = CONFIG_TEST_LOCK.lock().await;
    let token = admin_token().await;
    cleanup_backup_config();

    // Configure an s3 target with prefix and retention.
    let status = post_backup_config(
        &token,
        &BackupConfigUpdatePayload {
            schedule: Some("0 0 2 * * *".into()),
            prefix: Some("prod-archive".into()),
            retention: Some(serde_json::json!({
                "keep_last": 14, "keep_daily": 10, "keep_weekly": 4, "keep_monthly": 6
            })),
            s3_endpoint: Some("http://localhost:9000".into()),
            s3_bucket: Some("bichon".into()),
            s3_region: Some("us-east-1".into()),
            s3_access_key: Some("minioadmin".into()),
            s3_secret_key: Some("minioadmin123".into()),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let raw = get_backup_config_raw(&token).await;
    assert_eq!(raw["schedule"], "0 0 2 * * *");
    assert_eq!(raw["prefix"], "prod-archive");
    assert_eq!(raw["retention"]["keep_last"], 14);
    assert_eq!(raw["retention"]["keep_daily"], 10);
    assert_eq!(raw["s3_endpoint"], "http://localhost:9000");
    assert_eq!(raw["s3_bucket"], "bichon");
    assert_eq!(raw["s3_access_key_set"], true);
    assert_eq!(raw["s3_secret_key_set"], true);
    assert!(raw.get("s3_access_key").is_none());
    assert!(raw.get("s3_secret_key").is_none());

    // '********' keeps the stored secret (access_key_set stays true).
    let status = post_backup_config(
        &token,
        &BackupConfigUpdatePayload {
            schedule: Some("0 0 4 * * *".into()),
            s3_secret_key: Some("********".into()),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let raw = get_backup_config_raw(&token).await;
    assert_eq!(raw["schedule"], "0 0 4 * * *");
    assert_eq!(raw["s3_secret_key_set"], true, "keep must not clear the secret");

    // '' clears the page override → falls back to env (unset).
    let status = post_backup_config(
        &token,
        &BackupConfigUpdatePayload {
            s3_secret_key: Some("".into()),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let raw = get_backup_config_raw(&token).await;
    assert_eq!(raw["s3_secret_key_set"], false, "clearing must unset the secret");

    cleanup_backup_config();
}

#[tokio::test]
async fn backup_config_s3_validation_and_unknown_fields() {
    setup().await;
    let _guard = CONFIG_TEST_LOCK.lock().await;
    let token = admin_token().await;
    cleanup_backup_config();

    // Without a bucket the target is rejected up front.
    let status = post_backup_config(
        &token,
        &BackupConfigUpdatePayload {
            s3_endpoint: Some("http://localhost:9000".into()),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "s3 without a bucket must be rejected"
    );

    // Unknown fields are ignored (the form no longer accepts target_type).
    let status = post_backup_config(
        &token,
        &BackupConfigUpdatePayload {
            s3_endpoint: Some("http://localhost:9000".into()),
            s3_bucket: Some("bichon".into()),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    cleanup_backup_config();
}

#[tokio::test]
async fn backup_config_forbidden_without_backup_permission() {
    setup().await;
    let token = admin_token().await;
    create_user(&token, "backup-config-denied-user", vec![MEMBER_ROLE_ID]).await;
    let member_token = login("backup-config-denied-user", "testpass123").await;

    let status = post_backup_config(
        &member_token,
        &BackupConfigUpdatePayload {
            s3_bucket: Some("bichon".into()),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let cli = api_client(build_api_route());
    let resp = cli
        .get("/api/v1/backup/config")
        .header("Authorization", &format!("Bearer {}", member_token))
        .send()
        .await;
    assert_eq!(resp.0.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn backup_config_allowed_with_backup_manage_permission() {
    setup().await;
    let _guard = CONFIG_TEST_LOCK.lock().await;
    let token = admin_token().await;
    let role_id = create_role_with_backup_manage(&token).await;
    create_user(&token, "backup-config-operator-user", vec![role_id]).await;
    let op_token = login("backup-config-operator-user", "testpass123").await;

    let status = post_backup_config(
        &op_token,
        &BackupConfigUpdatePayload {
            s3_bucket: Some("bichon".into()),
            s3_endpoint: Some("http://localhost:9000".into()),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(status, StatusCode::OK, "backup:manage must grant config access");

    cleanup_backup_config();
}
