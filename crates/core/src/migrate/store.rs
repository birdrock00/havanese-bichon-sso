use std::path::PathBuf;

use bytes::Bytes;
use mail_parser::MimeHeaders;

use crate::{
    envelope::extractor::extract_references, message::content::AttachmentInfo,
    store::tantivy::tokenizers::EuroTokenizer, utils::compute_content_hash,
};

use fjall::{
    config::{BlockSizePolicy, CompressionPolicy},
    CompressionType, Database, Keyspace, KeyspaceCreateOptions, KvSeparationOptions,
};
use mail_parser::MessageParser;
use tantivy::{Index, IndexWriter, TantivyDocument};
use uuid::Uuid;

use crate::{
    common::AddrVec,
    envelope::extractor::{compute_thread_id, generate_message_id},
    error::{code::ErrorCode, BichonResult},
    raise_error,
    store::envelope::Envelope,
    store::tantivy::{
        model::{AttachmentModel, EnvelopeWithAttachments},
        schema::SchemaTools,
    },
    utc_now,
};

pub struct LegacyDirs {
    pub envelope_dir: PathBuf,
    pub eml_dir: PathBuf,
}

pub struct NewDirs {
    pub envelope_dir: PathBuf,
    pub attachment_dir: PathBuf,
    pub storage_dir: PathBuf,
}

impl LegacyDirs {
    pub fn new(index: PathBuf, data: PathBuf) -> Self {
        Self {
            envelope_dir: index,
            eml_dir: data,
        }
    }
}

impl NewDirs {
    pub fn new(index: PathBuf, data: PathBuf) -> Self {
        Self {
            envelope_dir: index.join("mail_metadata"),
            attachment_dir: index.join("attachment_metadata"),
            storage_dir: data,
        }
    }
}

pub struct DetachOutput {
    pub infos: Vec<AttachmentInfo>,
    pub blobs: Vec<(String, Bytes)>,
}

pub fn detach_attachments_standalone(
    original_body: &[u8],
    message: &mail_parser::Message<'_>,
) -> (Vec<u8>, DetachOutput) {
    let mut stripped_eml = original_body.to_vec();
    let mut infos = Vec::new();
    let mut blobs = Vec::new();

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

    for (raw_start, raw_end, att) in ranges {
        let content_hash = compute_content_hash(att.contents());
        blobs.push((
            content_hash.clone(),
            Bytes::copy_from_slice(&original_body[raw_start..raw_end]),
        ));

        let placeholder = format!("<<BICHON_DETACH_HASH:{}>>", &content_hash);
        stripped_eml.splice(raw_start..raw_end, placeholder.as_bytes().iter().cloned());

        infos.push(AttachmentInfo {
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
            content_hash,
            is_message: att.is_message(),
        });
    }

    (stripped_eml, DetachOutput { infos, blobs })
}

pub struct NewIndexWriter {
    pub envelope_writer: IndexWriter,
    pub attachment_writer: IndexWriter,
    pub email_ks: Keyspace,
    pub attachment_ks: Keyspace,
    pending: usize,
}

const COMMIT_THRESHOLD: usize = 500;

impl NewIndexWriter {
    pub fn open(dirs: NewDirs) -> BichonResult<Self> {
        // ── envelope index ──────────────────────────────────────────────
        std::fs::create_dir_all(&dirs.envelope_dir)
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
        let envelope_index = if dirs
            .envelope_dir
            .read_dir()
            .map(|mut d| d.next().is_none())
            .unwrap_or(true)
        {
            Index::create_in_dir(&dirs.envelope_dir, SchemaTools::email_schema())
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?
        } else {
            Index::open_in_dir(&dirs.envelope_dir)
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?
        };

        envelope_index
            .tokenizers()
            .register("euro", EuroTokenizer::new());

        let envelope_writer = envelope_index
            .writer_with_num_threads(2, 128 * 1024 * 1024)
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

        // ── attachment index ─────────────────────────────────────────────
        std::fs::create_dir_all(&dirs.attachment_dir)
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
        let attachment_index = if dirs
            .attachment_dir
            .read_dir()
            .map(|mut d| d.next().is_none())
            .unwrap_or(true)
        {
            Index::create_in_dir(&dirs.attachment_dir, SchemaTools::attachment_schema())
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?
        } else {
            Index::open_in_dir(&dirs.attachment_dir)
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?
        };

        attachment_index
            .tokenizers()
            .register("euro", EuroTokenizer::new());
        let attachment_writer = attachment_index
            .writer_with_num_threads(2, 64 * 1024 * 1024)
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

        // ── blob store ───────────────────────────────────────────────────
        std::fs::create_dir_all(&dirs.storage_dir)
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
        let db = Database::builder(&dirs.storage_dir)
            .cache_size(64 * 1024 * 1024)
            .open()
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

        let email_ks = db
            .keyspace("email", || {
                KeyspaceCreateOptions::default()
                    .max_memtable_size(16 * 1024 * 1024)
                    .data_block_size_policy(BlockSizePolicy::all(4 * 1024))
                    .data_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4))
                    .with_kv_separation(Some(
                        KvSeparationOptions::default()
                            .separation_threshold(1024)
                            .compression(CompressionType::Lz4)
                            .file_target_size(512 * 1024 * 1024),
                    ))
            })
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

        let attachment_ks = db
            .keyspace("attachments", || {
                KeyspaceCreateOptions::default()
                    .max_memtable_size(16 * 1024 * 1024)
                    .data_block_size_policy(BlockSizePolicy::all(4 * 1024))
                    .data_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4))
                    .with_kv_separation(Some(
                        KvSeparationOptions::default()
                            .separation_threshold(1024)
                            .compression(CompressionType::Lz4)
                            .file_target_size(512 * 1024 * 1024),
                    ))
            })
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

        Ok(Self {
            envelope_writer,
            attachment_writer,
            email_ks,
            attachment_ks,
            pending: 0,
        })
    }

    pub fn ingest(
        &mut self,
        eml_bytes: &[u8],
        account_id: u64,
        mailbox_id: u64,
        uid: u32,
        internal_date: i64,
    ) -> BichonResult<()> {
        let email_content_hash = compute_content_hash(eml_bytes);

        let message = MessageParser::new()
            .parse(eml_bytes)
            .ok_or_else(|| raise_error!("failed to parse eml".into(), ErrorCode::InternalError))?;

        // ── text / preview ────────────────────────────────────────────────
        let text = message
            .body_text(0)
            .map(|c| c.into_owned())
            .or_else(|| {
                message
                    .body_html(0)
                    .map(|html| crate::utils::html::extract_text(html.into_owned()))
            })
            .unwrap_or_default();
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let preview = if text.chars().count() > 100 {
            text.chars().take(100).collect::<String>() + "..."
        } else {
            text.clone()
        };

        // ── headers ───────────────────────────────────────────────────────
        let message_id = message
            .message_id()
            .map(String::from)
            .unwrap_or_else(generate_message_id);

        let in_reply_to = message.in_reply_to().as_text().map(String::from);
        let references = extract_references(&message);
        let thread_id = compute_thread_id(in_reply_to, references, &message_id);

        let subject = message.subject().map(String::from).unwrap_or_default();
        let date = message.date().map(|d| d.to_timestamp() * 1000).unwrap_or(0);
        let internal_date = if internal_date == 0 {
            date
        } else {
            internal_date
        };

        let parse_addrs = |addrs: Option<&mail_parser::Address<'_>>| {
            addrs
                .map(|addr| {
                    AddrVec::from(addr)
                        .0
                        .into_iter()
                        .filter_map(|a| a.address)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };

        let from = message
            .from()
            .and_then(|addr| AddrVec::from(addr).0.into_iter().next())
            .and_then(|a| a.address)
            .unwrap_or_else(|| "unknown".to_string());
        let to = parse_addrs(message.to());
        let cc = parse_addrs(message.cc());
        let bcc = parse_addrs(message.bcc());

        // ── detach attachments → blob ──────────────────────────────────────
        let (stripped_eml, attachment_output) = detach_attachments_standalone(eml_bytes, &message);

        if !self
            .email_ks
            .contains_key(&email_content_hash)
            .unwrap_or(false)
        {
            self.email_ks
                .insert(&email_content_hash, stripped_eml.as_slice())
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
        }

        // write attachment blobs
        for (hash, data) in &attachment_output.blobs {
            if !self.attachment_ks.contains_key(hash).unwrap_or(false) {
                self.attachment_ks
                    .insert(hash, data.as_ref())
                    .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
            }
        }

        // ── build envelope doc ────────────────────────────────────────────
        let envelope_id = Uuid::new_v4().to_string();
        let now = utc_now!();

        let attachment_docs: Vec<TantivyDocument> = attachment_output
            .infos
            .iter()
            .filter(|a| !a.inline || a.content_id.is_none())
            .map(|a| {
                AttachmentModel {
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
                }
                .into_document()
            })
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
            size: eml_bytes.len() as u32,
            thread_id,
            attachment_count: message.attachment_count(),
            regular_attachment_count: attachment_docs.len(),
            tags: None,
            account_email: None,
            mailbox_name: None,
            content_hash: email_content_hash,
        };

        let ea = EnvelopeWithAttachments {
            envelope,
            attachments: Some(attachment_output.infos),
        };
        let envelope_doc = ea.to_document(&text, 0)?;

        self.envelope_writer
            .add_document(envelope_doc)
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

        for doc in attachment_docs {
            self.attachment_writer
                .add_document(doc)
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
        }

        self.pending += 1;
        if self.pending >= COMMIT_THRESHOLD {
            self.commit()?;
        }

        Ok(())
    }

    pub fn commit(&mut self) -> BichonResult<()> {
        if self.pending == 0 {
            return Ok(());
        }
        self.envelope_writer
            .commit()
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
        self.attachment_writer
            .commit()
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
        tracing::info!(count = self.pending, "committed batch");
        self.pending = 0;
        Ok(())
    }
}
