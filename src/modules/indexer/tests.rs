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

use std::{path::PathBuf, time::Duration};

use chrono::Utc;
use serde_json::json;
use tantivy::{
    aggregation::{
        agg_req::Aggregations,
        agg_result::{AggregationResult, BucketEntries, BucketResult, MetricResult},
        AggregationCollector, Key,
    },
    collector::TopDocs,
    doc,
    indexer::UserOperation,
    query::{AllQuery, QueryParser, TermQuery},
    schema::{IndexRecordOption, Schema, Value},
    Index, IndexWriter, TantivyDocument, Term,
};

use crate::{
    modules::{
        dashboard::TimeBucket,
        indexer::{
            fields::{F_FROM, F_HAS_ATTACHMENT, F_INTERNAL_DATE, F_SIZE},
            schema::SchemaTools,
        },
    },
    utc_now,
};

#[tokio::test]
async fn test1() {
    let index = Index::open_in_dir(PathBuf::from("E:/bichon-data/envelope")).unwrap();
    let reader = index.reader().unwrap();
    let mut query_parser = QueryParser::for_index(&index, SchemaTools::envelope_default_fields());
    query_parser.set_conjunction_by_default();
    let searcher = reader.searcher();

    let now_ms = utc_now!();
    let week_ago_ms = (Utc::now() - Duration::from_secs(60 * 60 * 24 * 7)).timestamp_millis();
    let aggregations: Aggregations = serde_json::from_value(json!({
        "total_size": {
            "sum": { "field": F_SIZE }
        },
        "recent_7d_histogram": {
            "histogram": {
                "field": F_INTERNAL_DATE,
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
        "attachment_stats": {
            "terms": {
                "field": F_HAS_ATTACHMENT
            }
        }
    }))
    .unwrap();

    let query = AllQuery;
    let agg_collector = AggregationCollector::from_aggs(aggregations, Default::default());
    let agg_results = searcher.search(&query, &agg_collector).unwrap();

    let total_size = agg_results.0.get("total_size").unwrap();

    if let AggregationResult::MetricResult(MetricResult::Sum(count)) = total_size {
        let total_size = count.value.map(|v| v as u64).unwrap();
        println!("{:#?}", total_size);
    }

    let recent_7d_histogram = agg_results.0.get("recent_7d_histogram").unwrap();

    let mut recent_activity = Vec::with_capacity(15);
    if let AggregationResult::BucketResult(BucketResult::Histogram { buckets, .. }) =
        recent_7d_histogram
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
    println!("recent_activity: {:#?}", recent_activity);

    let top_from_values = agg_results.0.get("top_from_values").unwrap();

    if let AggregationResult::BucketResult(BucketResult::Terms { buckets, .. }) = top_from_values {
        println!("{:#?}", buckets);
    }

    let attachment_stats = agg_results.0.get("attachment_stats").unwrap();

    if let AggregationResult::BucketResult(BucketResult::Terms { buckets, .. }) = attachment_stats {
        println!("{:#?}", buckets);
    }

    //agg_results.
}

#[tokio::test]
async fn test2() {
    use tantivy::schema::{FAST, INDEXED, STORED, STRING};
    let mut builder = Schema::builder();
    let a = builder.add_u64_field("a", INDEXED | FAST);
    let b = builder.add_text_field("b", STRING | STORED | FAST);

    let schema = builder.build();
    let index = Index::create_in_ram(schema);
    let mut index_writer: IndexWriter = index.writer(50_000_000).unwrap();

    let delete_term1 = Term::from_field_u64(a, 1u64);
    let delete_term2 = Term::from_field_u64(a, 2u64);
    let delete_term3 = Term::from_field_u64(a, 3u64);

    let operations = vec![
        UserOperation::Delete(delete_term1),
        UserOperation::Add(doc!(
        a => 1u64,
        b => "v1"
        )),
        UserOperation::Delete(delete_term2),
        UserOperation::Add(doc!(
        a => 2u64,
        b => "v1"
        )),
        UserOperation::Delete(delete_term3),
        UserOperation::Add(doc!(
        a => 3u64,
        b => "v1"
        )),
    ];

    index_writer.run(operations).unwrap();
    index_writer.commit().unwrap();

    let reader = index.reader().unwrap();
    let searcher = reader.searcher();
    let tq = TermQuery::new(Term::from_field_u64(a, 3), IndexRecordOption::Basic);
    let docs = searcher.search(&tq, &TopDocs::with_limit(1)).unwrap();
    assert!(docs.first().is_some());
    if let Some((_, doc_address)) = docs.first() {
        let old_doc: TantivyDocument = searcher.doc_async(*doc_address).await.unwrap();

        let mut new_doc = TantivyDocument::new();
        for (field, value) in old_doc.field_values() {
            if field == a {
                new_doc.add_field_value(a, value);
            }
            if field == b {
                assert_eq!(Some("v1"), value.as_str())
            }
        }
        new_doc.add_text(b, "v2");

        let delete_term = Term::from_field_u64(a, 3);
        index_writer.delete_term(delete_term);
        index_writer.add_document(new_doc).unwrap();
        index_writer.commit().unwrap();
    }

    reader.reload().unwrap();
    let searcher = reader.searcher();
    let docs = searcher.search(&tq, &TopDocs::with_limit(1)).unwrap();
    assert!(docs.first().is_some());
    if let Some((_, doc_address)) = docs.first() {
        let doc: TantivyDocument = searcher.doc_async(*doc_address).await.unwrap();
        for (field, value) in doc.field_values() {
            if field == b {
                assert_eq!(Some("v2"), value.as_str())
            }
        }
    }

    let delete_term = Term::from_field_u64(a, 3);
    index_writer.delete_term(delete_term);
    index_writer.commit().unwrap();

    reader.reload().unwrap();
    let searcher = reader.searcher();
    let docs = searcher.search(&tq, &TopDocs::with_limit(1)).unwrap();
    assert!(docs.first().is_none());
    if let Some((_, doc_address)) = docs.first() {
        let doc: TantivyDocument = searcher.doc_async(*doc_address).await.unwrap();
        for (field, value) in doc.field_values() {
            if field == b {
                let value = value.as_str();
                println!("{:#?}", value);
            }
        }
    }
}
