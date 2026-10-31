//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project

//! Pins the shape of the source-mailbox cleanup API.
//!
//! Cleanup previews and moves run as background jobs because verifying every
//! candidate over IMAP can outlast any HTTP deadline. These assertions guard the
//! two halves of that contract: every start endpoint answers with a job (never
//! with the finished result), and the poll/cancel endpoints the UI depends on
//! exist in the document.

use poem_openapi::OpenApiService;

use crate::rest::api::{create_openapi_service, RustMailOpenApi};

fn spec() -> serde_json::Value {
    let service: OpenApiService<RustMailOpenApi, ()> = create_openapi_service();
    serde_json::from_str(&service.spec()).expect("the OpenAPI spec must serialize")
}

#[test]
fn cleanup_jobs_are_pollable_endpoints() {
    let doc = spec();
    let paths = doc["paths"].as_object().expect("paths");
    for path in [
        "/api/v1/accounts/{account_id}/remote-cleanup/jobs",
        "/api/v1/accounts/{account_id}/remote-cleanup/jobs/{job_id}",
        "/api/v1/accounts/{account_id}/remote-cleanup/jobs/{job_id}/cancel",
    ] {
        assert!(paths.contains_key(path), "missing cleanup job path {path}");
    }
    // A client that lost its job id (reload, remounted dialog) reattaches
    // through this listing instead of starting a second job, which the server
    // refuses while the first one runs.
    assert_eq!(
        paths["/api/v1/accounts/{account_id}/remote-cleanup/jobs"]["get"]["operationId"],
        "accounts_remote_cleanup_jobs"
    );
    assert_eq!(
        paths["/api/v1/accounts/{account_id}/remote-cleanup/jobs/{job_id}"]["get"]["operationId"],
        "accounts_remote_cleanup_job"
    );
    assert_eq!(
        paths["/api/v1/accounts/{account_id}/remote-cleanup/jobs/{job_id}/cancel"]["post"]
            ["operationId"],
        "accounts_remote_cleanup_job_cancel"
    );
}

#[test]
fn cleanup_start_endpoints_return_a_job() {
    let doc = spec();
    let paths = doc["paths"].as_object().expect("paths");
    for path in [
        "/api/v1/accounts/{account_id}/remote-cleanup/preview",
        "/api/v1/accounts/{account_id}/remote-cleanup/execute",
    ] {
        let operation = paths
            .get(path)
            .and_then(|entry| entry.get("post"))
            .unwrap_or_else(|| panic!("missing cleanup start path {path}"));
        // poem-openapi emits the payload as a `$ref` to the component schema,
        // so look at both the inline schema and the referenced name.
        let response = &operation["responses"]["200"];
        let schema = response["content"]["application/json"]["schema"].to_string();
        let described = format!("{response}");
        assert!(
            schema.contains("CleanupJob") || described.contains("CleanupJob"),
            "{path} must answer with a CleanupJob, got {described}"
        );
    }
}

#[test]
fn a_cleanup_job_exposes_its_result_and_error() {
    let doc = spec();
    let job = &doc["components"]["schemas"]["CleanupJob"];
    let properties = job["properties"].as_object().expect("CleanupJob properties");
    for field in ["job_id", "account_id", "kind", "status", "preview", "run", "error"] {
        assert!(
            properties.contains_key(field),
            "CleanupJob is missing '{field}'"
        );
    }
}

#[test]
fn run_reports_name_the_messages_they_did_not_move() {
    let doc = spec();
    let schemas = &doc["components"]["schemas"];

    let message = schemas["RemoteCleanupSkippedMessage"]["properties"]
        .as_object()
        .expect("RemoteCleanupSkippedMessage properties");
    for field in ["folder", "uid", "message_id", "subject", "reason", "failed", "size"] {
        assert!(
            message.contains_key(field),
            "RemoteCleanupSkippedMessage is missing '{field}'"
        );
    }

    // Counts alone cannot answer "which messages?"; both report shapes must
    // carry the list.
    let folder = schemas["RemoteCleanupFolderStats"]["properties"]
        .as_object()
        .expect("RemoteCleanupFolderStats properties");
    assert!(
        folder.contains_key("unprocessed"),
        "a move run's per-folder stats must name the skipped/failed messages"
    );
}
