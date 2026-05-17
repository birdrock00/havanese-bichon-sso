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

use std::collections::HashSet;

use bichon_core::{
    account::migration::AccountModel,
    common::auth::ClientContext,
    dashboard::DashboardStats,
    message::search::{
        search_attachment_impl, search_messages_impl, AttachmentSearchFilter, AttachmentSearchRequest,
        EmailSearchFilter, EmailSearchRequest, SortBy,
    },
    message::{content::retrieve_email_content, list::get_thread_messages},
    users::permissions::Permission,
};
use poem_mcpserver::{content::Text, Tools};

/// Tools for interacting with the Bichon email archive via MCP.
///
/// Provides email search, content retrieval, thread viewing, attachment search,
/// dashboard statistics, and account management capabilities.
pub struct BichonMcpTools {
    context: ClientContext,
}

impl BichonMcpTools {
    pub fn new(context: ClientContext) -> Self {
        Self { context }
    }

    /// Derive the set of account IDs that the current user is authorized to access.
    /// Returns None if the user has global access (DATA_READ_ALL or ACCOUNT_MANAGE_ALL),
    /// meaning "all accounts are accessible".
    fn authorized_accounts(&self) -> Option<HashSet<u64>> {
        if self.context.has_permission(None, Permission::DATA_READ_ALL)
            || self.context.has_permission(None, Permission::ACCOUNT_MANAGE_ALL)
        {
            None
        } else {
            Some(
                self.context
                    .user
                    .account_access_map
                    .keys()
                    .copied()
                    .collect(),
            )
        }
    }

    /// Restrict the given account_ids to only those the user is authorized to access.
    /// If the user has global access, the input is returned as-is.
    fn restrict_account_ids(
        &self,
        account_ids: Option<Vec<u64>>,
    ) -> Option<HashSet<u64>> {
        let global = self.authorized_accounts();
        match (global, account_ids) {
            // User has global access → return input as HashSet (or None)
            (None, Some(ids)) => Some(ids.into_iter().collect()),
            (None, None) => None,
            // User is scoped → intersect with authorized set
            (Some(authorized), Some(ids)) => {
                let filtered: HashSet<u64> =
                    ids.into_iter().filter(|id| authorized.contains(id)).collect();
                if filtered.is_empty() {
                    Some(filtered)
                } else {
                    Some(filtered)
                }
            }
            (Some(authorized), None) => Some(authorized),
        }
    }
}

#[Tools]
impl BichonMcpTools {
    /// Search archived emails using full-text and structured filters.
    async fn search_emails(
        &self,
        /// Full-text search across all message fields (subject, body, from, to, etc.)
        text: Option<String>,
        /// Filter by email subject line
        subject: Option<String>,
        /// Filter by sender email address
        from: Option<String>,
        /// Filter by recipient email address
        to: Option<String>,
        /// Start date as Unix timestamp in milliseconds
        since: Option<i64>,
        /// End date as Unix timestamp in milliseconds
        before: Option<i64>,
        /// Filter by specific account IDs
        account_ids: Option<Vec<u64>>,
        /// Filter by specific mailbox/folder IDs
        mailbox_ids: Option<Vec<u64>>,
        /// Only return messages that have attachments
        has_attachment: Option<bool>,
        /// Filter by attachment filename
        attachment_name: Option<String>,
        /// Filter by tags/labels
        tags: Option<Vec<String>>,
        /// Page number (1-based, default 1)
        page: Option<u64>,
        /// Results per page (default 50, max 500)
        page_size: Option<u64>,
    ) -> Result<Text<String>, String> {
        let authorized_ids = self.restrict_account_ids(account_ids);

        let filter = EmailSearchFilter {
            text,
            subject,
            from,
            to,
            since,
            before,
            account_ids: authorized_ids.clone(),
            mailbox_ids: mailbox_ids.map(|v| v.into_iter().collect()),
            has_attachment,
            attachment_name,
            tags: tags.map(|v| v.into_iter().collect()),
            ..Default::default()
        };

        let request = EmailSearchRequest {
            filter,
            page: page.unwrap_or(1),
            page_size: page_size.unwrap_or(50).min(500),
            sort_by: Some(SortBy::DATE),
            desc: Some(true),
        };

        search_messages_impl(authorized_ids, request)
            .map(|result| Text(serde_json::to_string_pretty(&result).unwrap_or_default()))
            .map_err(|e| format!("Search failed: {:#}", e))
    }

    /// Retrieve the full content of a specific email including plain text, HTML body,
    /// and attachment metadata (filenames, sizes, content hashes).
    async fn get_email_content(
        &self,
        /// The ID of the email account
        account_id: u64,
        /// The envelope ID of the email message
        envelope_id: String,
    ) -> Result<Text<String>, String> {
        self.context
            .require_permission(Some(account_id), Permission::DATA_READ)
            .map_err(|e| format!("Permission denied: {:#}", e))?;

        retrieve_email_content(account_id, envelope_id)
            .map(|content| Text(serde_json::to_string_pretty(&content).unwrap_or_default()))
            .map_err(|e| format!("Failed to retrieve email content: {:#}", e))
    }

    /// Retrieve all messages in a specific email thread/conversation.
    async fn get_thread(
        &self,
        /// The ID of the email account
        account_id: u64,
        /// The thread ID (from the thread_id field of an envelope)
        thread_id: String,
        /// Page number (1-based, default 1)
        page: Option<u64>,
        /// Results per page (default 50, max 500)
        page_size: Option<u64>,
    ) -> Result<Text<String>, String> {
        self.context
            .require_permission(Some(account_id), Permission::DATA_READ)
            .map_err(|e| format!("Permission denied: {:#}", e))?;

        get_thread_messages(
            account_id,
            &thread_id,
            page.unwrap_or(1),
            page_size.unwrap_or(50).min(500),
        )
        .map(|result| Text(serde_json::to_string_pretty(&result).unwrap_or_default()))
        .map_err(|e| format!("Failed to retrieve thread: {:#}", e))
    }

    /// Search email attachments using filters like filename, extension, content type,
    /// category, sender, and date range.
    async fn search_attachments(
        &self,
        /// Full-text search in attachment content and metadata
        text: Option<String>,
        /// Filter by attachment filename
        attachment_name: Option<String>,
        /// Filter by file extension (e.g., pdf, docx, jpg)
        attachment_extension: Option<String>,
        /// Filter by category (document, image, spreadsheet, etc.)
        attachment_category: Option<String>,
        /// Filter by MIME content type (e.g., application/pdf)
        attachment_content_type: Option<String>,
        /// Filter by sender email address
        from: Option<String>,
        /// Start date as Unix timestamp in milliseconds
        since: Option<i64>,
        /// End date as Unix timestamp in milliseconds
        before: Option<i64>,
        /// Filter by specific account IDs
        account_ids: Option<Vec<u64>>,
        /// Minimum attachment size in bytes
        min_size: Option<u64>,
        /// Maximum attachment size in bytes
        max_size: Option<u64>,
        /// Page number (1-based, default 1)
        page: Option<u64>,
        /// Results per page (default 50, max 500)
        page_size: Option<u64>,
    ) -> Result<Text<String>, String> {
        let authorized_ids = self.restrict_account_ids(account_ids);

        let filter = AttachmentSearchFilter {
            text,
            attachment_name,
            attachment_extension,
            attachment_category,
            attachment_content_type,
            from,
            since,
            before,
            account_ids: authorized_ids.clone(),
            min_size,
            max_size,
            ..Default::default()
        };

        let request: AttachmentSearchRequest =
            serde_json::from_value(serde_json::json!({
                "filter": filter,
                "page": page.unwrap_or(1),
                "page_size": page_size.unwrap_or(50).min(500),
                "sort_by": "DATE",
                "desc": true,
            }))
            .map_err(|e| format!("Invalid request: {e}"))?;

        search_attachment_impl(authorized_ids, request)
            .map(|result| Text(serde_json::to_string_pretty(&result).unwrap_or_default()))
            .map_err(|e| format!("Attachment search failed: {:#}", e))
    }

    /// Retrieve summary statistics about the email archive including total emails,
    /// attachments, storage usage, top senders, and recent activity.
    async fn get_dashboard_stats(&self) -> Result<Text<String>, String> {
        if !self.context.has_permission(None, Permission::SYSTEM_ACCESS) {
            return Err("Permission denied: requires system:access".into());
        }

        DashboardStats::get(self.context.clone())
            .await
            .map(|stats| Text(serde_json::to_string_pretty(&stats).unwrap_or_default()))
            .map_err(|e| format!("Failed to retrieve dashboard stats: {:#}", e))
    }

    /// List all email accounts the current user has access to.
    /// Returns minimal account information (ID and email address).
    async fn list_accounts(&self) -> Result<Text<String>, String> {
        let accounts = AccountModel::minimal_list(false)
            .map_err(|e| format!("Failed to list accounts: {:#}", e))?;

        let global_access = self
            .context
            .has_permission(None, Permission::ACCOUNT_MANAGE_ALL);

        let visible: Vec<_> = if global_access {
            accounts
        } else {
            let authorized: HashSet<u64> =
                self.context.user.account_access_map.keys().copied().collect();
            accounts
                .into_iter()
                .filter(|a| authorized.contains(&a.id))
                .collect()
        };

        Ok(Text(
            serde_json::to_string_pretty(&visible).unwrap_or_default(),
        ))
    }

    /// Retrieve detailed configuration and status of a specific email account.
    async fn get_account(
        &self,
        /// The ID of the email account
        account_id: u64,
    ) -> Result<Text<String>, String> {
        self.context
            .require_permission(Some(account_id), Permission::ACCOUNT_READ_DETAILS)
            .map_err(|e| format!("Permission denied: {:#}", e))?;

        let account = AccountModel::get(account_id)
            .map_err(|e| format!("Account not found: {:#}", e))?;

        Ok(Text(
            serde_json::to_string_pretty(&account).unwrap_or_default(),
        ))
    }
}
