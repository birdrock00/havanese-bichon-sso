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
    collections::{BTreeSet, HashSet},
    sync::LazyLock,
};

use crate::{
    modules::{
        error::{code::ErrorCode, BichonResult},
        users::role::RoleType,
    },
    raise_error,
};

pub static VALID_PERMISSION_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    Permission::all_permissions()
        .into_iter()
        .map(|(key, _)| key)
        .collect()
});

pub struct Permission;

impl Permission {
    // ----------------------------------------------------------------------
    // 1. Global Management Permissions (System, Users, Tokens)
    // ----------------------------------------------------------------------

    /// Basic platform access. Required for any user to log in and access the dashboard.
    /// This provides no administrative powers.
    pub const SYSTEM_ACCESS: &str = "system:access";

    /// Manage core system configurations (OAuth Client ID/Secret, Proxy settings).
    pub const ROOT: &str = "system:root";

    /// Create, modify, and delete all users and their roles (Admin only).
    pub const USER_MANAGE: &str = "user:manage";

    /// View the minimal user list, basic user profiles,
    /// including visibility into account-level roles (Managers and Admins).
    pub const USER_VIEW: &str = "user:view";

    /// View and revoke all access tokens in the system.
    pub const TOKEN_MANAGE: &str = "token:manage";

    /// Create new email account connections.
    pub const ACCOUNT_CREATE: &str = "account:create";

    // ----------------------------------------------------------------------
    // 2. Global "ALL" Scoped Permissions (Reserved for Admin)
    // ----------------------------------------------------------------------

    /// Manage configuration for all accounts (Global control).
    pub const ACCOUNT_MANAGE_ALL: &str = "account:manage:all";

    /// Read mail data from all accounts (Search, view messages).
    pub const DATA_READ_ALL: &str = "data:read:all";

    /// Download raw EML/MIME files from all accounts.
    pub const DATA_RAW_DOWNLOAD_ALL: &str = "data:raw:download:all";

    /// Delete messages from all accounts.
    pub const DATA_DELETE_ALL: &str = "data:delete:all";

    /// Manage metadata (e.g., tags, categories, notes) for messages in ALL email accounts.
    pub const DATA_MANAGE_ALL: &str = "data:manage:all";

    /// Export messages in batches from all accounts.
    pub const DATA_EXPORT_BATCH_ALL: &str = "data:export:batch:all";

    // ----------------------------------------------------------------------
    // 3. Scoped/Limited Permissions (Manager & Viewer)
    //    Authorization requires checking the user's Account Access List (ACL)
    // ----------------------------------------------------------------------

    /// Manage (modify/delete/sync) configuration for a specific set of accounts.
    pub const ACCOUNT_MANAGE: &str = "account:manage";

    /// Read details and sync status for a specific set of accounts.
    pub const ACCOUNT_READ_DETAILS: &str = "account:read_details";

    /// Manage mail data metadata (e.g., updating tags, adding notes)
    /// for specific accounts.
    pub const DATA_MANAGE: &str = "data:manage";

    /// Read mail data (Search, view) from a specific set of accounts.
    pub const DATA_READ: &str = "data:read";

    /// Download raw EML/MIME files from a specific set of accounts.
    pub const DATA_RAW_DOWNLOAD: &str = "data:raw:download";

    /// Delete messages from a specific set of accounts.
    pub const DATA_DELETE: &str = "data:delete";

    /// Export messages in batches from a specific set of accounts.
    pub const DATA_EXPORT_BATCH: &str = "data:export:batch";

    /// Import EML/PST data into a SPECIFIC account.
    /// Authorization requires checking access to the target account_id.
    pub const DATA_IMPORT_BATCH: &str = "data:import:batch";

    pub fn global_permissions() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                Self::SYSTEM_ACCESS,
                "Basic platform access for dashboard and personal settings.",
            ),
            (Self::ROOT, "Full system access and configuration."),
            (Self::USER_MANAGE, "Create, update, and delete users."),
            (
                Self::USER_VIEW,
                "Read-only access to user list and profiles.",
            ),
            (Self::TOKEN_MANAGE, "View and revoke all active API tokens."),
            (
                Self::ACCOUNT_CREATE,
                "Connect new email accounts to the system.",
            ),
            (
                Self::ACCOUNT_MANAGE_ALL,
                "Manage configurations for all email accounts.",
            ),
            (
                Self::DATA_READ_ALL,
                "Search and read messages across all accounts.",
            ),
            (
                Self::DATA_MANAGE_ALL,
                "Manage metadata and tags for all accounts.",
            ),
            (
                Self::DATA_RAW_DOWNLOAD_ALL,
                "Download raw EML data from any account.",
            ),
            (
                Self::DATA_DELETE_ALL,
                "Permanently delete messages from any account.",
            ),
            (
                Self::DATA_EXPORT_BATCH_ALL,
                "Export bulk message data from all accounts.",
            ),
        ]
    }

    pub fn account_permissions() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                Self::ACCOUNT_MANAGE,
                "Update or sync settings for authorized accounts.",
            ),
            (
                Self::ACCOUNT_READ_DETAILS,
                "View status and details of authorized accounts.",
            ),
            (
                Self::DATA_READ,
                "Read messages from authorized email accounts.",
            ),
            (
                Self::DATA_MANAGE,
                "Manage tags and metadata for authorized accounts.",
            ),
            (
                Self::DATA_RAW_DOWNLOAD,
                "Download raw EML files from authorized accounts.",
            ),
            (
                Self::DATA_DELETE,
                "Delete messages from authorized email accounts.",
            ),
            (
                Self::DATA_EXPORT_BATCH,
                "Export messages from authorized accounts.",
            ),
            (
                Self::DATA_IMPORT_BATCH,
                "Import external EML/PST data into authorized accounts.",
            ),
        ]
    }

    pub fn all_permissions() -> Vec<(&'static str, &'static str)> {
        let mut all = Self::global_permissions();
        all.extend(Self::account_permissions());
        all
    }

    fn is_account_permission(perm: &str) -> bool {
        Self::account_permissions().iter().any(|(p, _)| *p == perm)
    }

    fn is_global_permission(perm: &str) -> bool {
        Self::global_permissions().iter().any(|(p, _)| *p == perm)
    }

    pub fn validate_role_permissions(
        role_type: &RoleType,
        permissions: &BTreeSet<String>,
    ) -> BichonResult<()> {
        for p in permissions {
            match role_type {
                RoleType::Global => {
                    if !Self::is_global_permission(p) {
                        return Err(raise_error!(
                            format!("Permission '{}' is not a valid Global permission", p),
                            ErrorCode::InvalidParameter
                        ));
                    }
                }
                RoleType::Account => {
                    if !Self::is_account_permission(p) {
                        return Err(raise_error!(
                            format!("Permission '{}' is not a valid Account permission", p),
                            ErrorCode::InvalidParameter
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
