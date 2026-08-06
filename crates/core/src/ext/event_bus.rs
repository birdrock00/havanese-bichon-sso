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

// Event bus extension point.
//
// Community edition: NoopEventBus — all events are discarded.
// Pro edition: AuditEventBus — events are persisted to audit database.
// Enterprise edition: adds SIEM webhook to the same trait impl.
//
// The open-source server emits events at key points (login, view, delete, search).
// It never reads from the event bus — events are fire-and-forget.

use std::net::IpAddr;
use std::sync::{LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Free-form JSON value carried by events that need a content snapshot
/// (e.g. the subject / attachment names of a message being deleted, so the
/// audit trail stays self-describing after the content is gone).
pub type EventPayload = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Clone)]
pub enum Event {
    EmailViewed {
        email_id: String,
        user: String,
        ip: IpAddr,
        account_id: u64,
        mailbox_id: u64,
        /// Subject of the viewed message, so the audit trail is readable
        /// without resolving the envelope again.
        subject: Option<String>,
    },
    EmailDeleted {
        email_id: String,
        user: String,
        account_id: u64,
        mailbox_id: u64,
        /// Subject at deletion time (the message is gone afterwards).
        subject: Option<String>,
        /// Snapshot of the deleted message (attachment names, from/to, ...).
        snapshot: Option<EventPayload>,
    },
    /// Raw EML file downloaded (export).
    EmailExported {
        email_id: String,
        user: String,
        account_id: u64,
        subject: Option<String>,
    },
    /// Email restored back to the source IMAP server.
    EmailRestored {
        email_id: String,
        user: String,
        account_id: u64,
        subject: Option<String>,
    },
    /// Facet tags added/removed on emails.
    EmailTagged {
        user: String,
        account_id: u64,
        /// Number of emails whose tags were changed.
        count: u64,
    },
    /// Facet tags added/removed on attachments.
    AttachmentTagged {
        user: String,
        account_id: u64,
        /// Number of attachments whose tags were changed.
        count: u64,
    },
    /// Attachment content streamed for in-browser preview (not a download).
    AttachmentPreviewed {
        email_id: String,
        content_hash: String,
        user: String,
        account_id: u64,
        mailbox_id: u64,
        filename: Option<String>,
    },
    UserLoggedIn {
        user: String,
        ip: IpAddr,
    },
    UserCreated {
        created_by: String,
        new_user: String,
    },
    UserUpdated {
        updated_by: String,
        target_user: String,
    },
    UserRemoved {
        removed_by: String,
        target_user: String,
    },
    RoleCreated {
        created_by: String,
        role_name: String,
    },
    RoleUpdated {
        updated_by: String,
        role_name: String,
    },
    RoleRemoved {
        removed_by: String,
        role_name: String,
    },
    SearchPerformed {
        query: String,
        user: String,
    },
    SettingsChanged {
        key: String,
        user: String,
    },
    AttachmentDownloaded {
        email_id: String,
        /// The attachment's own content hash.
        content_hash: String,
        user: String,
        account_id: u64,
        mailbox_id: u64,
        filename: Option<String>,
        size: Option<u64>,
        ext: Option<String>,
        /// Content hash of the parent email (EML), when known.
        parent_content_hash: Option<String>,
    },
    AccountCreated {
        created_by: String,
        account_id: u64,
        email: String,
    },
    AccountUpdated {
        updated_by: String,
        account_id: u64,
        email: String,
    },
    AccountRemoved {
        removed_by: String,
        account_id: u64,
        email: String,
    },
    AccountDownloadStarted {
        user: String,
        account_id: u64,
        run_gap_fill: bool,
    },
    AccountDownloadStopped {
        user: String,
        account_id: u64,
    },
    /// Batch role / access assignment on one or more accounts.
    AccountRoleAssigned {
        user: String,
        target_user: String,
        account_count: usize,
        roles: Vec<String>,
    },
    AccessTokenCreated {
        user: String,
        target_user: String,
        name: Option<String>,
    },
    AccessTokenRemoved {
        user: String,
        token_user: String,
        name: Option<String>,
    },
    OAuth2ConfigCreated {
        user: String,
        oauth2_id: u64,
        name: String,
    },
    OAuth2ConfigUpdated {
        user: String,
        oauth2_id: u64,
        name: String,
    },
    OAuth2ConfigRemoved {
        user: String,
        oauth2_id: u64,
        name: String,
    },
    /// External OAuth2 token stored / refreshed for an account.
    OAuth2TokenStored {
        user: String,
        account_id: u64,
    },
    ImportPerformed {
        user: String,
        account_id: u64,
        format: String,
        total: u64,
        success: u64,
        failed: u64,
    },
    MailboxRemoved {
        user: String,
        account_id: u64,
        mailbox_id: u64,
    },
    ProxyCreated {
        user: String,
        url: String,
    },
    ProxyUpdated {
        user: String,
        url: String,
    },
    ProxyRemoved {
        user: String,
        url: String,
    },
    /// Pro edition: SSO (OIDC) login, logout, or license upload.
    SsoLogin {
        user: String,
        ip: Option<IpAddr>,
    },
    SsoLogout {
        user: String,
    },
    LicenseUploaded {
        user: String,
        email: String,
        edition: String,
    },
}

pub trait EventBus: Send + Sync {
    fn emit(&self, event: Event);
}

/// Default — all events are discarded.
struct NoopEventBus;
impl EventBus for NoopEventBus {
    fn emit(&self, _event: Event) {}
}

static EVENT_BUS: LazyLock<RwLock<Box<dyn EventBus>>> =
    LazyLock::new(|| RwLock::new(Box::new(NoopEventBus)));

/// Short-window dedup of view/download events.
///
/// The web UI can fire duplicate `message-content` requests for the same
/// email (React StrictMode double-effects, remote-content toggle, thread
/// expansion). Deduping here keeps the audit trail to one record per
/// intentional view without hiding repeated deliberate accesses.
static VIEW_DEDUP: LazyLock<Mutex<Vec<(String, Instant)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

const VIEW_DEDUP_WINDOW: Duration = Duration::from_secs(10);

fn is_duplicate_view(event: &Event) -> bool {
    let key = match event {
        Event::EmailViewed {
            user, email_id, ..
        } => Some(format!("email.viewed|{user}|{email_id}")),
        Event::EmailDeleted {
            user, email_id, ..
        } => Some(format!("email.deleted|{user}|{email_id}")),
        Event::AttachmentDownloaded {
            user,
            email_id,
            content_hash,
            ..
        } => Some(format!("attachment.downloaded|{user}|{email_id}|{content_hash}")),
        Event::AttachmentPreviewed {
            user,
            email_id,
            content_hash,
            ..
        } => Some(format!("attachment.previewed|{user}|{email_id}|{content_hash}")),
        _ => None,
    };
    let Some(key) = key else {
        return false;
    };

    let mut entries = VIEW_DEDUP.lock().unwrap();
    let now = Instant::now();
    entries.retain(|(_, at)| now.duration_since(*at) < VIEW_DEDUP_WINDOW);
    if entries.iter().any(|(k, _)| *k == key) {
        return true;
    }
    entries.push((key, now));
    false
}

/// Called by Pro/Enterprise at startup to replace the noop default.
pub fn set_event_bus(bus: Box<dyn EventBus>) {
    *EVENT_BUS.write().unwrap() = bus;
}

/// Fire-and-forget. Called by the server at key points.
pub fn emit(event: Event) {
    if is_duplicate_view(&event) {
        return;
    }
    EVENT_BUS.read().unwrap().emit(event);
}
