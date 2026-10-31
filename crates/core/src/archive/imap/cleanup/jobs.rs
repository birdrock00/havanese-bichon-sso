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

//! Background job registry for cleanup previews and runs.
//!
//! Verification walks every candidate over IMAP (several round trips each) and
//! a run paces its moves, so a preview or a run can take far longer than an
//! HTTP request may stay open. The API therefore starts a job, returns its id
//! immediately and the client polls [`CleanupJob`] until the status leaves
//! `Running`. The outcome (including the failure message) stays readable for
//! [`JOB_TTL_MS`], so a client that lost its connection — or closed its dialog —
//! can still read the result; expired entries are swept on every access.
//!
//! Jobs are in-memory only: a server restart forgets them. The durable record
//! of what happened is `RemoteCleanupState` (move runs), plus the audit trail.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{LazyLock, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::account::migration::AccountModel;
use crate::account::state::RemoteCleanupRun;
use crate::archive::imap::cleanup::execute::{execute_move, MoveRequest};
use crate::archive::imap::cleanup::preview::{self, CleanupPreview};
use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::raise_error;
use crate::utc_now;
use tracing::info;

/// How long a finished job (and its result) stays readable after it ends.
pub const JOB_TTL_MS: i64 = 30 * 60 * 1000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum CleanupJobKind {
    #[default]
    #[cfg_attr(feature = "web-api", oai(rename = "move_preview"))]
    MovePreview,
    #[cfg_attr(feature = "web-api", oai(rename = "move_run"))]
    MoveRun,
}

impl CleanupJobKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CleanupJobKind::MovePreview => "move_preview",
            CleanupJobKind::MoveRun => "move_run",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum CleanupJobStatus {
    #[default]
    Running,
    Success,
    Failed,
    /// The caller asked to stop; the worker ends at the next message.
    Cancelled,
}

/// Pollable snapshot of one cleanup operation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct CleanupJob {
    pub job_id: String,
    pub account_id: u64,
    pub kind: CleanupJobKind,
    pub status: CleanupJobStatus,
    /// The API user who started the job.
    pub user: Option<String>,
    pub created_at: i64,
    pub finished_at: Option<i64>,
    /// Set for `Success` on a preview job.
    #[serde(default)]
    pub preview: Option<CleanupPreview>,
    /// Set for `Success` on a move job. Operational failures (an abort inside
    /// the run) still arrive here with a `Failed` run status — `error` is only
    /// for a run that never started.
    #[serde(default)]
    pub run: Option<RemoteCleanupRun>,
    #[serde(default)]
    pub error: Option<String>,
}

struct JobEntry {
    job: CleanupJob,
    cancel: CancellationToken,
}

static JOBS: LazyLock<Mutex<HashMap<String, JobEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn jobs() -> MutexGuard<'static, HashMap<String, JobEntry>> {
    // A panic while the map was held cannot leave it inconsistent (every
    // operation is a single insert/remove/lookup), so recover instead of
    // poisoning cleanup for the rest of the process lifetime.
    JOBS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Drop finished jobs whose results expired. Running jobs are kept: a long
/// preview must not disappear from under its own poller.
fn sweep(map: &mut HashMap<String, JobEntry>) {
    let now = utc_now!();
    map.retain(|_, entry| {
        if entry.job.status == CleanupJobStatus::Running {
            return true;
        }
        let reference = entry.job.finished_at.unwrap_or(entry.job.created_at);
        now.saturating_sub(reference) <= JOB_TTL_MS
    });
}

/// Claim a job slot for the account. Refused while another cleanup job for the
/// same account is still running: the underlying operations already serialize
/// on a per-account lock, and letting the caller enqueue a second one would
/// only turn the conflict into a delayed failure.
fn reserve(
    account_id: u64,
    kind: CleanupJobKind,
    user: Option<String>,
) -> BichonResult<(String, CancellationToken)> {
    let mut map = jobs();
    sweep(&mut map);
    if map.values().any(|entry| {
        entry.job.account_id == account_id && entry.job.status == CleanupJobStatus::Running
    }) {
        return Err(raise_error!(
            "A cleanup job is already running for this account".into(),
            ErrorCode::InvalidParameter
        ));
    }
    let job_id = uuid::Uuid::new_v4().to_string();
    let cancel = CancellationToken::new();
    map.insert(
        job_id.clone(),
        JobEntry {
            job: CleanupJob {
                job_id: job_id.clone(),
                account_id,
                kind,
                status: CleanupJobStatus::Running,
                user,
                created_at: utc_now!(),
                ..Default::default()
            },
            cancel: cancel.clone(),
        },
    );
    Ok((job_id, cancel))
}

/// Opaque token state: a preview's confirmation token is only valid for
/// [`crate::archive::imap::cleanup::PREVIEW_TTL_MS`], which can elapse while
/// the finished job is still readable (jobs live longer than tokens). Stamp it
/// when the snapshot is read so a reattached client knows Execute would be
/// refused instead of discovering it by trying.
fn stamp_token_expiry(mut job: CleanupJob) -> CleanupJob {
    if let Some(preview) = job.preview.as_mut() {
        preview.token_expired = preview.expires_at <= utc_now!();
    }
    job
}

fn snapshot(job_id: &str) -> Option<CleanupJob> {
    jobs().get(job_id).map(|entry| stamp_token_expiry(entry.job.clone()))
}

enum Outcome {
    Preview(CleanupPreview),
    Run(RemoteCleanupRun),
}

/// Record the worker's result. A cancellation wins over both a returned result
/// and an error: the executors stop at the next message and may still hand back
/// a partial report, and a worker that never produced one reports the stop as
/// an error. Either way the job ends as `Cancelled`, with the report kept.
fn finish(job_id: &str, result: BichonResult<Outcome>) {
    let mut map = jobs();
    let Some(entry) = map.get_mut(job_id) else {
        return;
    };
    let cancelled = entry.cancel.is_cancelled();
    entry.job.finished_at = Some(utc_now!());
    match result {
        Ok(Outcome::Preview(preview)) => {
            entry.job.status = if cancelled {
                CleanupJobStatus::Cancelled
            } else {
                CleanupJobStatus::Success
            };
            entry.job.preview = Some(preview);
        }
        Ok(Outcome::Run(run)) => {
            entry.job.status = if cancelled {
                CleanupJobStatus::Cancelled
            } else {
                CleanupJobStatus::Success
            };
            entry.job.run = Some(run);
        }
        Err(error) => {
            entry.job.status = if cancelled {
                CleanupJobStatus::Cancelled
            } else {
                CleanupJobStatus::Failed
            };
            entry.job.error = Some(error.to_string());
        }
    }
    let status = entry.job.status;
    let account_id = entry.job.account_id;
    let kind = entry.job.kind;
    drop(map);
    info!(
        account_id,
        job_id,
        kind = kind.as_str(),
        status = ?status,
        "remote-cleanup: job finished"
    );
}

/// Run `future` as the job's worker. The scope guard records a failure when
/// the task is aborted or panics, so a job can never stay `Running` forever.
fn spawn_job<F>(job_id: String, future: F)
where
    F: Future<Output = BichonResult<Outcome>> + Send + 'static,
{
    let guard = scopeguard::guard(job_id, |id| {
        finish(
            &id,
            Err(raise_error!(
                "cleanup job was interrupted before completion".into(),
                ErrorCode::InternalError
            )),
        );
    });
    tokio::spawn(async move {
        let result = future.await;
        let job_id = scopeguard::ScopeGuard::into_inner(guard);
        finish(&job_id, result);
    });
}

/// Reserve the slot, publish the job and let `work` run against its token.
fn launch<F>(
    account_id: u64,
    kind: CleanupJobKind,
    user: Option<String>,
    work: impl FnOnce(CancellationToken) -> F,
) -> BichonResult<CleanupJob>
where
    F: Future<Output = BichonResult<Outcome>> + Send + 'static,
{
    let (job_id, cancel) = reserve(account_id, kind, user)?;
    let job = snapshot(&job_id).unwrap_or_default();
    info!(
        account_id,
        job_id = %job_id,
        kind = kind.as_str(),
        "remote-cleanup: job started"
    );
    spawn_job(job_id, work(cancel));
    Ok(job)
}

/// Start a read-only move preview in the background.
pub fn start_move_preview(account: AccountModel, user: String) -> BichonResult<CleanupJob> {
    let account_id = account.id;
    launch(
        account_id,
        CleanupJobKind::MovePreview,
        Some(user.clone()),
        move |cancel| async move {
            preview::preview_move(&account, &user, &cancel)
                .await
                .map(Outcome::Preview)
        },
    )
}

/// Start a move run in the background. `request.cancel` is replaced with the
/// job's token so a cancel request reaches the worker.
pub fn start_move_run(
    account: AccountModel,
    mut request: MoveRequest,
) -> BichonResult<CleanupJob> {
    let account_id = account.id;
    let user = request.user.clone();
    launch(account_id, CleanupJobKind::MoveRun, user, move |cancel| {
        request.cancel = cancel;
        async move { execute_move(&account, request).await.map(Outcome::Run) }
    })
}

/// Every job still known for `account_id`, newest first and running jobs ahead
/// of finished ones.
///
/// The client normally follows a job by the id it got when starting it, but a
/// page reload or a remounted dialog loses that id while the job keeps running
/// server-side. Listing lets the client reattach to the running job (or show
/// its finished report) instead of starting a second one, which [`reserve`]
/// would refuse.
pub fn account_jobs(account_id: u64) -> Vec<CleanupJob> {
    let mut map = jobs();
    sweep(&mut map);
    let mut result: Vec<CleanupJob> = map
        .values()
        .filter(|entry| entry.job.account_id == account_id)
        .map(|entry| stamp_token_expiry(entry.job.clone()))
        .collect();
    result.sort_by(|a, b| {
        let a_running = a.status == CleanupJobStatus::Running;
        let b_running = b.status == CleanupJobStatus::Running;
        b_running
            .cmp(&a_running)
            .then(b.created_at.cmp(&a.created_at))
    });
    result
}

/// Read a job belonging to `account_id`. Jobs are not globally listable: the
/// account id is part of the lookup so one account cannot read another's
/// results.
pub fn get_job(account_id: u64, job_id: &str) -> BichonResult<CleanupJob> {
    let mut map = jobs();
    sweep(&mut map);
    map.get(job_id)
        .filter(|entry| entry.job.account_id == account_id)
        .map(|entry| stamp_token_expiry(entry.job.clone()))
        .ok_or_else(|| {
            raise_error!(
                "cleanup job not found or expired; check the cleanup reports for the outcome"
                    .into(),
                ErrorCode::ResourceNotFound
            )
        })
}

/// Ask a running job to stop. The worker checks the token before every
/// message, so a long run ends at the next message boundary rather than
/// mid-move. Finished jobs are returned unchanged.
pub fn cancel_job(account_id: u64, job_id: &str) -> BichonResult<CleanupJob> {
    let mut map = jobs();
    sweep(&mut map);
    let entry = map
        .get_mut(job_id)
        .filter(|entry| entry.job.account_id == account_id)
        .ok_or_else(|| {
            raise_error!(
                "cleanup job not found or expired; check the cleanup reports for the outcome"
                    .into(),
                ErrorCode::ResourceNotFound
            )
        })?;
    if entry.job.status == CleanupJobStatus::Running {
        entry.cancel.cancel();
    }
    Ok(stamp_token_expiry(entry.job.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert(account_id: u64, job_id: &str, status: CleanupJobStatus, age_ms: i64) -> String {
        let mut map = jobs();
        map.insert(
            job_id.to_string(),
            JobEntry {
                job: CleanupJob {
                    job_id: job_id.to_string(),
                    account_id,
                    status,
                    created_at: utc_now!() - age_ms,
                    finished_at: Some(utc_now!() - age_ms),
                    ..Default::default()
                },
                cancel: CancellationToken::new(),
            },
        );
        job_id.to_string()
    }

    #[test]
    fn a_second_running_job_for_the_same_account_is_refused() {
        let (job_id, _) = reserve(90_001, CleanupJobKind::MovePreview, None).unwrap();
        let second = reserve(90_001, CleanupJobKind::MoveRun, None);
        assert!(second.is_err(), "one cleanup job per account at a time");
        // A different account is independent.
        assert!(reserve(90_002, CleanupJobKind::MoveRun, None).is_ok());
        jobs().remove(&job_id);
    }

    #[test]
    fn a_finished_job_does_not_block_a_new_one() {
        let (job_id, _) = reserve(90_003, CleanupJobKind::MoveRun, None).unwrap();
        finish(&job_id, Ok(Outcome::Run(RemoteCleanupRun::default())));
        assert!(reserve(90_003, CleanupJobKind::MovePreview, None).is_ok());
    }

    #[test]
    fn sweep_keeps_running_jobs_and_drops_expired_results() {
        let running = insert(90_010, "job-running", CleanupJobStatus::Running, 0);
        let expired = insert(90_011, "job-expired", CleanupJobStatus::Success, JOB_TTL_MS + 1);
        let fresh = insert(90_012, "job-fresh", CleanupJobStatus::Failed, 1_000);
        let mut map = jobs();
        // A running job has no finish time to measure the TTL from.
        map.get_mut(&running).unwrap().job.finished_at = None;
        sweep(&mut map);
        assert!(map.contains_key(&running));
        assert!(map.contains_key(&fresh));
        assert!(!map.contains_key(&expired));
        map.remove(&running);
        map.remove(&fresh);
    }

    #[test]
    fn a_cancelled_job_reports_cancelled_not_failed() {
        let (job_id, cancel) = reserve(90_020, CleanupJobKind::MovePreview, Some("u".into())).unwrap();
        cancel.cancel();
        finish(
            &job_id,
            Err(raise_error!(
                "cleanup preview was cancelled".into(),
                ErrorCode::InvalidParameter
            )),
        );
        let job = get_job(90_020, &job_id).unwrap();
        assert_eq!(job.status, CleanupJobStatus::Cancelled);
        assert!(job.error.is_some());
    }

    #[test]
    fn a_job_is_only_readable_by_its_own_account() {
        let (job_id, _) = reserve(90_030, CleanupJobKind::MovePreview, None).unwrap();
        assert!(get_job(90_030, &job_id).is_ok());
        assert!(get_job(90_031, &job_id).is_err());
        assert!(get_job(90_030, "no-such-job").is_err());
    }

    #[test]
    fn account_jobs_lists_running_first_then_newest() {
        let older = insert(90_040, "job-older", CleanupJobStatus::Success, 10_000);
        let newer = insert(90_040, "job-newer", CleanupJobStatus::Failed, 1_000);
        let (running, _) = reserve(90_040, CleanupJobKind::MovePreview, None).unwrap();
        insert(90_041, "job-other-account", CleanupJobStatus::Success, 0);

        let listed = account_jobs(90_040);
        let ids: Vec<&str> = listed.iter().map(|job| job.job_id.as_str()).collect();
        assert_eq!(ids, vec![running.as_str(), newer.as_str(), older.as_str()]);
        assert!(
            account_jobs(90_041).iter().all(|job| job.job_id != running),
            "another account's jobs must not leak into the list"
        );

        jobs().remove(&older);
        jobs().remove(&newer);
        jobs().remove(&running);
        jobs().remove("job-other-account");
    }

    #[test]
    fn a_reattached_preview_reports_an_expired_token() {
        let (job_id, _) = reserve(90_050, CleanupJobKind::MovePreview, None).unwrap();
        {
            let mut map = jobs();
            let job = &mut map.get_mut(&job_id).unwrap().job;
            job.status = CleanupJobStatus::Success;
            job.preview = Some(CleanupPreview {
                expires_at: utc_now!() - 1,
                ..Default::default()
            });
        }
        let expired = get_job(90_050, &job_id).unwrap();
        assert!(
            expired.preview.unwrap().token_expired,
            "a token past expires_at must not be offered as executable"
        );

        {
            let mut map = jobs();
            let job = &mut map.get_mut(&job_id).unwrap().job;
            job.preview = Some(CleanupPreview {
                expires_at: utc_now!() + 60_000,
                ..Default::default()
            });
        }
        let fresh = get_job(90_050, &job_id).unwrap();
        assert!(!fresh.preview.unwrap().token_expired);
        jobs().remove(&job_id);
    }
}
