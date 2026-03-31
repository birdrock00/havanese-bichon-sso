use std::{collections::HashSet, io::Cursor};

use crate::{
    modules::{
        envelope::extractor::reattach_eml_content,
        error::{code::ErrorCode, BichonResult},
        utils::compute_content_hash,
    },
    raise_error,
};
use bytes::Bytes;
use mail_parser::MessageParser;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, Object)]
pub struct AttachmentMetadata {
    /// A collection of unique file extensions found in attachments.
    /// Example: ["pdf", "docx", "png"]
    pub extensions: HashSet<String>,

    /// A collection of high-level attachment categories.
    /// Example: ["document", "image", "archive"]
    pub categories: HashSet<String>,

    /// A collection of unique MIME types (Content-Type) for the attachments.
    /// Example: ["application/pdf", "image/jpeg"]
    pub content_types: HashSet<String>,
}

pub async fn retrieve_attachment_content(
    account_id: u64,
    envelope_id: String,
    content_hash: &str,
) -> BichonResult<Cursor<Bytes>> {
    let (_, eml) = reattach_eml_content(account_id, envelope_id).await?;
    let message = MessageParser::default().parse(&eml).ok_or_else(|| {
        raise_error!(
            "Failed to parse parent EML".into(),
            ErrorCode::InternalError
        )
    })?;

    let attachment_content: &[u8] = message
        .attachments()
        .find(|att| compute_content_hash(att.contents()) == content_hash)
        .map(|att| att.contents())
        .ok_or_else(|| {
            raise_error!(
                "Target nested EML not found".into(),
                ErrorCode::ResourceNotFound
            )
        })?;
    Ok(Cursor::new(Bytes::copy_from_slice(attachment_content)))
}

pub async fn retrieve_nested_attachment_content(
    account_id: u64,
    envelope_id: String,
    content_hash: &str,
    nested_content_hash: &str,
) -> BichonResult<Cursor<Bytes>> {
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

    let attachment_content = nested_message
        .attachments()
        .find(|att| compute_content_hash(att.contents()) == nested_content_hash)
        .map(|att| att.contents())
        .ok_or_else(|| {
            raise_error!(
                "Target nested EML not found".into(),
                ErrorCode::ResourceNotFound
            )
        })?;

    Ok(Cursor::new(Bytes::copy_from_slice(attachment_content)))
}
