use std::io::Cursor;

use crate::{
    {
        dashboard::Group,
        envelope::extractor::reattach_eml_content,
        error::{code::ErrorCode, BichonResult},
        utils::compute_content_hash,
    },
    raise_error,
};
use bytes::Bytes;
use mail_parser::MessageParser;
//use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AttachmentMetadata {
    /// Statistics of attachment file extensions (key + count).
    /// Each item represents a file extension and its occurrence count.
    /// Example: [{ key: "pdf", count: 10 }, { key: "png", count: 5 }]
    pub extensions: Vec<Group>,

    /// Statistics of attachment categories (key + count).
    /// Each item represents a high-level category and its occurrence count.
    /// Example: [{ key: "document", count: 8 }, { key: "image", count: 6 }]
    pub categories: Vec<Group>,

    /// Statistics of attachment MIME types (Content-Type) (key + count).
    /// Each item represents a MIME type and its occurrence count.
    /// Example: [{ key: "application/pdf", count: 10 }, { key: "image/jpeg", count: 5 }]
    pub content_types: Vec<Group>,
}

pub async fn retrieve_attachment_content(
    account_id: u64,
    envelope_id: String,
    content_hash: &str,
) -> BichonResult<Cursor<Bytes>> {
    let (_, eml) = reattach_eml_content(account_id, envelope_id).await?;
    let message = MessageParser::default().parse(&eml).ok_or_else(|| {
        raise_error!(
            "Failed to parse EML".into(),
            ErrorCode::InternalError
        )
    })?;

    let attachment_content: &[u8] = message
        .attachments()
        .find(|att| compute_content_hash(att.contents()) == content_hash)
        .map(|att| att.contents())
        .ok_or_else(|| {
            raise_error!(
                "Target attachment not found".into(),
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
