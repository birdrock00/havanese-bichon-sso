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

use std::{
    collections::{HashMap, HashSet},
    ops::Bound,
    path::PathBuf,
    sync::{Arc, LazyLock},
    time::Duration,
};

use crate::{
    modules::{
        account::migration::AccountModel,
        common::signal::SIGNAL_MANAGER,
        dashboard::{DashboardStats, Group, LargestEmail, TimeBucket},
        error::{code::ErrorCode, BichonResult},
        message::{
            search::{EmailSearchFilter, SortBy},
            tags::{TagAction, TagCount, TagsRequest},
        },
        rest::response::DataPage,
        settings::dir::DATA_DIR_MANAGER,
        store::{
            envelope::Envelope,
            storage::BLOB_MANAGER,
            tantivy::{
                fatal_commit,
                fields::{
                    F_ACCOUNT_ID, F_DATE, F_FROM, F_REGULAR_ATTACHMENT_COUNT, F_SIZE, F_TAGS,
                    F_THREAD_ID, F_UID,
                },
                model::{extract_contacts, EnvelopeWithAttachments},
                schema::SchemaTools,
            },
        },
    },
    raise_error, utc_now,
};

use chrono::Utc;
use serde_json::json;
use tantivy::{
    aggregation::{
        agg_req::Aggregations,
        agg_result::{
            AggregationResult, AggregationResults, BucketEntries, BucketResult, MetricResult,
        },
        AggregationCollector, Key,
    },
    collector::{Count, DocSetCollector, FacetCollector, TopDocs},
    indexer::{LogMergePolicy, UserOperation},
    query::{AllQuery, BooleanQuery, EmptyQuery, Occur, Query, QueryParser, RangeQuery, TermQuery},
    schema::{IndexRecordOption, Value},
    DocAddress, Index, IndexReader, IndexWriter, Order, TantivyDocument, Term,
};
use tantivy::{query::RegexQuery, schema::Facet, Searcher};
use tokio::{
    sync::{mpsc, Mutex},
    task::{self, JoinHandle},
};
use tracing::{info, warn};

pub static ENVELOPE_MANAGER: LazyLock<IndexManager> = LazyLock::new(IndexManager::new);

pub struct IndexManager {
    index: Arc<Index>,
    index_writer: Arc<Mutex<IndexWriter>>,
    sender: mpsc::Sender<TantivyDocument>,
    reader: IndexReader,
    query_parser: QueryParser,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl IndexManager {
    pub async fn shutdown(&self) {
        let mut guard = self.handle.lock().await;
        if let Some(handle) = guard.take() {
            let _ = handle.await;
        }
    }
    pub fn new() -> Self {
        let index = Self::open_or_create_index(&DATA_DIR_MANAGER.envelope_dir);
        let mut merge_policy = LogMergePolicy::default();
        merge_policy.set_min_num_segments(25);
        merge_policy.set_min_layer_size(10_000);
        merge_policy.set_max_docs_before_merge(100_000);

        let index_writer = index
            .writer_with_num_threads(4, 67_108_864)
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to create IndexWriter with 4 threads and 64MB buffer for {:?}: {}",
                    &DATA_DIR_MANAGER.envelope_dir, e
                )
            });
        index_writer.set_merge_policy(Box::new(merge_policy));
        let index_writer = Arc::new(Mutex::new(index_writer));
        let reader = index.reader().unwrap_or_else(|e| {
            panic!(
                "Failed to create IndexReader for {:?}: {}",
                &DATA_DIR_MANAGER.envelope_dir, e
            )
        });
        let mut query_parser = QueryParser::for_index(&index, SchemaTools::email_default_fields());
        query_parser.set_conjunction_by_default();

        let (sender, mut receiver) = mpsc::channel::<TantivyDocument>(100);

        let writer = index_writer.clone();
        let handler = task::spawn(async move {
            let mut shutdown = SIGNAL_MANAGER.subscribe();
            let mut commit_interval = tokio::time::interval(Duration::from_secs(60));
            let mut pending_count = 0;
            let commit_threshold = 1000;
            loop {
                tokio::select! {
                    maybe_msg = receiver.recv() => {
                        match maybe_msg {
                            Some(doc) => {
                                let mut writer = writer.lock().await;
                                let mut batch_count = 0;
                                match writer.add_document(doc) {
                                    Ok(_) => {
                                        batch_count += 1;
                                    }
                                    Err(e) => {
                                        eprintln!("[ERROR] Failed to add document: {e:?}");
                                        tracing::error!("Tantivy: Failed to add document: {e:?}");
                                    }
                                }
                                while let Ok(next_doc) = receiver.try_recv() {
                                    match writer.add_document(next_doc) {
                                        Ok(_) => batch_count += 1,
                                        Err(e) => {
                                            eprintln!("[ERROR] Failed to add document: {e:?}");
                                            tracing::error!("Tantivy: Failed to add document: {e:?}");
                                        }
                                    }
                                }
                                if batch_count > 0 {
                                    pending_count += batch_count;
                                }
                                if pending_count >= commit_threshold {
                                    tracing::info!(
                                        "Tantivy: Reached threshold ({} docs), committing...",
                                        pending_count
                                    );
                                    fatal_commit(&mut writer);
                                    pending_count = 0;
                                    commit_interval.reset();
                                }
                            }
                            None => {
                                tracing::info!("Tantivy: Receiver closed. Finalizing...");
                                if pending_count > 0 {
                                    let mut writer = writer.lock().await;
                                    fatal_commit(&mut writer);
                                }
                                break;
                            },
                        }
                    }
                    _ = commit_interval.tick() => {
                        if pending_count > 0 {
                            let mut writer = writer.lock().await;
                            fatal_commit(&mut writer);
                            pending_count = 0;
                            tracing::debug!("Tantivy: Periodic commit finished.");
                        }
                    }
                    _ = shutdown.recv() => {
                        tracing::info!("Tantivy: Shutdown signal received. Performing final commit...");
                        if pending_count > 0 {
                            let mut writer = writer.lock().await;
                            fatal_commit(&mut writer);
                        }
                        tracing::info!("Tantivy: Shutdown cleanup complete.");
                        break;
                    }
                }
            }
        });
        Self {
            index: Arc::new(index),
            index_writer,
            sender,
            reader,
            query_parser,
            handle: Mutex::new(Some(handler)),
        }
    }

    pub async fn queue(&self, doc: TantivyDocument) {
        let _ = self.sender.send(doc).await;
    }

    fn open_or_create_index(index_dir: &PathBuf) -> Index {
        let need_create = !index_dir.exists()
            || index_dir
                .read_dir()
                .map(|mut d| d.next().is_none())
                .unwrap_or(true);
        if need_create {
            info!(
                "Email index not found or empty, creating new index at {}",
                index_dir.display()
            );
            std::fs::create_dir_all(&index_dir).unwrap_or_else(|e| {
                panic!("Failed to create index directory {:?}: {}", index_dir, e)
            });
            Index::create_in_dir(&index_dir, SchemaTools::email_schema())
                .unwrap_or_else(|e| panic!("Failed to create index in {:?}: {}", index_dir, e))
        } else {
            info!("Opening existing email index at {}", index_dir.display());
            Self::open(&index_dir)
        }
    }

    fn open(index_dir: &PathBuf) -> Index {
        Index::open_in_dir(index_dir)
            .unwrap_or_else(|e| panic!("Failed to open index in {:?}: {}", index_dir, e))
    }

    fn account_query(&self, account_id: u64) -> Box<TermQuery> {
        let account_term =
            Term::from_field_u64(SchemaTools::email_fields().f_account_id, account_id);
        Box::new(TermQuery::new(account_term, IndexRecordOption::Basic))
    }

    fn mailbox_query(&self, account_id: u64, mailbox_id: u64) -> Box<dyn Query> {
        let account_query = TermQuery::new(
            Term::from_field_u64(SchemaTools::email_fields().f_account_id, account_id),
            IndexRecordOption::Basic,
        );
        let mailbox_query = TermQuery::new(
            Term::from_field_u64(SchemaTools::email_fields().f_mailbox_id, mailbox_id),
            IndexRecordOption::Basic,
        );
        let boolean_query = BooleanQuery::new(vec![
            (Occur::Must, Box::new(account_query)),
            (Occur::Must, Box::new(mailbox_query)),
        ]);
        Box::new(boolean_query)
    }

    fn envelope_query(&self, account_id: u64, eid: &str) -> Box<dyn Query> {
        let account_id_query = TermQuery::new(
            Term::from_field_u64(SchemaTools::email_fields().f_account_id, account_id),
            IndexRecordOption::Basic,
        );
        let envelope_id_query = TermQuery::new(
            Term::from_field_text(SchemaTools::email_fields().f_id, eid),
            IndexRecordOption::Basic,
        );
        let boolean_query = BooleanQuery::new(vec![
            (Occur::Must, Box::new(account_id_query)),
            (Occur::Must, Box::new(envelope_id_query)),
        ]);
        Box::new(boolean_query)
    }

    fn filter_query(
        &self,
        accounts: Option<HashSet<u64>>,
        filter: EmailSearchFilter,
        parser: QueryParser,
    ) -> BichonResult<Box<dyn Query>> {
        let f = SchemaTools::email_fields();
        let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        if let Some(authorized_ids) = accounts {
            if authorized_ids.is_empty() {
                let term = Term::from_field_u64(f.f_account_id, u64::MAX);
                subqueries.push((
                    Occur::Must,
                    Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                ));
            } else {
                let mut account_must_queries = Vec::new();
                for id in authorized_ids {
                    let term = Term::from_field_u64(f.f_account_id, id);
                    account_must_queries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                subqueries.push((
                    Occur::Must,
                    Box::new(BooleanQuery::new(account_must_queries)),
                ));
            }
        }

        if let Some(ref text) = filter.text {
            let query = parser
                .parse_query(text)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter))?;
            subqueries.push((Occur::Must, Box::new(query)));
        }

        if let Some(ref subject_val) = filter.subject {
            let query_parser = QueryParser::for_index(&self.index, vec![f.f_subject]);
            if let Ok(q) = query_parser.parse_query(subject_val) {
                subqueries.push((Occur::Must, q));
            }
        }

        if let Some(ref body_val) = filter.body {
            let query_parser = QueryParser::for_index(&self.index, vec![f.f_body]);
            if let Ok(q) = query_parser.parse_query(body_val) {
                subqueries.push((Occur::Must, q));
            }
        }

        if let Some(ref tags) = filter.tags {
            if !tags.is_empty() {
                let mut should_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

                for tag in tags {
                    let facet = Facet::from_text(tag).map_err(|e| {
                        raise_error!(format!("{:#?}", e), ErrorCode::InvalidParameter)
                    })?;

                    let term = Term::from_facet(f.f_tags, &facet);

                    should_queries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                    ));
                }
                subqueries.push((Occur::Must, Box::new(BooleanQuery::new(should_queries))));
            }
        }

        for (field, opt_value) in [
            (f.f_from, &filter.from),
            (f.f_to, &filter.to),
            (f.f_cc, &filter.cc),
            (f.f_bcc, &filter.bcc),
        ] {
            if let Some(ref v) = opt_value {
                if let Ok(query) = RegexQuery::from_pattern(v.as_str(), field) {
                    subqueries.push((Occur::Must, Box::new(query)));
                }
            }
        }

        if let Some(has) = filter.has_attachment {
            let lower: Bound<Term>;
            let upper: Bound<Term>;

            if has {
                lower = Bound::Included(Term::from_field_u64(f.f_regular_attachment_count, 1));
                upper =
                    Bound::Included(Term::from_field_u64(f.f_regular_attachment_count, u64::MAX));
            } else {
                lower = Bound::Included(Term::from_field_u64(f.f_regular_attachment_count, 0));
                upper = Bound::Included(Term::from_field_u64(f.f_regular_attachment_count, 0));
            }

            subqueries.push((Occur::Must, Box::new(RangeQuery::new(lower, upper))));
        }

        if let Some(ref name) = filter.attachment_name {
            if name.contains('.') {
                let term = Term::from_field_text(f.f_attachment_name_exact, name);
                let query = TermQuery::new(term, IndexRecordOption::Basic);
                subqueries.push((Occur::Should, Box::new(query)));
            }

            let query_parser = QueryParser::for_index(&self.index, vec![f.f_attachment_name_text]);
            if let Ok(q) = query_parser.parse_query(name) {
                subqueries.push((Occur::Must, q));
            }
        }

        if let Some(ref id) = filter.id {
            let term = Term::from_field_text(f.f_id, id);
            let query = TermQuery::new(term, IndexRecordOption::Basic);
            subqueries.push((Occur::Must, Box::new(query)));
        }

        if let Some(ref extension) = filter.attachment_extension {
            let term = Term::from_field_text(f.f_attachment_ext, extension);
            let query = TermQuery::new(term, IndexRecordOption::Basic);
            subqueries.push((Occur::Must, Box::new(query)));
        }

        if let Some(ref category) = filter.attachment_category {
            let term = Term::from_field_text(f.f_attachment_category, category);
            let query = TermQuery::new(term, IndexRecordOption::Basic);
            subqueries.push((Occur::Must, Box::new(query)));
        }

        if let Some(ref content_type) = filter.attachment_content_type {
            let term = Term::from_field_text(f.f_attachment_content_type, content_type);
            let query = TermQuery::new(term, IndexRecordOption::Basic);
            subqueries.push((Occur::Must, Box::new(query)));
        }

        let start_bound = if let Some(from) = filter.since {
            Bound::Included(Term::from_field_i64(f.f_date, from))
        } else {
            Bound::Unbounded
        };

        let end_bound = if let Some(to) = filter.before {
            Bound::Included(Term::from_field_i64(f.f_date, to))
        } else {
            Bound::Unbounded
        };

        if start_bound != Bound::Unbounded || end_bound != Bound::Unbounded {
            let q = RangeQuery::new(start_bound, end_bound);
            subqueries.push((Occur::Must, Box::new(q)));
        }

        if let Some(account_ids) = filter.account_ids {
            let mut should_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
            for id in account_ids {
                let term = Term::from_field_u64(f.f_account_id, id);
                should_queries.push((
                    Occur::Should,
                    Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                ));
            }
            subqueries.push((Occur::Must, Box::new(BooleanQuery::new(should_queries))));
        }

        if let Some(mailbox_ids) = filter.mailbox_ids {
            let mut should_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
            for id in mailbox_ids {
                let term = Term::from_field_u64(f.f_mailbox_id, id);
                should_queries.push((
                    Occur::Should,
                    Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                ));
            }
            subqueries.push((Occur::Must, Box::new(BooleanQuery::new(should_queries))));
        }

        let start_bound = if let Some(from) = filter.min_size {
            Bound::Included(Term::from_field_u64(f.f_size, from))
        } else {
            Bound::Unbounded
        };

        let end_bound = if let Some(to) = filter.max_size {
            Bound::Included(Term::from_field_u64(f.f_size, to))
        } else {
            Bound::Unbounded
        };

        if start_bound != Bound::Unbounded || end_bound != Bound::Unbounded {
            let q = RangeQuery::new(start_bound, end_bound);
            subqueries.push((Occur::Must, Box::new(q)));
        }

        if let Some(ref msg_id) = filter.message_id {
            let term = Term::from_field_text(f.f_message_id, msg_id);
            subqueries.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }

        if subqueries.is_empty() {
            return Ok(Box::new(AllQuery));
        }

        Ok(Box::new(BooleanQuery::new(subqueries)))
    }

    fn thread_query(&self, account_id: u64, thread_id: &str) -> Box<dyn Query> {
        let account_query = TermQuery::new(
            Term::from_field_u64(SchemaTools::email_fields().f_account_id, account_id),
            IndexRecordOption::Basic,
        );
        let thread_query = TermQuery::new(
            Term::from_field_text(SchemaTools::email_fields().f_thread_id, thread_id),
            IndexRecordOption::Basic,
        );
        let boolean_query = BooleanQuery::new(vec![
            (Occur::Must, Box::new(account_query)),
            (Occur::Must, Box::new(thread_query)),
        ]);
        Box::new(boolean_query)
    }

    pub async fn get_envelope_by_id(
        &self,
        account_id: u64,
        envelope_id: &str,
    ) -> BichonResult<Option<EnvelopeWithAttachments>> {
        let searcher = self.create_searcher()?;
        let f = SchemaTools::email_fields();

        let query = BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(f.f_account_id, account_id),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f.f_id, envelope_id),
                    IndexRecordOption::Basic,
                )),
            ),
        ]);

        let docs: Vec<(f32, DocAddress)> = searcher
            .search(&query, &TopDocs::with_limit(1).order_by_score())
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        if let Some((_, doc_address)) = docs.first() {
            let doc: TantivyDocument = searcher
                .doc(*doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            let envelope = EnvelopeWithAttachments::from_tantivy_doc(&doc)?;
            Ok(Some(envelope))
        } else {
            Ok(None)
        }
    }

    pub async fn top_10_largest_emails(
        &self,
        accounts: &Option<HashSet<u64>>,
    ) -> BichonResult<Vec<LargestEmail>> {
        self.reader
            .reload()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        let searcher = self.reader.searcher();

        let query: Box<dyn Query> = match accounts {
            Some(ref ids) if !ids.is_empty() => {
                let mut subqueries = Vec::new();
                for &id in ids {
                    let term = Term::from_field_u64(SchemaTools::email_fields().f_account_id, id);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                Box::new(BooleanQuery::new(subqueries))
            }
            Some(_) => Box::new(EmptyQuery),
            None => Box::new(AllQuery),
        };

        let mailbox_docs: Vec<(Option<u64>, DocAddress)> = searcher
            .search(
                &query,
                &TopDocs::with_limit(10).order_by_fast_field(F_SIZE, Order::Desc),
            )
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        let mut result = Vec::new();

        for (_, doc_address) in mailbox_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            let envelope = LargestEmail::from_tantivy_doc(&doc)?;
            result.push(envelope);
        }
        Ok(result)
    }

    pub fn total_emails(&self, accounts: &Option<HashSet<u64>>) -> BichonResult<u64> {
        let searcher = self.create_searcher()?;

        match accounts {
            Some(ref ids) if !ids.is_empty() => {
                let mut subqueries = Vec::new();
                for &id in ids {
                    let term = Term::from_field_u64(SchemaTools::email_fields().f_account_id, id);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                let query = Box::new(BooleanQuery::new(subqueries)) as Box<dyn Query>;
                let count = searcher
                    .search(&query, &Count)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
                Ok(count as u64)
            }
            Some(_) => Ok(0),
            None => Ok(searcher.num_docs()),
        }
    }

    pub async fn get_max_uid(&self, account_id: u64, mailbox_id: u64) -> BichonResult<Option<u64>> {
        let searcher = self.create_searcher()?;

        let query = self.mailbox_query(account_id, mailbox_id);
        let agg_req: Aggregations = serde_json::from_value(json!({
            "max_uid": {
                "max": {
                    "field": F_UID
                }
            }
        }))
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        let collector = AggregationCollector::from_aggs(agg_req, Default::default());

        let agg_res = searcher
            .search(query.as_ref(), &collector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Ok(Self::extract_max_uid(&agg_res))
    }

    fn extract_max_uid(agg_res: &AggregationResults) -> Option<u64> {
        agg_res.0.get("max_uid").and_then(|result| match result {
            AggregationResult::MetricResult(MetricResult::Max(max)) => {
                max.value.and_then(|value| {
                    (value >= 0.0 && value <= u64::MAX as f64).then(|| value.trunc() as u64)
                })
            }
            _ => None,
        })
    }

    pub async fn delete_account_envelopes(&self, account_id: u64) -> BichonResult<()> {
        let query = self.account_query(account_id);
        let (eml_content_hashes, attachments_content_hashes) =
            self.collect_content_hashes(query).await?;

        let query = self.account_query(account_id);

        let mut writer = self.index_writer.lock().await;
        writer
            .delete_query(query)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        writer
            .commit()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        if !eml_content_hashes.is_empty() || !attachments_content_hashes.is_empty() {
            self.cleanup_unused_content(eml_content_hashes, attachments_content_hashes)
                .await?;
        }
        Ok(())
    }

    pub async fn delete_mailbox_envelopes(
        &self,
        account_id: u64,
        mailbox_ids: Vec<u64>,
    ) -> BichonResult<()> {
        if mailbox_ids.is_empty() {
            return Ok(());
        }

        let mut eml_content_hashes: HashSet<String> = HashSet::new();
        let mut attachments_content_hashes: HashSet<String> = HashSet::new();

        for mailbox_id in &mailbox_ids {
            let query = self.mailbox_query(account_id, *mailbox_id);
            let (eml_hashes, attachment_hashes) = self.collect_content_hashes(query).await?;
            eml_content_hashes.extend(eml_hashes);
            attachments_content_hashes.extend(attachment_hashes);
        }

        let mut queries: Vec<Box<dyn Query>> = Vec::with_capacity(mailbox_ids.len());
        for mailbox_id in mailbox_ids {
            queries.push(self.mailbox_query(account_id, mailbox_id));
        }
        let mut writer = self.index_writer.lock().await;
        for query in queries {
            writer
                .delete_query(query)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        }
        writer
            .commit()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        if !eml_content_hashes.is_empty() || !attachments_content_hashes.is_empty() {
            self.cleanup_unused_content(eml_content_hashes, attachments_content_hashes)
                .await?;
        }
        Ok(())
    }

    async fn collect_content_hashes(
        &self,
        query: Box<dyn Query>,
    ) -> BichonResult<(HashSet<String>, HashSet<String>)> {
        let mut eml_content_hashes = HashSet::new();
        let mut attachments_content_hashes = HashSet::new();

        let fields = SchemaTools::email_fields();
        let searcher = self.create_searcher()?;

        let docs = searcher
            .search(&query, &DocSetCollector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        for doc_address in docs {
            let doc = searcher
                .doc::<TantivyDocument>(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

            // Extract content_hash
            if let Some(content_hash_value) = doc.get_first(fields.f_content_hash) {
                if let Some(str) = content_hash_value.as_str() {
                    eml_content_hashes.insert(str.to_string());
                }
            }

            // Extract attachment_content_hash
            let attachment_hash_values = doc.get_all(fields.f_attachment_content_hash);
            for hash_value in attachment_hash_values {
                if let Some(str) = hash_value.as_str() {
                    attachments_content_hashes.insert(str.to_string());
                }
            }
        }

        Ok((eml_content_hashes, attachments_content_hashes))
    }

    async fn cleanup_unused_content(
        &self,
        eml_content_hashes: HashSet<String>,
        attachments_content_hashes: HashSet<String>,
    ) -> BichonResult<()> {
        let searcher = self.create_searcher()?;
        let fields = SchemaTools::email_fields();
        let mut eml: HashSet<String> = HashSet::new();
        for content_hash in eml_content_hashes {
            // Check if any other emails still reference this content hash
            let hash_term = Term::from_field_text(fields.f_content_hash, &content_hash);
            let hash_query = TermQuery::new(hash_term, IndexRecordOption::Basic);
            let count = searcher
                .search(&hash_query, &Count)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

            // If no references found, delete from KV store
            if count == 0 {
                eml.insert(content_hash);
            }
        }

        let mut attachments: HashSet<String> = HashSet::new();
        for content_hash in attachments_content_hashes {
            // Check if any other emails still reference this content hash
            let hash_term = Term::from_field_text(fields.f_content_hash, &content_hash);
            let hash_query = TermQuery::new(hash_term, IndexRecordOption::Basic);
            let count = searcher
                .search(&hash_query, &Count)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

            // If no references found, delete from KV store
            if count == 0 {
                attachments.insert(content_hash);
            }
        }

        BLOB_MANAGER.delete(&eml, &attachments)
    }

    pub async fn delete_envelopes_multi_account(
        &self,
        deletes: HashMap<u64, Vec<String>>,
    ) -> BichonResult<()> {
        if deletes.is_empty() {
            tracing::warn!("delete_envelopes_multi_account: deletes is empty, nothing to delete");
            return Ok(());
        }

        let mut eml_content_hashes: HashSet<String> = HashSet::new();
        let mut attachments_content_hashes: HashSet<String> = HashSet::new();

        for (account_id, envelope_ids) in &deletes {
            let unique_ids: HashSet<&String> = envelope_ids.iter().collect();
            if unique_ids.is_empty() {
                continue;
            }

            for eid in unique_ids {
                let query = self.envelope_query(*account_id, eid);
                let (eml_hashes, attachment_hashes) = self.collect_content_hashes(query).await?;
                eml_content_hashes.extend(eml_hashes);
                attachments_content_hashes.extend(attachment_hashes);
            }
        }

        let mut writer = self.index_writer.lock().await;

        for (account_id, envelope_ids) in deletes {
            let unique_ids: HashSet<&String> = envelope_ids.iter().collect();
            if unique_ids.is_empty() {
                continue;
            }
            for eid in unique_ids {
                let query = self.envelope_query(account_id, eid);
                writer
                    .delete_query(query)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            }
        }
        writer
            .commit()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        if !eml_content_hashes.is_empty() || !attachments_content_hashes.is_empty() {
            self.cleanup_unused_content(eml_content_hashes, attachments_content_hashes)
                .await?;
        }

        Ok(())
    }

    fn collect_facets_recursive(
        query: &dyn Query,
        searcher: &Searcher,
        parent_facet: &str,
        all_facets: &mut Vec<TagCount>,
    ) -> BichonResult<()> {
        let mut facet_collector = FacetCollector::for_field(F_TAGS);
        facet_collector.add_facet(parent_facet);

        let facet_counts = searcher
            .search(query, &facet_collector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        for (facet, count) in facet_counts.get(parent_facet) {
            all_facets.push(TagCount {
                tag: facet.to_string(),
                count,
            });
            Self::collect_facets_recursive(query, searcher, &facet.to_string(), all_facets)?;
        }

        Ok(())
    }

    pub async fn get_all_tags(
        &self,
        accounts: Option<HashSet<u64>>,
    ) -> BichonResult<Vec<TagCount>> {
        let searcher = self.reader.searcher();

        let query: Box<dyn Query> = match accounts {
            Some(ref ids) if !ids.is_empty() => {
                let mut subqueries = Vec::new();
                for &id in ids {
                    let term = Term::from_field_u64(SchemaTools::email_fields().f_account_id, id);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                Box::new(BooleanQuery::new(subqueries))
            }
            Some(_) => Box::new(EmptyQuery),
            None => Box::new(AllQuery),
        };

        let mut all_facets = Vec::new();
        Self::collect_facets_recursive(&query, &searcher, "/", &mut all_facets)?;
        Ok(all_facets)
    }

    pub async fn get_all_contacts(
        &self,
        accounts: Option<HashSet<u64>>,
    ) -> BichonResult<HashSet<String>> {
        let searcher = self.create_searcher()?;

        let query: Box<dyn Query> = match accounts {
            Some(ref ids) if !ids.is_empty() => {
                let mut subqueries = Vec::new();
                for &id in ids {
                    let term = Term::from_field_u64(SchemaTools::email_fields().f_account_id, id);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                Box::new(BooleanQuery::new(subqueries))
            }
            Some(_) => Box::new(EmptyQuery),
            None => Box::new(AllQuery),
        };

        let mut contacts_set: HashSet<String> = HashSet::new();

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(1_000_000).order_by_score())
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        for (_score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            let contacts = extract_contacts(&doc).await?;
            for value in contacts {
                contacts_set.insert(value);
            }
        }
        Ok(contacts_set)
    }

    pub async fn update_envelope_tags(&self, request: TagsRequest) -> BichonResult<()> {
        if request.updates.is_empty() {
            tracing::warn!("update_envelope_tags: request is empty, nothing to update");
            return Ok(());
        }
        let searcher = self.create_searcher()?;
        let mut writer = self.index_writer.lock().await;

        let f_tags = SchemaTools::email_fields().f_tags;
        let f_id = SchemaTools::email_fields().f_id;
        let deduplicated_updates: HashMap<u64, HashSet<String>> = request
            .updates
            .into_iter()
            .map(|(account_id, envelope_ids)| (account_id, envelope_ids.into_iter().collect()))
            .collect();

        let mut operations = Vec::new();

        for (account_id, envelope_ids) in &deduplicated_updates {
            for eid in envelope_ids {
                let query = self.envelope_query(*account_id, eid);
                let docs = searcher
                    .search(query.as_ref(), &TopDocs::with_limit(1).order_by_score())
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

                if let Some((_, doc_address)) = docs.first() {
                    let old_doc: TantivyDocument = searcher
                        .doc(*doc_address)
                        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

                    let mut current_tags: HashSet<String> = old_doc
                        .get_all(f_tags)
                        .filter_map(|val| val.as_facet())
                        .map(|facet| facet.to_string())
                        .collect();

                    match request.action {
                        TagAction::Add => {
                            for tag in &request.tags {
                                current_tags.insert(tag.clone());
                            }
                        }
                        TagAction::Remove => {
                            for tag in &request.tags {
                                current_tags.remove(tag);
                            }
                        }
                        TagAction::Overwrite => {
                            current_tags = request.tags.iter().cloned().collect();
                        }
                    }

                    let mut new_doc = TantivyDocument::new();

                    for (field, value) in old_doc.field_values() {
                        if field != f_tags {
                            new_doc.add_field_value(field, value);
                        }
                    }
                    for tag in current_tags {
                        new_doc.add_facet(f_tags, &tag);
                    }

                    let delete_term = Term::from_field_text(f_id, eid);
                    operations.push(UserOperation::Delete(delete_term));
                    operations.push(UserOperation::Add(new_doc));
                }
            }
        }

        writer
            .run(operations)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        // commit
        writer
            .commit()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        Ok(())
    }

    pub async fn search(
        &self,
        accounts: Option<HashSet<u64>>,
        filter: EmailSearchFilter,
        page: u64,
        page_size: u64,
        desc: bool,
        sort_by: SortBy,
    ) -> BichonResult<DataPage<Envelope>> {
        assert!(page > 0, "Page number must be greater than 0");
        assert!(page_size > 0, "Page size must be greater than 0");
        let query = self.filter_query(accounts, filter, self.query_parser.clone())?;
        let searcher = self.create_searcher()?;
        let total = searcher
            .search(&query, &Count)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
            as u64;

        if total == 0 {
            return Ok(DataPage {
                current_page: Some(page),
                page_size: Some(page_size),
                total_items: 0,
                items: vec![],
                total_pages: Some(0),
            });
        }
        let offset = (page - 1) * page_size;
        let total_pages = total.div_ceil(page_size);
        if offset > total {
            return Ok(DataPage {
                current_page: Some(page),
                page_size: Some(page_size),
                total_items: total,
                items: vec![],
                total_pages: Some(total_pages),
            });
        }

        let order = if desc { Order::Desc } else { Order::Asc };
        let mailbox_docs: Vec<DocAddress>;

        match sort_by {
            SortBy::DATE => {
                let date_docs: Vec<(Option<i64>, DocAddress)> = searcher
                    .search(
                        &query,
                        &TopDocs::with_limit(page_size as usize)
                            .and_offset(offset as usize)
                            .order_by_fast_field(F_DATE, order),
                    )
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
                mailbox_docs = date_docs.into_iter().map(|(_, addr)| addr).collect();
            }
            SortBy::SIZE => {
                let size_docs: Vec<(Option<u64>, DocAddress)> = searcher
                    .search(
                        &query,
                        &TopDocs::with_limit(page_size as usize)
                            .and_offset(offset as usize)
                            .order_by_fast_field(F_SIZE, order),
                    )
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
                mailbox_docs = size_docs.into_iter().map(|(_, addr)| addr).collect();
            }
        }

        let mut result = Vec::new();

        for doc_address in mailbox_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            let envelope = EnvelopeWithAttachments::from_tantivy_doc(&doc)?.envelope;
            result.push(envelope);
        }
        Ok(DataPage {
            current_page: Some(page),
            page_size: Some(page_size),
            total_items: total,
            items: result,
            total_pages: Some(total_pages),
        })
    }

    fn create_searcher(&self) -> BichonResult<Searcher> {
        self.reader
            .reload()
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Ok(self.reader.searcher())
    }

    pub async fn num_messages_in_thread(
        &self,
        searcher: &Searcher,
        account_id: u64,
        thread_id: &str,
    ) -> BichonResult<u64> {
        let query = self.thread_query(account_id, thread_id);

        let agg_req: Aggregations = serde_json::from_value(json!({
            "thread_count": {
                "value_count": {
                    "field": F_THREAD_ID
                }
            }
        }))
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        let collector = AggregationCollector::from_aggs(agg_req, Default::default());

        let agg_res = searcher
            .search(query.as_ref(), &collector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Self::extract_thread_count(&agg_res)
    }

    fn extract_thread_count(agg_res: &AggregationResults) -> BichonResult<u64> {
        let Some(result) = agg_res.0.get("thread_count") else {
            return Err(raise_error!(
                "Missing aggregation result: thread_count".into(),
                ErrorCode::InternalError
            ));
        };

        match result {
            AggregationResult::MetricResult(MetricResult::Count(count)) => {
                Ok(count.value.map(|v| v as u64).ok_or_else(|| {
                    raise_error!(
                        "Failed to get count value from aggregation result: value is None".into(),
                        ErrorCode::InternalError
                    )
                })?)
            }
            other => Err(raise_error!(
                format!("Unexpected aggregation result type: {other:?}"),
                ErrorCode::InternalError
            )),
        }
    }

    pub async fn list_thread_envelopes(
        &self,
        account_id: u64,
        thread_id: &str,
        page: u64,
        page_size: u64,
        desc: bool,
    ) -> BichonResult<DataPage<Envelope>> {
        assert!(page > 0, "Page number must be greater than 0");
        assert!(page_size > 0, "Page size must be greater than 0");
        let searcher = self.create_searcher()?;
        let total = self
            .num_messages_in_thread(&searcher, account_id, thread_id)
            .await?;
        if total == 0 {
            return Ok(DataPage {
                current_page: Some(page),
                page_size: Some(page_size),
                total_items: 0,
                items: vec![],
                total_pages: Some(0),
            });
        }
        let offset = (page - 1) * page_size;
        let total_pages = total.div_ceil(page_size);
        if offset > total {
            return Ok(DataPage {
                current_page: Some(page),
                page_size: Some(page_size),
                total_items: total,
                items: vec![],
                total_pages: Some(total_pages),
            });
        }

        let query = self.thread_query(account_id, thread_id);

        let order = if desc { Order::Desc } else { Order::Asc };
        let thread_docs: Vec<(Option<i64>, DocAddress)> = searcher
            .search(
                query.as_ref(),
                &TopDocs::with_limit(page_size as usize)
                    .and_offset(offset as usize)
                    .order_by_fast_field(F_DATE, order),
            )
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        let mut result = Vec::new();

        for (_, doc_address) in thread_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
            let envelope = EnvelopeWithAttachments::from_tantivy_doc(&doc)?.envelope;
            result.push(envelope);
        }
        Ok(DataPage {
            current_page: Some(page),
            page_size: Some(page_size),
            total_items: total,
            items: result,
            total_pages: Some(total_pages),
        })
    }

    pub async fn get_dashboard_stats(
        &self,
        accounts: &Option<HashSet<u64>>,
    ) -> BichonResult<DashboardStats> {
        let searcher = self.create_searcher()?;
        let now_ms = utc_now!();
        let week_ago_ms = (Utc::now() - Duration::from_secs(60 * 60 * 24 * 30)).timestamp_millis();

        let aggregations: Aggregations = serde_json::from_value(json!({
            "total_size": {
                "sum": { "field": F_SIZE }
            },
            "recent_30d_histogram": {
                "histogram": {
                    "field": F_DATE,
                    "interval": 86400000,
                    "hard_bounds": {
                        "min": week_ago_ms,
                        "max": now_ms
                    }
                }
            },
            "top_from_values": {
                "terms": {
                    "field": F_FROM,
                    "size": 10
                }
            },
            "top_account_values": {
                "terms": {
                    "field": F_ACCOUNT_ID,
                    "size": 10
                }
            },
            "attachment_stats": {
                "range": {
                    "field": F_REGULAR_ATTACHMENT_COUNT,
                    "ranges": [
                        { "to": 1, "key": "no_attachment" },
                        { "from": 1, "key": "has_attachment" }
                    ]
                }
            }
        }))
        .unwrap();

        let query: Box<dyn Query> = match accounts {
            Some(ref ids) if !ids.is_empty() => {
                let mut subqueries = Vec::new();
                for &id in ids {
                    let term = Term::from_field_u64(SchemaTools::email_fields().f_account_id, id);
                    subqueries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>,
                    ));
                }
                Box::new(BooleanQuery::new(subqueries))
            }
            Some(_) => Box::new(EmptyQuery),
            None => Box::new(AllQuery),
        };

        let agg_collector = AggregationCollector::from_aggs(aggregations, Default::default());
        let agg_results = searcher
            .search(&query, &agg_collector)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

        let mut stats = DashboardStats::default();
        let total_size = agg_results.0.get("total_size").ok_or_else(|| {
            raise_error!(
                "missing 'total_size' aggregation result".into(),
                ErrorCode::InternalError
            )
        })?;

        if let AggregationResult::MetricResult(MetricResult::Sum(v)) = total_size {
            let total_size = v.value.map(|v| v as u64).ok_or_else(|| {
                raise_error!(
                    "'total_size' sum metric has no value".into(),
                    ErrorCode::InternalError
                )
            })?;
            stats.total_size_bytes = total_size;
        }

        let recent_30d_histogram = agg_results.0.get("recent_30d_histogram").ok_or_else(|| {
            raise_error!(
                "missing 'recent_30d_histogram' aggregation result".into(),
                ErrorCode::InternalError
            )
        })?;

        let mut recent_activity = Vec::with_capacity(31);
        if let AggregationResult::BucketResult(BucketResult::Histogram { buckets, .. }) =
            recent_30d_histogram
        {
            if let BucketEntries::Vec(bucket_list) = buckets {
                for entry in bucket_list {
                    if let Key::F64(ms) = entry.key {
                        recent_activity.push(TimeBucket {
                            timestamp_ms: ms as i64,
                            count: entry.doc_count,
                        });
                    }
                }
            }
        }
        stats.recent_activity = recent_activity;
        let mut top_senders = Vec::with_capacity(11);
        let top_from_values = agg_results.0.get("top_from_values").unwrap();
        if let AggregationResult::BucketResult(BucketResult::Terms { buckets, .. }) =
            top_from_values
        {
            for entry in buckets {
                if let Key::Str(sender) = &entry.key {
                    top_senders.push(Group {
                        key: sender.clone(),
                        count: entry.doc_count,
                    });
                }
            }
        }
        stats.top_senders = top_senders;

        let mut top_accounts = Vec::with_capacity(11);
        let top_account_values = agg_results.0.get("top_account_values").unwrap();
        if let AggregationResult::BucketResult(BucketResult::Terms { buckets, .. }) =
            top_account_values
        {
            for entry in buckets {
                if let Key::U64(account_id) = &entry.key {
                    match AccountModel::get(*account_id) {
                        Ok(account) => {
                            top_accounts.push(Group {
                                key: account.email,
                                count: entry.doc_count,
                            });
                        }
                        Err(e) => {
                            warn!(
                                account_id = account_id,
                                error = %e,
                                "orphaned account index detected, scheduling cleanup"
                            );
                            let account_id = *account_id;
                            tokio::spawn(async move {
                                if let Err(e) =
                                    ENVELOPE_MANAGER.delete_account_envelopes(account_id).await
                                {
                                    tracing::error!(
                                        account_id = account_id,
                                        error = %e,
                                        "failed to cleanup envelope index"
                                    );
                                }
                            });
                        }
                    }
                }
            }
        }
        stats.top_accounts = top_accounts;

        let attachment_stats = agg_results.0.get("attachment_stats").unwrap();
        if let AggregationResult::BucketResult(BucketResult::Range { buckets, .. }) =
            attachment_stats
        {
            match buckets {
                BucketEntries::Vec(bucket_vec) => {
                    for entry in bucket_vec {
                        match entry.key.to_string().as_str() {
                            "no_attachment" => {
                                stats.without_attachment_count = entry.doc_count;
                            }
                            "has_attachment" => {
                                stats.with_attachment_count = entry.doc_count;
                            }
                            _ => {}
                        }
                    }
                }
                BucketEntries::HashMap(bucket_map) => {
                    if let Some(entry) = bucket_map.get("no_attachment") {
                        stats.without_attachment_count = entry.doc_count;
                    }
                    if let Some(entry) = bucket_map.get("has_attachment") {
                        stats.with_attachment_count = entry.doc_count;
                    }
                }
            }
        }
        Ok(stats)
    }
}
