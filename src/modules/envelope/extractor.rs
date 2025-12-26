//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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

use crate::modules::common::AddrVec;
use crate::modules::envelope::utils::normalize_subject;
use crate::modules::error::code::ErrorCode;
use crate::modules::error::BichonResult;
use crate::modules::utils::create_hash;
use crate::{calculate_hash, raise_error, utc_now};
use crate::{id, modules::indexer::envelope::Envelope};
use async_imap::types::Fetch;
use mail_parser::{HeaderName, Message, MessageParser, MimeHeaders};

pub fn extract_envelope(fetch: &Fetch, account_id: u64, mailbox_id: u64) -> BichonResult<Envelope> {
    let internal_date = fetch
        .internal_date()
        .map(|d| d.timestamp_millis())
        .unwrap_or(0);
    let uid = fetch.uid.unwrap_or(0);

    let body = fetch
        .body()
        .ok_or_else(|| raise_error!("No body available".into(), ErrorCode::InternalError))?;

    let size = fetch.size.unwrap_or(body.len() as u32);
    let message = MessageParser::new().parse(body).ok_or_else(|| {
        raise_error!(
            "Email header parse result is not available".into(),
            ErrorCode::InternalError
        )
    })?;

    let text = if let Some(text) = message.body_text(0).map(|cow| cow.into_owned()) {
        text
    } else if let Some(html) = message.body_html(0).map(|cow| cow.into_owned()) {
        html2text::config::plain()
            .allow_width_overflow()
            .string_from_read(html.as_bytes(), 100)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
    } else {
        String::new()
    };

    let message_id = message
        .message_id()
        .map(String::from)
        .unwrap_or(generate_message_id());
    let in_reply_to = message.in_reply_to().as_text().map(String::from);
    let references = extract_references(&message);
    let thread_id = compute_thread_id(in_reply_to, references, &message_id);

    let mut subject = message.subject().map(String::from).unwrap_or_default();

    if subject.contains('\u{FFFD}') {
        subject = normalize_subject(message.header_raw(HeaderName::Subject));
    }

    let date = message.date().map(|d| d.to_timestamp() * 1000).unwrap_or(0);
    let bcc: Option<Vec<String>> = message.bcc().map(|addr| {
        AddrVec::from(addr)
            .0
            .into_iter()
            .filter_map(|a| a.address)
            .collect()
    });
    let cc: Option<Vec<String>> = message.cc().map(|addr| {
        AddrVec::from(addr)
            .0
            .into_iter()
            .filter_map(|a| a.address)
            .collect()
    });
    let to: Option<Vec<String>> = message.to().map(|addr| {
        AddrVec::from(addr)
            .0
            .into_iter()
            .filter_map(|a| a.address)
            .collect()
    });
    let from = message
        .from()
        .and_then(|addr| AddrVec::from(addr).0.into_iter().next())
        .and_then(|add| add.address)
        .unwrap_or_else(|| "unknown".to_string());

    let attachments: Vec<String> = message
        .attachments()
        .filter_map(|att| att.attachment_name())
        .map(|name| name.to_string())
        .collect();
    let envelope = Envelope {
        id: create_hash(account_id, &message_id),
        message_id,
        account_id,
        mailbox_id,
        uid,
        subject,
        text,
        from,
        to: to.unwrap_or_default(),
        cc: cc.unwrap_or_default(),
        bcc: bcc.unwrap_or_default(),
        date,
        internal_date,
        size,
        thread_id,
        attachments,
        tags: None,
    };
    Ok(envelope)
}

pub fn extract_envelope_from_eml(
    body: &[u8],
    account_id: u64,
    mailbox_id: u64,
) -> BichonResult<Envelope> {
    let uid = 0;
    let size = body.len() as u32;
    let message = MessageParser::new().parse(body).ok_or_else(|| {
        raise_error!(
            "Email header parse result is not available".into(),
            ErrorCode::InternalError
        )
    })?;

    let text = if let Some(text) = message.body_text(0).map(|cow| cow.into_owned()) {
        text
    } else if let Some(html) = message.body_html(0).map(|cow| cow.into_owned()) {
        html2text::config::plain()
            .allow_width_overflow()
            .string_from_read(html.as_bytes(), 100)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
    } else {
        String::new()
    };

    let message_id = message
        .message_id()
        .map(String::from)
        .unwrap_or(generate_message_id());
    let in_reply_to = message.in_reply_to().as_text().map(String::from);
    let references = extract_references(&message);
    let thread_id = compute_thread_id(in_reply_to, references, &message_id);

    let mut subject = message.subject().map(String::from).unwrap_or_default();
    if subject.contains('\u{FFFD}') {
        subject = normalize_subject(message.header_raw(HeaderName::Subject));
    }

    let date = message.date().map(|d| d.to_timestamp() * 1000).unwrap_or(0);
    let bcc: Option<Vec<String>> = message.bcc().map(|addr| {
        AddrVec::from(addr)
            .0
            .into_iter()
            .filter_map(|a| a.address)
            .collect()
    });
    let cc: Option<Vec<String>> = message.cc().map(|addr| {
        AddrVec::from(addr)
            .0
            .into_iter()
            .filter_map(|a| a.address)
            .collect()
    });
    let to: Option<Vec<String>> = message.to().map(|addr| {
        AddrVec::from(addr)
            .0
            .into_iter()
            .filter_map(|a| a.address)
            .collect()
    });
    let from = message
        .from()
        .and_then(|addr| AddrVec::from(addr).0.into_iter().next())
        .and_then(|add| add.address)
        .unwrap_or_else(|| "unknown".to_string());

    let attachments: Vec<String> = message
        .attachments()
        .filter_map(|att| att.attachment_name())
        .map(|name| name.to_string())
        .collect();
    let envelope = Envelope {
        id: create_hash(account_id, &message_id),
        message_id,
        account_id,
        mailbox_id,
        uid,
        subject,
        text,
        from,
        to: to.unwrap_or_default(),
        cc: cc.unwrap_or_default(),
        bcc: bcc.unwrap_or_default(),
        date,
        internal_date: date,
        size,
        thread_id,
        attachments,
        tags: None,
    };
    Ok(envelope)
}

pub fn compute_thread_id(
    in_reply_to: Option<String>,
    references: Option<Vec<String>>,
    message_id: &str,
) -> u64 {
    if in_reply_to.is_some() && references.as_ref().map_or(false, |r| !r.is_empty()) {
        return calculate_hash!(&references.as_ref().unwrap()[0]);
    }
    calculate_hash!(message_id)
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
