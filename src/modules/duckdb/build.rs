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

use arrow::array::{Int32Array, Int64Array, ListBuilder, StringBuilder, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

use crate::modules::indexer::envelope::Envelope;

pub const DEFAULT_SHARD_ID: u64 = 0;

pub fn build_record_batch(items: &[Envelope]) -> RecordBatch {
    let capacity = items.len();

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("account_id", DataType::UInt64, false),
        Field::new("mailbox_id", DataType::UInt64, false),
        Field::new("uid", DataType::UInt64, false),
        Field::new("content_hash", DataType::Utf8, false),
        Field::new("subject", DataType::Utf8, true),
        Field::new("body", DataType::Utf8, true),
        Field::new("sender", DataType::Utf8, true),
        Field::new(
            "recipients",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        ),
        Field::new(
            "cc",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        ),
        Field::new(
            "bcc",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        ),
        Field::new("sent_at", DataType::Int64, true),
        Field::new("received_at", DataType::Int64, true),
        Field::new("size_bytes", DataType::UInt64, true),
        Field::new("thread_id", DataType::Utf8, true),
        Field::new("message_id", DataType::Utf8, true),
        Field::new("attachment_count", DataType::Int32, false),
        Field::new("regular_attachment_count", DataType::Int32, false),
        Field::new(
            "tags",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        ),
        Field::new("shard_id", DataType::UInt64, false),
    ]));

    let mut id_b = StringBuilder::with_capacity(capacity, capacity * 20);
    let mut account_id_b = UInt64Array::builder(capacity);
    let mut mailbox_id_b = UInt64Array::builder(capacity);
    let mut uid_b = UInt64Array::builder(capacity);

    let mut content_hash_b = StringBuilder::with_capacity(capacity, capacity * 64);
    let mut subject_b = StringBuilder::with_capacity(capacity, capacity * 20);
    let mut body_b = StringBuilder::with_capacity(capacity, capacity * 100);
    let mut from_b = StringBuilder::with_capacity(capacity, capacity * 20);

    let mut to_b = ListBuilder::new(StringBuilder::new());
    let mut cc_b = ListBuilder::new(StringBuilder::new());
    let mut bcc_b = ListBuilder::new(StringBuilder::new());

    let mut date_b = Int64Array::builder(capacity);
    let mut internal_date_b = Int64Array::builder(capacity);
    let mut size_b = UInt64Array::builder(capacity);
    let mut thread_id_b = StringBuilder::with_capacity(capacity, capacity * 20);
    let mut msg_id_b = StringBuilder::with_capacity(capacity, capacity * 30);
    let mut att_count_b = Int32Array::builder(capacity);
    let mut regular_att_count_b = Int32Array::builder(capacity);
    let mut tags_b = ListBuilder::new(StringBuilder::new());
    let mut shard_id_b = UInt64Array::builder(capacity);

    for e in items {
        id_b.append_value(&e.id);
        account_id_b.append_value(e.account_id);
        mailbox_id_b.append_value(e.mailbox_id);
        uid_b.append_value(e.uid as u64);
        content_hash_b.append_value(&e.content_hash);
        subject_b.append_value(&e.subject);
        body_b.append_value(&e.text);
        from_b.append_value(&e.from);
        for addr in &e.to {
            to_b.values().append_value(addr);
        }
        to_b.append(true);
        for addr in &e.cc {
            cc_b.values().append_value(addr);
        }
        cc_b.append(true);
        for addr in &e.bcc {
            bcc_b.values().append_value(addr);
        }
        bcc_b.append(true);
        date_b.append_value(e.date);
        internal_date_b.append_value(e.internal_date);
        size_b.append_value(e.size as u64);
        thread_id_b.append_value(&e.thread_id);
        msg_id_b.append_value(&e.message_id);
        att_count_b.append_value(e.attachment_count as i32);
        regular_att_count_b.append_value(e.regular_attachment_count as i32);
        tags_b.append(true);
        shard_id_b.append_value(DEFAULT_SHARD_ID);
    }

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id_b.finish()),
            Arc::new(account_id_b.finish()),
            Arc::new(mailbox_id_b.finish()),
            Arc::new(uid_b.finish()),
            Arc::new(content_hash_b.finish()),
            Arc::new(subject_b.finish()),
            Arc::new(body_b.finish()),
            Arc::new(from_b.finish()),
            Arc::new(to_b.finish()),
            Arc::new(cc_b.finish()),
            Arc::new(bcc_b.finish()),
            Arc::new(date_b.finish()),
            Arc::new(internal_date_b.finish()),
            Arc::new(size_b.finish()),
            Arc::new(thread_id_b.finish()),
            Arc::new(msg_id_b.finish()),
            Arc::new(att_count_b.finish()),
            Arc::new(regular_att_count_b.finish()),
            Arc::new(tags_b.finish()),
            Arc::new(shard_id_b.finish()),
        ],
    )
    .expect("Failed to build RecordBatch")
}
