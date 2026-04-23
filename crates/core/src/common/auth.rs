use std::{
    collections::{BTreeSet, HashSet},
    net::IpAddr,
};

use crate::{
    error::{code::ErrorCode, BichonResult},
    raise_error,
    users::{permissions::Permission, role::UserRole, UserModel},
};

#[derive(Clone, Debug)]
pub struct ClientContext {
    pub ip_addr: Option<IpAddr>,
    pub user: UserModel,
}

impl ClientContext {
    pub async fn require_any_permission(
        &self,
        requirements: Vec<(Option<u64>, &str)>,
    ) -> BichonResult<()> {
        for (account_id, permission) in requirements {
            if self.has_permission(account_id, permission).await {
                return Ok(());
            }
        }
        Err(raise_error!(
            "Access denied: Insufficient permissions to perform this action.".into(),
            ErrorCode::Forbidden
        ))
    }

    pub async fn check_has_permission(
        user: &UserModel,
        account_id: Option<u64>,
        permission: &str,
    ) -> bool {
        if user.is_admin().await {
            return true;
        }

        let mut global_perms = HashSet::new();
        for rid in &user.global_roles {
            if let Some(role) = UserRole::find(*rid).await.ok().flatten() {
                global_perms.extend(role.permissions);
            }
        }

        if Self::check_global_logic(&global_perms, permission) {
            return true;
        }

        if let Some(aid) = account_id {
            if let Some(role_id) = user.account_access_map.get(&aid) {
                if let Some(role) = UserRole::find(*role_id).await.ok().flatten() {
                    if role.permissions.contains(&permission.to_string())
                        || Self::check_account_logic(&role.permissions, permission)
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub async fn has_permission(&self, account_id: Option<u64>, permission: &str) -> bool {
        if self.user.is_admin().await {
            return true;
        }

        let mut global_perms = HashSet::new();
        for rid in &self.user.global_roles {
            if let Some(role) = UserRole::find(*rid).await.ok().flatten() {
                global_perms.extend(role.permissions);
            }
        }

        if Self::check_global_logic(&global_perms, permission) {
            return true;
        }

        if let Some(aid) = account_id {
            if let Some(role_id) = self.user.account_access_map.get(&aid) {
                if let Some(role) = UserRole::find(*role_id).await.ok().flatten() {
                    if role.permissions.contains(&permission.to_string())
                        || Self::check_account_logic(&role.permissions, permission)
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn check_global_logic(global: &HashSet<String>, perm: &str) -> bool {
        if global.contains(perm) {
            return true;
        }

        match perm {
            Permission::DATA_READ => global.contains(Permission::DATA_READ_ALL),
            Permission::DATA_DELETE => global.contains(Permission::DATA_DELETE_ALL),
            Permission::DATA_RAW_DOWNLOAD => global.contains(Permission::DATA_RAW_DOWNLOAD_ALL),
            Permission::DATA_EXPORT_BATCH => global.contains(Permission::DATA_EXPORT_BATCH_ALL),
            Permission::ACCOUNT_MANAGE | Permission::ACCOUNT_READ_DETAILS => {
                global.contains(Permission::ACCOUNT_MANAGE_ALL)
            }
            _ => false,
        }
    }

    fn check_account_logic(scoped_perms: &BTreeSet<String>, perm: &str) -> bool {
        if scoped_perms.contains(perm) {
            return true;
        }
        match perm {
            Permission::DATA_READ | Permission::ACCOUNT_READ_DETAILS => {
                scoped_perms.contains(Permission::ACCOUNT_MANAGE)
            }
            _ => false,
        }
    }

    pub async fn require_permission(
        &self,
        account_id: Option<u64>,
        permission: &str,
    ) -> BichonResult<()> {
        if self.has_permission(account_id, permission).await {
            Ok(())
        } else {
            Err(raise_error!(
                format!("Access Denied: Missing permission '{}'", permission),
                ErrorCode::Forbidden
            ))
        }
    }
}
