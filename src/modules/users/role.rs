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
    fmt::{self, Display},
};

use native_db::*;
use native_model::{native_model, Model};
use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

use crate::{
    id,
    modules::{
        database::{
            async_find_impl, delete_impl, insert_impl, list_all_impl, manager::DB_MANAGER,
            update_impl, with_transaction,
        },
        error::{code::ErrorCode, BichonResult},
        users::{
            payload::{RoleCreateRequest, RoleUpdateRequest},
            permissions::*,
        },
    },
    raise_error, utc_now,
};

/// Enumerates the built-in roles in the Bichon system.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum BuiltinRole {
    Admin,
    Manager,
    Member,
    AccountManager,
    AccountViewer,
}

impl Display for BuiltinRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BuiltinRole::Admin => "admin",
            BuiltinRole::Manager => "manager",
            BuiltinRole::Member => "member",
            BuiltinRole::AccountManager => "account_manager",
            BuiltinRole::AccountViewer => "account_viewer",
        };
        write!(f, "{}", s)
    }
}

impl BuiltinRole {
    pub fn description(&self) -> &'static str {
        match self {
            BuiltinRole::Admin => {
                "Full system administrator with unrestricted access to all accounts, user management, and system configurations."
            }
            BuiltinRole::Manager => {
                "Standard operational manager. Can manage users, create accounts, and perform data operations on authorized email accounts."
            }
            BuiltinRole::Member => {
                "Regular platform member. Provides basic login access to the system without any administrative or global management privileges."
            }
            BuiltinRole::AccountManager => {
                "Specific account manager. Has full administrative control over a particular email account, including configuration and data deletion."
            }
            BuiltinRole::AccountViewer => {
                "Specific account observer. Has read-only access to messages and metadata for a particular email account."
            }
        }
    }

    /// Retrieves the set of static permissions associated with the role.
    pub fn get_permissions(&self) -> HashSet<&'static str> {
        match self {
            BuiltinRole::Admin => Self::admin_permissions(),
            BuiltinRole::Manager => Self::manager_permissions(),
            BuiltinRole::Member => Self::member_permissions(),
            BuiltinRole::AccountManager => Self::account_owner_permissions(),
            BuiltinRole::AccountViewer => Self::account_viewer_permissions(),
        }
    }

    /// Admin Role: Full control over the system and all data.
    fn admin_permissions() -> HashSet<&'static str> {
        [
            // System-Wide
            Permission::ROOT,
            Permission::USER_MANAGE,
            Permission::USER_VIEW,
            Permission::TOKEN_MANAGE,
            // Account Configuration
            Permission::ACCOUNT_CREATE,
            Permission::ACCOUNT_MANAGE_ALL, // Global account management
            // Data Access (Global ALL)
            Permission::DATA_READ_ALL,
            Permission::DATA_MANAGE_ALL,
            Permission::DATA_RAW_DOWNLOAD_ALL,
            Permission::DATA_DELETE_ALL,
            Permission::DATA_EXPORT_BATCH_ALL,
        ]
        .into_iter()
        .collect()
    }

    /// Manager Role: Data and account configuration management, limited user management.
    /// ALL data/account access must be scoped by the user's ACL.
    fn manager_permissions() -> HashSet<&'static str> {
        [Permission::USER_VIEW, Permission::ACCOUNT_CREATE]
            .into_iter()
            .collect()
    }

    fn member_permissions() -> HashSet<&'static str> {
        [Permission::SYSTEM_ACCESS].into_iter().collect()
    }

    fn account_owner_permissions() -> HashSet<&'static str> {
        [
            Permission::ACCOUNT_MANAGE,
            Permission::ACCOUNT_READ_DETAILS,
            Permission::DATA_READ,
            Permission::DATA_MANAGE,
            Permission::DATA_RAW_DOWNLOAD,
            Permission::DATA_DELETE,
            Permission::DATA_EXPORT_BATCH,
            Permission::DATA_IMPORT_BATCH,
        ]
        .into_iter()
        .collect()
    }

    fn account_viewer_permissions() -> HashSet<&'static str> {
        [Permission::ACCOUNT_READ_DETAILS, Permission::DATA_READ]
            .into_iter()
            .collect()
    }
}

// Global Roles (Starting with 1)
pub const DEFAULT_ADMIN_ROLE_ID: u64 = 100_000_000_000_000; // System Admin
pub const DEFAULT_MANAGER_ROLE_ID: u64 = 100_100_000_000_000; // System Manager
pub const DEFAULT_MEMBER_ROLE_ID: u64 = 100_200_000_000_000; // Regular Member (system:access)

// Account-specific Roles (Starting with 2)
pub const DEFAULT_ACCOUNT_MANAGER_ROLE_ID: u64 = 200_100_000_000_000;
pub const DEFAULT_ACCOUNT_VIEWER_ROLE_ID: u64 = 200_200_000_000_000;

fn is_builtin(id: u64) -> bool {
    matches!(
        id,
        DEFAULT_ADMIN_ROLE_ID
            | DEFAULT_MANAGER_ROLE_ID
            | DEFAULT_MEMBER_ROLE_ID
            | DEFAULT_ACCOUNT_MANAGER_ROLE_ID
            | DEFAULT_ACCOUNT_VIEWER_ROLE_ID
    )
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Enum)]
pub enum RoleType {
    #[default]
    Global,
    Account,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Object)]
#[native_model(id = 9, version = 1)]
#[native_db]
pub struct UserRole {
    #[primary_key]
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    pub permissions: BTreeSet<String>,
    pub is_builtin: bool,
    pub created_at: i64,
    pub role_type: RoleType,
    pub updated_at: i64,
}

impl UserRole {
    pub async fn ensure_default_roles_exists() -> BichonResult<()> {
        let builtin_roles = vec![
            (BuiltinRole::Admin, DEFAULT_ADMIN_ROLE_ID, RoleType::Global),
            (
                BuiltinRole::Manager,
                DEFAULT_MANAGER_ROLE_ID,
                RoleType::Global,
            ),
            (
                BuiltinRole::Member,
                DEFAULT_MEMBER_ROLE_ID,
                RoleType::Global,
            ),
            (
                BuiltinRole::AccountManager,
                DEFAULT_ACCOUNT_MANAGER_ROLE_ID,
                RoleType::Account,
            ),
            (
                BuiltinRole::AccountViewer,
                DEFAULT_ACCOUNT_VIEWER_ROLE_ID,
                RoleType::Account,
            ),
        ];

        with_transaction(DB_MANAGER.meta_db(), move |rw| {
            let now = utc_now!();

            for (role, role_id, role_type) in builtin_roles {
                let exists = rw
                    .get()
                    .primary::<UserRole>(role_id)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
                    .is_some();

                if !exists {
                    let permissions: BTreeSet<String> = role
                        .get_permissions()
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect();

                    rw.insert(UserRole {
                        id: role_id,
                        name: role.to_string(),
                        description: Some(role.description().to_string()),
                        permissions,
                        created_at: now,
                        updated_at: now,
                        is_builtin: true,
                        role_type,
                    })
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
                }
            }
            Ok(())
        })
        .await?;

        Ok(())
    }

    pub async fn list_all() -> BichonResult<Vec<UserRole>> {
        list_all_impl(DB_MANAGER.meta_db()).await
    }

    pub async fn find(role_id: u64) -> BichonResult<Option<UserRole>> {
        async_find_impl(DB_MANAGER.meta_db(), role_id).await
    }

    pub async fn create(request: RoleCreateRequest) -> BichonResult<UserRole> {
        let _ = &request.validate().await?;
        let now = utc_now!();
        let new_role = UserRole {
            id: id!(64),
            name: request.name,
            description: request.description,
            permissions: request.permissions,
            created_at: now,
            updated_at: now,
            is_builtin: false,
            role_type: request.role_type,
        };
        insert_impl(DB_MANAGER.meta_db(), new_role.clone()).await?;
        Ok(new_role)
    }

    pub async fn update(id: u64, request: RoleUpdateRequest) -> BichonResult<()> {
        if is_builtin(id) && request.permissions.is_some() {
            return Err(raise_error!(
                "The permissions of a builtin role are immutable. Please create a custom role instead.".into(),
                ErrorCode::Forbidden
            ));
        }
        let _ = &request.validate().await?;

        if let Some(permissions) = &request.permissions {
            let role = Self::find(id).await?.ok_or_else(|| {
                raise_error!(
                    format!("UserRole with id={} not found", id),
                    ErrorCode::ResourceNotFound
                )
            })?;
            Permission::validate_role_permissions(&role.role_type, permissions)?;
        }

        update_impl(
            DB_MANAGER.meta_db(),
            move |rw| {
                rw.get()
                    .primary::<UserRole>(id)
                    .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
                    .ok_or_else(|| {
                        raise_error!(
                            format!("UserRole with id={} not found", id),
                            ErrorCode::ResourceNotFound
                        )
                    })
            },
            move |current| {
                let mut updated = current.clone();
                if let Some(name) = request.name {
                    updated.name = name;
                }

                if let Some(desc) = request.description {
                    updated.description = Some(desc);
                }

                if let Some(permissions) = request.permissions {
                    updated.permissions = permissions;
                }
                updated.updated_at = utc_now!();
                Ok(updated)
            },
        )
        .await?;
        Ok(())
    }

    pub async fn delete(id: u64) -> BichonResult<()> {
        if is_builtin(id) {
            return Err(raise_error!(
                format!("Cannot delete a default system role (ID: {}).", id),
                ErrorCode::InvalidParameter
            ));
        }
        delete_impl(DB_MANAGER.meta_db(), move |rw| {
            rw.get()
                .primary::<UserRole>(id)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
                .ok_or_else(|| {
                    raise_error!(
                        format!("UserRole '{}' not found during deletion process.", id),
                        ErrorCode::ResourceNotFound
                    )
                })
        })
        .await
    }
}
