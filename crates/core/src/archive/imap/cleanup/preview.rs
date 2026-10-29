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

//! Read-only previews and the short-lived confirmation tokens that bind a
//! preview to an execution.

use std::collections::{BTreeMap, HashMap};
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::account::migration::{
    AccountModel, MAX_REMOTE_CLEANUP_PER_RUN, REMOTE_CLEANUP_FOLDER,
};
use crate::archive::imap::cleanup::plan;
use crate::archive::imap::cleanup::verify::{self, VerifyIssue};
use crate::archive::imap::cleanup::{
    release, try_acquire, DAY_MS, MAX_LOCAL_INTEGRITY_FAILURES, PREVIEW_TTL_MS,
};
use crate::backup::{config as backup_config, manager::BACKUP_MANAGER};
use crate::error::code::ErrorCode;
use crate::ext::event_bus::{emit, Event};
use crate::imap::executor::ImapExecutor;
use crate::raise_error;
use crate::{error::BichonResult, utc_now};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum CleanupPreviewKind {
    #[default]
    #[cfg_attr(feature = "web-api", oai(rename = "move"))]
    Move,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct CleanupSkipCount {
    pub reason: String,
    pub count: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct CleanupFolderPreview {
    pub folder: String,
    pub candidates: u64,
    pub verified: u64,
    pub bytes: u64,
    pub skipped: Vec<CleanupSkipCount>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct CleanupPreview {
    pub account_id: u64,
    pub kind: CleanupPreviewKind,
    pub generated_at: i64,
    pub expires_at: i64,
    pub destination_folder: String,
    pub destination_exists: bool,
    /// Whether the server advertises `MOVE` (RFC 6851). It is the only move
    /// path cleanup uses; without it execution is refused.
    pub supports_move: bool,
    pub candidates: u64,
    pub verified: u64,
    pub bytes: u64,
    pub folders: Vec<CleanupFolderPreview>,
    pub skipped: Vec<CleanupSkipCount>,
    pub warnings: Vec<String>,
    /// Bichon backup state at preview time. `backup_is_recent` means a
    /// successful backup finished within the last seven days.
    #[serde(default)]
    pub backup_configured: bool,
    #[serde(default)]
    pub backup_last_success_at: Option<i64>,
    #[serde(default)]
    pub backup_is_recent: bool,
    /// A move execution would stop at the first identity mismatch. The preview
    /// still lists the messages it could verify; the UI should warn that the
    /// run will abort before reaching the end.
    #[serde(default)]
    pub will_abort: bool,
    /// Must be passed to the matching execute call.
    pub token: String,
    /// Computed when a job snapshot is read, not when the preview is made: the
    /// confirmation token has passed `expires_at`, so Execute will be refused
    /// and a new preview is required. Always false in the fresh response.
    #[serde(default)]
    pub token_expired: bool,
}

#[derive(Clone, Debug)]
struct Ticket {
    account_id: u64,
    kind: CleanupPreviewKind,
    payload_digest: String,
    config_digest: String,
    expires_at: i64,
}

static TICKETS: LazyLock<Mutex<HashMap<String, Ticket>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn issue_ticket(
    account_id: u64,
    kind: CleanupPreviewKind,
    payload_digest: String,
    config_digest: String,
) -> (String, i64) {
    let now = utc_now!();
    let expires_at = now.saturating_add(PREVIEW_TTL_MS);
    let token = uuid::Uuid::new_v4().to_string();
    let mut tickets = TICKETS.lock().unwrap_or_else(|e| e.into_inner());
    tickets.retain(|_, ticket| ticket.expires_at > now);
    tickets.insert(
        token.clone(),
        Ticket {
            account_id,
            kind,
            payload_digest,
            config_digest,
            expires_at,
        },
    );
    (token, expires_at)
}

/// Validate and consume a token. Tokens are single-use: any failed execute
/// requires a new preview.
pub(crate) fn consume_ticket(
    token: &str,
    account_id: u64,
    kind: CleanupPreviewKind,
    payload_digest: &str,
    config_digest: &str,
) -> Result<(), String> {
    let mut tickets = TICKETS.lock().unwrap_or_else(|e| e.into_inner());
    let Some(ticket) = tickets.remove(token) else {
        return Err("preview token is unknown or already used".into());
    };
    if ticket.expires_at <= utc_now!() {
        return Err("preview token has expired; run the preview again".into());
    }
    if ticket.account_id != account_id || ticket.kind != kind {
        return Err("preview token belongs to a different account or operation".into());
    }
    if ticket.config_digest != config_digest {
        return Err("cleanup configuration changed since the preview".into());
    }
    if ticket.payload_digest != payload_digest {
        return Err("the eligible message set changed since the preview; run it again".into());
    }
    Ok(())
}

fn skipped_vec(map: BTreeMap<String, u64>) -> Vec<CleanupSkipCount> {
    map.into_iter()
        .map(|(reason, count)| CleanupSkipCount { reason, count })
        .collect()
}

fn add_skip(map: &mut BTreeMap<String, u64>, reason: &str) {
    *map.entry(reason.to_string()).or_insert(0) += 1;
}

fn backup_snapshot() -> (bool, Option<i64>, bool) {
    let configured = backup_config::enabled();
    let last_success_at = BACKUP_MANAGER.state().last_success_at;
    let is_recent = configured
        && last_success_at
            .map(|at| utc_now!() - at <= 7 * DAY_MS)
            .unwrap_or(false);
    (configured, last_success_at, is_recent)
}

fn backup_warning() -> Option<String> {
    if !backup_config::enabled() {
        return Some(
            "No backup is configured. After a move, bichon_auto_created_trash still holds the \
             message, but the local archive is the only indexed copy."
                .to_string(),
        );
    }
    match BACKUP_MANAGER.state().last_success_at {
        Some(at) if utc_now!() - at <= 7 * DAY_MS => None,
        Some(at) => Some(format!(
            "Last successful backup was {} day(s) ago.",
            (utc_now!() - at) / DAY_MS
        )),
        None => Some("No successful backup has been recorded yet.".to_string()),
    }
}

/// Read-only preview of what a move run would move.
///
/// `cancel` is checked between candidates: verifying the window is a batch of
/// IMAP round trips, so on a slow server it can run for a while and must be
/// interruptible.
pub async fn preview_move(
    account: &AccountModel,
    user: &str,
    cancel: &CancellationToken,
) -> BichonResult<CleanupPreview> {
    let config = account.remote_cleanup.as_ref().ok_or_else(|| {
        raise_error!(
            "Source-mailbox cleanup is not configured".into(),
            ErrorCode::InvalidParameter
        )
    })?;

    if !try_acquire(account.id).await {
        return Err(raise_error!(
            "A cleanup operation is already running for this account".into(),
            ErrorCode::InvalidParameter
        ));
    }
    let _guard = scopeguard::guard(account.id, release);

    let now = utc_now!();
    let scan = plan::collect(account, now)?;
    let config_digest = plan::config_digest(config);

    let mut warnings = Vec::new();
    if account.is_on_hold() {
        warnings.push(
            "This account is on legal hold; execution will be refused.".to_string(),
        );
    }
    let (backup_configured, backup_last_success_at, backup_is_recent) = backup_snapshot();
    if let Some(warning) = backup_warning() {
        warnings.push(warning);
    }

    let mut session = ImapExecutor::create_connection(account.id).await?;
    let supports_move = ImapExecutor::supports_move(&mut session).await?;
    if !supports_move {
        warnings.push(
            "The server does not advertise MOVE (RFC 6851); source-mailbox cleanup cannot run. \
             Moving mail without MOVE would require EXPUNGE, which Bichon never issues."
                .to_string(),
        );
    }
    let destination_exists = ImapExecutor::mailbox_exists(&mut session, REMOTE_CLEANUP_FOLDER).await?;
    if !destination_exists {
        warnings.push(format!(
            "'{REMOTE_CLEANUP_FOLDER}' does not exist yet; it will be created on execute."
        ));
    }

    // Filter flagged messages before walking the window: otherwise the oldest
    // flagged mail would occupy it forever. The preview token is bound to this
    // whole window; verification skips must not shrink the preview below the
    // batch size, so the walk continues until `per_run` messages are verified
    // (or the window is exhausted — then there is nothing more to move).
    let (kept, flagged_skipped, _flagged_dropped) =
        plan::apply_flagged_filter(&mut session, &scan.candidates, config.skip_flagged).await?;
    let per_run = config
        .max_messages_per_run
        .clamp(1, MAX_REMOTE_CLEANUP_PER_RUN) as usize;
    let candidate_digest = plan::candidate_digest(&kept);
    if scan.scan_limited {
        warnings.push(format!(
            "The scan budget was reached after {} candidates; some skipped messages may hide newer eligible mail.",
            scan.candidates.len()
        ));
    }

    let mut folder_previews = Vec::new();
    let mut total_verified = 0u64;
    // How many candidates the walk actually looked at. Larger than `verified`
    // when some were skipped or failed; that is the window the batch filled
    // from, not the batch itself.
    let mut total_examined = 0u64;
    let mut total_bytes = 0u64;
    let mut will_abort = false;
    let mut remote_abort = false;
    // Local-integrity failures do not stop the preview (it is read-only), but
    // execution stops after three of them. Surface that up front.
    let mut local_integrity_failures = 0u64;
    // Only skips found while verifying the candidates in this window. The
    // scan-wide counts in `scan.skipped` (too recently archived, too recent,
    // ...) describe messages this run never looked at, so reporting them as
    // "skipped" next to the verified count is misleading.
    let mut total_skipped: BTreeMap<String, u64> = BTreeMap::new();
    for (reason, count) in flagged_skipped {
        *total_skipped.entry(reason).or_insert(0) += count;
    }
    // Same folder order as `execute_move`, so the previewed set is exactly
    // what execution walks first.
    for folder in &scan.folders {
        let folder_candidates: Vec<&plan::Candidate> = kept
            .iter()
            .filter(|candidate| candidate.mailbox.id == folder.id)
            .collect();
        if folder_candidates.is_empty() {
            continue;
        }
        let folder_name = folder.name.clone();
        let mut folder_examined = 0u64;
        let mut folder_skipped = BTreeMap::new();
        let mut verified = 0u64;
        let mut bytes = 0u64;
        let mut reached_batch = false;

        for candidate in folder_candidates {
            if total_verified as usize >= per_run {
                reached_batch = true;
                break;
            }
            folder_examined += 1;
            total_examined += 1;
            if cancel.is_cancelled() {
                return Err(raise_error!(
                    "cleanup preview was cancelled".into(),
                    ErrorCode::InvalidParameter
                ));
            }
            match verify::verify_local(account.id, candidate) {
                Ok(()) => {}
                Err(VerifyIssue::Skip(reason)) => {
                    add_skip(&mut folder_skipped, reason);
                    add_skip(&mut total_skipped, reason);
                    continue;
                }
                Err(VerifyIssue::LocalIntegrity(code)) => {
                    local_integrity_failures += 1;
                    add_skip(&mut folder_skipped, code);
                    add_skip(&mut total_skipped, code);
                    continue;
                }
                Err(VerifyIssue::Fatal { code, .. }) => {
                    // verify_local does not produce this today; count it the
                    // same way rather than dropping the message silently.
                    local_integrity_failures += 1;
                    add_skip(&mut folder_skipped, code);
                    add_skip(&mut total_skipped, code);
                    continue;
                }
            }
            match verify::verify_remote(
                &mut session,
                candidate,
                config.skip_flagged,
                cancel,
            )
            .await
            {
                Ok(_) => {
                    verified += 1;
                    total_verified += 1;
                    bytes += candidate.size;
                }
                Err(VerifyIssue::Skip(reason)) => {
                    add_skip(&mut folder_skipped, reason);
                    add_skip(&mut total_skipped, reason);
                }
                Err(VerifyIssue::LocalIntegrity(code)) => {
                    local_integrity_failures += 1;
                    add_skip(&mut folder_skipped, code);
                    add_skip(&mut total_skipped, code);
                }
                Err(VerifyIssue::Fatal { code, .. }) => {
                    // Execution stops at the first fatal identity mismatch.
                    will_abort = true;
                    remote_abort = true;
                    add_skip(&mut folder_skipped, code);
                    add_skip(&mut total_skipped, code);
                }
            }
        }

        total_bytes += bytes;
        folder_previews.push(CleanupFolderPreview {
            folder: folder_name,
            candidates: folder_examined,
            verified,
            bytes,
            skipped: skipped_vec(folder_skipped),
        });
        if reached_batch {
            break;
        }
    }
    session.logout().await.ok();

    if local_integrity_failures > 0 {
        if local_integrity_failures >= MAX_LOCAL_INTEGRITY_FAILURES as u64 {
            will_abort = true;
        }
        warnings.push(format!(
            "{local_integrity_failures} candidate message(s) failed local archive verification; execution counts them as failures and stops after {MAX_LOCAL_INTEGRITY_FAILURES}."
        ));
    }
    if remote_abort {
        warnings.push(
            "At least one message failed identity verification; execution will stop at the \
             first such message."
                .to_string(),
        );
    }
    let (token, expires_at) = issue_ticket(
        account.id,
        CleanupPreviewKind::Move,
        candidate_digest,
        config_digest,
    );
    let preview = CleanupPreview {
        account_id: account.id,
        kind: CleanupPreviewKind::Move,
        generated_at: now,
        expires_at,
        destination_folder: REMOTE_CLEANUP_FOLDER.to_string(),
        destination_exists,
        supports_move,
        // Candidates examined to fill the batch, not the batch size itself.
        candidates: total_examined,
        verified: total_verified,
        bytes: total_bytes,
        folders: folder_previews,
        skipped: skipped_vec(total_skipped),
        warnings,
        backup_configured,
        backup_last_success_at,
        backup_is_recent,
        will_abort,
        token,
        token_expired: false,
    };
    emit(Event::RemoteCleanupPreviewed {
        user: user.to_string(),
        account_id: account.id,
        kind: "move".into(),
        candidates: preview.candidates,
        verified: preview.verified,
        bytes: preview.bytes,
    });
    Ok(preview)
}

#[cfg(test)]
mod tests {
    use super::{consume_ticket, issue_ticket, CleanupPreviewKind};

    #[cfg(feature = "web-api")]
    #[test]
    fn preview_kind_uses_lowercase_on_both_wire_layers() {
        use poem_openapi::types::{ParseFromJSON, ToJSON};

        let parsed = CleanupPreviewKind::parse_from_json(Some(serde_json::Value::String(
            "move".into(),
        )))
        .expect("poem-openapi must accept the lowercase value the UI sends");
        assert_eq!(parsed, CleanupPreviewKind::Move);
        assert_eq!(
            CleanupPreviewKind::Move.to_json(),
            Some(serde_json::Value::String("move".into()))
        );
    }

    #[test]
    fn token_is_single_use() {
        let (token, _) = issue_ticket(7, CleanupPreviewKind::Move, "payload".into(), "cfg".into());
        assert!(consume_ticket(&token, 7, CleanupPreviewKind::Move, "payload", "cfg").is_ok());
        assert!(consume_ticket(&token, 7, CleanupPreviewKind::Move, "payload", "cfg").is_err());
    }

    #[test]
    fn token_rejects_changed_payload_or_config() {
        let (token, _) = issue_ticket(7, CleanupPreviewKind::Move, "payload".into(), "cfg".into());
        assert!(
            consume_ticket(&token, 7, CleanupPreviewKind::Move, "other", "cfg").is_err(),
            "a changed candidate set must invalidate the token"
        );

        let (token, _) = issue_ticket(7, CleanupPreviewKind::Move, "payload".into(), "cfg".into());
        assert!(
            consume_ticket(&token, 7, CleanupPreviewKind::Move, "payload", "other").is_err(),
            "a changed configuration must invalidate the token"
        );
    }

    #[test]
    fn token_is_bound_to_the_account() {
        let (token, _) = issue_ticket(7, CleanupPreviewKind::Move, "p".into(), "c".into());
        assert!(consume_ticket(&token, 8, CleanupPreviewKind::Move, "p", "c").is_err());
    }

    #[test]
    fn expired_token_is_rejected() {
        let (token, _) = issue_ticket(7, CleanupPreviewKind::Move, "p".into(), "c".into());
        {
            let mut tickets = super::TICKETS.lock().unwrap();
            tickets.get_mut(&token).unwrap().expires_at = 0;
        }
        assert!(consume_ticket(&token, 7, CleanupPreviewKind::Move, "p", "c").is_err());
    }
}
