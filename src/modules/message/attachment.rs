use std::collections::HashSet;

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
