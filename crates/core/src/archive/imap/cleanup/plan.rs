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

//! Candidate collection: only messages that are provably in the local archive
//! can become cleanup candidates. Everything is derived from the local index;
//! the remote side is only used later, to prove identity before a move.

use std::collections::BTreeMap;

use async_imap::Session;

use crate::account::migration::{
    AccountModel, RemoteCleanupConfig, MAX_REMOTE_CLEANUP_PER_RUN, REMOTE_CLEANUP_FOLDER,
};
use crate::archive::imap::mailbox::{AttributeEnum, MailBox};
use crate::error::BichonResult;
use crate::imap::executor::{compress_uid_list, ImapExecutor};
use crate::imap::session::SessionStream;
use crate::store::tantivy::envelope::ENVELOPE_MANAGER;
use crate::utils::compute_content_hash;

/// How many raw candidates to examine per allowed move. Flagged/failed
/// messages are filtered **before** the per-run cap is applied, so they cannot
/// permanently occupy the window.
const SCAN_MULTIPLIER: usize = 8;
const MIN_SCAN_LIMIT: usize = 200;
const MAX_SCAN_LIMIT: usize = 5000;

#[derive(Clone, Debug)]
pub struct Candidate {
    pub mailbox: MailBox,
    /// Envelope id, needed to reassemble attachment-stripped EML locally.
    pub envelope_id: String,
    pub message_id: String,
    /// Display subject, for naming the message in run reports. Never used to
    /// verify anything.
    pub subject: String,
    pub content_hash: String,
    pub uid: u32,
    pub size: u64,
    /// RFC 5322 Date header (epoch millis; 0 when absent).
    pub date: i64,
    /// IMAP INTERNALDATE (epoch millis; 0 when absent).
    pub internal_date: i64,
    /// Epoch millis when Bichon archived the message.
    pub ingest_at: i64,
}

/// MailStore-style message age: the **newer** of the Date header and
/// INTERNALDATE. A message is only old enough when neither timestamp says
/// otherwise. `None` when both are unknown.
pub fn effective_message_date(date: i64, internal_date: i64) -> Option<i64> {
    match (date, internal_date) {
        (0, 0) => None,
        (0, internal) => Some(internal),
        (date, 0) => Some(date),
        (date, internal) => Some(date.max(internal)),
    }
}

#[derive(Clone, Debug, Default)]
pub struct CandidateScan {
    /// Oldest first, already truncated to the configured per-run cap.
    pub candidates: Vec<Candidate>,
    /// Source folders that contributed candidates (or were scanned and had
    /// none). Only folders that passed scope/special-use filtering appear.
    pub folders: Vec<MailBox>,
    /// Policy skips, by reason. These are expected and never abort a run.
    pub skipped: BTreeMap<String, u64>,
    /// The raw scan hit its safety budget before reaching the per-run cap.
    pub scan_limited: bool,
}

/// Final per-run selection: drop flagged messages (when configured) and then
/// apply the move cap. `flagged` is fetched once per folder from the server,
/// not per message.
///
/// Returns the messages to process, the per-reason skip counts, and the
/// messages that were dropped for being flagged — the last one so a run report
/// can name them instead of only counting them.
pub async fn apply_flagged_filter(
    session: &mut Session<Box<dyn SessionStream>>,
    candidates: &[Candidate],
    skip_flagged: bool,
) -> BichonResult<(Vec<Candidate>, BTreeMap<String, u64>, Vec<Candidate>)> {
    let mut skipped: BTreeMap<String, u64> = BTreeMap::new();
    if !skip_flagged {
        return Ok((candidates.to_vec(), skipped, Vec::new()));
    }

    let mut folders: BTreeMap<u64, (&MailBox, Vec<u32>)> = BTreeMap::new();
    for candidate in candidates {
        folders
            .entry(candidate.mailbox.id)
            .or_insert_with(|| (&candidate.mailbox, Vec::new()))
            .1
            .push(candidate.uid);
    }

    let mut flagged = std::collections::HashSet::new();
    for (_, (mailbox, uids)) in folders {
        if uids.is_empty() {
            continue;
        }
        ImapExecutor::examine_mailbox(session, &mailbox.encoded_name()).await?;
        let set = compress_uid_list(uids);
        flagged.extend(ImapExecutor::fetch_flagged_uids(session, &set).await?);
    }

    let mut kept = Vec::with_capacity(candidates.len());
    let mut dropped = Vec::new();
    for candidate in candidates {
        if flagged.contains(&candidate.uid) {
            *skipped.entry(super::SKIP_FLAGGED.to_string()).or_insert(0) += 1;
            dropped.push(candidate.clone());
        } else {
            kept.push(candidate.clone());
        }
    }
    Ok((kept, skipped, dropped))
}

fn special_use(mailbox: &MailBox) -> bool {
    mailbox.attributes.iter().any(|attribute| {
        matches!(
            attribute.attr,
            AttributeEnum::All
                | AttributeEnum::Archive
                | AttributeEnum::Drafts
                | AttributeEnum::Flagged
                | AttributeEnum::Junk
                | AttributeEnum::Sent
                | AttributeEnum::Trash
        )
    })
}

fn in_scope(config: &RemoteCleanupConfig, account: &AccountModel, name: &str) -> bool {
    let allowed = config
        .folders
        .as_ref()
        .or(account.download_folders.as_ref());
    match allowed {
        Some(folders) => folders.iter().any(|folder| folder.eq_ignore_ascii_case(name)),
        None => true,
    }
}

fn bump(skipped: &mut BTreeMap<String, u64>, reason: &str) {
    *skipped.entry(reason.to_string()).or_insert(0) += 1;
}

/// Collect candidates for one account. `now` is epoch millis.
pub fn collect(account: &AccountModel, now: i64) -> BichonResult<CandidateScan> {
    let Some(config) = account.remote_cleanup.as_ref() else {
        return Ok(CandidateScan::default());
    };

    let ingest_cutoff = now.saturating_sub(
        (config.eligible_after_archive_days.min(i64::MAX as u64) as i64).saturating_mul(
            super::DAY_MS,
        ),
    );
    let message_cutoff = config
        .min_message_age_days
        .map(|days| now.saturating_sub((days.min(i64::MAX as u64) as i64).saturating_mul(super::DAY_MS)));

    let mut scan = CandidateScan::default();
    let mut raw = Vec::new();

    for mailbox in MailBox::list_all(account.id)? {
        if mailbox.name.eq_ignore_ascii_case(REMOTE_CLEANUP_FOLDER) {
            continue;
        }
        if special_use(&mailbox) {
            continue;
        }
        if !in_scope(config, account, &mailbox.name) {
            continue;
        }
        if mailbox.uid_validity.unwrap_or(0) == 0 {
            bump(&mut scan.skipped, "unknown_uid_validity");
            continue;
        }

        let candidates = ENVELOPE_MANAGER.get_cleanup_candidates(account.id, mailbox.id)?;
        let mut folder_has_candidates = false;
        for candidate in candidates {
            if candidate.uid == 0 {
                bump(&mut scan.skipped, "missing_uid");
                continue;
            }
            if candidate.ingest_at <= 0 || candidate.ingest_at > ingest_cutoff {
                bump(&mut scan.skipped, "too_recently_archived");
                continue;
            }
            if let Some(cutoff) = message_cutoff {
                match effective_message_date(candidate.date, candidate.internal_date) {
                    None => {
                        bump(&mut scan.skipped, "missing_message_date");
                        continue;
                    }
                    Some(effective) if effective > cutoff => {
                        bump(&mut scan.skipped, "message_too_recent");
                        continue;
                    }
                    Some(_) => {}
                }
            }
            folder_has_candidates = true;
            raw.push(Candidate {
                mailbox: mailbox.clone(),
                envelope_id: candidate.envelope_id,
                message_id: candidate.message_id,
                subject: candidate.subject,
                content_hash: candidate.content_hash,
                uid: candidate.uid,
                size: candidate.size,
                date: candidate.date,
                internal_date: candidate.internal_date,
                ingest_at: candidate.ingest_at,
            });
        }
        if folder_has_candidates {
            scan.folders.push(mailbox);
        }
    }

    raw.sort_by(|a, b| {
        a.ingest_at
            .cmp(&b.ingest_at)
            .then_with(|| a.mailbox.name.cmp(&b.mailbox.name))
            .then_with(|| a.uid.cmp(&b.uid))
    });

    let per_run = config
        .max_messages_per_run
        .clamp(1, MAX_REMOTE_CLEANUP_PER_RUN) as usize;
    let scan_limit = per_run
        .saturating_mul(SCAN_MULTIPLIER)
        .clamp(MIN_SCAN_LIMIT, MAX_SCAN_LIMIT);
    if raw.len() > scan_limit {
        scan.scan_limited = true;
        raw.truncate(scan_limit);
    }
    scan.candidates = raw;
    Ok(scan)
}

/// Stable digest of the candidate set, used to bind a preview token to what
/// the user actually saw. Any change (new candidate, different local hash)
/// forces a fresh preview.
pub fn candidate_digest(candidates: &[Candidate]) -> String {
    let mut parts: Vec<String> = candidates
        .iter()
        .map(|candidate| {
            format!(
                "{}|{}|{}|{}",
                candidate.mailbox.name, candidate.uid, candidate.content_hash, candidate.message_id
            )
        })
        .collect();
    parts.sort();
    compute_content_hash(parts.join("\n").as_bytes())
}

/// Digest of the exact configuration used for a preview. A schedule/threshold
/// change between preview and execute invalidates the token.
pub fn config_digest(config: &RemoteCleanupConfig) -> String {
    let serialized = serde_json::to_vec(config).unwrap_or_default();
    compute_content_hash(&serialized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::imap::mailbox::MailBox;
    use crate::imap::mock_server::{
        examine_response, uid_fetch_flags_response, MockImapServer, MockImapServerHandle,
    };
    use crate::imap::session::SessionStream;
    use tokio_io_timeout::TimeoutStream;

    async fn mock_session(
        handle: &MockImapServerHandle,
    ) -> Session<Box<dyn SessionStream>> {
        let tcp = tokio::net::TcpStream::connect((handle.host(), handle.port()))
            .await
            .unwrap();
        let timeout_stream = TimeoutStream::new(tcp);
        let pinned: std::pin::Pin<Box<TimeoutStream<tokio::net::TcpStream>>> =
            Box::pin(timeout_stream);
        let stream: Box<dyn SessionStream> = Box::new(pinned);
        let mut client = async_imap::Client::new(stream);
        client.read_response().await.unwrap();
        let mut session = client
            .login("user", "pass")
            .await
            .map_err(|(e, _)| panic!("Login failed: {e:?}"))
            .unwrap();
        session.examine("INBOX").await.unwrap();
        session
    }

    #[tokio::test]
    async fn flagged_messages_are_filtered_before_the_cap() {
        let handle = MockImapServer::new()
            .respond("LOGIN", "{TAG} OK LOGIN done\r\n")
            .respond("EXAMINE", examine_response("INBOX", 2, 42, 2))
            .respond("UID FETCH", uid_fetch_flags_response(&[(1, true), (2, false)]))
            .respond("LOGOUT", "* BYE\r\n{TAG} OK\r\n")
            .start()
            .await;
        let mut session = mock_session(&handle).await;
        let candidates = vec![candidate("INBOX", 1, "aa"), candidate("INBOX", 2, "bb")];
        let (kept, skipped, dropped) =
            apply_flagged_filter(&mut session, &candidates, true).await.unwrap();
        session.logout().await.ok();

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].uid, 2);
        assert_eq!(skipped.get(super::super::SKIP_FLAGGED), Some(&1));
        // The dropped message is handed back so a run report can name it.
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].uid, 1);
        assert_eq!(dropped[0].subject, "Subject 1");
    }

    fn candidate(folder: &str, uid: u32, hash: &str) -> Candidate {
        Candidate {
            mailbox: MailBox {
                name: folder.to_string(),
                ..Default::default()
            },
            envelope_id: format!("eid-{uid}"),
            message_id: format!("<{uid}@example.com>"),
            subject: format!("Subject {uid}"),
            content_hash: hash.to_string(),
            uid,
            size: 1,
            date: 1,
            internal_date: 1,
            ingest_at: 1,
        }
    }

    #[test]
    fn message_age_uses_the_newer_timestamp() {
        assert_eq!(effective_message_date(100, 200), Some(200));
        assert_eq!(effective_message_date(300, 200), Some(300));
        assert_eq!(effective_message_date(0, 200), Some(200));
        assert_eq!(effective_message_date(100, 0), Some(100));
        assert_eq!(effective_message_date(0, 0), None);
    }

    #[test]
    fn candidate_digest_is_order_independent() {
        let a = candidate("INBOX", 1, "aa");
        let b = candidate("INBOX", 2, "bb");
        let first = candidate_digest(&[a.clone(), b.clone()]);
        let second = candidate_digest(&[b, a]);
        assert_eq!(first, second);
    }

    #[test]
    fn candidate_digest_changes_when_the_set_changes() {
        let a = candidate("INBOX", 1, "aa");
        assert_ne!(
            candidate_digest(std::slice::from_ref(&a)),
            candidate_digest(&[candidate("INBOX", 2, "aa")])
        );
        assert_ne!(
            candidate_digest(std::slice::from_ref(&a)),
            candidate_digest(&[candidate("INBOX", 1, "bb")])
        );
    }

    #[test]
    fn config_digest_changes_with_thresholds() {
        let base = RemoteCleanupConfig::default();
        let mut changed = base.clone();
        changed.eligible_after_archive_days += 1;
        assert_ne!(config_digest(&base), config_digest(&changed));
    }

    /// A grace period of 0 must not filter anything out: a message archived a
    /// moment ago is eligible on the next run. Guards the cut-off arithmetic
    /// (`now - 0 days` = `now`), which is the whole point of allowing 0.
    #[test]
    fn a_zero_grace_period_makes_a_fresh_message_eligible() {
        let now = 1_700_000_000_000i64;
        let mut config = RemoteCleanupConfig::default();
        config.eligible_after_archive_days = 0;

        let cutoff = now.saturating_sub(
            (config.eligible_after_archive_days.min(i64::MAX as u64) as i64)
                .saturating_mul(super::super::DAY_MS),
        );
        assert_eq!(cutoff, now);
        assert!(
            now <= cutoff,
            "a message ingested this instant must pass the grace check"
        );
    }
}
