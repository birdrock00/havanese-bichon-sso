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

//! Per-message proof. Both halves run in the same pass and a move is only
//! allowed to consume [`VerifiedMove`], which cannot be constructed from a
//! bare UID.

use crate::archive::imap::cleanup::plan::Candidate;
use crate::envelope::extractor::reattach_eml_content;
use crate::imap::executor::ImapExecutor;
use crate::imap::session::SessionStream;
use crate::utils::compute_content_hash;
use async_imap::Session;
use tokio_util::sync::CancellationToken;

/// Stable reason keys for local-archive proof failures.
const LOCAL_ENVELOPE_MISSING: &str = "local_envelope_missing";
const LOCAL_BLOB_EMPTY: &str = "local_blob_empty";
const LOCAL_HASH_MISMATCH: &str = "local_hash_mismatch";
const LOCAL_BLOB_UNREADABLE: &str = "local_blob_unreadable";

/// Stable reason keys for remote-identity failures. The run report counts and
/// the WebUI translates these codes; the detail string is for logs and the
/// run's stop message.
const REMOTE_UIDVALIDITY_MISSING: &str = "local_uidvalidity_missing";
const REMOTE_EXAMINE_FAILED: &str = "examine_failed";
const REMOTE_UIDVALIDITY_CHANGED: &str = "uidvalidity_changed";
const REMOTE_FLAG_LOOKUP_FAILED: &str = "flag_lookup_failed";
const REMOTE_HEADER_FETCH_FAILED: &str = "header_fetch_failed";
const REMOTE_SIZE_MISMATCH: &str = "size_mismatch";
const REMOTE_INTERNAL_DATE_MISMATCH: &str = "internal_date_mismatch";
const REMOTE_MESSAGE_ID_MISMATCH: &str = "message_id_mismatch";
const REMOTE_MESSAGE_ID_MISSING: &str = "remote_message_id_missing";
const REMOTE_MESSAGE_ID_SEARCH_FAILED: &str = "message_id_search_failed";
const REMOTE_MESSAGE_ID_AMBIGUOUS: &str = "message_id_ambiguous";

/// Reused by the executor, which re-checks UIDVALIDITY before a move.
pub const FATAL_UIDVALIDITY_CHANGED: &str = REMOTE_UIDVALIDITY_CHANGED;

/// Proof that one archived message may be moved out of its source folder.
#[derive(Clone, Debug)]
pub struct VerifiedMove {
    pub folder: String,
    pub folder_id: u64,
    pub uid: u32,
    pub uid_validity: u32,
    pub message_id: String,
    pub content_hash: String,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub enum VerifyIssue {
    /// Expected policy skip. Continue with the next message.
    Skip(&'static str),
    /// The local archive is not intact. A single occurrence skips the
    /// message; repeated occurrences stop the run (see
    /// `MAX_LOCAL_INTEGRITY_FAILURES`).
    LocalIntegrity(&'static str),
    /// Identity or remote state is ambiguous. Stop the run immediately.
    ///
    /// `code` is the stable reason key used for counting and translation;
    /// `detail` carries the human-readable specifics for logs and the run's
    /// stop message.
    Fatal {
        code: &'static str,
        detail: String,
    },
}

impl VerifyIssue {
    pub fn fatal(code: &'static str, detail: impl Into<String>) -> Self {
        VerifyIssue::Fatal {
            code,
            detail: detail.into(),
        }
    }
}

/// Prove the local archive still contains this message, byte for byte.
///
/// The stored blob is attachment-stripped: ingest replaces raw attachment
/// bodies with hash placeholders. Hashing the blob directly therefore fails
/// for every message with a detached attachment, so the EML must be
/// reassembled first — the same path reads and exports use.
pub fn verify_local(account_id: u64, candidate: &Candidate) -> Result<(), VerifyIssue> {
    verify_local_content(account_id, &candidate.envelope_id, &candidate.content_hash)
        .map_err(VerifyIssue::LocalIntegrity)
}

fn verify_local_content(
    account_id: u64,
    envelope_id: &str,
    content_hash: &str,
) -> Result<(), &'static str> {
    let envelope_id = match envelope_id.is_empty() {
        false => envelope_id.to_string(),
        true => return Err(LOCAL_ENVELOPE_MISSING),
    };

    let (_, bytes) =
        reattach_eml_content(account_id, envelope_id).map_err(|_| LOCAL_BLOB_UNREADABLE)?;
    if bytes.is_empty() {
        return Err(LOCAL_BLOB_EMPTY);
    }
    if compute_content_hash(&bytes) != content_hash {
        return Err(LOCAL_HASH_MISMATCH);
    }
    Ok(())
}

/// Prove the remote message at `candidate.uid` is the archived message.
///
/// Verification compares the identity metadata the archive recorded — the
/// resolved UID, Message-ID, size and INTERNALDATE — with the server's copy.
/// The body itself is not re-downloaded: the move is reversible, the archive
/// keeps its own full copy, and a body hash would only add a full download per
/// message to catch a server that rewrites content while preserving all of the
/// above.
///
/// The caller must not rely on the session's current selection; this function
/// selects the source folder itself.
///
/// The stored UID is only a *locator*. After a UIDVALIDITY change the envelope
/// keeps its old UID while the mailbox row already carries the new
/// UIDVALIDITY (see `download::flow::reconcile_uid_validity_change`, which
/// re-downloads only what is missing), so the stored UID can be stale while the
/// message itself is still there. The Message-ID is the real identity, so a
/// missing UID falls back to a `HEADER Message-ID` search and the normal
/// verification then runs against whatever UID that search resolves to.
pub async fn verify_remote(
    session: &mut Session<Box<dyn SessionStream>>,
    candidate: &Candidate,
    skip_flagged: bool,
    token: &CancellationToken,
) -> Result<VerifiedMove, VerifyIssue> {
    let mailbox = &candidate.mailbox;
    let expected_uid_validity = mailbox.uid_validity.unwrap_or(0);
    if expected_uid_validity == 0 {
        return Err(VerifyIssue::fatal(
            REMOTE_UIDVALIDITY_MISSING,
            "local mailbox has no UIDVALIDITY",
        ));
    }

    let remote_uid_validity = ImapExecutor::examine_mailbox(session, &mailbox.encoded_name())
        .await
        .map_err(|e| VerifyIssue::fatal(REMOTE_EXAMINE_FAILED, format!("examine failed: {e}")))?;
    if remote_uid_validity != expected_uid_validity {
        return Err(VerifyIssue::fatal(
            REMOTE_UIDVALIDITY_CHANGED,
            format!(
                "UIDVALIDITY changed for '{}' (local {expected_uid_validity}, remote {remote_uid_validity})",
                mailbox.name
            ),
        ));
    }

    let uid = resolve_remote_uid(session, candidate, token).await?;

    if skip_flagged {
        let flagged = ImapExecutor::uid_is_flagged(session, &mailbox.encoded_name(), uid)
            .await
            .map_err(|e| {
                VerifyIssue::fatal(REMOTE_FLAG_LOOKUP_FAILED, format!("flag lookup failed: {e}"))
            })?;
        if flagged {
            return Err(VerifyIssue::Skip(super::SKIP_FLAGGED));
        }
    }

    let uid_set = uid.to_string();
    let headers = ImapExecutor::fetch_uid_headers(session, &uid_set, token.clone(), None)
        .await
        .map_err(|e| {
            VerifyIssue::fatal(REMOTE_HEADER_FETCH_FAILED, format!("header fetch failed: {e}"))
        })?;
    let header = headers
        .into_iter()
        .find(|header| header.uid == uid)
        // The UID resolved a moment ago and is gone now: the message was
        // deleted between the search and the fetch.
        .ok_or_else(|| VerifyIssue::Skip(super::SKIP_SOURCE_UID_MISSING))?;

    if header.size != 0 && candidate.size != 0 && header.size != candidate.size {
        return Err(VerifyIssue::fatal(
            REMOTE_SIZE_MISMATCH,
            format!(
                "size mismatch for uid {} in '{}' (remote {}, local {})",
                uid, mailbox.name, header.size, candidate.size
            ),
        ));
    }
    if header.internal_date != 0
        && candidate.internal_date != 0
        && header.internal_date != candidate.internal_date
    {
        return Err(VerifyIssue::fatal(
            REMOTE_INTERNAL_DATE_MISMATCH,
            format!("internal date mismatch for uid {} in '{}'", uid, mailbox.name),
        ));
    }
    match (&header.message_id, candidate.message_id.as_str()) {
        (Some(remote), local) if !local.is_empty() => {
            if remote != local {
                return Err(VerifyIssue::fatal(
                    REMOTE_MESSAGE_ID_MISMATCH,
                    format!("message-id mismatch for uid {} in '{}'", uid, mailbox.name),
                ));
            }
        }
        (None, local) if !local.is_empty() => {
            return Err(VerifyIssue::fatal(
                REMOTE_MESSAGE_ID_MISSING,
                format!(
                    "remote message has no message-id but the local archive does (uid {} in '{}')",
                    uid, mailbox.name
                ),
            ));
        }
        _ => {}
    }

    Ok(VerifiedMove {
        folder: mailbox.name.clone(),
        folder_id: mailbox.id,
        uid,
        uid_validity: expected_uid_validity,
        message_id: candidate.message_id.clone(),
        content_hash: candidate.content_hash.clone(),
        size: candidate.size,
    })
}

/// Find the UID that currently holds this message in the source mailbox.
///
/// Tries the archived UID first — the common case, one round trip. When it is
/// gone, searches by Message-ID and keeps only the hits that also agree on the
/// metadata the archive recorded, so a message that was copied or re-ingested
/// cannot be mistaken for the original.
///
/// Every path that finds nothing returns [`SKIP_SOURCE_UID_MISSING`]: not
/// being locatable by UID *or* Message-ID means the message really is gone from
/// the source mailbox.
async fn resolve_remote_uid(
    session: &mut Session<Box<dyn SessionStream>>,
    candidate: &Candidate,
    token: &CancellationToken,
) -> Result<u32, VerifyIssue> {
    let mailbox = &candidate.mailbox;

    // Fast path: the UID the archive recorded still exists **and** still holds
    // this message. Checking the UID alone is not enough: a mailbox restored or
    // rebuilt server-side can keep its UIDVALIDITY while reassigning UIDs, so
    // the stored UID may now point at an unrelated message. Requiring the
    // Message-ID here turns that case into the stale-UID fallback below instead
    // of a fatal identity error that would abort the whole run.
    let stored = candidate.uid.to_string();
    let headers = ImapExecutor::fetch_uid_headers(session, &stored, token.clone(), None)
        .await
        .map_err(|e| {
            VerifyIssue::fatal(REMOTE_HEADER_FETCH_FAILED, format!("header fetch failed: {e}"))
        })?;
    if let Some(header) = headers.iter().find(|header| header.uid == candidate.uid) {
        let same_message = candidate.message_id.is_empty()
            || header.message_id.as_deref() == Some(candidate.message_id.as_str());
        if same_message {
            return Ok(candidate.uid);
        }
        tracing::warn!(
            folder = %mailbox.name,
            stored_uid = candidate.uid,
            stored_message_id = %candidate.message_id,
            remote_message_id = ?header.message_id,
            "remote-cleanup: stored UID now holds a different message; resolving by Message-ID"
        );
    }

    // The stored UID is stale (typically a UIDVALIDITY change that reconciled
    // by Message-ID without rewriting already-archived UIDs, or a server-side
    // mailbox restore that reassigned UIDs). The message is still identified by
    // its Message-ID, so look it up that way.
    if candidate.message_id.is_empty() {
        return Err(VerifyIssue::Skip(super::SKIP_SOURCE_UID_MISSING));
    }
    let found = ImapExecutor::uid_search_in(
        session,
        &mailbox.encoded_name(),
        &super::header_search(&candidate.message_id),
    )
    .await
    .map_err(|e| {
        VerifyIssue::fatal(
            REMOTE_MESSAGE_ID_SEARCH_FAILED,
            format!("message-id search failed: {e}"),
        )
    })?;
    if found.is_empty() {
        return Err(VerifyIssue::Skip(super::SKIP_SOURCE_UID_MISSING));
    }

    // More than one copy can share a Message-ID. Keep only the ones whose size
    // and INTERNALDATE agree with what was archived, then require a single
    // winner: guessing between two live copies could move the wrong one.
    let set = crate::imap::executor::compress_uid_list(found);
    let headers = ImapExecutor::fetch_uid_headers(session, &set, token.clone(), None)
        .await
        .map_err(|e| {
            VerifyIssue::fatal(REMOTE_HEADER_FETCH_FAILED, format!("header fetch failed: {e}"))
        })?;
    let mut matches: Vec<u32> = headers
        .iter()
        .filter(|header| {
            header.message_id.as_deref() == Some(candidate.message_id.as_str())
                && (header.size == 0 || candidate.size == 0 || header.size == candidate.size)
                && (header.internal_date == 0
                    || candidate.internal_date == 0
                    || header.internal_date == candidate.internal_date)
        })
        .map(|header| header.uid)
        .collect();
    matches.sort_unstable();
    matches.dedup();

    match matches.as_slice() {
        [uid] => {
            tracing::warn!(
                folder = %mailbox.name,
                stored_uid = candidate.uid,
                resolved_uid = uid,
                message_id = %candidate.message_id,
                "remote-cleanup: stored UID is stale, message located by Message-ID"
            );
            Ok(*uid)
        }
        [] => Err(VerifyIssue::Skip(super::SKIP_SOURCE_UID_MISSING)),
        // Two live copies that the archive cannot tell apart: refuse rather
        // than move one of them.
        _ => Err(VerifyIssue::fatal(
            REMOTE_MESSAGE_ID_AMBIGUOUS,
            format!(
                "message-id {} is ambiguous in '{}' ({} matching copies)",
                candidate.message_id,
                mailbox.name,
                matches.len()
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::imap::cleanup::plan::Candidate;
    use crate::archive::imap::mailbox::MailBox;
    use crate::imap::mock_server::{
        examine_response, uid_fetch_metadata_response, uid_search_response, MockImapServer,
        MockImapServerHandle,
    };
    use crate::imap::session::SessionStream;
    use tokio_io_timeout::TimeoutStream;

    async fn mock_session(handle: &MockImapServerHandle) -> Session<Box<dyn SessionStream>> {
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

    fn candidate(uid: u32) -> Candidate {
        Candidate {
            mailbox: MailBox {
                name: "INBOX".into(),
                uid_validity: Some(42),
                ..Default::default()
            },
            envelope_id: "eid".into(),
            // mail_parser strips the angle brackets; the index stores it bare.
            message_id: "m@test.example".into(),
            subject: "Subject".into(),
            content_hash: "hash".into(),
            uid,
            // The metadata filter only compares size/INTERNALDATE when both
            // sides know them; the mock reports a synthetic date and no size,
            // so leave the archived values unknown.
            size: 0,
            date: 1,
            internal_date: 0,
            ingest_at: 1,
        }
    }

    /// The stored UID is stale after a UIDVALIDITY change. The message is
    /// still there under a *new* UID, so the Message-ID search must find it
    /// instead of silently skipping the message forever.
    #[tokio::test]
    async fn a_stale_uid_is_resolved_by_message_id() {
        let handle = MockImapServer::new()
            .respond("LOGIN", "{TAG} OK LOGIN done\r\n")
            .respond("EXAMINE", examine_response("INBOX", 10, 42, 11))
            // UID 5 is gone.
            .respond_once("UID FETCH", uid_fetch_metadata_response(&[]))
            // The Message-ID search finds the relocated copy at UID 77.
            .respond("UID SEARCH", uid_search_response(&[77]))
            .respond("UID FETCH", uid_fetch_metadata_response(&[(77, "m@test.example")]))
            .respond("LOGOUT", "* BYE\r\n{TAG} OK\r\n")
            .start()
            .await;
        let mut session = mock_session(&handle).await;

        let resolved = resolve_remote_uid(&mut session, &candidate(5), &CancellationToken::new())
            .await;
        session.logout().await.ok();

        assert_eq!(resolved.unwrap(), 77);
    }

    /// The stored UID still exists but now holds a *different* message — what a
    /// server-side mailbox restore that keeps UIDVALIDITY looks like. That is a
    /// stale UID, not an ambiguous identity: resolve by Message-ID instead of
    /// aborting the whole run.
    #[tokio::test]
    async fn a_reused_uid_is_resolved_by_message_id() {
        let handle = MockImapServer::new()
            .respond("LOGIN", "{TAG} OK LOGIN done\r\n")
            .respond("EXAMINE", examine_response("INBOX", 10, 42, 11))
            // UID 5 exists, but it carries someone else's Message-ID.
            .respond_once(
                "UID FETCH",
                uid_fetch_metadata_response(&[(5, "other@test.example")]),
            )
            // The Message-ID search finds the archived message at UID 77.
            .respond("UID SEARCH", uid_search_response(&[77]))
            .respond("UID FETCH", uid_fetch_metadata_response(&[(77, "m@test.example")]))
            .respond("LOGOUT", "* BYE\r\n{TAG} OK\r\n")
            .start()
            .await;
        let mut session = mock_session(&handle).await;

        let resolved = resolve_remote_uid(&mut session, &candidate(5), &CancellationToken::new())
            .await;
        session.logout().await.ok();

        assert_eq!(
            resolved.unwrap(),
            77,
            "a UID that now holds another message must fall back to Message-ID"
        );
    }

    /// Not found by UID *or* Message-ID: the message really is gone, so the
    /// skip is still the right outcome.
    #[tokio::test]
    async fn a_message_absent_under_every_identity_is_skipped() {
        let handle = MockImapServer::new()
            .respond("LOGIN", "{TAG} OK LOGIN done\r\n")
            .respond("EXAMINE", examine_response("INBOX", 10, 42, 11))
            .respond("UID FETCH", uid_fetch_metadata_response(&[]))
            .respond("UID SEARCH", uid_search_response(&[]))
            .respond("LOGOUT", "* BYE\r\n{TAG} OK\r\n")
            .start()
            .await;
        let mut session = mock_session(&handle).await;

        let resolved = resolve_remote_uid(&mut session, &candidate(5), &CancellationToken::new())
            .await;
        session.logout().await.ok();

        match resolved {
            Err(VerifyIssue::Skip(super::super::SKIP_SOURCE_UID_MISSING)) => {}
            other => panic!("expected source_uid_missing, got {other:?}"),
        }
    }

    /// Two live copies share the Message-ID and the archive cannot tell them
    /// apart. Moving either one could be wrong, so this must stop the run
    /// rather than pick one.
    #[tokio::test]
    async fn two_indistinguishable_copies_are_ambiguous() {
        let handle = MockImapServer::new()
            .respond("LOGIN", "{TAG} OK LOGIN done\r\n")
            .respond("EXAMINE", examine_response("INBOX", 10, 42, 11))
            .respond_once("UID FETCH", uid_fetch_metadata_response(&[]))
            .respond("UID SEARCH", uid_search_response(&[71, 72]))
            .respond(
                "UID FETCH",
                uid_fetch_metadata_response(&[
                    (71, "m@test.example"),
                    (72, "m@test.example"),
                ]),
            )
            .respond("LOGOUT", "* BYE\r\n{TAG} OK\r\n")
            .start()
            .await;
        let mut session = mock_session(&handle).await;

        let resolved = resolve_remote_uid(&mut session, &candidate(5), &CancellationToken::new())
            .await;
        session.logout().await.ok();

        match resolved {
            Err(VerifyIssue::Fatal { code, detail }) => {
                assert_eq!(code, "message_id_ambiguous");
                assert!(detail.contains("ambiguous"), "unexpected detail: {detail}")
            }
            other => panic!("two indistinguishable copies must stop the run: {other:?}"),
        }
    }
}
