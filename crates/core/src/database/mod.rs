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

use crate::common::paginated::Paginated;
use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::raise_error;
use bichon_memdb::{MemDb, Transaction};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod manager;

/// Trait for models that can be stored in MemDb collections.
pub trait MemDbModel: Serialize + DeserializeOwned + Clone + Send + 'static {
    /// The collection name this model is stored under.
    fn collection() -> &'static str;
    /// The primary key as a string for MemDb storage.
    fn key(&self) -> String;
}

// ─── Insert ───────────────────────────────────────────────────────────────

pub fn insert_impl<M: MemDbModel>(db: &MemDb, item: M) -> BichonResult<()> {
    let coll = db.collection(M::collection());
    let key = item.key();
    coll.insert(key, &item)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
}

pub fn batch_insert_impl<M: MemDbModel>(db: &MemDb, items: Vec<M>) -> BichonResult<()> {
    let txn = db.transaction();
    let mut txn = txn;
    for item in &items {
        txn = txn
            .insert(M::collection(), item.key(), item)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    }
    txn.commit()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
}

// ─── Upsert ────────────────────────────────────────────────────────────────

pub fn upsert_impl<M: MemDbModel>(db: &MemDb, item: M) -> BichonResult<()> {
    let coll = db.collection(M::collection());
    coll.upsert(item.key(), &item)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
}

pub fn batch_upsert_impl<M: MemDbModel>(db: &MemDb, items: Vec<M>) -> BichonResult<()> {
    let txn = db.transaction();
    let mut txn = txn;
    for item in &items {
        txn = txn
            .upsert(M::collection(), item.key(), item)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    }
    txn.commit()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
}

// ─── Find ──────────────────────────────────────────────────────────────────

pub fn find_impl<M: MemDbModel>(db: &MemDb, key: &str) -> BichonResult<Option<M>> {
    let coll = db.collection(M::collection());
    coll.get(key)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
}

// ─── Filter (replaces secondary key queries) ──────────────────────────────

pub fn filter_impl<M, F>(db: &MemDb, predicate: F) -> BichonResult<Vec<M>>
where
    M: MemDbModel,
    F: Fn(&M) -> bool + Send + 'static,
{
    let coll = db.collection(M::collection());
    coll.filter(predicate)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
}

// ─── Update (read-modify-write under a single spawn_blocking) ─────────────

pub fn update_impl<M: MemDbModel>(
    db: &MemDb,
    key: &str,
    update_fn: impl FnOnce(M) -> BichonResult<M> + Send + 'static,
) -> BichonResult<M> {
    let coll = db.collection(M::collection());
    let current: M = coll
        .get_required(key)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    let updated = update_fn(current)?;
    coll.upsert(key, &updated)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    Ok(updated)
}

// ─── Delete ────────────────────────────────────────────────────────────────

pub fn delete_impl<M: MemDbModel>(db: &MemDb, key: &str) -> BichonResult<()> {
    let coll = db.collection(M::collection());
    let existed = coll
        .delete(key)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    if !existed {
        return Err(raise_error!(
            format!("{} '{}' not found for deletion", M::collection(), key),
            ErrorCode::ResourceNotFound
        ));
    }
    Ok(())
}

pub fn batch_delete_impl<M: MemDbModel>(db: &MemDb, keys: Vec<String>) -> BichonResult<usize> {
    let txn = db.transaction();
    let mut txn = txn;
    let mut count = 0usize;
    for key in &keys {
        txn = txn.delete(M::collection(), key.clone());
        count += 1;
    }
    txn.commit()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    Ok(count)
}

// ─── List / Count ──────────────────────────────────────────────────────────

pub fn list_all_impl<M: MemDbModel>(db: &MemDb) -> BichonResult<Vec<M>> {
    let coll = db.collection(M::collection());
    coll.list_all()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
}

pub fn count_impl<M: MemDbModel>(db: &MemDb) -> BichonResult<usize> {
    let coll = db.collection(M::collection());
    Ok(coll.count())
}

// ─── Paginate ──────────────────────────────────────────────────────────────

pub fn paginate_impl<M: MemDbModel>(
    db: &MemDb,
    page: Option<u64>,
    page_size: Option<u64>,
    desc: Option<bool>,
) -> BichonResult<Paginated<M>> {
    let coll = db.collection(M::collection());
    let total_items = coll.count() as u64;

    let (offset, total_pages) = match (page, page_size) {
        (Some(p), Some(s)) if p > 0 && s > 0 => {
            let offset = (p - 1) * s;
            let total_pages = if total_items > 0 {
                (total_items as f64 / s as f64).ceil() as u64
            } else {
                0
            };
            (Some(offset), Some(total_pages))
        }
        (Some(0), _) | (_, Some(0)) => {
            return Err(raise_error!(
                "'page' and 'page_size' must be greater than 0.".into(),
                ErrorCode::InvalidParameter
            ));
        }
        _ => (None, None),
    };

    let all: Vec<M> = coll
        .list_all()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

    let items: Vec<M> = match desc {
        Some(true) => {
            let iter: Vec<M> = all.into_iter().rev().collect();
            let skip = offset.unwrap_or(0) as usize;
            let take = page_size.unwrap_or(total_items) as usize;
            iter.into_iter().skip(skip).take(take).collect()
        }
        _ => {
            let skip = offset.unwrap_or(0) as usize;
            let take = page_size.unwrap_or(total_items) as usize;
            all.into_iter().skip(skip).take(take).collect()
        }
    };

    Ok(Paginated::new(
        page,
        page_size,
        total_items,
        total_pages,
        items,
    ))
}

// ─── Transaction ───────────────────────────────────────────────────────────

/// Execute operations within a single atomic transaction (one WAL entry).
pub fn with_transaction(
    db: &MemDb,
    f: impl FnOnce(Transaction) -> BichonResult<Transaction> + Send + 'static,
) -> BichonResult<()> {
    let txn = db.transaction();
    let txn = f(txn)?;
    txn.commit()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
}
