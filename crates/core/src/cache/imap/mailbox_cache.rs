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

use crate::cache::imap::mailbox::MailBox;
use crate::utc_now;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::LazyLock;
use tokio::sync::Mutex;

struct CacheEntry {
    mailboxes: Vec<MailBox>,
    fetched_at: i64,
}

static CACHE: LazyLock<Mutex<LruCache<u64, CacheEntry>>> = LazyLock::new(|| {
    Mutex::new(LruCache::new(NonZeroUsize::new(64).unwrap()))
});

const TTL_MS: i64 = 10 * 60 * 1000; // 10 minutes

pub async fn get(account_id: u64) -> Option<Vec<MailBox>> {
    let mut guard = CACHE.lock().await;
    if let Some(entry) = guard.get(&account_id) {
        if utc_now!() - entry.fetched_at < TTL_MS {
            return Some(entry.mailboxes.clone());
        }
        guard.pop(&account_id);
    }
    None
}

pub async fn set(account_id: u64, mailboxes: Vec<MailBox>) {
    let mut guard = CACHE.lock().await;
    guard.put(
        account_id,
        CacheEntry {
            mailboxes,
            fetched_at: utc_now!(),
        },
    );
}

pub async fn invalidate(account_id: u64) {
    let mut guard = CACHE.lock().await;
    guard.pop(&account_id);
}

// Background fetch state tracking
#[derive(Clone, Debug)]
pub enum FetchStatus {
    Fetching { examined: usize, total: usize },
    Ready,
    Error(String),
}

static FETCH_STATES: LazyLock<Mutex<HashMap<u64, FetchStatus>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn fetch_status(account_id: u64) -> Option<FetchStatus> {
    FETCH_STATES.lock().await.get(&account_id).cloned()
}

pub async fn set_fetching(account_id: u64) {
    FETCH_STATES.lock().await.insert(
        account_id,
        FetchStatus::Fetching {
            examined: 0,
            total: 0,
        },
    );
}

pub async fn update_fetch_progress(account_id: u64, examined: usize, total: usize) {
    let mut guard = FETCH_STATES.lock().await;
    guard.insert(
        account_id,
        FetchStatus::Fetching { examined, total },
    );
}

pub async fn set_fetch_ready(account_id: u64) {
    FETCH_STATES
        .lock()
        .await
        .insert(account_id, FetchStatus::Ready);
}

pub async fn set_fetch_error(account_id: u64, error: String) {
    FETCH_STATES
        .lock()
        .await
        .insert(account_id, FetchStatus::Error(error));
}

pub async fn clear_fetch_state(account_id: u64) {
    FETCH_STATES.lock().await.remove(&account_id);
}
