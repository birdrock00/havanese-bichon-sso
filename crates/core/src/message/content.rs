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

use crate::base64_encode;
use crate::account::migration::AccountModel;
use crate::envelope::extractor::{
    extract_envelope_from_nested_message, reattach_eml_content,
};
use crate::error::code::ErrorCode;
use crate::store::envelope::Envelope;
use crate::utils::compute_content_hash;
use crate::{error::BichonResult, raise_error};
use mail_parser::{MessageParser, MimeHeaders};
//use poem_openapi::Object;
use serde::{Deserialize, Serialize};
/// Represents metadata of an attachment in a Gmail message.
///
/// This struct stores information required to identify, download,
/// and render an attachment, including inline images embedded
/// in HTML emails.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AttachmentInfo {
    /// MIME content type of the attachment (e.g., `image/png`, `application/pdf`).
    pub file_type: String,
    /// Whether the attachment is marked as inline (true) or a regular file (false).
    pub inline: bool,
    /// Original filename of the attachment, if provided.
    pub filename: Option<String>,
    /// Size of the attachment in bytes.
    pub size: usize,
    pub content_id: Option<String>,
    /// Hash of the content.
    pub content_hash: String,
    pub is_message: bool,
}

impl AttachmentInfo {
    pub fn is_inline(&self) -> bool {
        self.inline && self.content_id.is_some()
    }

    pub fn get_extension(&self) -> Option<String> {
        self.filename
            .as_deref()
            .and_then(|f| std::path::Path::new(f).extension())
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
    }

    pub fn get_category(&self) -> &'static str {
        if let Some(ext) = self.get_extension() {
            let category = match ext.as_str() {
                "doc" | "docx" | "pdf" | "rtf" | "odt" | "pages" | "pptx" | "ppt" => {
                    Some("document")
                }
                "xls" | "xlsx" | "ods" | "numbers" | "csv" => Some("spreadsheet"),
                "ical" | "ics" | "vcs" | "ifb" | "icalendar" => Some("event"),
                "txt" | "log" | "md" => Some("text"),
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "avif" | "heic" | "heif"
                | "webp" => Some("image"),
                "mp4" | "mkv" | "mov" | "avi" | "webm" => Some("video"),
                "wav" | "mp3" | "aac" | "ogg" | "wma" | "flac" | "aiff" => Some("audio"),
                "psd" | "eps" | "svg" | "cdr" | "ai" => Some("graphics_2d"),
                "stl" | "obj" | "3mf" | "amf" | "f3d" | "sldprt" | "stp" | "step" | "dwg"
                | "x_t" | "x_b" | "sat" | "ipt" => Some("graphics_3d"),
                "c" | "h" | "html" | "css" | "js" | "ts" | "vue" | "tsx" | "svelte" | "py"
                | "java" | "cs" | "go" | "rb" | "php" | "swift" | "rs" | "r" | "jl" | "lua"
                | "sql" => Some("code"),
                "tsv" | "xml" | "json" | "yml" | "yaml" | "toml" | "env" | "ini" => Some("data"),
                "ps1" | "sh" | "bat" | "cmd" | "exe" | "msi" | "dmg" | "pkg" | "deb" | "rpm" => {
                    Some("executable")
                }
                "zip" | "gz" | "tgz" | "7z" | "rar" | "tar" | "bz2" | "zst" | "xz" | "iso"
                | "img" => Some("archive"),
                "eml" | "msg" => Some("message"),
                _ => None,
            };

            if let Some(cat) = category {
                return cat;
            }
        }

        let mime = self.file_type.to_lowercase();
        if mime.starts_with("image/") {
            return "image";
        }
        if mime.starts_with("video/") {
            return "video";
        }
        if mime.starts_with("audio/") {
            return "audio";
        }
        if mime.starts_with("text/") {
            return "text";
        }
        if mime == "message/rfc822" {
            return "message";
        }
        if mime.contains("compressed") || mime.contains("zip") || mime.contains("archive") {
            return "archive";
        }
        if mime.contains("pdf") || mime.contains("msword") || mime.contains("officedocument") {
            return "document";
        }
        if mime.contains("spreadsheet") || mime.contains("excel") {
            return "spreadsheet";
        }

        "other"
    }
}
/// Represents the content of an email message in both plain text and HTML formats.
///
/// This struct contains optional fields for plain text and HTML versions of
/// the email message body. At least one of them may be present.
///
/// # Fields
///
/// - `plain`: The plain text version of the message, if available.
/// - `html`: The HTML version of the message, if available.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct FullMessageContent {
    /// Optional plain text version of the message.
    pub text: Option<String>,
    /// Optional HTML version of the message.
    pub html: Option<String>,
    // all Attachments include inline attachments
    pub attachments: Option<Vec<AttachmentInfo>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct FullNestedMessageContent {
    /// Optional plain text version of the message.
    pub text: Option<String>,
    /// Optional HTML version of the message.
    pub html: Option<String>,
    // all Attachments include inline attachments
    pub attachments: Option<Vec<AttachmentInfo>>,
    /// Metadata for the email envelope.
    pub envelope: Envelope,
}

pub async fn retrieve_email_content(
    account_id: u64,
    envelope_id: String,
) -> BichonResult<FullMessageContent> {
    AccountModel::check_account_exists(account_id).await?;
    let (envelope, eml) = reattach_eml_content(account_id, envelope_id).await?;
    let message = MessageParser::default().parse(&eml).ok_or_else(|| {
        raise_error!(
            "Failed to parse EML data — the message may be corrupted.".into(),
            ErrorCode::InternalError
        )
    })?;
    let mut html: Option<String> = message.body_html(0).map(|cow| cow.into_owned());
    let text: Option<String> = message.body_text(0).map(|cow| cow.into_owned());
    let mut attachments = Vec::new();
    for attachment in message.attachments() {
        let content_type = attachment.content_type().ok_or_else(|| {
            raise_error!(
                format!(
                    "Attachment is missing Content-Type (email id={})",
                    &envelope.id
                ),
                ErrorCode::InternalError
            )
        })?;
        let filename = attachment.attachment_name().map(|name| name.to_string());
        let disposition = attachment.content_disposition();
        let file_type = format!(
            "{}/{}",
            content_type.c_type.as_ref(),
            content_type.c_subtype.as_deref().unwrap_or("")
        );

        let inline = disposition.map(|d| d.is_inline()).unwrap_or(false);

        if inline {
            if let Some(html1) = html.as_deref() {
                if let Some(cid) = attachment.content_id() {
                    if html1.contains(cid) {
                        let data = attachment.contents();
                        let base64_encoded = base64_encode!(data);
                        let html_content = html1.replace(
                            &format!("cid:{}", cid),
                            &format!("data:{};base64,{}", file_type, base64_encoded),
                        );
                        html = Some(html_content);
                    }
                }
            }
        }
        //inline attachment will not be displayed in email attachment list
        if inline && attachment.content_id().is_some() {
            continue;
        }
        let is_message = attachment.is_message();
        let content_hash = compute_content_hash(attachment.contents());
        attachments.push(AttachmentInfo {
            filename: filename.or(Some(content_hash.clone())), //  Fallback to content_hash as the default filename if it is not provided.
            size: attachment.contents().len(),
            inline,
            file_type,
            is_message,
            content_hash,
            content_id: attachment.content_id().map(Into::into),
        });
    }
    Ok(FullMessageContent {
        text,
        html,
        attachments: Some(attachments),
    })
}

pub async fn retrieve_nested_eml_content(
    account_id: u64,
    envelope_id: String,
    content_hash: &str,
) -> BichonResult<FullNestedMessageContent> {
    let (_, eml) = reattach_eml_content(account_id, envelope_id).await?;
    let parent_message = MessageParser::default().parse(&eml).ok_or_else(|| {
        raise_error!(
            "Failed to parse parent EML".into(),
            ErrorCode::InternalError
        )
    })?;

    let attachment_content = parent_message
        .attachments()
        .find(|att| compute_content_hash(att.contents()) == content_hash)
        .map(|att| att.contents())
        .ok_or_else(|| {
            raise_error!(
                "Target nested EML not found".into(),
                ErrorCode::ResourceNotFound
            )
        })?;

    let nested_message = MessageParser::default()
        .parse(attachment_content)
        .ok_or_else(|| {
            raise_error!(
                "Failed to parse nested EML".into(),
                ErrorCode::InternalError
            )
        })?;

    let mut html = nested_message.body_html(0).map(|c| c.into_owned());
    let text = nested_message.body_text(0).map(|c| c.into_owned());

    let mut attachments = Vec::new();

    let has_html = html.is_some();

    for attachment in nested_message.attachments() {
        let cid = attachment.content_id();
        let disposition = attachment.content_disposition();
        let is_inline = disposition.map(|d| d.is_inline()).unwrap_or(false);

        if has_html && is_inline && cid.is_some() {
            let content_id = cid.unwrap();
            let html_ref = html.as_mut().unwrap();

            let cid_pattern = format!("cid:{}", content_id);
            if html_ref.contains(&cid_pattern) {
                let data = attachment.contents();
                let ct = attachment
                    .content_type()
                    .map(|ct| format!("{}/{}", ct.c_type, ct.c_subtype.as_deref().unwrap_or("")))
                    .unwrap_or_else(|| "image/png".to_string());

                let base64_data = format!("data:{};base64,{}", ct, base64_encode!(data));
                *html_ref = html_ref.replace(&cid_pattern, &base64_data);
                continue;
            }
        }

        let file_type = attachment.content_type().map_or_else(
            || "application/octet-stream".to_string(),
            |ct| format!("{}/{}", ct.c_type, ct.c_subtype.as_deref().unwrap_or("")),
        );
        let content_hash = compute_content_hash(attachment.contents());
        attachments.push(AttachmentInfo {
            filename: attachment
                .attachment_name()
                .map(|n| n.to_string())
                .or(Some(content_hash.clone())), // Fallback to content_hash as the default filename if it is not provided.
            size: attachment.contents().len(),
            inline: is_inline,
            file_type,
            content_hash,
            is_message: attachment.is_message(),
            content_id: cid.map(Into::into),
        });
    }

    let envelope = extract_envelope_from_nested_message(nested_message, account_id)?;

    Ok(FullNestedMessageContent {
        text,
        html,
        attachments: Some(attachments),
        envelope,
    })
}
