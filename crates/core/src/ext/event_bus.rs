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
use std::sync::{LazyLock, RwLock};

#[derive(Debug, Clone)]
pub enum Event {
    EmailViewed {
        email_id: String,
        user: String,
        ip: IpAddr,
    },
    EmailDeleted {
        email_id: String,
        user: String,
    },
    UserLoggedIn {
        user: String,
        ip: IpAddr,
    },
    UserCreated {
        created_by: String,
        new_user: String,
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
        content_hash: String,
        user: String,
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

/// Called by Pro/Enterprise at startup to replace the noop default.
pub fn set_event_bus(bus: Box<dyn EventBus>) {
    *EVENT_BUS.write().unwrap() = bus;
}

/// Fire-and-forget. Called by the server at key points.
pub fn emit(event: Event) {
    EVENT_BUS.read().unwrap().emit(event);
}
