use std::collections::HashSet;
use tantivy::{schema::Value, TantivyDocument};

use crate::{
    modules::{
        account::migration::AccountModel,
        cache::imap::mailbox::MailBox,
        error::{code::ErrorCode, BichonResult},
        message::content::AttachmentInfo,
        store::{envelope::Envelope, tantivy::schema::SchemaTools},
    },
    raise_error,
};

#[derive(Debug, Clone)]
pub struct EnvelopeWithAttachments {
    pub envelope: Envelope,
    pub attachments: Option<Vec<AttachmentInfo>>,
}

impl EnvelopeWithAttachments {
    pub fn to_document(&self, body_text: &str, shard_id: u64) -> BichonResult<TantivyDocument> {
        let fields = SchemaTools::email_fields();
        let mut doc = TantivyDocument::new();

        doc.add_text(fields.f_id, &self.envelope.id);
        doc.add_text(fields.f_message_id, &self.envelope.message_id);
        doc.add_u64(fields.f_account_id, self.envelope.account_id);
        doc.add_u64(fields.f_mailbox_id, self.envelope.mailbox_id);
        doc.add_u64(fields.f_uid, self.envelope.uid as u64);
        doc.add_text(fields.f_subject, &self.envelope.subject);
        doc.add_text(fields.f_preview, &self.envelope.preview);
        doc.add_text(fields.f_content_hash, &self.envelope.content_hash);
        doc.add_text(fields.f_from, &self.envelope.from);
        doc.add_text(fields.f_body, body_text);

        for to in &self.envelope.to {
            doc.add_text(fields.f_to, to);
        }
        for cc in &self.envelope.cc {
            doc.add_text(fields.f_cc, cc);
        }
        for bcc in &self.envelope.bcc {
            doc.add_text(fields.f_bcc, bcc);
        }

        doc.add_i64(fields.f_date, self.envelope.date);
        doc.add_i64(fields.f_internal_date, self.envelope.internal_date);
        doc.add_u64(fields.f_size, self.envelope.size as u64);
        doc.add_i64(fields.f_ingest_at, self.envelope.ingest_at);
        doc.add_text(fields.f_thread_id, &self.envelope.thread_id);

        if let Some(ref atts) = self.attachments {
            let atts_json = serde_json::to_string(atts).unwrap_or_else(|_| "[]".to_string());
            doc.add_text(fields.f_attachments, atts_json);
            let mut search_terms = Vec::new();

            for att in atts {
                if let Some(ref filename) = att.filename {
                    search_terms.push(filename.clone());
                }

                if let Some(ext) = att.get_extension() {
                    search_terms.push(ext.clone());
                    doc.add_text(fields.f_attachment_ext, ext);
                }
                let category = att.get_category().to_string();
                search_terms.push(category.clone());
                let file_type = att.file_type.to_lowercase();
                search_terms.push(file_type.clone());

                doc.add_text(fields.f_attachment_category, category);
                doc.add_text(fields.f_attachment_content_type, file_type);

                doc.add_text(fields.f_attachment_content_hash, &att.content_hash);
            }
            doc.add_text(fields.f_attachment_glue, search_terms.join(" "));
        }

        doc.add_u64(
            fields.f_attachment_count,
            self.envelope.attachment_count as u64,
        );
        doc.add_u64(
            fields.f_regular_attachment_count,
            self.envelope.regular_attachment_count as u64,
        );
        doc.add_u64(fields.f_shard_id, shard_id);
        Ok(doc)
    }

    pub fn from_tantivy_doc(doc: &TantivyDocument) -> BichonResult<Self> {
        let fields = SchemaTools::email_fields();

        let attachments_raw = extract_string_field(doc, fields.f_attachments).ok();
        let attachments: Option<Vec<AttachmentInfo>> =
            attachments_raw.and_then(|json| serde_json::from_str(&json).ok());

        let tags: Vec<String> = doc
            .get_all(fields.f_tags)
            .filter_map(|value| value.as_facet())
            .map(|f| f.to_string())
            .collect();

        let account_id = extract_u64_field(doc, fields.f_account_id)?;
        let mailbox_id = extract_u64_field(doc, fields.f_mailbox_id)?;

        let account = AccountModel::get(account_id)?;
        let mailbox = MailBox::get(mailbox_id)?;
        let envelope = Envelope {
            id: extract_string_field(doc, fields.f_id)?,
            message_id: extract_string_field(doc, fields.f_message_id)?,
            account_id,
            account_email: Some(account.email),
            mailbox_id,
            mailbox_name: Some(mailbox.name),
            uid: extract_u64_field(doc, fields.f_uid)? as u32,
            subject: extract_string_field(doc, fields.f_subject)?,
            preview: extract_string_field(doc, fields.f_preview).unwrap_or_default(),
            from: extract_string_field(doc, fields.f_from)?,
            to: extract_vec_string_field(doc, fields.f_to)?,
            cc: extract_vec_string_field(doc, fields.f_cc)?,
            bcc: extract_vec_string_field(doc, fields.f_bcc)?,
            date: extract_i64_field(doc, fields.f_date)?,
            internal_date: extract_i64_field(doc, fields.f_internal_date)?,
            size: extract_u64_field(doc, fields.f_size)? as u32,
            thread_id: extract_string_field(doc, fields.f_thread_id)?,
            attachment_count: extract_u64_field(doc, fields.f_attachment_count)? as usize,
            regular_attachment_count: extract_u64_field(doc, fields.f_regular_attachment_count)?
                as usize,
            tags: if tags.is_empty() { None } else { Some(tags) },
            content_hash: extract_string_field(doc, fields.f_content_hash)?,
            ingest_at: extract_i64_field(doc, fields.f_ingest_at)?,
        };

        Ok(EnvelopeWithAttachments {
            envelope,
            attachments,
        })
    }
}

fn extract_u64_field(
    document: &TantivyDocument,
    field: tantivy::schema::Field,
) -> BichonResult<u64> {
    let value = document.get_first(field).ok_or_else(|| {
        raise_error!(
            format!("miss '{}' field in tantivy document", stringify!(field)),
            ErrorCode::InternalError
        )
    })?;
    value.as_u64().ok_or_else(|| {
        raise_error!(
            format!("'{}' field is not a u64", stringify!(field)),
            ErrorCode::InternalError
        )
    })
}

fn extract_i64_field(
    document: &TantivyDocument,
    field: tantivy::schema::Field,
) -> BichonResult<i64> {
    let value = document.get_first(field).ok_or_else(|| {
        raise_error!(
            format!("miss '{}' field in tantivy document", stringify!(field)),
            ErrorCode::InternalError
        )
    })?;
    value.as_i64().ok_or_else(|| {
        raise_error!(
            format!("'{}' field is not a i64", stringify!(field)),
            ErrorCode::InternalError
        )
    })
}

fn extract_string_field(
    document: &TantivyDocument,
    field: tantivy::schema::Field,
) -> BichonResult<String> {
    let value = document.get_first(field).ok_or_else(|| {
        raise_error!(
            format!("'{}' field not found", stringify!(field)),
            ErrorCode::InternalError
        )
    })?;
    value.as_str().map(|s| s.to_string()).ok_or_else(|| {
        raise_error!(
            format!("'{}' field is not a string", stringify!(field)),
            ErrorCode::InternalError
        )
    })
}

fn extract_vec_string_field(
    document: &TantivyDocument,
    field: tantivy::schema::Field,
) -> BichonResult<Vec<String>> {
    let value = document
        .get_all(field)
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    Ok(value)
}

pub async fn extract_contacts(doc: &TantivyDocument) -> BichonResult<HashSet<String>> {
    let fields = SchemaTools::email_fields();
    let mut all_contacts = HashSet::new();

    if let Ok(from_val) = extract_string_field(doc, fields.f_from) {
        if !from_val.is_empty() {
            all_contacts.insert(from_val);
        }
    }

    let multi_fields = [fields.f_to, fields.f_cc, fields.f_bcc];

    for field in multi_fields {
        if let Ok(vals) = extract_vec_string_field(doc, field) {
            for v in vals {
                if !v.is_empty() {
                    all_contacts.insert(v);
                }
            }
        }
    }

    Ok(all_contacts)
}
