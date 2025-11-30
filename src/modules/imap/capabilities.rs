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

use crate::modules::error::code::ErrorCode;
use crate::modules::imap::session::SessionStream;
use crate::{modules::error::BichonResult, raise_error};
use async_imap::types::Capability;
use async_imap::{types::Capabilities, Session};

pub async fn fetch_capabilities(
    session: &mut Session<Box<dyn SessionStream>>,
) -> BichonResult<Capabilities> {
    session
        .capabilities()
        .await
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::ImapCommandFailed))
}

pub fn check_capabilities(capabilities: &Capabilities) -> BichonResult<()> {
    if !capabilities.has_str("IMAP4rev1") {
        return Err(raise_error!(
            "Server does not support IMAP4rev1".into(),
            ErrorCode::Incompatible
        ));
    }
    Ok(())
}

pub fn capability_to_string(capability: &Capability) -> String {
    match capability {
        Capability::Imap4rev1 => "IMAP4rev1".into(),
        Capability::Auth(v) => format!("AUTH={}", v),
        Capability::Atom(v) => v.into(),
    }
}
