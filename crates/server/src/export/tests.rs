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

use super::*;
use std::sync::LazyLock;
use bichon_core::common::signal::SignalManager;
use bichon_core::context::{executors::BichonContext, Initialize};
use bichon_core::error::code::ErrorCode;
use bichon_core::settings::cli::SETTINGS;
use bichon_core::settings::dir::DataDirManager;
use bichon_core::store::blob::BLOB_MANAGER;
use bichon_core::store::tantivy::attachment::ATTACHMENT_MANAGER;
use bichon_core::store::tantivy::envelope::ENVELOPE_MANAGER;
use bichon_core::users::manager::UserManager;
use tokio::sync::OnceCell;

static SETUP: OnceCell<()> = OnceCell::const_new();

async fn setup() {
    SETUP
        .get_or_init(|| async {
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
        })
        .await;
}

fn register_job(user_id: u64, job_id: &str, status: &str, artifact_name: Option<&str>) {
    let job = ExportJob {
        job_id: job_id.to_string(),
        user_id,
        username: "tester".into(),
        saved_search_id: "ss-1".into(),
        saved_search_name: "Test Search".into(),
        format: ExportFormat::Mbox,
        status: status.to_string(),
        accounts: vec![],
        total_emails: 0,
        total_size: 0,
        processed: 0,
        exported: 0,
        failed: 0,
        error: None,
        artifact_name: artifact_name.map(str::to_string),
        artifact_size: 0,
        artifact_hash: None,
        created_at: now_ms(),
        finished_at: Some(now_ms()),
        cancel: Arc::new(AtomicBool::new(false)),
        verify_status: VERIFY_STATUS_IDLE.to_string(),
        verify_checked: 0,
        verify_total: 0,
        verify_matched: 0,
        verify_mismatched: 0,
        verify_error: None,
        verify_result: None,
        verify_started_at: None,
        verify_finished_at: None,
    };
    registry()
        .lock()
        .unwrap()
        .insert(job_id.to_string(), Arc::new(Mutex::new(job)));
}

async fn write_artifact(job_id: &str, name: &str, content: &[u8]) -> PathBuf {
    let dir = job_dir(job_id);
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let path = dir.join(name);
    tokio::fs::write(&path, content).await.unwrap();
    path
}

async fn write_manifest(job_id: &str, artifact_hash: Option<&str>) {
    let dir = job_dir(job_id);
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let summary = serde_json::json!({
        "artifact": format!("{}.mbox", job_id),
        "artifact_hash": artifact_hash,
        "hash_algorithm": "sha256",
    });
    tokio::fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .await
    .unwrap();
}

async fn write_manifest_jsonl(job_id: &str, lines: &[serde_json::Value]) {
    let dir = job_dir(job_id);
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let mut data = String::new();
    for line in lines {
        data.push_str(&serde_json::to_string(line).unwrap());
        data.push('\n');
    }
    tokio::fs::write(dir.join("manifest.jsonl"), data).await.unwrap();
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex_encode(&hasher.finalize())
}

#[tokio::test]
async fn verify_rejects_unfinished_job() {
    setup().await;
    register_job(1, "exp-running", STATUS_RUNNING, Some("exp-running.mbox"));
    let err = verify_export(1, "exp-running").await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidParameter);
}

#[tokio::test]
async fn verify_rejects_job_without_artifact_name() {
    setup().await;
    register_job(1, "exp-no-artifact", STATUS_FINISHED, None);
    let err = verify_export(1, "exp-no-artifact").await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::InternalError);
}

#[tokio::test]
async fn verify_rejects_missing_artifact_file() {
    setup().await;
    register_job(1, "exp-missing-file", STATUS_FINISHED, Some("missing.mbox"));
    let err = verify_export(1, "exp-missing-file").await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::ResourceNotFound);
}

#[tokio::test]
async fn verify_rejects_unknown_job() {
    setup().await;
    let err = verify_export(1, "exp-does-not-exist").await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::ResourceNotFound);
}

#[tokio::test]
async fn verify_rejects_other_users_job() {
    setup().await;
    register_job(1, "exp-owned", STATUS_FINISHED, Some("exp-owned.mbox"));
    let err = verify_export(2, "exp-owned").await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::Forbidden);
}

#[tokio::test]
async fn verify_matches_artifact_hash() {
    setup().await;
    let content = b"From sender Mon Jan  1 00:00:00 2024\nSubject: t\n\nbody\r\n";
    register_job(1, "exp-hash-ok", STATUS_FINISHED, Some("exp-hash-ok.mbox"));
    write_artifact("exp-hash-ok", "exp-hash-ok.mbox", content).await;
    let hash = sha256_hex(content);
    write_manifest("exp-hash-ok", Some(&hash)).await;

    let view = verify_export(1, "exp-hash-ok").await.unwrap();
    assert!(view.artifact_hash_match);
    assert_eq!(view.expected_artifact_hash.as_deref(), Some(hash.as_str()));
    assert_eq!(view.checked, 0);
    assert_eq!(view.matched, 0);
    assert_eq!(view.mismatched, 0);
}

#[tokio::test]
async fn verify_reports_artifact_hash_mismatch_when_manifest_missing() {
    setup().await;
    let content = b"From sender Mon Jan  1 00:00:00 2024\nSubject: t\n\nbody\r\n";
    register_job(1, "exp-hash-none", STATUS_FINISHED, Some("exp-hash-none.mbox"));
    write_artifact("exp-hash-none", "exp-hash-none.mbox", content).await;

    let view = verify_export(1, "exp-hash-none").await.unwrap();
    assert!(!view.artifact_hash_match);
    assert_eq!(view.expected_artifact_hash, None);
    assert!(view.actual_artifact_hash.is_some());
}

#[tokio::test]
async fn verify_reports_artifact_hash_mismatch_when_wrong_hash() {
    setup().await;
    let content = b"From sender Mon Jan  1 00:00:00 2024\nSubject: t\n\nbody\r\n";
    register_job(1, "exp-hash-wrong", STATUS_FINISHED, Some("exp-hash-wrong.mbox"));
    write_artifact("exp-hash-wrong", "exp-hash-wrong.mbox", content).await;
    write_manifest("exp-hash-wrong", Some("deadbeef")).await;

    let view = verify_export(1, "exp-hash-wrong").await.unwrap();
    assert!(!view.artifact_hash_match);
    assert_eq!(view.expected_artifact_hash.as_deref(), Some("deadbeef"));
}

#[tokio::test]
async fn verify_counts_missing_messages_as_mismatch() {
    setup().await;
    let content = b"From sender Mon Jan  1 00:00:00 2024\nSubject: t\n\nbody\r\n";
    register_job(1, "exp-jsonl", STATUS_FINISHED, Some("exp-jsonl.mbox"));
    write_artifact("exp-jsonl", "exp-jsonl.mbox", content).await;
    write_manifest("exp-jsonl", None).await;
    write_manifest_jsonl(
        "exp-jsonl",
        &[
            serde_json::json!({
                "account_id": 1,
                "envelope_id": "env-1",
                "message_id": "<m1@example.com>",
                "subject": "one",
                "content_hash": sha256_hex(b"message one"),
            }),
            serde_json::json!({
                "account_id": 1,
                "envelope_id": "env-2",
                "message_id": "<m2@example.com>",
                "subject": "two",
                "content_hash": sha256_hex(b"message two"),
            }),
        ],
    )
    .await;

    let view = verify_export(1, "exp-jsonl").await.unwrap();
    assert_eq!(view.checked, 2);
    assert_eq!(view.matched, 0);
    assert_eq!(view.mismatched, 2);
    assert_eq!(view.mismatches.len(), 2);
    assert!(view.mismatches.iter().all(|m| m.reason == "blob_missing"));
}

#[tokio::test]
async fn verify_skips_malformed_manifest_lines() {
    setup().await;
    let content = b"From sender Mon Jan  1 00:00:00 2024\nSubject: t\n\nbody\r\n";
    register_job(1, "exp-malformed", STATUS_FINISHED, Some("exp-malformed.mbox"));
    write_artifact("exp-malformed", "exp-malformed.mbox", content).await;
    write_manifest("exp-malformed", None).await;
    write_manifest_jsonl(
        "exp-malformed",
        &[
            serde_json::json!({"account_id": 1, "envelope_id": "env-1"}),
            serde_json::json!({"envelope_id": "env-no-account"}),
            serde_json::Value::String("not-a-json-object".into()),
        ],
    )
    .await;

    let view = verify_export(1, "exp-malformed").await.unwrap();
    assert_eq!(view.checked, 1);
    assert_eq!(view.mismatched, 1);
}

#[tokio::test]
async fn verify_progress_reports_stored_result() {
    setup().await;
    let content = b"From sender Mon Jan  1 00:00:00 2024\nSubject: t\n\nbody\r\n";
    register_job(1, "exp-progress", STATUS_FINISHED, Some("exp-progress.mbox"));
    write_artifact("exp-progress", "exp-progress.mbox", content).await;
    let hash = sha256_hex(content);
    write_manifest("exp-progress", Some(&hash)).await;

    let view = verify_export(1, "exp-progress").await.unwrap();
    let progress = export_verify_progress(1, "exp-progress").unwrap();
    assert_eq!(progress.status, "finished");
    assert_eq!(progress.total, 0);
    assert_eq!(progress.result, Some(view));
    assert!(progress.finished_at.is_some());
}

#[tokio::test]
async fn start_export_verify_runs_in_background_and_is_idempotent() {
    setup().await;
    let content = b"From sender Mon Jan  1 00:00:00 2024\nSubject: t\n\nbody\r\n";
    register_job(1, "exp-bg", STATUS_FINISHED, Some("exp-bg.mbox"));
    write_artifact("exp-bg", "exp-bg.mbox", content).await;
    let hash = sha256_hex(content);
    write_manifest("exp-bg", Some(&hash)).await;
    write_manifest_jsonl(
        "exp-bg",
        &[serde_json::json!({
            "account_id": 1,
            "envelope_id": "env-bg-1",
            "message_id": "<m1@example.com>",
            "subject": "one",
            "content_hash": "deadbeef",
        })],
    )
    .await;

    let started = start_export_verify(1, "exp-bg").unwrap();
    assert!(matches!(started.status.as_str(), "running" | "finished"));

    let mut result = None;
    for _ in 0..100 {
        let progress = export_verify_progress(1, "exp-bg").unwrap();
        if progress.status == "finished" {
            result = progress.result;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let view = result.expect("background verification should finish");
    assert_eq!(view.checked, 1);
    assert_eq!(view.mismatched, 1);
    assert_eq!(view.mismatches[0].reason, "blob_missing");

    // Idempotent: starting again returns the stored result.
    let again = start_export_verify(1, "exp-bg").unwrap();
    assert_eq!(again.status, "finished");
    assert!(again.result.is_some());
}

#[tokio::test]
async fn download_ticket_roundtrip_is_single_use() {
    setup().await;
    register_job(1, "exp-ticket", STATUS_FINISHED, Some("exp-ticket.mbox"));
    let ticket = create_download_ticket(1, "tester", "exp-ticket").unwrap();
    let resolved = resolve_download_ticket(&ticket).expect("ticket should resolve");
    assert_eq!(resolved.0, 1);
    assert_eq!(resolved.1, "tester");
    assert_eq!(resolved.2, "exp-ticket");
    assert!(resolve_download_ticket(&ticket).is_none(), "ticket is single use");
}

#[tokio::test]
async fn download_ticket_checks_owner_and_status() {
    setup().await;
    register_job(1, "exp-ticket-owner", STATUS_FINISHED, Some("exp-ticket-owner.mbox"));
    let err = create_download_ticket(2, "other", "exp-ticket-owner").unwrap_err();
    assert_eq!(err.code(), ErrorCode::Forbidden);

    register_job(1, "exp-ticket-running", STATUS_RUNNING, None);
    let err = create_download_ticket(1, "tester", "exp-ticket-running").unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidParameter);
}

#[test]
fn detects_detach_placeholders() {
    assert!(contains_detach_placeholder(b"body <<BICHON_DETACH_HASH:abc123>> end"));
    assert!(!contains_detach_placeholder(b"body with no placeholder"));
    assert!(contains_detach_placeholder(b"<<BICHON_DETACH_HASH:"));
}

#[test]
fn classifies_mismatch_reasons() {
    assert_eq!(classify_mismatch(Some("expected"), None), "blob_missing");
    assert_eq!(
        classify_mismatch(
            Some("expected"),
            Some(&("actual".to_string(), vec!["missing-hash".to_string()]))
        ),
        "attachment_missing"
    );
    assert_eq!(
        classify_mismatch(Some("expected"), Some(&("actual".to_string(), vec![]))),
        "content_changed"
    );
}

#[tokio::test]
async fn missing_attachment_hashes_reports_absent_blobs() {
    setup().await;
    // Both hashes are unknown to the (empty) blob store, so both placeholders
    // are reported as genuinely absent.
    let present_hash = "a".repeat(64);
    let absent_hash = "b".repeat(64);
    let eml = format!(
        "x<<BICHON_DETACH_HASH:{}>>y<<BICHON_DETACH_HASH:{}>>z",
        present_hash, absent_hash
    );
    let missing = missing_attachment_hashes(eml.as_bytes());
    assert_eq!(missing, vec![present_hash, absent_hash]);
}
