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
use tantivy::schema::{FacetOptions, Field, INDEXED};
use tantivy::schema::{Schema, FAST, STORED, STRING, TEXT};

use crate::modules::store::tantivy::fields::{
    EmailFields, F_ACCOUNT_ID, F_ATTACHMENTS, F_ATTACHMENT_CATEGORY, F_ATTACHMENT_CONTENT_HASH,
    F_ATTACHMENT_CONTENT_TYPE, F_ATTACHMENT_COUNT, F_ATTACHMENT_EXT, F_ATTACHMENT_GLUE, F_BCC,
    F_BODY, F_CC, F_CONTENT_HASH, F_DATE, F_FROM, F_ID, F_INGEST_AT, F_INTERNAL_DATE, F_MAILBOX_ID,
    F_MESSAGE_ID, F_PREVIEW, F_REGULAR_ATTACHMENT_COUNT, F_SHARD_ID, F_SIZE, F_SUBJECT, F_TAGS,
    F_THREAD_ID, F_TO, F_UID,
};

static EMAIL_FIELDS: LazyLock<Arc<EmailFields>> = LazyLock::new(|| {
    let (_, fields) = SchemaTools::create_email_schema();
    Arc::new(fields)
});

pub struct SchemaTools;

impl SchemaTools {
    pub fn email_schema() -> Schema {
        let (schema, _) = Self::create_email_schema();
        schema
    }

    pub fn email_fields() -> &'static EmailFields {
        &EMAIL_FIELDS
    }

    pub fn email_default_fields() -> Vec<Field> {
        let fields = Self::email_fields();
        vec![
            fields.f_subject,
            fields.f_body,
            fields.f_attachment_glue,
            fields.f_from,
            fields.f_to,
        ]
    }

    pub fn create_email_schema() -> (Schema, EmailFields) {
        let mut builder = Schema::builder();

        let f_id = builder.add_text_field(F_ID, STRING | STORED | FAST);
        let f_message_id = builder.add_text_field(F_MESSAGE_ID, STRING | STORED);
        let f_account_id = builder.add_u64_field(F_ACCOUNT_ID, INDEXED | STORED | FAST);
        let f_mailbox_id = builder.add_u64_field(F_MAILBOX_ID, INDEXED | STORED | FAST);
        let f_uid = builder.add_u64_field(F_UID, INDEXED | STORED | FAST);
        let f_subject = builder.add_text_field(F_SUBJECT, TEXT | STORED);
        let f_body = builder.add_text_field(F_BODY, TEXT);
        let f_preview = builder.add_text_field(F_PREVIEW, STORED);
        let f_content_hash = builder.add_text_field(F_CONTENT_HASH, STRING | FAST | STORED);
        let f_from = builder.add_text_field(F_FROM, STRING | STORED | FAST);
        let f_to = builder.add_text_field(F_TO, STRING | STORED);
        let f_cc = builder.add_text_field(F_CC, STRING | STORED);
        let f_bcc = builder.add_text_field(F_BCC, STRING | STORED);
        let f_date = builder.add_i64_field(F_DATE, INDEXED | STORED | FAST);
        let f_internal_date = builder.add_i64_field(F_INTERNAL_DATE, INDEXED | STORED | FAST);
        let f_ingest_at = builder.add_i64_field(F_INGEST_AT, INDEXED | STORED | FAST);
        let f_size = builder.add_u64_field(F_SIZE, INDEXED | STORED | FAST);
        let f_thread_id = builder.add_text_field(F_THREAD_ID, STRING | STORED | FAST);
        let f_attachment_count = builder.add_u64_field(F_ATTACHMENT_COUNT, INDEXED | STORED | FAST);
        let f_regular_attachment_count =
            builder.add_u64_field(F_REGULAR_ATTACHMENT_COUNT, INDEXED | STORED | FAST);
        let f_attachment_glue = builder.add_text_field(F_ATTACHMENT_GLUE, TEXT);
        let f_attachments = builder.add_text_field(F_ATTACHMENTS, STORED);
        let f_attachment_content_hash =
            builder.add_text_field(F_ATTACHMENT_CONTENT_HASH, STRING | FAST | STORED);

        let f_attachment_ext = builder.add_text_field(F_ATTACHMENT_EXT, STRING | FAST | STORED);
        let f_attachment_category =
            builder.add_text_field(F_ATTACHMENT_CATEGORY, STRING | FAST | STORED);
        let f_attachment_content_type =
            builder.add_text_field(F_ATTACHMENT_CONTENT_TYPE, STRING | FAST | STORED);
        
        let f_tags = builder.add_facet_field(F_TAGS, FacetOptions::default().set_stored());
        let f_shard_id = builder.add_u64_field(F_SHARD_ID, INDEXED | STORED | FAST);

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
            f_date,
            f_internal_date,
            f_ingest_at,
            f_size,
            f_thread_id,
            f_attachment_count,
            f_regular_attachment_count,
            f_attachments,
            f_attachment_glue,
            f_attachment_content_hash,
            f_attachment_ext,
            f_attachment_category,
            f_attachment_content_type,
            f_tags,
            f_shard_id,
        };
        (builder.build(), fields)
    }
}
