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

use crate::error::code::ErrorCode;
use crate::imap::session::SessionStream;
use crate::{error::BichonResult, raise_error};
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
    if !has_capability(capabilities, "IMAP4rev1") {
        return Err(raise_error!(
            "Server does not support IMAP4rev1".into(),
            ErrorCode::Incompatible
        ));
    }
    Ok(())
}

/// Whether the server advertises `name` (e.g. `MOVE`).
///
/// Capability names are case-insensitive ([RFC 3501 §7.2.1]), but the client's
/// `has_str` compares atoms exactly — only `IMAP4rev1` is special-cased. A
/// server that lists `move` in lowercase is therefore missed by `has_str`,
/// which for cleanup would wrongly mean "cannot move safely".
pub fn has_capability(capabilities: &Capabilities, name: &str) -> bool {
    capabilities
        .iter()
        .any(|capability| capability_matches(capability, name))
}

fn capability_matches(capability: &Capability, name: &str) -> bool {
    match capability {
        Capability::Imap4rev1 => name.eq_ignore_ascii_case("IMAP4rev1"),
        Capability::Auth(mechanism) => {
            name.get(..5)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("AUTH="))
                && name[5..].eq_ignore_ascii_case(mechanism)
        }
        Capability::Atom(atom) => atom.eq_ignore_ascii_case(name),
    }
}

pub fn capability_to_string(capability: &Capability) -> String {
    match capability {
        Capability::Imap4rev1 => "IMAP4rev1".into(),
        Capability::Auth(v) => format!("AUTH={}", v),
        Capability::Atom(v) => v.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::capability_matches;
    use async_imap::types::Capability;

    #[test]
    fn capability_names_are_matched_case_insensitively() {
        assert!(capability_matches(&Capability::Atom("move".into()), "MOVE"));
        assert!(capability_matches(&Capability::Atom("IDLE".into()), "idle"));
        assert!(capability_matches(&Capability::Imap4rev1, "imap4rev1"));
        assert!(capability_matches(
            &Capability::Auth("PLAIN".into()),
            "AUTH=plain"
        ));
        assert!(!capability_matches(&Capability::Atom("IDLE".into()), "MOVE"));
    }
}
