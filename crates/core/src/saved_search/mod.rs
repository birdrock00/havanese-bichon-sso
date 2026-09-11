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

use crate::database::manager::DB_MANAGER;
use crate::database::{
    MemDbModel, batch_delete_impl, delete_impl, filter_impl, find_impl, insert_impl, update_impl,
};
use crate::error::code::ErrorCode;
use crate::raise_error;
use crate::{error::BichonResult, utc_now};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Maximum number of saved searches a single user may keep. Prevents a user
/// from unboundedly growing the shared collection.
pub const MAX_SAVED_SEARCHES_PER_USER: usize = 50;
/// Upper bound on a saved search display name.
pub const MAX_SAVED_SEARCH_NAME_LEN: usize = 100;

/// Which search surface a saved search belongs to. Only `Email` saved
/// searches can drive exports; `Attachment` ones are reused from the
/// attachment search page.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum SavedSearchKind {
    Email,
    Attachment,
}

/// A named, reusable search condition owned by one user.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct SavedSearchModel {
    pub id: String,
    pub user_id: u64,
    pub name: String,
    pub kind: SavedSearchKind,
    /// The serialized search filter (`EmailSearchFilter` / `AttachmentSearchFilter`).
    pub filter: Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl MemDbModel for SavedSearchModel {
    fn collection() -> &'static str {
        "saved_searches"
    }
    fn key(&self) -> String {
        self.id.clone()
    }
}

/// Request body for creating a saved search.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct SavedSearchCreateRequest {
    pub name: String,
    pub kind: SavedSearchKind,
    /// The serialized search filter (`EmailSearchFilter` / `AttachmentSearchFilter`).
    pub filter: Value,
}

/// Request body for renaming a saved search. Conditions are immutable.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct SavedSearchRenameRequest {
    pub name: String,
}

impl SavedSearchModel {
    /// Create a saved search for `user_id`. Conditions are immutable; only the
    /// name can change afterwards.
    pub fn create(
        user_id: u64,
        name: String,
        kind: SavedSearchKind,
        filter: Value,
    ) -> BichonResult<SavedSearchModel> {
        let name = Self::validate_name(name)?;

        let existing = Self::list_for_user(user_id)?;
        if existing.len() >= MAX_SAVED_SEARCHES_PER_USER {
            return Err(raise_error!(
                format!(
                    "Saved search limit reached: at most {MAX_SAVED_SEARCHES_PER_USER} saved searches per user."
                ),
                ErrorCode::TooManyRequest
            ));
        }
        if Self::has_duplicate_name(&existing, None, &kind, &name) {
            return Err(raise_error!(
                format!("A saved search named '{name}' already exists."),
                ErrorCode::AlreadyExists
            ));
        }

        let now = utc_now!();
        let model = SavedSearchModel {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            name,
            kind,
            filter,
            created_at: now,
            updated_at: now,
        };
        insert_impl(DB_MANAGER.db(), model.clone())?;
        Ok(model)
    }

    /// List the current user's saved searches, newest first.
    pub fn list_for_user(user_id: u64) -> BichonResult<Vec<SavedSearchModel>> {
        let uid = user_id;
        let mut items =
            filter_impl::<SavedSearchModel, _>(DB_MANAGER.db(), move |s| s.user_id == uid)?;
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(items)
    }

    /// Fetch a saved search and verify the caller owns it.
    pub fn get_owned(user_id: u64, id: &str) -> BichonResult<SavedSearchModel> {
        let found = find_impl::<SavedSearchModel>(DB_MANAGER.db(), id)?.ok_or_else(|| {
            raise_error!(
                format!("Saved search '{id}' not found."),
                ErrorCode::ResourceNotFound
            )
        })?;
        if found.user_id != user_id {
            return Err(raise_error!(
                "Permission denied: this saved search belongs to another user.".into(),
                ErrorCode::PermissionDenied
            ));
        }
        Ok(found)
    }

    /// Rename a saved search. The condition itself is immutable.
    pub fn rename(user_id: u64, id: &str, new_name: String) -> BichonResult<SavedSearchModel> {
        let new_name = Self::validate_name(new_name)?;
        let current = Self::get_owned(user_id, id)?;

        let existing = Self::list_for_user(user_id)?;
        if Self::has_duplicate_name(&existing, Some(id), &current.kind, &new_name) {
            return Err(raise_error!(
                format!("A saved search named '{new_name}' already exists."),
                ErrorCode::AlreadyExists
            ));
        }

        let id_owned = id.to_string();
        update_impl::<SavedSearchModel>(DB_MANAGER.db(), &id_owned, move |mut current| {
            current.name = new_name.clone();
            current.updated_at = utc_now!();
            Ok(current)
        })
    }

    /// Delete a saved search, enforcing ownership.
    pub fn delete(user_id: u64, id: &str) -> BichonResult<()> {
        Self::get_owned(user_id, id)?;
        delete_impl::<SavedSearchModel>(DB_MANAGER.db(), id)
    }

    /// Remove every saved search owned by `user_id`. Called when the user is
    /// deleted so orphaned records never accumulate in the collection.
    pub fn delete_all_for_user(user_id: u64) -> BichonResult<usize> {
        let uid = user_id;
        let items = filter_impl::<SavedSearchModel, _>(DB_MANAGER.db(), move |s| s.user_id == uid)?;
        let keys: Vec<String> = items.into_iter().map(|s| s.id).collect();
        if keys.is_empty() {
            return Ok(0);
        }
        batch_delete_impl::<SavedSearchModel>(DB_MANAGER.db(), keys)
    }

    fn validate_name(name: String) -> BichonResult<String> {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() || trimmed.chars().count() > MAX_SAVED_SEARCH_NAME_LEN {
            return Err(raise_error!(
                format!(
                    "Saved search name must be 1-{MAX_SAVED_SEARCH_NAME_LEN} characters."
                ),
                ErrorCode::InvalidParameter
            ));
        }
        Ok(trimmed)
    }

    /// Case-insensitive duplicate name check within the same kind, optionally
    /// excluding one record (itself during rename).
    fn has_duplicate_name(
        existing: &[SavedSearchModel],
        exclude_id: Option<&str>,
        kind: &SavedSearchKind,
        name: &str,
    ) -> bool {
        existing.iter().any(|s| {
            exclude_id.map_or(true, |id| s.id != id)
                && s.kind == *kind
                && s.name.eq_ignore_ascii_case(name)
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, kind: SavedSearchKind, name: &str) -> SavedSearchModel {
        SavedSearchModel {
            id: id.to_string(),
            user_id: 1,
            name: name.to_string(),
            kind,
            filter: Value::Null,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn validate_name_trims_and_rejects_blank() {
        assert!(SavedSearchModel::validate_name("  ".to_string()).is_err());
        assert!(SavedSearchModel::validate_name(String::new()).is_err());
        assert_eq!(
            SavedSearchModel::validate_name("  Quarterly Tax  ".to_string()).unwrap(),
            "Quarterly Tax"
        );
    }

    #[test]
    fn validate_name_rejects_too_long() {
        let long = "x".repeat(MAX_SAVED_SEARCH_NAME_LEN + 1);
        assert!(SavedSearchModel::validate_name(long).is_err());
        let ok = "x".repeat(MAX_SAVED_SEARCH_NAME_LEN);
        assert!(SavedSearchModel::validate_name(ok).is_ok());
    }

    #[test]
    fn duplicate_name_is_case_insensitive_within_kind() {
        let existing = vec![
            sample("1", SavedSearchKind::Email, "Tax"),
            sample("2", SavedSearchKind::Attachment, "Tax"),
        ];
        // Same name in Email kind -> duplicate (case-insensitive).
        assert!(SavedSearchModel::has_duplicate_name(
            &existing,
            None,
            &SavedSearchKind::Email,
            "tax"
        ));
        // Same name in a different kind -> not a duplicate.
        assert!(!SavedSearchModel::has_duplicate_name(
            &existing,
            None,
            &SavedSearchKind::Attachment,
            "tax"
        ));
        // Renaming a record to its own name -> allowed (excluded).
        assert!(!SavedSearchModel::has_duplicate_name(
            &existing,
            Some("1"),
            &SavedSearchKind::Email,
            "Tax"
        ));
    }
}