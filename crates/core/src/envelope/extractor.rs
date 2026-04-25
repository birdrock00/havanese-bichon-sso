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

use crate::common::AddrVec;
use crate::envelope::meta::parse_bichon_metadata;
use crate::envelope::utils::normalize_subject;
use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::message::content::AttachmentInfo;
use crate::store::storage::{DetachedEmail, BLOB_MANAGER};
use crate::store::tantivy::attachment::ATTACHMENT_MANAGER;
use crate::store::tantivy::envelope::ENVELOPE_MANAGER;
use crate::store::tantivy::model::{AttachmentModel, EnvelopeWithAttachments};
use crate::utils::html::extract_text;
use crate::utils::{compute_content_hash, hex_hash};
use crate::{id, store::envelope::Envelope};
use crate::{raise_error, utc_now};
use async_imap::types::Fetch;
use bytes::Bytes;
use mail_parser::{Address, HeaderName, Message, MessageParser, MimeHeaders};
use tantivy::TantivyDocument;
use tantivy::schema::Facet;
use tracing::error;
use uuid::Uuid;

pub async fn extract_envelope_and_store_it(
    fetch: Fetch,
    account_id: u64,
    mailbox_id: u64,
) -> BichonResult<()> {
    let internal_date = fetch
        .internal_date()
        .map(|d| d.timestamp_millis())
        .unwrap_or(0);
    let uid = fetch.uid.unwrap_or(0);
    let body = fetch
        .body()
        .ok_or_else(|| raise_error!("No body available".into(), ErrorCode::InternalError))?;
    let size = fetch.size.unwrap_or(body.len() as u32);
    extract_envelope_core(body, uid, size, internal_date, account_id, mailbox_id).await
}

pub async fn extract_envelope_from_eml(
    body: &[u8],
    account_id: u64,
    mailbox_id: u64,
) -> BichonResult<()> {
    extract_envelope_core(body, 0, body.len() as u32, 0, account_id, mailbox_id).await
}

pub async fn extract_envelope_from_smtp(
    body: &[u8],
    account_id: u64,
    mailbox_id: u64,
) -> BichonResult<()> {
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
) -> BichonResult<()> {
    let email_content_hash = compute_content_hash(body);
    let message: Message<'_> = MessageParser::new().parse(body).ok_or_else(|| {
        raise_error!(
            "Email header parse result is not available".into(),
            ErrorCode::InternalError
        )
    })?;

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
    let attachments = detach_and_store_attachments(body, &message, &email_content_hash).await;

    let envelope_id = Uuid::new_v4().to_string();
    let now = utc_now!();


    let mut final_tags = Vec::new();

    if let Some(meta_header) = message.header_raw("X-Bichon-Metadata") {
        if let Some(bmd) = parse_bichon_metadata(meta_header) {
            if let Some(tags) = bmd.tags {
                let validated_tags: Result<Vec<String>, _> = tags
                    .iter()
                    .map(|tag| {
                        Facet::from_text(tag)
                            .map(|_| tag.clone()) 
                            .map_err(|e| e)
                    })
                    .collect();

                match validated_tags {
                    Ok(valid_list) => {
                        final_tags = valid_list;
                    }
                    Err(e) => {
                        eprintln!(
                            "Tag validation failed, ignoring all tags: {:#?}",
                            e
                        );
                    }
                }
            }
        }
    }

    let attachment_docs: Vec<TantivyDocument> = attachments
        .iter()
        .filter(|a| !a.inline || a.content_id.is_none())
        .map(|a| AttachmentModel {
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
            text: None,
            has_text: false,
            is_ocr: false,
            page_count: None,
            is_indexed: false,
            is_message: a.is_message,
            name: a.filename.clone(),
            tags: None,
            auto_tags: None,
        }).map(|a|a.into_document())
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
        content_hash: email_content_hash,
    };
    // 'attachments' contains both regular and inline attachments
    let ea = EnvelopeWithAttachments {
        envelope,
        attachments: Some(attachments),
    };
    let doc = ea.to_document(&body_text, 0)?;
    ENVELOPE_MANAGER.queue(doc).await;
    for doc in attachment_docs {
        ATTACHMENT_MANAGER.queue(doc).await;
    }
    Ok(())
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

fn extract_references(message: &Message<'_>) -> Option<Vec<String>> {
    match message.references() {
        mail_parser::HeaderValue::Text(cow) => Some(vec![cow.to_string()]),
        mail_parser::HeaderValue::TextList(vec) => {
            Some(vec.iter().map(|cow| cow.to_string()).collect())
        }
        _ => None,
    }
}

pub async fn detach_and_store_attachments(
    original_body: &[u8],
    message: &Message<'_>,
    eml_content_hash: &str,
) -> Vec<AttachmentInfo> {
    let mut stripped_eml = original_body.to_vec();
    let mut attachment_infos = Vec::new();
    // Step 1: Collect and sort attachment ranges in reverse to maintain offset integrity
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

    ranges.sort_by(|a, b| b.0.cmp(&a.0));
    let mut attachments = Vec::with_capacity(ranges.len());
    for (raw_start, raw_end, att) in ranges {
        // Step 2: Extract raw bytes and store them as standalone documents
        let raw_bytes = &original_body[raw_start..raw_end];
        let content_hash = compute_content_hash(att.contents());

        attachments.push((content_hash.clone(), Bytes::copy_from_slice(raw_bytes)));//

        // Step 3: Replace raw attachment content with a hash-based placeholder
        let placeholder = format!("<<BICHON_DETACH_HASH:{}>>", &content_hash);
        let p_bytes = placeholder.as_bytes();
        stripped_eml.splice(raw_start..raw_end, p_bytes.iter().cloned());

        let info = AttachmentInfo {
            filename: att.attachment_name().map(|n| n.to_string()),
            size: att.contents().len(),
            inline: att
                .content_disposition()
                .map(|d| d.is_inline())
                .unwrap_or(false),
            file_type: att
                .content_type()
                .map(|ct| {
                    format!(
                        "{}/{}",
                        ct.c_type.as_ref(),
                        ct.c_subtype.as_deref().unwrap_or("")
                    )
                })
                .unwrap_or_else(|| "application/octet-stream".to_string()),
            content_id: att.content_id().map(|id| id.to_string()),
            content_hash: content_hash.clone(),
            is_message: att.is_message(),
        };

        attachment_infos.push(info);
    }
    // Step 4: Store the final stripped EML content
    BLOB_MANAGER
        .queue(DetachedEmail {
            email: (eml_content_hash.to_string(), Bytes::from(stripped_eml)),
            attachments: Some(attachments),
        })
        .await;

    attachment_infos
}

pub async fn reattach_eml_content(
    account_id: u64,
    envelope_id: String,
) -> BichonResult<(Envelope, Bytes)> {
    let e = ENVELOPE_MANAGER
        .get_envelope_by_id(account_id, &envelope_id)
        .await?
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

    let mut restored_eml = restored_eml.to_vec();
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

            tasks.push((
                absolute_start,
                absolute_end,
                detail.content_hash.clone(),
            ));
            search_cursor = absolute_end;
        }
    }

    tasks.sort_by(|a, b| b.0.cmp(&a.0));

    for (start, end, hash) in tasks {
        if let Some(original_data) = BLOB_MANAGER.get_attachment(&hash)? {
            restored_eml.splice(start..end, original_data.iter().cloned());
        } else {
            error!("[ERROR] Missing attachment blob for hash: {}", hash);
        }
    }

    Ok((e.envelope, Bytes::from(restored_eml)))
}

#[cfg(test)]
mod test {
    use html2text::config;

    #[test]
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
}
