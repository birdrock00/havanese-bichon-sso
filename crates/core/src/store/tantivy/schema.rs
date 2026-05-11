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

use std::sync::{Arc, LazyLock};
use tantivy::schema::{
    FacetOptions, Field, IndexRecordOption, TextFieldIndexing, TextOptions, INDEXED,
};
use tantivy::schema::{Schema, FAST, STORED, STRING};

use crate::store::tantivy::fields::{
    AttachmentFields, EmailFields, F_ACCOUNT_ID, F_ATTACHMENTS, F_ATTACHMENT_CATEGORY,
    F_ATTACHMENT_CONTENT_HASH, F_ATTACHMENT_CONTENT_TYPE, F_ATTACHMENT_COUNT, F_ATTACHMENT_EXT,
    F_ATTACHMENT_NAME_EXACT, F_ATTACHMENT_NAME_TEXT, F_AUTO_TAGS, F_BCC, F_BCC_TEXT, F_BODY, F_CC,
    F_CC_TEXT, F_CONTENT_HASH, F_DATE, F_ENVELOPE_ID, F_FROM, F_FROM_TEXT, F_HAS_TEXT, F_ID,
    F_INGEST_AT, F_INTERNAL_DATE, F_IS_INDEXED, F_IS_MESSAGE, F_IS_OCR, F_MAILBOX_ID, F_MESSAGE_ID,
    F_NAME_EXACT, F_NAME_TEXT, F_PAGE_COUNT, F_PREVIEW, F_REGULAR_ATTACHMENT_COUNT, F_SHARD_ID,
    F_SIZE, F_SUBJECT, F_TAGS, F_TEXT, F_THREAD_ID, F_TO, F_TO_TEXT, F_UID,
};

// ─── Lazy Globals ─────────────────────────────────────────────────────────────

static EMAIL_FIELDS: LazyLock<Arc<EmailFields>> = LazyLock::new(|| Arc::new(EmailSchema::fields()));

static ATTACHMENT_FIELDS: LazyLock<Arc<AttachmentFields>> =
    LazyLock::new(|| Arc::new(AttachmentSchema::fields()));

// ─── Public API ───────────────────────────────────────────────────────────────

pub struct SchemaTools;

impl SchemaTools {
    pub fn email_schema() -> Schema {
        EmailSchema::build().0
    }
    pub fn attachment_schema() -> Schema {
        AttachmentSchema::build().0
    }

    pub fn email_fields() -> &'static EmailFields {
        &EMAIL_FIELDS
    }
    pub fn attachment_fields() -> &'static AttachmentFields {
        &ATTACHMENT_FIELDS
    }

    pub fn email_default_fields() -> Vec<Field> {
        let f = Self::email_fields();
        vec![
            f.f_subject,
            f.f_body,
            f.f_attachment_name_text,
            f.f_from_text,
            f.f_to_text,
            f.f_cc_text,
            f.f_bcc_text,
        ]
    }

    pub fn attachment_default_fields() -> Vec<Field> {
        let f = Self::attachment_fields();
        vec![f.f_subject, f.f_text, f.f_name_text, f.f_from_text]
    }

    pub fn create_email_schema() -> (Schema, EmailFields) {
        EmailSchema::build()
    }
    pub fn create_attachment_schema() -> (Schema, AttachmentFields) {
        AttachmentSchema::build()
    }
}

// ─── Schema builders ──────────────────────────────────────────────────────────

struct EmailSchema;

impl EmailSchema {
    fn build() -> (Schema, EmailFields) {
        let mut b = Schema::builder();

        let f_id = b.add_text_field(F_ID, STRING | STORED | FAST);
        let f_message_id = b.add_text_field(F_MESSAGE_ID, STRING | STORED);
        let f_account_id = b.add_u64_field(F_ACCOUNT_ID, INDEXED | STORED | FAST);
        let f_mailbox_id = b.add_u64_field(F_MAILBOX_ID, INDEXED | STORED | FAST);
        let f_uid = b.add_u64_field(F_UID, INDEXED | STORED | FAST);
        let f_subject = b.add_text_field(F_SUBJECT, text_store("euro"));
        let f_body = b.add_text_field(F_BODY, text_no_store("euro"));
        let f_preview = b.add_text_field(F_PREVIEW, STORED);
        let f_content_hash = b.add_text_field(F_CONTENT_HASH, STRING | STORED | FAST);
        let f_from = b.add_text_field(F_FROM, STRING | STORED | FAST);
        let f_to = b.add_text_field(F_TO, STRING | STORED);
        let f_cc = b.add_text_field(F_CC, STRING | STORED);
        let f_bcc = b.add_text_field(F_BCC, STRING | STORED);
        let f_from_text = b.add_text_field(F_FROM_TEXT, text_no_store("euro"));
        let f_to_text = b.add_text_field(F_TO_TEXT, text_no_store("euro"));
        let f_cc_text = b.add_text_field(F_CC_TEXT, text_no_store("euro"));
        let f_bcc_text = b.add_text_field(F_BCC_TEXT, text_no_store("euro"));
        let f_date = b.add_i64_field(F_DATE, INDEXED | STORED | FAST);
        let f_internal_date = b.add_i64_field(F_INTERNAL_DATE, INDEXED | STORED | FAST);
        let f_ingest_at = b.add_i64_field(F_INGEST_AT, INDEXED | STORED | FAST);
        let f_size = b.add_u64_field(F_SIZE, INDEXED | STORED | FAST);
        let f_thread_id = b.add_text_field(F_THREAD_ID, STRING | STORED | FAST);
        let f_attachment_count = b.add_u64_field(F_ATTACHMENT_COUNT, INDEXED | STORED | FAST);
        let f_regular_attachment_count =
            b.add_u64_field(F_REGULAR_ATTACHMENT_COUNT, INDEXED | STORED | FAST);
        let f_attachment_name_text =
            b.add_text_field(F_ATTACHMENT_NAME_TEXT, text_no_store("euro"));
        let f_attachment_name_exact = b.add_text_field(F_ATTACHMENT_NAME_EXACT, STRING);
        let f_attachments = b.add_text_field(F_ATTACHMENTS, STORED);
        let f_attachment_content_hash =
            b.add_text_field(F_ATTACHMENT_CONTENT_HASH, STRING | STORED | FAST);
        let f_attachment_ext = b.add_text_field(F_ATTACHMENT_EXT, STRING | STORED | FAST);
        let f_attachment_category = b.add_text_field(F_ATTACHMENT_CATEGORY, STRING | STORED | FAST);
        let f_attachment_content_type =
            b.add_text_field(F_ATTACHMENT_CONTENT_TYPE, STRING | STORED | FAST);
        let f_tags = b.add_facet_field(F_TAGS, FacetOptions::default().set_stored());
        let f_shard_id = b.add_u64_field(F_SHARD_ID, INDEXED | STORED | FAST);

        let fields = EmailFields {
            f_id,
            f_message_id,
            f_account_id,
            f_mailbox_id,
            f_uid,
            f_subject,
            f_body,
            f_preview,
            f_content_hash,
            f_from,
            f_to,
            f_cc,
            f_bcc,
            f_from_text,
            f_to_text,
            f_cc_text,
            f_bcc_text,
            f_date,
            f_internal_date,
            f_ingest_at,
            f_size,
            f_thread_id,
            f_attachment_count,
            f_regular_attachment_count,
            f_attachments,
            f_attachment_name_text,
            f_attachment_name_exact,
            f_attachment_content_hash,
            f_attachment_ext,
            f_attachment_category,
            f_attachment_content_type,
            f_tags,
            f_shard_id,
        };

        (b.build(), fields)
    }

    fn fields() -> EmailFields {
        Self::build().1
    }
}

struct AttachmentSchema;

impl AttachmentSchema {
    fn build() -> (Schema, AttachmentFields) {
        let mut b = Schema::builder();

        let f_id = b.add_text_field(F_ID, STRING | STORED | FAST);
        let f_envelope_id = b.add_text_field(F_ENVELOPE_ID, STRING | STORED | FAST);
        let f_account_id = b.add_u64_field(F_ACCOUNT_ID, INDEXED | STORED | FAST);
        let f_mailbox_id = b.add_u64_field(F_MAILBOX_ID, INDEXED | STORED | FAST);
        let f_subject = b.add_text_field(F_SUBJECT, text_store("euro"));
        let f_content_hash = b.add_text_field(F_CONTENT_HASH, STRING | STORED | FAST);
        let f_from = b.add_text_field(F_FROM, STRING | STORED | FAST);
        let f_from_text = b.add_text_field(F_FROM_TEXT, text_no_store("euro"));
        let f_date = b.add_i64_field(F_DATE, INDEXED | STORED | FAST);
        let f_ingest_at = b.add_i64_field(F_INGEST_AT, INDEXED | STORED | FAST);
        let f_size = b.add_u64_field(F_SIZE, INDEXED | STORED | FAST);
        let f_ext = b.add_text_field(F_ATTACHMENT_EXT, STRING | STORED | FAST);
        let f_category = b.add_text_field(F_ATTACHMENT_CATEGORY, STRING | STORED | FAST);
        let f_content_type = b.add_text_field(F_ATTACHMENT_CONTENT_TYPE, STRING | STORED | FAST);
        let f_shard_id = b.add_u64_field(F_SHARD_ID, INDEXED | STORED | FAST);
        let f_text = b.add_text_field(F_TEXT, text_no_store("euro"));
        let f_has_text = b.add_bool_field(F_HAS_TEXT, INDEXED | STORED | FAST);
        let f_is_ocr = b.add_bool_field(F_IS_OCR, INDEXED | STORED | FAST);
        let f_page_count = b.add_u64_field(F_PAGE_COUNT, INDEXED | STORED | FAST);
        let f_is_indexed = b.add_bool_field(F_IS_INDEXED, INDEXED | STORED | FAST);
        let f_is_message = b.add_bool_field(F_IS_MESSAGE, INDEXED | STORED | FAST);
        let f_name_text = b.add_text_field(F_NAME_TEXT, text_no_store("euro"));
        let f_name_exact = b.add_text_field(F_NAME_EXACT, STRING | STORED);
        let f_tags = b.add_facet_field(F_TAGS, FacetOptions::default().set_stored());
        let f_auto_tags = b.add_facet_field(F_AUTO_TAGS, FacetOptions::default().set_stored());

        let fields = AttachmentFields {
            f_id,
            f_envelope_id,
            f_account_id,
            f_mailbox_id,
            f_subject,
            f_content_hash,
            f_from,
            f_from_text,
            f_date,
            f_ingest_at,
            f_size,
            f_ext,
            f_category,
            f_content_type,
            f_shard_id,
            f_text,
            f_has_text,
            f_is_ocr,
            f_page_count,
            f_is_indexed,
            f_is_message,
            f_name_text,
            f_name_exact,
            f_tags,
            f_auto_tags,
        };

        (b.build(), fields)
    }

    fn fields() -> AttachmentFields {
        Self::build().1
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn text_no_store(tokenizer: &str) -> TextOptions {
    TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer(tokenizer)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    )
}

fn text_store(tokenizer: &str) -> TextOptions {
    text_no_store(tokenizer).set_stored()
}
