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

//! Server-side batch export engine.
//!
//! Exports are driven by an email saved search. The caller passes an
//! authorized account scope (computed from RBAC by the API layer); this
//! module intersects it with the saved search filter and streams matching
//! raw messages into an mbox artifact under the data-dir temp folder,
//! writing a per-message manifest for later compliance workflows.
//!
//! Jobs live in an in-memory registry; artifacts on disk. A TTL sweep
//! (`sweep_expired`) reclaims artifacts after `EXPORT_TTL_SECS`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bichon_core::account::migration::AccountModel;
use bichon_core::envelope::meta::BichonMetadata;
use bichon_core::error::{code::ErrorCode, BichonResult};
use bichon_core::export::{
    ExportAccount, ExportFormat, ExportJobView, ExportPreviewView,
};
use bichon_core::ext::event_bus::{emit, Event};
use bichon_core::message::search::{
    search_messages_impl, EmailSearchFilter, EmailSearchRequest, SortBy,
};
use bichon_core::raise_error;
use bichon_core::saved_search::{SavedSearchKind, SavedSearchModel};
use bichon_core::settings::dir::DATA_DIR_MANAGER;
use bichon_core::store::blob::get_reader;
use bichon_core::store::envelope::Envelope;
use bichon_core::{base64_encode, utc_now};
use chrono::{TimeZone, Utc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info, warn};

const EXPORT_DIR_NAME: &str = "exports";
const EXPORT_PAGE_SIZE: u64 = 500;
/// Export artifacts are kept for this long after the job ends, then swept.
pub const EXPORT_TTL_SECS: i64 = 3600;
/// Safety buffer applied on top of the estimated export size for the disk
/// space check.
const DISK_SAFETY_FACTOR: f64 = 1.2;

const STATUS_PENDING: &str = "pending";
const STATUS_RUNNING: &str = "running";
const STATUS_FINISHED: &str = "finished";
const STATUS_FAILED: &str = "failed";
const STATUS_CANCELLED: &str = "cancelled";

/// In-memory state of one export job. Mutated by both the control API and
/// the background runner.
struct ExportJob {
    job_id: String,
    user_id: u64,
    username: String,
    saved_search_id: String,
    saved_search_name: String,
    format: ExportFormat,
    status: String,
    accounts: Vec<ExportAccount>,
    total_emails: u64,
    total_size: u64,
    processed: u64,
    exported: u64,
    failed: u64,
    error: Option<String>,
    artifact_name: Option<String>,
    artifact_size: u64,
    created_at: i64,
    finished_at: Option<i64>,
    cancel: Arc<AtomicBool>,
}

impl ExportJob {
    fn to_view(&self) -> ExportJobView {
        ExportJobView {
            job_id: self.job_id.clone(),
            status: self.status.clone(),
            saved_search_id: self.saved_search_id.clone(),
            saved_search_name: self.saved_search_name.clone(),
            format: self.format,
            accounts: self.accounts.clone(),
            total_emails: self.total_emails,
            total_size: self.total_size,
            processed: self.processed,
            exported: self.exported,
            failed: self.failed,
            error: self.error.clone(),
            artifact_name: self.artifact_name.clone(),
            artifact_size: self.artifact_size,
            created_at: self.created_at,
            finished_at: self.finished_at,
        }
    }
}

static EXPORT_JOBS: OnceLock<Mutex<HashMap<String, Arc<Mutex<ExportJob>>>>> =
    OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, Arc<Mutex<ExportJob>>>> {
    EXPORT_JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Global cap on how many export jobs may run at the same time.
const MAX_CONCURRENT_EXPORTS: usize = 2;

static EXPORT_SEMAPHORE: OnceLock<tokio::sync::Semaphore> = OnceLock::new();

fn export_semaphore() -> &'static tokio::sync::Semaphore {
    EXPORT_SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(MAX_CONCURRENT_EXPORTS))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn export_root_dir() -> PathBuf {
    DATA_DIR_MANAGER.temp_dir.join(EXPORT_DIR_NAME)
}

fn job_dir(job_id: &str) -> PathBuf {
    export_root_dir().join(job_id)
}

fn new_job_id() -> String {
    format!(
        "exp_{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}

fn ensure_email_kind(kind: &SavedSearchKind) -> BichonResult<()> {
    match kind {
        SavedSearchKind::Email => Ok(()),
        SavedSearchKind::Attachment => Err(raise_error!(
            "Only email saved searches can be exported; attachment searches are not exportable."
                .into(),
            ErrorCode::InvalidParameter
        )),
    }
}

fn parse_filter(value: serde_json::Value) -> BichonResult<EmailSearchFilter> {
    serde_json::from_value(value).map_err(|e| {
        raise_error!(
            format!("Saved search filter is invalid: {e}"),
            ErrorCode::InvalidParameter
        )
    })
}

fn find_owned(user_id: u64, job_id: &str) -> BichonResult<Arc<Mutex<ExportJob>>> {
    let jobs = registry().lock().unwrap();
    let job = jobs.get(job_id).cloned().ok_or_else(|| {
        raise_error!(
            format!("Export job '{job_id}' not found."),
            ErrorCode::ResourceNotFound
        )
    })?;
    let owner = job.lock().unwrap().user_id;
    if owner != user_id {
        return Err(raise_error!(
            "Permission denied: this export job belongs to another user.".into(),
            ErrorCode::Forbidden
        ));
    }
    Ok(job)
}

/// Runs a metadata-only pass over the search result to count matching emails
/// and sum their sizes, and resolves the accounts they belong to.
fn estimate(
    scope: &Option<HashSet<u64>>,
    filter: &EmailSearchFilter,
) -> BichonResult<(Vec<ExportAccount>, u64, u64)> {
    let mut account_ids: HashSet<u64> = HashSet::new();
    let mut total_size: u64 = 0;
    let mut page: u64 = 1;
    let mut total_emails: u64 = 0;

    loop {
        let request = EmailSearchRequest {
            filter: filter.clone(),
            page,
            page_size: EXPORT_PAGE_SIZE,
            sort_by: Some(SortBy::DATE),
            desc: Some(false),
        };
        let data = search_messages_impl(scope.clone(), request)?;
        if total_emails == 0 {
            total_emails = data.total_items;
        }
        for env in &data.items {
            total_size += env.size as u64;
            account_ids.insert(env.account_id);
        }
        let pages = data.total_pages.unwrap_or(1).max(1);
        if page >= pages {
            break;
        }
        page += 1;
    }

    let mut ids: Vec<u64> = account_ids.into_iter().collect();
    ids.sort_unstable();
    let mut accounts = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(acc) = AccountModel::find(id)? {
            accounts.push(ExportAccount {
                id,
                email: acc.email,
                name: acc.account_name,
            });
        }
    }
    Ok((accounts, total_emails, total_size))
}

/// Verifies there is enough free disk space for the estimated export size.
fn check_disk_space(required: u64) -> BichonResult<()> {
    let dir = export_root_dir();
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk = disks
        .list()
        .iter()
        .find(|d| dir.starts_with(d.mount_point()))
        .ok_or_else(|| {
            raise_error!(
                "Could not identify the disk for the export directory.".into(),
                ErrorCode::InternalError
            )
        })?;

    let free_space = disk.available_space();
    let required_with_buffer = (required as f64 * DISK_SAFETY_FACTOR) as u64;
    if free_space < required_with_buffer {
        return Err(raise_error!(
            format!(
                "Insufficient disk space for export: {} bytes required (with safety buffer), {} bytes available.",
                required_with_buffer, free_space
            ),
            ErrorCode::InternalError
        ));
    }
    Ok(())
}

/// Computes what an export from a saved search would contain, restricted to
/// the caller's authorized account scope.
pub fn preview(
    user_id: u64,
    saved_search_id: &str,
    scope: Option<HashSet<u64>>,
) -> BichonResult<ExportPreviewView> {
    let saved = SavedSearchModel::get_owned(user_id, saved_search_id)?;
    ensure_email_kind(&saved.kind)?;
    let filter = parse_filter(saved.filter.clone())?;
    let (accounts, total_emails, total_size) = estimate(&scope, &filter)?;
    Ok(ExportPreviewView {
        saved_search_id: saved.id,
        saved_search_name: saved.name,
        format: ExportFormat::Mbox,
        accounts,
        total_emails,
        total_size,
    })
}

/// Validates the request, estimates the scope, checks disk space and starts
/// the background export. Returns the initial job view.
pub fn create_export(
    user_id: u64,
    username: String,
    saved_search_id: &str,
    format: ExportFormat,
    scope: Option<HashSet<u64>>,
) -> BichonResult<ExportJobView> {
    sweep_expired();

    let permit: tokio::sync::SemaphorePermit<'static> = export_semaphore().try_acquire().map_err(|_| {
        raise_error!(
            format!(
                "Too many concurrent exports running (limit {MAX_CONCURRENT_EXPORTS}). Please retry after an existing export finishes."
            )
            .into(),
            ErrorCode::TooManyRequest
        )
    })?;

    let saved = SavedSearchModel::get_owned(user_id, saved_search_id)?;
    ensure_email_kind(&saved.kind)?;
    let filter = parse_filter(saved.filter.clone())?;

    let (accounts, total_emails, total_size) = estimate(&scope, &filter)?;
    check_disk_space(total_size)?;

    let job_id = new_job_id();
    let dir = job_dir(&job_id);
    std::fs::create_dir_all(&dir).map_err(|e| {
        raise_error!(
            format!("Failed to create export directory: {e}"),
            ErrorCode::InternalError
        )
    })?;

    let job = Arc::new(Mutex::new(ExportJob {
        job_id: job_id.clone(),
        user_id,
        username: username.clone(),
        saved_search_id: saved.id.clone(),
        saved_search_name: saved.name.clone(),
        format,
        status: STATUS_RUNNING.to_string(),
        accounts,
        total_emails,
        total_size,
        processed: 0,
        exported: 0,
        failed: 0,
        error: None,
        artifact_name: None,
        artifact_size: 0,
        created_at: now_ms(),
        finished_at: None,
        cancel: Arc::new(AtomicBool::new(false)),
    }));

    {
        let mut jobs = registry().lock().unwrap();
        jobs.insert(job_id.clone(), job.clone());
    }

    let view = job.lock().unwrap().to_view();
    tokio::spawn(run_export(job, scope, filter, permit));
    emit(Event::ExportStarted {
        user: username,
        export_id: job_id,
        saved_search_id: saved_search_id.to_string(),
        format: view.format.as_str().to_string(),
        account_count: view.accounts.len() as u64,
        email_count: view.total_emails,
    });
    Ok(view)
}

/// Snapshot of a job owned by `user_id`.
pub fn get_export(user_id: u64, job_id: &str) -> BichonResult<ExportJobView> {
    let job = find_owned(user_id, job_id)?;
    let view = job.lock().unwrap().to_view();
    Ok(view)
}

/// Lists the caller's export jobs, newest first.
pub fn list_exports(user_id: u64) -> Vec<ExportJobView> {
    let jobs = registry().lock().unwrap();
    let mut views: Vec<ExportJobView> = jobs
        .values()
        .filter_map(|job| {
            let guard = job.lock().unwrap();
            (guard.user_id == user_id).then(|| guard.to_view())
        })
        .collect();
    views.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    views
}

/// Requests cancellation of a running job. The background runner stops at the
/// next page boundary.
pub fn cancel_export(user_id: u64, job_id: &str) -> BichonResult<ExportJobView> {
    let job = find_owned(user_id, job_id)?;
    {
        let mut guard = job.lock().unwrap();
        if matches!(guard.status.as_str(), STATUS_PENDING | STATUS_RUNNING) {
            guard.cancel.store(true, Ordering::SeqCst);
            guard.status = STATUS_CANCELLED.to_string();
            guard.finished_at = Some(now_ms());
        }
    }
    if job.lock().unwrap().status == STATUS_CANCELLED {
        let guard = job.lock().unwrap();
        emit(Event::ExportCancelled {
            user: guard.username.clone(),
            export_id: job_id.to_string(),
        });
    }
    let view = job.lock().unwrap().to_view();
    Ok(view)
}

/// Removes a job from the registry and deletes its artifact directory.
pub fn delete_export(user_id: u64, job_id: &str) -> BichonResult<()> {
    let _job = find_owned(user_id, job_id)?;
    remove_job_files(job_id);
    registry().lock().unwrap().remove(job_id);
    Ok(())
}

/// Path to the finished artifact plus its filename, ready for download.
pub fn download_artifact(user_id: u64, job_id: &str) -> BichonResult<(PathBuf, String)> {
    let job = find_owned(user_id, job_id)?;
    let (status, name) = {
        let guard = job.lock().unwrap();
        (guard.status.clone(), guard.artifact_name.clone())
    };
    if status != STATUS_FINISHED {
        return Err(raise_error!(
            "Export is not ready yet.".into(),
            ErrorCode::InvalidParameter
        ));
    }
    let name = name.ok_or_else(|| {
        raise_error!(
            "Export has no artifact.".into(),
            ErrorCode::InternalError
        )
    })?;
    let path = job_dir(job_id).join(&name);
    if !path.is_file() {
        return Err(raise_error!(
            "Export artifact is missing.".into(),
            ErrorCode::ResourceNotFound
        ));
    }
    Ok((path, name))
}

fn remove_job_files(job_id: &str) {
    let dir = job_dir(job_id);
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            warn!("Failed to remove export dir {}: {e}", dir.display());
        }
    }
}

/// Removes finished/stale jobs older than `EXPORT_TTL_SECS`, plus orphan
/// directories under the export root that no longer map to a live job.
pub fn sweep_expired() {
    let now = now_ms();
    let ttl_ms = EXPORT_TTL_SECS * 1000;
    let mut expired: Vec<String> = Vec::new();
    {
        let jobs = registry().lock().unwrap();
        for (id, job) in jobs.iter() {
            let guard = job.lock().unwrap();
            let reference = match guard.status.as_str() {
                STATUS_FINISHED | STATUS_FAILED | STATUS_CANCELLED => {
                    guard.finished_at.unwrap_or(guard.created_at)
                }
                _ => guard.created_at,
            };
            if now.saturating_sub(reference) > ttl_ms {
                expired.push(id.clone());
            }
        }
    }
    for id in &expired {
        info!("Sweeping expired export job {id}");
        remove_job_files(id);
        registry().lock().unwrap().remove(id);
    }

    let root = export_root_dir();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if registry().lock().unwrap().contains_key(&name) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_dir() {
            continue;
        }
        let stale = meta
            .modified()
            .ok()
            .and_then(|t| t.elapsed().ok())
            .map(|el| el.as_secs() > EXPORT_TTL_SECS as u64)
            .unwrap_or(false);
        if stale {
            info!("Sweeping orphan export dir {name}");
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Spawns a periodic TTL sweep. Called once at server startup.
pub fn spawn_export_cleanup() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(900));
        loop {
            interval.tick().await;
            sweep_expired();
        }
    });
}

async fn run_export(
    job: Arc<Mutex<ExportJob>>,
    scope: Option<HashSet<u64>>,
    filter: EmailSearchFilter,
    _permit: tokio::sync::SemaphorePermit<'static>,
) {
    let dir = {
        let guard = job.lock().unwrap();
        job_dir(&guard.job_id)
    };

    let result = run_export_inner(&job, &dir, scope, filter).await;
    if let Err(e) = result {
        let message = e.to_string();
        let (username, export_id, saved_search_id) = {
            let guard = job.lock().unwrap();
            (
                guard.username.clone(),
                guard.job_id.clone(),
                guard.saved_search_id.clone(),
            )
        };
        {
            let mut guard = job.lock().unwrap();
            if guard.status != STATUS_CANCELLED {
                guard.status = STATUS_FAILED.to_string();
                guard.error = Some(message.clone());
                guard.finished_at = Some(now_ms());
            }
        }
        emit(Event::ExportFailed {
            user: username,
            export_id: export_id.clone(),
            saved_search_id,
            error: message.clone(),
        });
        error!("Export {export_id} failed: {message}");
    }
}

async fn run_export_inner(
    job: &Arc<Mutex<ExportJob>>,
    dir: &Path,
    scope: Option<HashSet<u64>>,
    filter: EmailSearchFilter,
) -> BichonResult<()> {
    let job_id = job.lock().unwrap().job_id.clone();
    let artifact_name = format!("{job_id}.mbox");
    let mbox_path = dir.join(&artifact_name);
    let mut mbox = tokio::fs::File::create(&mbox_path).await.map_err(|e| {
        raise_error!(
            format!("Failed to create mbox file: {e}"),
            ErrorCode::InternalError
        )
    })?;

    let manifest_path = dir.join("manifest.jsonl");
    let mut manifest = tokio::fs::File::create(&manifest_path).await.map_err(|e| {
        raise_error!(
            format!("Failed to create manifest file: {e}"),
            ErrorCode::InternalError
        )
    })?;

    let mut page: u64 = 1;
    loop {
        if is_cancelled(job) {
            return Ok(());
        }
        let request = EmailSearchRequest {
            filter: filter.clone(),
            page,
            page_size: EXPORT_PAGE_SIZE,
            sort_by: Some(SortBy::DATE),
            desc: Some(false),
        };
        let data = search_messages_impl(scope.clone(), request)?;
        let pages = data.total_pages.unwrap_or(1).max(1);

        for envelope in data.items {
            if is_cancelled(job) {
                return Ok(());
            }
            let ok = export_one(&mut mbox, &mut manifest, &envelope).await;
            let mut guard = job.lock().unwrap();
            guard.processed += 1;
            if ok.is_ok() {
                guard.exported += 1;
            } else {
                guard.failed += 1;
                if let Err(e) = ok {
                    warn!(
                        "Failed to export message {} (account {}): {e}",
                        envelope.id, envelope.account_id
                    );
                }
            }
        }

        if page >= pages {
            break;
        }
        page += 1;
    }

    mbox.flush().await.ok();
    manifest.flush().await.ok();
    let artifact_size = tokio::fs::metadata(&mbox_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    let (username, export_id, saved_search_id, saved_search_name, accounts, exported, failed, created_at) = {
        let guard = job.lock().unwrap();
        (
            guard.username.clone(),
            guard.job_id.clone(),
            guard.saved_search_id.clone(),
            guard.saved_search_name.clone(),
            guard.accounts.clone(),
            guard.exported,
            guard.failed,
            guard.created_at,
        )
    };

    write_summary_manifest(
        dir,
        &artifact_name,
        artifact_size,
        &username,
        &saved_search_id,
        &saved_search_name,
        &accounts,
        exported,
        failed,
        created_at,
    )
    .await?;

    {
        let mut guard = job.lock().unwrap();
        guard.status = STATUS_FINISHED.to_string();
        guard.artifact_name = Some(artifact_name);
        guard.artifact_size = artifact_size;
        guard.finished_at = Some(now_ms());
    }

    emit(Event::ExportCompleted {
        user: username,
        export_id: export_id.clone(),
        saved_search_id,
        exported,
        failed,
        artifact_size,
    });
    info!("Export {export_id} finished: {exported} exported, {failed} failed");
    Ok(())
}

fn is_cancelled(job: &Arc<Mutex<ExportJob>>) -> bool {
    let guard = job.lock().unwrap();
    guard.cancel.load(Ordering::SeqCst) || guard.status == STATUS_CANCELLED
}

/// Writes one mbox entry (From_ line + metadata header + raw EML) and one
/// manifest line.
async fn export_one(
    mbox: &mut tokio::fs::File,
    manifest: &mut tokio::fs::File,
    envelope: &Envelope,
) -> BichonResult<()> {
    let mut reader = get_reader(envelope.account_id, envelope.id.clone()).await?;
    let mut raw = Vec::new();
    reader.read_to_end(&mut raw).await.map_err(|e| {
        raise_error!(
            format!("Failed to read message {}: {e}", envelope.id),
            ErrorCode::InternalError
        )
    })?;

    let date_dt = Utc.timestamp_opt(envelope.date / 1000, 0).unwrap();
    let date_str = date_dt.format("%a %b %e %H:%M:%S %Y").to_string();
    let from_line = format!("From {} {}\n", envelope.from.clone(), date_str);

    let metadata = BichonMetadata {
        account_email: envelope.account_email.clone(),
        mailbox_name: envelope.mailbox_name.clone(),
        tags: envelope.tags.clone(),
    };
    let encoded = base64_encode!(serde_json::to_string(&metadata).map_err(|e| {
        raise_error!(
            format!("Failed to serialize metadata: {e}"),
            ErrorCode::InternalError
        )
    })?);
    let header = format!("X-Bichon-Metadata: {}\r\n", encoded);

    mbox.write_all(from_line.as_bytes()).await.map_err(|e| {
        raise_error!(
            format!("Failed to write mbox entry: {e}"),
            ErrorCode::InternalError
        )
    })?;
    mbox.write_all(header.as_bytes()).await.map_err(|e| {
        raise_error!(
            format!("Failed to write mbox header: {e}"),
            ErrorCode::InternalError
        )
    })?;
    mbox.write_all(&raw).await.map_err(|e| {
        raise_error!(
            format!("Failed to write message body: {e}"),
            ErrorCode::InternalError
        )
    })?;
    mbox.write_all(b"\n\n").await.map_err(|e| {
        raise_error!(
            format!("Failed to finalize mbox entry: {e}"),
            ErrorCode::InternalError
        )
    })?;

    let line = serde_json::json!({
        "account_id": envelope.account_id,
        "envelope_id": envelope.id,
        "message_id": envelope.message_id,
        "subject": envelope.subject,
        "content_hash": envelope.content_hash,
        "size": envelope.size,
        "date": envelope.date,
        "mailbox_id": envelope.mailbox_id,
        "uid": envelope.uid,
    });
    let mut buf = serde_json::to_vec(&line).map_err(|e| {
        raise_error!(
            format!("Failed to serialize manifest line: {e}"),
            ErrorCode::InternalError
        )
    })?;
    buf.push(b'\n');
    manifest.write_all(&buf).await.map_err(|e| {
        raise_error!(
            format!("Failed to write manifest line: {e}"),
            ErrorCode::InternalError
        )
    })?;

    Ok(())
}

async fn write_summary_manifest(
    dir: &Path,
    artifact_name: &str,
    artifact_size: u64,
    username: &str,
    saved_search_id: &str,
    saved_search_name: &str,
    accounts: &[ExportAccount],
    exported: u64,
    failed: u64,
    created_at: i64,
) -> BichonResult<()> {
    let summary = serde_json::json!({
        "job_id": artifact_name.trim_end_matches(".mbox"),
        "status": "finished",
        "user": username,
        "saved_search_id": saved_search_id,
        "saved_search_name": saved_search_name,
        "format": ExportFormat::Mbox.as_str(),
        "created_at": created_at,
        "finished_at": now_ms(),
        "accounts": accounts,
        "total_emails": exported + failed,
        "total_size": artifact_size,
        "exported": exported,
        "failed": failed,
        "artifact": artifact_name,
        "artifact_size": artifact_size,
    });
    let data = serde_json::to_vec_pretty(&summary).map_err(|e| {
        raise_error!(
            format!("Failed to serialize manifest: {e}"),
            ErrorCode::InternalError
        )
    })?;
    tokio::fs::write(dir.join("manifest.json"), data).await.map_err(|e| {
        raise_error!(
            format!("Failed to write manifest: {e}"),
            ErrorCode::InternalError
        )
    })
}

#[allow(dead_code)]
fn _utc_now_ms() -> i64 {
    utc_now!()
}