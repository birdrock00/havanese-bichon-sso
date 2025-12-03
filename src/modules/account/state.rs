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


use crate::{
    modules::{
        database::{async_find_impl, delete_impl, manager::DB_MANAGER, update_impl, upsert_impl},
        error::{code::ErrorCode, BichonResult},
    },
    raise_error, utc_now,
};
use native_db::*;
use native_model::{native_model, Model};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const ERROR_COUNT_PER_ACCOUNT: usize = 30;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, Object)]
pub struct MailboxBatchProgress {
    pub total_batches: u32,
    pub current_batch: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, Object)]
#[native_model(id = 2, version = 1)]
#[native_db]
pub struct AccountRunningState {
    #[primary_key]
    pub account_id: u64,
    pub last_incremental_sync_start: i64,
    pub last_incremental_sync_end: Option<i64>,
    pub errors: Vec<AccountError>,
    pub is_initial_sync_completed: bool,
    pub progress: Option<BTreeMap<String, MailboxBatchProgress>>,
    pub initial_sync_start_time: Option<i64>,
    pub initial_sync_end_time: Option<i64>,
    pub initial_sync_failed_time: Option<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, Object)]
pub struct AccountError {
    pub error: String,
    pub at: i64,
}

impl AccountRunningState {
    pub async fn add(account_id: u64) -> BichonResult<()> {
        let info = AccountRunningState {
            account_id,
            last_incremental_sync_start: 0,
            last_incremental_sync_end: None,
            errors: vec![],
            is_initial_sync_completed: false,
            progress: None,
            initial_sync_start_time: Some(utc_now!()),
            initial_sync_end_time: None,
            initial_sync_failed_time: None,
        };
        upsert_impl(DB_MANAGER.envelope_db(), info).await
    }

    pub async fn get(account_id: u64) -> BichonResult<Option<AccountRunningState>> {
        async_find_impl(DB_MANAGER.envelope_db(), account_id).await
    }

    async fn update_account_running_state(
        account_id: u64,
        updater: impl FnOnce(&AccountRunningState) -> BichonResult<AccountRunningState> + Send + 'static,
    ) -> BichonResult<()> {
        update_impl(
            DB_MANAGER.envelope_db(),
            move |rw| {
                rw.get()
                    .primary::<AccountRunningState>(account_id)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
                    .ok_or_else(|| {
                        raise_error!(
                            format!("Cannot find sync info of account={}", account_id),
                            ErrorCode::ResourceNotFound
                        )
                    })
            },
            updater,
        )
        .await?;
        Ok(())
    }

    pub async fn delete(account_id: u64) -> BichonResult<()> {
        if Self::get(account_id).await?.is_none() {
            return Ok(());
        }

        delete_impl(DB_MANAGER.envelope_db(), move |rw| {
            rw.get()
                .primary::<AccountRunningState>(account_id)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
                .ok_or_else(|| {
                    raise_error!(
                        format!(
                            "AccountRunningState '{}' not found during deletion process.",
                            account_id
                        ),
                        ErrorCode::ResourceNotFound
                    )
                })
        })
        .await
    }

    // pub async fn set_initial_sync_start(account_id: u64) -> BichonResult<()> {
    //     Self::update_account_running_state(account_id, move |current| {
    //         let mut updated = current.clone();
    //         updated.initial_sync_start_time = Some(utc_now!());
    //         Ok(updated)
    //     })
    //     .await
    // }

    pub async fn set_initial_sync_completed(account_id: u64) -> BichonResult<()> {
        Self::update_account_running_state(account_id, move |current| {
            let mut updated = current.clone();
            updated.is_initial_sync_completed = true;
            updated.initial_sync_end_time = Some(utc_now!());
            Ok(updated)
        })
        .await
    }

    pub async fn set_initial_sync_failed(account_id: u64) -> BichonResult<()> {
        Self::update_account_running_state(account_id, move |current| {
            let mut updated = current.clone();
            updated.initial_sync_failed_time = Some(utc_now!());
            Ok(updated)
        })
        .await
    }

    pub async fn set_current_sync_batch_number(
        account_id: u64,
        syncing_folder: String,
        batch_number: u32,
    ) -> BichonResult<()> {
        Self::update_account_running_state(account_id, move |current| {
            let mut updated = current.clone();
            let mut progress_map = updated.progress.clone().unwrap_or_default();
            let entry =
                progress_map
                    .entry(syncing_folder.to_string())
                    .or_insert(MailboxBatchProgress {
                        total_batches: 0,
                        current_batch: 0,
                    });
            entry.current_batch = batch_number;
            updated.progress = Some(progress_map);
            Ok(updated)
        })
        .await
    }

    pub async fn set_folder_initial_sync_completed(
        account_id: u64,
        syncing_folder: String,
    ) -> BichonResult<()> {
        Self::update_account_running_state(account_id, move |current| {
            let mut updated = current.clone();
            let mut progress_map = updated.progress.clone().unwrap_or_default();
            let entry =
                progress_map
                    .entry(syncing_folder.to_string())
                    .or_insert(MailboxBatchProgress {
                        total_batches: 0,
                        current_batch: 0,
                    });
            entry.current_batch = entry.total_batches;
            updated.progress = Some(progress_map);
            Ok(updated)
        })
        .await
    }

    pub async fn set_initial_current_syncing_folder(
        account_id: u64,
        current_syncing_folder: String,
        total_sync_batches: u32,
    ) -> BichonResult<()> {
        Self::update_account_running_state(account_id, move |current| {
            let mut updated = current.clone();
            let mut progress_map = updated.progress.clone().unwrap_or_default();
            progress_map.insert(
                current_syncing_folder.clone(),
                MailboxBatchProgress {
                    total_batches: total_sync_batches,
                    current_batch: 0,
                },
            );
            updated.progress = Some(progress_map);
            Ok(updated)
        })
        .await
    }

    pub async fn set_incremental_sync_start(account_id: u64) -> BichonResult<()> {
        Self::update_account_running_state(account_id, move |current| {
            let mut updated = current.clone();
            updated.last_incremental_sync_start = utc_now!();
            updated.last_incremental_sync_end = None;
            Ok(updated)
        })
        .await
    }

    pub async fn set_incremental_sync_end(account_id: u64) -> BichonResult<()> {
        Self::update_account_running_state(account_id, move |current| {
            let mut updated = current.clone();
            updated.last_incremental_sync_end = Some(utc_now!());
            Ok(updated)
        })
        .await
    }

    pub async fn append_error_message(account_id: u64, error: String) -> BichonResult<()> {
        Self::update_account_running_state(account_id, move |current| {
            let mut updated = current.clone();
            updated.append_error_log(error);
            Ok(updated)
        })
        .await
    }

    pub fn append_error_log(&mut self, error: String) {
        let new_error = AccountError {
            error,
            at: utc_now!(),
        };

        self.errors.push(new_error);
        if self.errors.len() > ERROR_COUNT_PER_ACCOUNT {
            self.errors.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_single_error() {
        let mut account_state = AccountRunningState {
            account_id: 1000u64,
            last_incremental_sync_start: 1000,
            last_incremental_sync_end: Some(2000),
            errors: Vec::new(),
            ..Default::default()
        };

        account_state.append_error_log(String::from("Error 1"));
        assert_eq!(account_state.errors.len(), 1);
        assert_eq!(account_state.errors[0].error, "Error 1");
    }

    #[test]
    fn test_insert_multiple_errors() {
        let mut account_state = AccountRunningState {
            account_id: 1000u64,
            last_incremental_sync_start: 1000,
            last_incremental_sync_end: Some(2000),
            errors: Vec::new(),
            ..Default::default()
        };

        for i in 1..=5 {
            account_state.append_error_log(format!("Error {}", i));
        }

        assert_eq!(account_state.errors.len(), 5);
        assert_eq!(account_state.errors[4].error, "Error 5");
    }

    #[test]
    fn test_error_limit_exceeded() {
        let mut account_state = AccountRunningState {
            account_id: 1000u64,
            last_incremental_sync_start: 1000,
            last_incremental_sync_end: Some(2000),
            errors: Vec::new(),
            ..Default::default()
        };

        for i in 1..=25 {
            account_state.append_error_log(format!("Error {}", i));
        }

        // Should only keep the last 10 errors
        assert_eq!(account_state.errors.len(), ERROR_COUNT_PER_ACCOUNT);
        assert_eq!(account_state.errors[0].error, "Error 6");
        assert_eq!(account_state.errors[19].error, "Error 25");
    }

    #[test]
    fn test_insert_error_after_limit() {
        let mut account_state = AccountRunningState {
            account_id: 1000u64,
            last_incremental_sync_start: 1000,
            last_incremental_sync_end: Some(2000),
            errors: Vec::new(),
            ..Default::default()
        };

        // Insert exactly 10 errors
        for i in 1..=20 {
            account_state.append_error_log(format!("Error {}", i));
        }

        // Insert one more error to exceed the limit
        account_state.append_error_log(String::from("Error 21"));

        assert_eq!(account_state.errors.len(), ERROR_COUNT_PER_ACCOUNT);
        assert_eq!(account_state.errors[0].error, "Error 2"); // The first error is removed
        assert_eq!(account_state.errors[19].error, "Error 21"); // The last inserted error
    }
}
