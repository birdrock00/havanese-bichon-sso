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

// Attachment text extraction extension point.
//
// Community edition: NoopExtractor — no attachments are text-indexed.
// Pro edition: PdfExtractor — extracts text from PDF, Word, etc.
//
// Used in: crates/core/src/envelope/extractor.rs

use std::sync::{LazyLock, RwLock};

pub struct ExtractedText {
    pub text: String,
    pub page_count: Option<u32>,
    pub is_ocr: bool,
}

pub trait AttachmentTextExtractor: Send + Sync {
    /// Returns None if this extractor doesn't handle the file type.
    /// Returns Some(ExtractedText) if text was successfully extracted.
    fn extract(&self, content_type: &str, ext: &str, bytes: &[u8]) -> Option<ExtractedText>;
}

/// Default — all attachments are skipped.
struct NoopExtractor;
impl AttachmentTextExtractor for NoopExtractor {
    fn extract(&self, _ct: &str, _ext: &str, _bytes: &[u8]) -> Option<ExtractedText> {
        None
    }
}

static EXTRACTOR: LazyLock<RwLock<Box<dyn AttachmentTextExtractor>>> =
    LazyLock::new(|| RwLock::new(Box::new(NoopExtractor)));

/// Called by Pro/Enterprise at startup to replace the noop default.
pub fn set_extractor(extractor: Box<dyn AttachmentTextExtractor>) {
    *EXTRACTOR.write().unwrap() = extractor;
}

/// Called by the attachment pipeline during IMAP sync.
/// The caller should wrap this in spawn_blocking for CPU-bound extraction.
pub fn extract_text(content_type: &str, ext: &str, bytes: &[u8]) -> Option<ExtractedText> {
    EXTRACTOR.read().unwrap().extract(content_type, ext, bytes)
}
