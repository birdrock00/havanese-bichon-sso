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
//
// Capability gate for the optional account roles (`AccountKind`).
//
// Community edition: no check is registered — the roles exist in the data
// model (so rows written by a Pro installation still deserialize) but cannot be
// created, and nothing acts on them.
// Pro/Enterprise: `bichon_pro::initialize()` registers a live check. A callback
// rather than a boolean because the licence status is populated *after*
// `initialize()` runs, and because it can lapse while the server keeps
// running — the role must stop being creatable then, not at the next restart.

use std::sync::{LazyLock, RwLock};

type RoleCheck = fn() -> bool;

static VIEW_ROLE_CHECK: LazyLock<RwLock<Option<RoleCheck>>> = LazyLock::new(|| RwLock::new(None));

/// Called by Pro/Enterprise at startup with a live licence check.
pub fn set_view_role_check(check: RoleCheck) {
    *VIEW_ROLE_CHECK.write().unwrap() = Some(check);
}

/// Whether a `view` account may be created right now.
///
/// Evaluated in `AccountCreateRequest::create_entity`, so a community build
/// refuses the role even when the API is called directly — the gate is not only
/// the WebUI hiding a field.
pub fn view_role_enabled() -> bool {
    VIEW_ROLE_CHECK
        .read()
        .unwrap()
        .map(|check| check())
        .unwrap_or(false)
}
