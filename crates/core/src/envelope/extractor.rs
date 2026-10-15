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

use async_imap::types::Fetch;
use bytes::Bytes;
use mail_parser::{Address, HeaderName, Message, MessageParser, MimeHeaders};
use tantivy::{schema::Facet, TantivyDocument};
use tracing::error;
use uuid::Uuid;

use crate::{
<<<<<<< HEAD
    account::migration::AccountModel,
    archive::imap::mailbox::MailBox,
    backup::gate::{BACKUP_ACQUIRE_TIMEOUT, WRITE_GATE},
=======
    account::migration::{AccountModel, ExtractionRules},
    archive::imap::mailbox::MailBox,
    backup::gate::{BACKUP_ACQUIRE_TIMEOUT, WRITE_GATE, WriteGate},
>>>>>>> upstream/main
    common::AddrVec,
    envelope::{meta::parse_bichon_metadata, utils::normalize_subject},
    error::{code::ErrorCode, BichonResult},
    id,
    imap::executor::ImapExecutor,
    message::content::AttachmentInfo,
    raise_error,
    store::{
        blob::{DetachedEmail, BLOB_MANAGER},
        envelope::Envelope,
        tantivy::{
            attachment::ATTACHMENT_MANAGER,
            dedup_cache::DEDUP_CACHE,
            envelope::ENVELOPE_MANAGER,
            model::{AttachmentModel, EnvelopeWithAttachments},
        },
    },
    utc_now,
    utils::{compute_content_hash, hex_hash, html::extract_text},
};

/// The outcome of extracting an envelope. `Duplicate` means the message was
/// skipped because its content hash was already archived. `Imported` covers
/// every other processed message, including mail dropped by archive rules,
/// which has always counted as a success on the import surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum ExtractOutcome {
    Imported,
    Duplicate,
}

pub async fn extract_envelope_and_store_it(
    fetch: Fetch,
    account_id: u64,
    mailbox_id: u64,
) -> BichonResult<()> {
    let _write_guard = WRITE_GATE.acquire(BACKUP_ACQUIRE_TIMEOUT).await?;
    let internal_date = fetch
        .internal_date()
        .map(|d| d.timestamp_millis())
        .unwrap_or(0);
    let uid = fetch.uid.unwrap_or(0);
    let body = match fetch.body() {
        Some(b) => b,
        None => {
            tracing::warn!(
                account_id,
                uid = fetch.uid,
                "FETCH response has no body, skipping message"
            );
            return Ok(());
        }
    };
    let size = fetch.size.unwrap_or(body.len() as u32);
    extract_envelope_core(body, uid, size, internal_date, account_id, mailbox_id)
        .await
        .map(|_| ())
}

pub async fn extract_envelope_from_eml(
    body: &[u8],
    account_id: u64,
    mailbox_id: u64,
) -> BichonResult<ExtractOutcome> {
    let _write_guard = WRITE_GATE.acquire(BACKUP_ACQUIRE_TIMEOUT).await?;
    extract_envelope_core(body, 0, body.len() as u32, 0, account_id, mailbox_id).await
}

pub async fn extract_envelope_from_smtp(
    body: &[u8],
    account_id: u64,
    mailbox_id: u64,
) -> BichonResult<ExtractOutcome> {
    let _write_guard = WRITE_GATE.acquire(BACKUP_ACQUIRE_TIMEOUT).await?;
    extract_envelope_core(
        body,
        0,
        body.len() as u32,
        utc_now!(),
        account_id,
        mailbox_id,
    )
    .await
}

async fn extract_envelope_core(
    body: &[u8],
    uid: u32,
    size: u32,
    internal_date: i64,
    account_id: u64,
    mailbox_id: u64,
) -> BichonResult<ExtractOutcome> {
    // The content hash of the original raw EML
    let email_content_hash = compute_content_hash(body);
    if DEDUP_CACHE.contains(account_id, mailbox_id, &email_content_hash) {
        tracing::debug!("Duplicate email detected");
        // println!("Duplicate email detected");
        return Ok(ExtractOutcome::Duplicate);
    }
    // A message that cannot be parsed is a per-message failure, not a system
    // failure: streaming callers match on EmailParseError to skip the message
    // (it is NOT archived) and continue with the rest of the batch instead of
    // letting one broken email abort the whole folder (#377).
    let message: Message<'_> = MessageParser::new().parse(body).ok_or_else(|| {
        raise_error!(
            format!(
                "Email parse failed (uid {uid}, {size} bytes): message not archived"
            ),
            ErrorCode::EmailParseError
        )
    })?;

    let account = AccountModel::get(account_id).ok();
    if let Some(ref account) = account {
        if let Some(ref rules) = account.archive_rules {
            let sender = message.from().and_then(|addr| {
                AddrVec::from(addr)
                    .0
                    .into_iter()
                    .next()
                    .and_then(|a| a.address)
            });
            let subject = message.subject().map(|s| s.to_string());

            let is_spam = !rules.spam_headers.is_empty()
                && rules.spam_headers.iter().any(|h| {
                    message
                        .header_raw(h.clone())
                        .map(|v| matches!(v.trim().to_lowercase().as_str(), "yes" | "true"))
                        .unwrap_or(false)
                });

            if !rules.should_archive(sender.as_deref(), subject.as_deref(), size, is_spam) {
                tracing::debug!(
                    account_id,
                    uid,
                    sender = sender.as_deref().unwrap_or("?"),
                    subject = subject.as_deref().unwrap_or("?"),
                    "Email filtered out by archive rules"
                );
                return Ok(ExtractOutcome::Imported);
            }
        }
    }

    let preview_limit = 100;
    let text = if let Some(text) = message.body_text(0).map(|cow| cow.into_owned()) {
        text
    } else if let Some(html) = message.body_html(0).map(|cow| cow.into_owned()) {
        extract_text(html)
    } else {
        String::new()
    };

    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");

    let preview = if text.chars().count() > preview_limit {
        text.chars().take(preview_limit).collect::<String>() + "..."
    } else {
        text.clone()
    };

    let body_text = text;

    let message_id = message
        .message_id()
        .map(String::from)
        .unwrap_or_else(generate_message_id);

    let in_reply_to = message.in_reply_to().as_text().map(String::from);
    let references = extract_references(&message);
    let thread_id = compute_thread_id(in_reply_to, references, &message_id);

    let mut subject = message.subject().map(String::from).unwrap_or_default();
    if subject.contains('\u{FFFD}') {
        subject = normalize_subject(message.header_raw(HeaderName::Subject));
    }

    let date = message.date().map(|d| d.to_timestamp() * 1000).unwrap_or(0);
    let internal_date = if internal_date == 0 {
        date
    } else {
        internal_date
    };

    // Retention floor: never ingest a message whose effective date is already
    // outside the account's retention window. The sweep purges those envelopes,
    // so without this a full UID re-sync (UIDVALIDITY change / mailbox rebuild)
    // would treat them as missing and re-download them, fighting the sweep
    // forever. While an account is on legal hold retention is suspended, so the
    // floor is disabled and nothing is dropped.
    if let Some(ref account) = account {
        let days = account.retention_days_effective();
        if days > 0 && !account.is_on_hold() {
            if let Some(effective) = crate::retention::effective_date_ms(date, internal_date) {
                if effective < crate::retention::retention_cutoff_ms(days) {
                    tracing::debug!(
                        account_id,
                        uid,
                        effective,
                        "Email outside retention window, skipping ingest"
                    );
                    return Ok(ExtractOutcome::Imported);
                }
            }
        }
    }
    let parse_addrs = |addrs: Option<&Address<'_>>| {
        addrs
            .map(|addr| {
                AddrVec::from(addr)
                    .0
                    .into_iter()
                    .filter_map(|a| a.address)
                    .collect()
            })
            .unwrap_or_default()
    };

    let bcc = parse_addrs(message.bcc());
    let cc = parse_addrs(message.cc());
    let to = parse_addrs(message.to());

    let from = message
        .from()
        .and_then(|addr| AddrVec::from(addr).0.into_iter().next())
        .and_then(|add| add.address)
        .unwrap_or_else(|| "unknown".to_string());
    let attachment_count = message.attachment_count();
    let attachments =
        detach_and_store_attachments(body, &message, &email_content_hash, account_id, mailbox_id)
            .await;

    let envelope_id = Uuid::new_v4().to_string();
    let now = utc_now!();

    let mut final_tags = Vec::new();

    if let Some(meta_header) = message.header_raw("X-Bichon-Metadata") {
        if let Some(bmd) = parse_bichon_metadata(meta_header) {
            if let Some(tags) = bmd.tags {
                let validated_tags: Result<Vec<String>, _> = tags
                    .iter()
                    .map(|tag| Facet::from_text(tag).map(|_| tag.clone()).map_err(|e| e))
                    .collect();

                match validated_tags {
                    Ok(valid_list) => {
                        final_tags = valid_list;
                    }
                    Err(e) => {
                        eprintln!("Tag validation failed, ignoring all tags: {:#?}", e);
                    }
                }
            }
        }
    }

    let attachment_docs: Vec<TantivyDocument> = attachments
        .iter()
        .filter(|a| !a.inline || a.content_id.is_none())
        .map(|a| {
            let has_text = a.extracted_text.is_some();
            AttachmentModel {
                id: Uuid::new_v4().to_string(),
                envelope_id: envelope_id.clone(),
                account_id,
                account_email: None,
                mailbox_id,
                mailbox_name: None,
                subject: subject.clone(),
                content_hash: a.content_hash.clone(),
                from: from.clone(),
                date,
                ingest_at: now,
                size: a.size as u64,
                ext: a.get_extension(),
                category: a.get_category().to_string(),
                content_type: a.file_type.clone(),
                shard_id: 0,
                text: a.extracted_text.clone(),
                has_text,
                is_ocr: a.extracted_is_ocr,
                page_count: a.extracted_page_count.map(|n| n as u64),
                is_indexed: has_text,
                is_message: a.is_message,
                name: a.filename.clone(),
                tags: None,
                auto_tags: None,
            }
        })
        .map(|a| a.into_document())
        .collect();

    let envelope = Envelope {
        id: envelope_id,
        message_id,
        account_id,
        mailbox_id,
        uid,
        subject,
        preview,
        from,
        to,
        cc,
        bcc,
        date,
        internal_date,
        ingest_at: now,
        size,
        thread_id,
        attachment_count,
        regular_attachment_count: attachment_docs.len(),
        tags: (!final_tags.is_empty()).then_some(final_tags),
        account_email: None,
        mailbox_name: None,
        content_hash: email_content_hash.clone(),
        account_name: None,
    };
    // 'attachments' contains both regular and inline attachments
    let ea = EnvelopeWithAttachments {
        envelope,
        attachments: Some(attachments),
    };
    let doc = ea.to_document(&body_text, 0)?;
    tracing::debug!(
        "[account {}][mailbox {}] extract: uid={} msg_id={} content_hash={}",
        account_id,
        mailbox_id,
        uid,
        &ea.envelope.message_id,
        &ea.envelope.content_hash,
    );
    ENVELOPE_MANAGER.queue(doc).await;
    DEDUP_CACHE.insert(account_id, mailbox_id, &email_content_hash);
    for doc in attachment_docs {
        ATTACHMENT_MANAGER.queue(doc).await;
    }
    Ok(ExtractOutcome::Imported)
}

pub fn extract_envelope_from_nested_message(
    message: Message<'_>,
    account_id: u64,
) -> BichonResult<Envelope> {
    let text = if let Some(text) = message.body_text(0).map(|cow| cow.into_owned()) {
        text
    } else if let Some(html) = message.body_html(0).map(|cow| cow.into_owned()) {
        extract_text(html)
    } else {
        String::new()
    };

    let message_id = message
        .message_id()
        .map(String::from)
        .unwrap_or_else(generate_message_id);

    let in_reply_to = message.in_reply_to().as_text().map(String::from);
    let references = extract_references(&message);
    let thread_id = compute_thread_id(in_reply_to, references, &message_id);

    let mut subject = message.subject().map(String::from).unwrap_or_default();
    if subject.contains('\u{FFFD}') {
        subject = normalize_subject(message.header_raw(HeaderName::Subject));
    }

    let date = message.date().map(|d| d.to_timestamp() * 1000).unwrap_or(0);

    let parse_addrs = |addrs: Option<&Address<'_>>| {
        addrs
            .map(|addr| {
                AddrVec::from(addr)
                    .0
                    .into_iter()
                    .filter_map(|a| a.address)
                    .collect()
            })
            .unwrap_or_default()
    };

    let bcc = parse_addrs(message.bcc());
    let cc = parse_addrs(message.cc());
    let to = parse_addrs(message.to());

    let from = message
        .from()
        .and_then(|addr| AddrVec::from(addr).0.into_iter().next())
        .and_then(|add| add.address)
        .unwrap_or_else(|| "unknown".to_string());

    let envelope = Envelope {
        id: Default::default(),
        message_id,
        account_id,
        mailbox_id: Default::default(),
        uid: Default::default(),
        subject,
        preview: text,
        from,
        to,
        cc,
        bcc,
        date,
        internal_date: Default::default(),
        ingest_at: Default::default(),
        size: Default::default(),
        thread_id,
        attachment_count: Default::default(),
        regular_attachment_count: Default::default(),
        tags: Default::default(),
        account_email: Default::default(),
        account_name: Default::default(),
        mailbox_name: Default::default(),
        content_hash: Default::default(),
    };

    Ok(envelope)
}

pub fn compute_thread_id(
    in_reply_to: Option<String>,
    references: Option<Vec<String>>,
    message_id: &str,
) -> String {
    if in_reply_to.is_some() && references.as_ref().map_or(false, |r| !r.is_empty()) {
        return hex_hash(&references.as_ref().unwrap()[0]);
    }
    hex_hash(message_id)
}

pub fn generate_message_id() -> String {
    let ts = utc_now!();
    let pid = std::process::id();
    format!("<{:016x}.{}.{}@{}>", id!(128), ts, pid, "bichon")
}

pub fn extract_references(message: &Message<'_>) -> Option<Vec<String>> {
    match message.references() {
        mail_parser::HeaderValue::Text(cow) => Some(vec![cow.to_string()]),
        mail_parser::HeaderValue::TextList(vec) => {
            Some(vec.iter().map(|cow| cow.to_string()).collect())
        }
        _ => None,
    }
}

<<<<<<< HEAD
pub async fn detach_and_store_attachments(
    original_body: &[u8],
    message: &Message<'_>,
    eml_content_hash: &str,
    account_id: u64,
    mailbox_id: u64,
) -> Vec<AttachmentInfo> {
    let rules = if account_id > 0 {
        AccountModel::get(account_id)
            .ok()
            .and_then(|a| a.extraction_rules)
    } else {
        None
    };

    let mailbox_name = match rules.as_ref().map(|r| !r.folders.is_empty()) {
        Some(true) => MailBox::get(mailbox_id).ok().map(|mb| mb.name),
        _ => None,
    };

    let sender = message
        .from()
        .and_then(|addr| AddrVec::from(addr).0.into_iter().next())
        .and_then(|add| add.address);

    let mut stripped_eml = original_body.to_vec();
    let mut attachment_infos = Vec::new();
    // Step 1: Collect and sort attachment ranges in reverse to maintain offset
    // integrity
=======
/// Prefix of the placeholder that replaces an attachment body inside a
/// stripped EML. Full form: `<<BICHON_DETACH_HASH:{64 hex chars}>>`.
///
/// Which hash the 64 hex chars carry depends on when the record was written,
/// so tooling that scans placeholders must consult the attachment metadata
/// (`AttachmentInfo::raw_content_hash`) — the prefix alone cannot tell the
/// domains apart, because both produce 64-character blake3 hex digests:
///
/// - **Current records** (`raw_content_hash` is `Some`): the
///   domain-separated hash of the *original encoded* MIME body
///   (`compute_raw_attachment_hash`). The blob is stored under the same key.
/// - **Legacy records** (pre-#375, `raw_content_hash` is `None`): the plain
///   hash of the *decoded* attachment contents (`compute_content_hash`),
///   which is also the blob key. `AttachmentInfo::storage_hash()` selects
///   the right one per record.
pub const DETACH_PLACEHOLDER_PREFIX: &str = "<<BICHON_DETACH_HASH:";

/// Decoded contents of an attachment selected for text extraction, collected
/// while detaching so the extraction can run in one background batch.
pub struct TextCandidate {
    pub content_hash: String,
    pub file_type: String,
    pub ext: String,
    pub bytes: Vec<u8>,
}

/// Context deciding which detached attachments are queued for text
/// extraction. Supplied by the live ingest path; offline tools pass `None`
/// to [`strip_attachment_bodies`] and skip extraction entirely.
pub struct ExtractionFilter<'a> {
    /// Per-account rules; `None` means no rules are configured and the
    /// default is to extract every supported type.
    pub rules: Option<&'a ExtractionRules>,
    pub mailbox_name: Option<&'a str>,
    pub sender: Option<&'a str>,
}

/// Output of [`strip_attachment_bodies`].
pub struct DetachedBody {
    /// The EML with every valid attachment body replaced by a
    /// `<<BICHON_DETACH_HASH:...>>` placeholder.
    pub stripped_eml: Vec<u8>,
    /// `(storage key, raw encoded MIME body)` pairs to persist in the blob
    /// store. Invalid attachment ranges contribute a shared empty record in
    /// the raw key domain.
    pub blobs: Vec<(String, Bytes)>,
    /// Attachment metadata, in descending raw-offset order.
    pub infos: Vec<AttachmentInfo>,
    /// Attachments selected for text extraction (empty unless an
    /// [`ExtractionFilter`] was supplied).
    pub text_candidates: Vec<TextCandidate>,
}

/// Strips attachment bodies out of a raw message and computes everything the
/// storage layer needs to persist and later reconstruct it: the stripped EML
/// with hash placeholders, the raw blob records, attachment metadata, and
/// text-extraction candidates.
///
/// This is the single canonical implementation shared by the live ingest path
/// ([`detach_and_store_attachments`]) and the offline store migration; the
/// two must never diverge, or a migrated store stops matching what the
/// runtime writes and reads back.
pub fn strip_attachment_bodies(
    original_body: &[u8],
    message: &Message<'_>,
    filter: Option<&ExtractionFilter<'_>>,
) -> DetachedBody {
    let mut stripped_eml = original_body.to_vec();
    let mut infos = Vec::new();
    let mut blobs = Vec::with_capacity(message.attachment_count());
    let mut text_candidates = Vec::new();

    // Collect and sort attachment ranges in reverse to maintain offset
    // integrity while splicing placeholders.
>>>>>>> upstream/main
    let mut ranges: Vec<_> = message
        .attachments()
        .map(|att| {
            (
                att.raw_body_offset() as usize,
                att.raw_end_offset() as usize,
                att,
            )
        })
        .collect();
<<<<<<< HEAD

    ranges.sort_by(|a, b| b.0.cmp(&a.0));
    let mut attachments = Vec::with_capacity(ranges.len());

    // Collect candidates for text extraction (non-inline, known document types).
    struct TextCandidate {
        content_hash: String,
        file_type: String,
        ext: String,
        bytes: Vec<u8>,
    }
    let mut text_candidates: Vec<TextCandidate> = Vec::new();
=======
    ranges.sort_by(|a, b| b.0.cmp(&a.0));
>>>>>>> upstream/main

    for (raw_start, raw_end, att) in ranges {
        // mail-parser may report attachment offsets past the body end for
        // malformed messages; clamp the range to avoid a slice panic.
        let body_len = original_body.len();
        let raw_start = raw_start.min(body_len);
        let raw_end = raw_end.min(body_len);
        let range_valid = raw_start < raw_end;

        // content hash is computed from the decoded attachment contents,
        // which is always available regardless of raw offset validity.
        let content_hash = compute_content_hash(att.contents());

<<<<<<< HEAD
        if range_valid {
            let raw_bytes = &original_body[raw_start..raw_end];
            // The actual content stored in the blob is the raw undecoded data.
            attachments.push((content_hash.clone(), Bytes::copy_from_slice(raw_bytes)));

            // Replace raw attachment content with a hash-based placeholder
            let placeholder = format!("<<BICHON_DETACH_HASH:{}>>", &content_hash);
            stripped_eml.splice(raw_start..raw_end, placeholder.as_bytes().iter().cloned());
        } else {
            // Invalid range: store a zero-length blob so the consistency
            // check passes; reattachment will log a warning for the missing
            // blob data but won't panic.
            attachments.push((content_hash.clone(), Bytes::new()));
        }
=======
        let raw_content_hash = if range_valid {
            let raw_bytes = &original_body[raw_start..raw_end];
            // Different transfer encodings of one file must not share a raw blob.
            let raw_hash = crate::utils::compute_raw_attachment_hash(raw_bytes);
            blobs.push((raw_hash.clone(), Bytes::copy_from_slice(raw_bytes)));

            // Replace raw attachment content with a hash-based placeholder
            let placeholder = format!("{DETACH_PLACEHOLDER_PREFIX}{raw_hash}>>");
            stripped_eml.splice(raw_start..raw_end, placeholder.as_bytes().iter().cloned());
            Some(raw_hash)
        } else {
            // No replacement is needed, but keep the empty storage record in
            // the raw key domain so it cannot shadow a decoded or email blob.
            let raw_hash = crate::utils::compute_raw_attachment_hash(b"");
            blobs.push((raw_hash.clone(), Bytes::new()));
            Some(raw_hash)
        };
>>>>>>> upstream/main

        let inline = att
            .content_disposition()
            .map(|d| d.is_inline())
            .unwrap_or_else(|| att.content_id().is_some());
        let file_type = att
            .content_type()
            .map(|ct| {
                format!(
                    "{}/{}",
                    ct.c_type.as_ref(),
                    ct.c_subtype.as_deref().unwrap_or("")
                )
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let has_cid = att.content_id().is_some();
        let att_name = att.attachment_name().map(|n| n.to_string());
        let ext = att_name
            .as_deref()
            .and_then(|n| {
                std::path::Path::new(n)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_ascii_lowercase())
            })
            .unwrap_or_default();

<<<<<<< HEAD
        let should_extract = rules.as_ref().map_or(true, |r| {
            r.should_extract(
                &ext,
                mailbox_name.as_deref(),
                att_name.as_deref(),
                sender.as_deref(),
            )
        });

        if !inline || !has_cid {
            let decoded_len = att.contents().len();
            if should_extract
                && decoded_len <= crate::ext::text_extractor::max_extract_bytes()
=======
        // Collect candidates for text extraction (non-inline, known document
        // types allowed by the filter).
        if let Some(filter) = filter {
            let should_extract = filter.rules.map_or(true, |r| {
                r.should_extract(
                    &ext,
                    filter.mailbox_name,
                    att_name.as_deref(),
                    filter.sender,
                )
            });

            if (!inline || !has_cid)
                && should_extract
                && att.contents().len() <= crate::ext::text_extractor::max_extract_bytes()
>>>>>>> upstream/main
                && crate::ext::text_extractor::should_try_extract(&file_type, &ext)
            {
                text_candidates.push(TextCandidate {
                    content_hash: content_hash.clone(),
                    file_type: file_type.clone(),
                    ext: ext.clone(),
                    bytes: att.contents().to_vec(),
                });
            }
        }

<<<<<<< HEAD
        let info = AttachmentInfo {
=======
        infos.push(AttachmentInfo {
>>>>>>> upstream/main
            filename: att.attachment_name().map(|n| n.to_string()),
            size: att.contents().len(),
            inline,
            file_type,
            content_id: att.content_id().map(|id| id.to_string()),
<<<<<<< HEAD
            content_hash: content_hash.clone(),
=======
            content_hash,
            raw_content_hash,
>>>>>>> upstream/main
            is_message: att.is_message(),
            extracted_text: None,
            extracted_page_count: None,
            extracted_is_ocr: false,
<<<<<<< HEAD
        };

        attachment_infos.push(info);
    }

    // Run text extraction in a single spawn_blocking batch.
=======
        });
    }

    DetachedBody {
        stripped_eml,
        blobs,
        infos,
        text_candidates,
    }
}

pub async fn detach_and_store_attachments(
    original_body: &[u8],
    message: &Message<'_>,
    eml_content_hash: &str,
    account_id: u64,
    mailbox_id: u64,
) -> Vec<AttachmentInfo> {
    let rules = if account_id > 0 {
        AccountModel::get(account_id)
            .ok()
            .and_then(|a| a.extraction_rules)
    } else {
        None
    };

    let mailbox_name = match rules.as_ref().map(|r| !r.folders.is_empty()) {
        Some(true) => MailBox::get(mailbox_id).ok().map(|mb| mb.name),
        _ => None,
    };

    let sender = message
        .from()
        .and_then(|addr| AddrVec::from(addr).0.into_iter().next())
        .and_then(|add| add.address);

    let filter = ExtractionFilter {
        rules: rules.as_ref(),
        mailbox_name: mailbox_name.as_deref(),
        sender: sender.as_deref(),
    };
    let detached = strip_attachment_bodies(original_body, message, Some(&filter));
    let mut attachment_infos = detached.infos;

    // Run text extraction in a single spawn_blocking batch.
    let text_candidates = detached.text_candidates;
>>>>>>> upstream/main
    if !text_candidates.is_empty() {
        if let Ok(mut extracted_map) = tokio::task::spawn_blocking(move || {
            let mut map: std::collections::HashMap<String, (String, Option<u32>, bool)> =
                std::collections::HashMap::new();
            for c in text_candidates {
                if let Some(r) =
                    crate::ext::text_extractor::extract_text(&c.file_type, &c.ext, &c.bytes)
                {
                    map.insert(c.content_hash, (r.text, r.page_count, r.is_ocr));
                }
            }
            map
        })
        .await
        {
            for info in &mut attachment_infos {
                if let Some((text, pages, is_ocr)) = extracted_map.remove(&info.content_hash) {
                    info.extracted_text = Some(text);
                    info.extracted_page_count = pages;
                    info.extracted_is_ocr = is_ocr;
                }
            }
        }
    }
    // Step 4: Store the final stripped EML content
    BLOB_MANAGER
        .queue(DetachedEmail {
<<<<<<< HEAD
            email: (eml_content_hash.to_string(), Bytes::from(stripped_eml)),
            attachments: Some(attachments),
=======
            email: (
                eml_content_hash.to_string(),
                Bytes::from(detached.stripped_eml),
            ),
            attachments: Some(detached.blobs),
>>>>>>> upstream/main
        })
        .await;

    attachment_infos
}

pub fn reattach_eml_content(
    account_id: u64,
    envelope_id: String,
) -> BichonResult<(Envelope, Bytes)> {
    let e = ENVELOPE_MANAGER
        .get_envelope_by_id(account_id, &envelope_id)?
        .ok_or_else(|| {
            raise_error!(
                format!(
                    "Envelope not found: account_id={} envelope_id={}",
                    account_id, &envelope_id
                ),
                ErrorCode::ResourceNotFound
            )
        })?;

    let restored_eml = BLOB_MANAGER
        .get_email(&e.envelope.content_hash)?
        .ok_or_else(|| {
            raise_error!(
                format!(
                "Original email content not found: account_id={} envelope_id={} content_hash={}",
                account_id, &envelope_id, &e.envelope.content_hash
            ),
                ErrorCode::ResourceNotFound
            )
        })?;

    if !e.envelope.has_any_attachments() {
        return Ok((e.envelope, restored_eml));
    }

<<<<<<< HEAD
    let mut restored_eml = restored_eml.to_vec();
=======
>>>>>>> upstream/main
    let actual_count = e.attachments.as_ref().map(|a| a.len()).unwrap_or(0);
    if e.envelope.attachment_count != actual_count {
        return Err(raise_error!(
            format!(
                "Consistency check failed: envelope.attachment_count ({}) does not match attachments.len ({})",
                e.envelope.attachment_count,
                actual_count
            ),
            ErrorCode::InternalError
        ));
    }

<<<<<<< HEAD
    let mut tasks = Vec::new();
    for detail in e.attachments.unwrap() {
        let placeholder_str = format!("<<BICHON_DETACH_HASH:{}>>", &detail.content_hash);
        let pattern = placeholder_str.as_bytes();
        let pattern_len = pattern.len();

        let mut search_cursor = 0;
        while let Some(pos) = restored_eml[search_cursor..]
            .windows(pattern_len)
            .position(|window| window == pattern)
        {
            let absolute_start = search_cursor + pos;
            let absolute_end = absolute_start + pattern_len;

            tasks.push((absolute_start, absolute_end, detail.content_hash.clone()));
            search_cursor = absolute_end;
=======
    let restored = reattach_parts(restored_eml.to_vec(), e.attachments.unwrap(), |hash| {
        BLOB_MANAGER.get_attachment(hash)
    })?;
    Ok((e.envelope, restored))
}

fn reattach_parts(
    mut restored_eml: Vec<u8>,
    attachments: Vec<AttachmentInfo>,
    load: impl Fn(&str) -> BichonResult<Option<Bytes>>,
) -> BichonResult<Bytes> {
    let mut tasks = Vec::new();
    for detail in attachments {
        let storage_hash = detail.storage_hash();

        // Primary scan: placeholders written by the record's own format
        // (raw-body hash for new records, decoded hash for legacy records).
        let found = schedule_placeholder(&mut tasks, &restored_eml, storage_hash, None);

        // Mixed-format recovery: a re-ingest refreshes the envelope metadata
        // to raw storage keys, but the blob store's `exists` dedup keeps the
        // previously stored stripped EML, whose placeholders still carry the
        // legacy decoded hash. When the primary scan finds nothing, retry with
        // the legacy placeholder; the old blob is still stored under that key.
        // The raw key is kept as an alternate load target for the case where
        // the legacy blob was garbage-collected after the original envelope
        // was deleted (the re-ingest stored the identical bytes under it).
        if found == 0 && detail.raw_content_hash.is_some() {
            schedule_placeholder(
                &mut tasks,
                &restored_eml,
                &detail.content_hash,
                Some(storage_hash),
            );
>>>>>>> upstream/main
        }
    }

    tasks.sort_by(|a, b| b.0.cmp(&a.0));
<<<<<<< HEAD

    for (start, end, hash) in tasks {
        if let Some(original_data) = BLOB_MANAGER.get_attachment(&hash)? {
=======
    tasks.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

    for (start, end, hash, alt) in tasks {
        let original_data = match load(&hash)? {
            Some(data) => Some(data),
            None => match alt.as_deref() {
                Some(alt) => load(alt)?,
                None => None,
            },
        };
        if let Some(original_data) = original_data {
>>>>>>> upstream/main
            restored_eml.splice(start..end, original_data.iter().cloned());
        } else {
            error!("[ERROR] Missing attachment blob for hash: {}", hash);
        }
    }

<<<<<<< HEAD
    Ok((e.envelope, Bytes::from(restored_eml)))
=======
    Ok(Bytes::from(restored_eml))
}

/// Schedules one replacement task per occurrence of the placeholder carrying
/// `hash` and returns the number of occurrences found. `alt` is a secondary
/// storage key tried at load time when the primary blob is absent.
fn schedule_placeholder(
    tasks: &mut Vec<(usize, usize, String, Option<String>)>,
    eml: &[u8],
    hash: &str,
    alt: Option<&str>,
) -> usize {
    let placeholder_str = format!("{DETACH_PLACEHOLDER_PREFIX}{hash}>>");
    let pattern = placeholder_str.as_bytes();
    let pattern_len = pattern.len();

    let mut found = 0;
    let mut search_cursor = 0;
    while let Some(pos) = eml[search_cursor..]
        .windows(pattern_len)
        .position(|window| window == pattern)
    {
        let absolute_start = search_cursor + pos;
        let absolute_end = absolute_start + pattern_len;

        tasks.push((
            absolute_start,
            absolute_end,
            hash.to_string(),
            alt.map(str::to_string),
        ));
        search_cursor = absolute_end;
        found += 1;
    }
    found
>>>>>>> upstream/main
}

/// Returns the raw EML for an indexed message, self-healing a missing content
/// blob.
///
/// Behaves like [`reattach_eml_content`], but when the message's content blob
/// is absent from the blob store it fetches that single message on demand from
/// the IMAP server (`UID FETCH <uid> (BODY.PEEK[])`), persists it for future
/// requests, and returns it. If the on-demand fetch itself fails, the original
/// "content not found" error from [`reattach_eml_content`] is surfaced
/// unchanged so the caller still produces its 404.
pub async fn reattach_eml_content_self_healing(
    account_id: u64,
    envelope_id: String,
) -> BichonResult<(Envelope, Bytes)> {
    let envelope = ENVELOPE_MANAGER
        .get_envelope_by_id(account_id, &envelope_id)?
        .ok_or_else(|| {
            raise_error!(
                format!(
                    "Envelope not found: account_id={} envelope_id={}",
                    account_id, &envelope_id
                ),
                ErrorCode::ResourceNotFound
            )
        })?
        .envelope;

    // Fast path: the content blob is present, reuse the regular reattach logic.
    if BLOB_MANAGER.get_email(&envelope.content_hash)?.is_some() {
        return reattach_eml_content(account_id, envelope_id);
    }

    // The blob is missing. Try to recover it directly from the IMAP server.
    match recover_message_blob(&envelope).await {
        Ok(raw_body) => {
            tracing::info!(
                account_id,
                envelope_id = %envelope_id,
                uid = envelope.uid,
                "Self-healed missing email content blob via on-demand IMAP fetch"
            );
            Ok((envelope, raw_body))
        }
        Err(e) => {
            tracing::warn!(
                account_id,
                envelope_id = %envelope_id,
                uid = envelope.uid,
                error = %e,
                "On-demand IMAP fetch for missing content blob failed; returning not-found"
            );
            Err(e)
        }
    }
}

/// Fetches one message from IMAP and re-stores its detached blob.
///
/// On success the freshly fetched raw RFC822 body is returned; it is also
/// queued (in detached form) into the blob store so subsequent requests hit the
/// cache. Fails if the message cannot be fetched, or if the fetched bytes do
/// not match the archived `content_hash` (the server-side message no longer
/// matches what Bichon archived, so it cannot be treated as a recovery of that
/// blob).
async fn recover_message_blob(envelope: &Envelope) -> BichonResult<Bytes> {
    let mailbox =
        MailBox::find_mailbox(envelope.account_id, envelope.mailbox_id)?.ok_or_else(|| {
            raise_error!(
                format!(
                    "Mailbox not found: account_id={} mailbox_id={}",
                    envelope.account_id, envelope.mailbox_id
                ),
                ErrorCode::ResourceNotFound
            )
        })?;

    let mut session = ImapExecutor::create_connection(envelope.account_id).await?;
<<<<<<< HEAD
    let result = ImapExecutor::fetch_single_message_body(
        &mut session,
        &mailbox.encoded_name(),
        envelope.uid,
    )
    .await;
    session.logout().await.ok();
    let raw_body = result?;

    let fetched_hash = compute_content_hash(&raw_body);
    if fetched_hash != envelope.content_hash {
        return Err(raise_error!(
            format!(
                "Fetched message does not match archived content: expected content_hash={} got={}",
                envelope.content_hash, fetched_hash
            ),
            ErrorCode::ImapUnexpectedResult
        ));
    }
=======
    let result = fetch_archived_body(&mut session, &mailbox.encoded_name(), envelope).await;
    session.logout().await.ok();
    let raw_body = result?;
    let fetched_hash = compute_content_hash(&raw_body);
>>>>>>> upstream/main

    // Re-create the detached blob (stripped EML + attachments) so the missing
    // blob is repopulated for future requests. The detached EML is queued under
    // `fetched_hash`, which equals `envelope.content_hash`.
    let message = MessageParser::new()
        .parse(raw_body.as_slice())
        .ok_or_else(|| {
            raise_error!(
                "Failed to parse fetched email content".into(),
                ErrorCode::InternalError
            )
        })?;
    // Persist the recovered blob for future requests — unless a backup window
    // is open, in which case we still return the content to the caller but skip
    // the write so the blob store stays byte-stable for the running snapshot.
<<<<<<< HEAD
    if !WRITE_GATE.is_paused() {
        detach_and_store_attachments(
            &raw_body,
            &message,
            &fetched_hash,
            envelope.account_id,
            envelope.mailbox_id,
        )
        .await;
    }
=======
    // Reads remain available during a backup. If the gate is open, hold the
    // admission guard through queueing so the backup cannot race the write.
    persist_recovered_blob_if_allowed(
        &raw_body,
        &message,
        &fetched_hash,
        envelope.account_id,
        envelope.mailbox_id,
        &WRITE_GATE,
    )
    .await;
>>>>>>> upstream/main

    Ok(Bytes::from(raw_body))
}

<<<<<<< HEAD
#[cfg(test)]
mod test {
    use html2text::config;

=======
async fn persist_recovered_blob_if_allowed(
    raw_body: &[u8],
    message: &Message<'_>,
    fetched_hash: &str,
    account_id: u64,
    mailbox_id: u64,
    gate: &WriteGate,
) -> bool {
    let Ok(_write_guard) = gate.check() else {
        return false;
    };
    detach_and_store_attachments(raw_body, message, fetched_hash, account_id, mailbox_id).await;
    true
}


/// Fetches the archived message from IMAP and returns it only if its content
/// hash equals the archived `content_hash`. Tries the stored UID first, then
/// the UIDs returned by `UID SEARCH HEADER Message-ID` (UID changed or the
/// message moved within the folder).
async fn fetch_archived_body(
    session: &mut async_imap::Session<Box<dyn crate::imap::session::SessionStream>>,
    encoded_mailbox: &str,
    envelope: &Envelope,
) -> BichonResult<Vec<u8>> {
    let mut last_err =
        match ImapExecutor::fetch_single_message_body(session, encoded_mailbox, envelope.uid).await {
            Ok(body) if compute_content_hash(&body) == envelope.content_hash => return Ok(body),
            Ok(body) => raise_error!(
                format!(
                    "Fetched message does not match archived content: expected content_hash={} got={}",
                    envelope.content_hash,
                    compute_content_hash(&body)
                ),
                ErrorCode::ImapUnexpectedResult
            ),
            Err(e) => e,
        };

    let message_id = envelope.message_id.trim();
    if message_id.is_empty() || message_id.contains(['"', '\\', '\r', '\n']) {
        return Err(last_err);
    }
    let uids = session
        .uid_search(format!("HEADER Message-ID \"{}\"", message_id))
        .await
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapUnexpectedResult))?;
    let uids = bounded_message_id_candidates(uids, envelope.uid);
    for uid in uids {
        match ImapExecutor::fetch_single_message_body(session, encoded_mailbox, uid).await {
            Ok(body) if recovered_body_matches(&body, envelope) => return Ok(body),
            Ok(_) => {}
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

const MAX_MESSAGE_ID_CANDIDATES: usize = 32;

fn bounded_message_id_candidates<I>(uids: I, stored_uid: u32) -> Vec<u32>
where
    I: IntoIterator<Item = u32>,
{
    let mut candidates: Vec<u32> = uids.into_iter().filter(|&uid| uid != stored_uid).collect();
    candidates.sort_unstable();
    candidates.truncate(MAX_MESSAGE_ID_CANDIDATES);
    candidates
}

fn recovered_body_matches(body: &[u8], envelope: &Envelope) -> bool {
    if compute_content_hash(body) != envelope.content_hash {
        return false;
    }
    if envelope.message_id.is_empty() {
        return true;
    }
    MessageParser::new()
        .parse(body)
        .and_then(|message| message.message_id().map(String::from))
        .is_some_and(|message_id| message_id == envelope.message_id)
}

/// Outcome of [`repair_envelope_blobs`].
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct BlobRepairReport {
    pub envelope_id: String,
    /// `complete` (nothing missing), `repaired`, `partial` or `failed`.
    pub status: String,
    /// Whether the email blob was missing before repair.
    pub email_missing_before: bool,
    /// Whether the email blob is still missing after repair.
    pub email_missing_after: bool,
    /// Backward-compatible alias for the pre-repair state.
    pub email_blob_missing: bool,
    pub missing_before: Vec<String>,
    pub missing_after: Vec<String>,
    pub error: Option<String>,
}

fn complete_repair_report(
    report: &mut BlobRepairReport,
    email_missing_after: bool,
    missing_after: Vec<String>,
) {
    report.email_missing_after = email_missing_after;
    report.missing_after = missing_after;
    report.status = if !email_missing_after && report.missing_after.is_empty() {
        "repaired"
    } else {
        "partial"
    }
    .into();
}

/// Content hashes referenced by `<<BICHON_DETACH_HASH:...>>` placeholders in a
/// detached EML.
fn placeholder_hashes(detached_eml: &[u8]) -> Vec<String> {
    const PREFIX: &[u8] = b"<<BICHON_DETACH_HASH:";
    let mut hashes = Vec::new();
    let mut rest = detached_eml;
    while let Some(pos) = rest.windows(PREFIX.len()).position(|w| w == PREFIX) {
        rest = &rest[pos + PREFIX.len()..];
        let Some(end) = rest.windows(2).position(|w| w == b">>") else {
            break;
        };
        if let Ok(h) = std::str::from_utf8(&rest[..end]) {
            if !hashes.iter().any(|x| x == h) {
                hashes.push(h.to_string());
            }
        }
        rest = &rest[end + 2..];
    }
    hashes
}

fn indexed_placeholder_hashes(
    detached_eml: &[u8],
    attachments: Option<&[AttachmentInfo]>,
) -> Vec<String> {
    let indexed = attachments
        .into_iter()
        .flat_map(|items| items.iter().map(|item| item.content_hash.as_str()))
        .collect::<std::collections::HashSet<_>>();
    placeholder_hashes(detached_eml)
        .into_iter()
        .filter(|hash| indexed.contains(hash.as_str()))
        .collect()
}

/// (email blob missing, attachment hashes missing from the blob store)
fn missing_blobs(
    envelope: &Envelope,
    attachments: Option<&[AttachmentInfo]>,
) -> BichonResult<(bool, Vec<String>)> {
    let Some(eml) = BLOB_MANAGER.get_email(&envelope.content_hash)? else {
        return Ok((true, Vec::new()));
    };
    let mut missing = Vec::new();
    for hash in indexed_placeholder_hashes(&eml, attachments) {
        if BLOB_MANAGER.get_attachment(&hash)?.is_none() {
            missing.push(hash);
        }
    }
    Ok((false, missing))
}

/// Re-fetches one archived message from IMAP and restores its missing email
/// or attachment blobs **in place**: the envelope, its id and its index entry
/// are kept, so no duplicate is created. Nothing is written unless the
/// fetched message hashes to the archived `content_hash`.
pub async fn repair_envelope_blobs(
    account_id: u64,
    envelope_id: &str,
) -> BichonResult<BlobRepairReport> {
    let envelope_with_attachments = ENVELOPE_MANAGER
        .get_envelope_by_id(account_id, envelope_id)?
        .ok_or_else(|| {
            raise_error!(
                format!("Envelope not found: account_id={account_id} envelope_id={envelope_id}"),
                ErrorCode::ResourceNotFound
            )
        })?;
    let attachments = envelope_with_attachments.attachments;
    let envelope = envelope_with_attachments.envelope;

    let (email_missing, missing_before) = missing_blobs(&envelope, attachments.as_deref())?;
    let mut report = BlobRepairReport {
        envelope_id: envelope_id.to_string(),
        email_missing_before: email_missing,
        email_missing_after: email_missing,
        email_blob_missing: email_missing,
        missing_before,
        ..Default::default()
    };
    if !email_missing && report.missing_before.is_empty() {
        report.status = "complete".into();
        return Ok(report);
    }
    if let Err(e) = recover_message_blob(&envelope).await {
        report.status = "failed".into();
        report.error = Some(e.to_string());
        return Ok(report);
    }
    // recover_message_blob only queues the write; wait for it to land.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(30), BLOB_MANAGER.drain()).await;
    let (email_missing_after, missing_after) = missing_blobs(&envelope, attachments.as_deref())?;
    complete_repair_report(&mut report, email_missing_after, missing_after);
    Ok(report)
}

#[cfg(test)]
mod test {
    use html2text::config;
>>>>>>> upstream/main
    use std::sync::Once;

    /// Extract tests touch the process-global settings/DB/DEDUP_CACHE, so they
    /// need the same one-time environment as the backup config tests, and they
    /// serialize with those via the shared lock (convention: see
    /// `backup::BACKUP_STATE_TESTS`).
    static TEST_ENV: Once = Once::new();

    fn init_test_env() {
        TEST_ENV.call_once(|| {
            let root = std::env::temp_dir().join(format!(
                "bichon-extractor-test-{}",
                std::process::id()
            ));
            std::fs::create_dir_all(&root).unwrap();
            std::env::set_var("BICHON_ROOT_DIR", &root);
            std::env::set_var("BICHON_ENCRYPT_PASSWORD", "test-password");
            let _ = &*crate::settings::cli::SETTINGS;
            let _ = &*crate::database::manager::DB_MANAGER;
        });
    }

<<<<<<< HEAD
=======
    // Keep the blob writer alive across these storage tests.
    static STORAGE_RUNTIME: std::sync::LazyLock<tokio::runtime::Runtime> =
        std::sync::LazyLock::new(|| tokio::runtime::Runtime::new().unwrap());

    #[test]
    fn empty_attachment_uses_the_raw_storage_key_space() {
        STORAGE_RUNTIME.block_on(async {
            let _run_guard = crate::backup::BACKUP_STATE_TESTS.lock().await;
            init_test_env();
            let raw = b"From: test@example.com\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=empty.bin\r\n\r\n";
            let message = mail_parser::MessageParser::new().parse(raw).unwrap();
            let infos = super::detach_and_store_attachments(
                raw, &message, &crate::utils::compute_content_hash(raw), 0, 0,
            ).await;
            assert_ne!(infos[0].storage_hash(), infos[0].content_hash);
        });
    }

    // NOTE: a full detach→blob-store→reattach roundtrip test existed here but
    // was removed: it relied on the process-global BLOB_MANAGER, which other
    // test modules re-initialize against their own temp data dirs, so it only
    // passed when run in isolation. The reattach logic is covered by the
    // `reattach_parts` tests below with injected blob loads.

    #[test]
    fn repeated_attachment_is_restored_once_per_placeholder() {
        let payload = b"BEGIN:VCALENDAR\r\nDESCRIPTION:Repeated invitation\r\nEND:VCALENDAR\r\n";
        let hash = crate::utils::compute_content_hash(payload);
        let placeholder = format!("<<BICHON_DETACH_HASH:{hash}>>");
        let stripped = format!("part-one\r\n{placeholder}\r\npart-two\r\n{placeholder}");
        let info = crate::message::content::AttachmentInfo {
            content_hash: hash,
            ..Default::default()
        };
        let restored =
            super::reattach_parts(stripped.into_bytes(), vec![info.clone(), info], |_| {
                Ok(Some(bytes::Bytes::from_static(payload)))
            })
            .expect("reattach repeated invitation");
        let expected = [
            b"part-one\r\n".as_slice(),
            payload,
            b"\r\npart-two\r\n",
            payload,
        ]
        .concat();
        assert_eq!(restored.as_ref(), expected);
    }

    #[test]
    fn distinct_transfer_encodings_keep_their_original_bytes() {
        let plain = b"BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let base64 = b"QkVHSU46VkNBTEVOREFSDQpFTkQ6VkNBTEVOREFSDQo=\r\n";
        let decoded_hash = crate::utils::compute_content_hash(plain);
        let plain_hash = crate::utils::compute_raw_attachment_hash(plain);
        let base64_hash = crate::utils::compute_raw_attachment_hash(base64);
        let infos = vec![
            crate::message::content::AttachmentInfo {
                content_hash: decoded_hash.clone(),
                raw_content_hash: Some(plain_hash.clone()),
                ..Default::default()
            },
            crate::message::content::AttachmentInfo {
                content_hash: decoded_hash,
                raw_content_hash: Some(base64_hash.clone()),
                ..Default::default()
            },
        ];
        let stripped = format!(
            "plain\r\n<<BICHON_DETACH_HASH:{plain_hash}>>base64\r\n<<BICHON_DETACH_HASH:{base64_hash}>>"
        );
        let restored = super::reattach_parts(stripped.into_bytes(), infos, |hash| {
            Ok(Some(if hash == plain_hash {
                bytes::Bytes::from_static(plain)
            } else {
                bytes::Bytes::from_static(base64)
            }))
        })
        .expect("reattach independently encoded copies");
        assert_eq!(
            restored.as_ref(),
            [b"plain\r\n".as_slice(), plain, b"base64\r\n", base64].concat()
        );
    }

    #[test]
    fn mixed_format_metadata_falls_back_to_legacy_placeholders() {
        // A stripped EML written by the pre-#375 format (decoded-hash
        // placeholders) combined with metadata refreshed by a re-ingest
        // (raw storage keys) must still reconstruct: the legacy blob under
        // the decoded key is still present in the blob store.
        let decoded_hash = "d".repeat(64);
        let raw_hash = "r".repeat(64);
        let info = crate::message::content::AttachmentInfo {
            content_hash: decoded_hash.clone(),
            raw_content_hash: Some(raw_hash),
            ..Default::default()
        };
        let stripped = format!("head\r\n<<BICHON_DETACH_HASH:{decoded_hash}>>\r\ntail");
        let restored =
            super::reattach_parts(stripped.into_bytes(), vec![info], |hash| {
                Ok(Some(if hash == decoded_hash {
                    bytes::Bytes::from_static(b"legacy body")
                } else {
                    bytes::Bytes::from_static(b"raw body")
                }))
            })
            .expect("reattach mixed-format record");
        assert_eq!(restored.as_ref(), b"head\r\nlegacy body\r\ntail".as_slice());
    }

    #[test]
    fn mixed_format_fallback_also_loads_the_raw_blob() {
        // Same mixed state, but the legacy blob under the decoded key was
        // garbage-collected after the original envelope was deleted. The
        // re-ingest stored the identical bytes under the raw key, which must
        // be used as the alternate load target.
        let decoded_hash = "d".repeat(64);
        let raw_hash = "r".repeat(64);
        let info = crate::message::content::AttachmentInfo {
            content_hash: decoded_hash.clone(),
            raw_content_hash: Some(raw_hash.clone()),
            ..Default::default()
        };
        let stripped = format!("<<BICHON_DETACH_HASH:{decoded_hash}>>");
        let restored =
            super::reattach_parts(stripped.into_bytes(), vec![info], |hash| {
                if hash == raw_hash {
                    Ok(Some(bytes::Bytes::from_static(b"raw body")))
                } else {
                    Ok(None)
                }
            })
            .expect("reattach via raw-key alternate");
        assert_eq!(restored.as_ref(), b"raw body".as_slice());
    }

    #[test]
    fn raw_blob_key_cannot_alias_legacy_attachment_or_email_key() {
        let payload = b"same bytes in a legacy decoded attachment or original email";
        assert_ne!(crate::utils::compute_raw_attachment_hash(payload), crate::utils::compute_content_hash(payload));
    }

    #[test]
    fn legacy_metadata_uses_the_original_decoded_storage_key() {
        let info: crate::message::content::AttachmentInfo = serde_json::from_str(
            r#"{"file_type":"application/pdf","inline":false,"filename":"a.pdf","size":4,"content_id":null,"content_hash":"legacy-hash","is_message":false,"extracted_text":null,"extracted_page_count":null}"#
        ).expect("read legacy attachment metadata");
        assert_eq!(info.storage_hash(), "legacy-hash");
    }

>>>>>>> upstream/main
    /// A body that cannot be parsed must surface as `EmailParseError` (not a
    /// generic InternalError): the batch streaming path matches on that code
    /// to skip the single message and keep the batch going (#377). Pins the
    /// classification contract end to end — if a parser upgrade starts
    /// parsing this junk, the input here needs replacing, not the assertion.
    #[tokio::test]
    async fn unparseable_body_raises_email_parse_error() {
        let _run_guard = crate::backup::BACKUP_STATE_TESTS.lock().await;
        init_test_env();
        // mail-parser 0.11 is extremely lenient (even binary junk parses as a
        // header-less message); the empty body is the input it still refuses,
        // so it pins the classification. If a parser upgrade starts parsing
        // this, replace the input — the assertion is the contract.
        let junk: &[u8] = b"";
        assert!(
            mail_parser::MessageParser::new().parse(junk).is_none(),
            "test input must actually be unparseable for the classification to apply"
        );
        let err = super::extract_envelope_from_eml(junk, 1, 1)
            .await
            .expect_err("unparseable body must be an error");
        assert_eq!(
            err.code(),
            crate::error::code::ErrorCode::EmailParseError,
            "parse failures must classify as EmailParseError, got {err:?}"
        );
    }

    #[test]
<<<<<<< HEAD
=======
    fn placeholder_hashes_finds_unique_hashes() {
        let eml = b"a<<BICHON_DETACH_HASH:abc>>b<<BICHON_DETACH_HASH:def>>c<<BICHON_DETACH_HASH:abc>>";
        assert_eq!(super::placeholder_hashes(eml), vec!["abc", "def"]);
        assert!(super::placeholder_hashes(b"no placeholders").is_empty());
        assert!(super::placeholder_hashes(b"<<BICHON_DETACH_HASH:unterminated").is_empty());
    }

    #[test]
    fn indexed_placeholder_hashes_ignores_literal_body_markers() {
        let attachments = vec![super::AttachmentInfo {
            content_hash: "real".into(),
            ..Default::default()
        }];
        assert_eq!(
            super::indexed_placeholder_hashes(
                b"<<BICHON_DETACH_HASH:literal>><<BICHON_DETACH_HASH:real>>",
                Some(&attachments),
            ),
            vec!["real"]
        );
    }

    #[test]
    fn repair_report_updates_post_state_without_changing_legacy_alias() {
        let mut report = super::BlobRepairReport {
            email_missing_before: true,
            email_blob_missing: true,
            ..Default::default()
        };
        super::complete_repair_report(&mut report, false, Vec::new());
        assert!(report.email_blob_missing);
        assert!(report.email_missing_before);
        assert!(!report.email_missing_after);
        assert_eq!(report.status, "repaired");
    }

    #[tokio::test]
    async fn paused_recovery_skips_persistence_and_releases_admission() {
        let gate = super::WriteGate::new();
        gate.pause();
        let raw = b"From: sender@example.test\r\n\r\nbody";
        let message = super::MessageParser::new().parse(raw).expect("fixture parses");

        assert!(!super::persist_recovered_blob_if_allowed(
            raw,
            &message,
            "paused-recovery-test",
            0,
            0,
            &gate,
        )
        .await);
        assert_eq!(gate.in_flight(), 0);

        gate.resume();
        assert!(super::persist_recovered_blob_if_allowed(
            raw,
            &message,
            "admitted-recovery-test",
            0,
            0,
            &gate,
        )
        .await);
        assert_eq!(gate.in_flight(), 0);
        super::BLOB_MANAGER.drain().await;
    }

    #[test]
    fn recovery_guards_reject_wrong_hash_and_message_id() {
        let body = b"From: sender@example.test\r\nMessage-ID: <original@example.test>\r\n\r\nbody";
        let parsed_id = super::MessageParser::new()
            .parse(body)
            .and_then(|message| message.message_id().map(String::from))
            .expect("message-id parses");
        let mut envelope = super::Envelope {
            message_id: parsed_id.clone(),
            content_hash: super::compute_content_hash(body),
            ..Default::default()
        };
        assert!(super::recovered_body_matches(body, &envelope));
        envelope.message_id = "<other@example.test>".into();
        assert!(!super::recovered_body_matches(body, &envelope));
        envelope.message_id = parsed_id;
        envelope.content_hash = super::compute_content_hash(b"different");
        assert!(!super::recovered_body_matches(body, &envelope));
    }

    #[test]
    fn message_id_candidates_are_bounded_and_exclude_stored_uid() {
        let candidates = super::bounded_message_id_candidates((1..=100).rev(), 50);
        assert_eq!(candidates.len(), 32);
        assert_eq!(candidates.first(), Some(&1));
        assert_eq!(candidates.last(), Some(&32));
        assert!(!candidates.contains(&50));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn detached_attachment_blob_is_idempotent_and_not_overwritten() {
        use bytes::Bytes;

        let raw = concat!(
            "Message-ID: <fixture@example.test>\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: multipart/mixed; boundary=fixture\r\n",
            "\r\n",
            "--fixture\r\n",
            "Content-Type: text/plain\r\n\r\n",
            "hello\r\n",
            "--fixture\r\n",
            "Content-Type: application/octet-stream\r\n",
            "Content-Disposition: attachment; filename=fixture.bin\r\n\r\n",
            "fixture bytes\r\n",
            "--fixture--\r\n",
        )
        .as_bytes();
        let message = super::MessageParser::new().parse(raw).expect("fixture parses");
        let email_hash = super::compute_content_hash(raw);
        let infos = super::detach_and_store_attachments(raw, &message, &email_hash, 0, 0).await;
        assert_eq!(infos.len(), 1);
        super::BLOB_MANAGER.drain().await;
        let original_email = super::BLOB_MANAGER
            .get_email(&email_hash).expect("email lookup").expect("email blob");
        let attachment_hash = infos[0].content_hash.clone();
        let original_attachment = super::BLOB_MANAGER
            .get_attachment(&attachment_hash).expect("attachment lookup").expect("attachment blob");

        super::BLOB_MANAGER.queue(super::DetachedEmail {
            email: (email_hash.clone(), Bytes::from_static(b"tampered email")),
            attachments: Some(vec![(attachment_hash.clone(), Bytes::from_static(b"tampered attachment"))]),
        }).await;
        super::BLOB_MANAGER.drain().await;
        assert_eq!(super::BLOB_MANAGER.get_email(&email_hash).unwrap(), Some(original_email.clone()));
        assert_eq!(super::BLOB_MANAGER.get_attachment(&attachment_hash).unwrap(), Some(original_attachment));

        let message = super::MessageParser::new().parse(raw).expect("fixture reparses");
        super::detach_and_store_attachments(raw, &message, &email_hash, 0, 0).await;
        super::BLOB_MANAGER.drain().await;
        assert_eq!(super::BLOB_MANAGER.get_email(&email_hash).unwrap(), Some(original_email));
    }


    #[test]
>>>>>>> upstream/main
    fn test_various_html_with_overflow_enabled() {
        let cases = [
            ("<p>Hello World</p>", "Simple paragraph"),
            ("<h1>Title</h1><p>Content</p>", "Heading + paragraph"),
            ("<ul><li>Item1</li><li>Item2</li></ul>", "Unordered list"),
            (
                "<strong>Bold</strong> and <em>italic</em>",
                "Inline formatting",
            ),
            (
                "<div><span>Nested</span> elements</div>",
                "Nested inline elements inside block",
            ),
            (
                "<table><tr><td>A</td><td>B</td></tr></table>",
                "Simple table",
            ),
            (
                "<pre>  preformatted text\n  line2</pre>",
                "Preformatted block",
            ),
            ("😃 emoji test", "Wide emoji"),
            ("<a href=\"#\">link</a>", "Anchor tag"),
            (
                "<blockquote><p>Quoted text</p></blockquote>",
                "Blockquote with paragraph",
            ),
        ];

        for (html, desc) in cases {
            let result = config::plain()
                .allow_width_overflow()
                .string_from_read(html.as_bytes(), 100);

            match result {
                Ok(output) => {
                    println!("✓ Rendered ({}) =>\n{}", desc, output);
                }
                Err(e) => panic!("Unexpected error for {}: {:?}", desc, e),
            }
        }
    }

    /// Verifies that [`super::detach_and_store_attachments`] does not panic
    /// when mail-parser reports attachment offsets past the raw body length.
    ///
    /// Regression test for: "range end index X out of range for slice of
    /// length Y" panic caused by a malformed email whose attachment
    /// `raw_end_offset` exceeded the actual body size.
<<<<<<< HEAD
    #[tokio::test]
    async fn detach_attachments_bounds_check() {
=======
    #[test]
    fn detach_attachments_bounds_check() {
        STORAGE_RUNTIME.block_on(check_attachment_bounds());
    }

    async fn check_attachment_bounds() {
        let _run_guard = crate::backup::BACKUP_STATE_TESTS.lock().await;
        init_test_env();
>>>>>>> upstream/main
        let raw = concat!(
            "From: sender@example.com\r\n",
            "To: recipient@example.com\r\n",
            "Subject: Test\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: multipart/mixed; boundary=\"bnd\"\r\n",
            "\r\n",
            "--bnd\r\n",
            "Content-Type: text/plain\r\n",
            "\r\n",
            "Hello\r\n",
            "--bnd\r\n",
            "Content-Type: application/octet-stream\r\n",
            "Content-Disposition: attachment; filename=\"test.bin\"\r\n",
            "\r\n",
            "AAAAABBBBBCCCCCDDDDDEEEEEAAAAABBBBBCCCCCDDDDDEEEEE\r\n",
            "--bnd--\r\n",
        )
        .as_bytes()
        .to_vec();

        let message = mail_parser::MessageParser::new()
            .parse(&raw)
            .expect("parse valid MIME message");
        assert_eq!(message.attachment_count(), 1);

        // Truncate the raw body so the attachment's raw_end_offset lies
        // past the body end — exactly the scenario reported by users.
        let truncated = &raw[..raw.len() - 20];
        assert!(truncated.len() < raw.len());

        // Must not panic.
        let infos =
            super::detach_and_store_attachments(truncated, &message, "test_content_hash", 0, 0)
                .await;

        // The attachment count must still match so the consistency check
        // in reattach_eml_content doesn't fail later.
        assert_eq!(infos.len(), 1);
    }
}
