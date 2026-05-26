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

use crate::autoconfig::entity::OAuth2Config;

/// Per-provider OAuth2 metadata, mirroring Thunderbird's `OAuth2Providers.sys.mjs`.
///
/// Each entry maps one or more IMAP hostname suffixes to a well-known OIDC issuer
/// and the IMAP-specific OAuth2 scopes.
struct Provider {
    /// Suffixes matched case-insensitively against the end of the IMAP hostname.
    host_suffixes: &'static [&'static str],
    /// The OIDC issuer URL used by the provider.
    issuer: &'static str,
    /// OAuth2 scope(s) required for IMAP access.
    scopes: &'static [&'static str],
}

const PROVIDERS: &[Provider] = &[
    // Google
    Provider {
        host_suffixes: &["imap.gmail.com", ".gmail.com", ".googlemail.com"],
        issuer: "https://accounts.google.com",
        scopes: &["https://mail.google.com/"],
    },
    // Microsoft (Outlook / Office 365 / Hotmail / Live)
    Provider {
        host_suffixes: &[
            "outlook.office365.com",
            ".outlook.com",
            ".hotmail.com",
            ".live.com",
            ".office365.com",
        ],
        issuer: "https://login.microsoftonline.com/common/v2.0",
        scopes: &[
            "https://outlook.office365.com/IMAP.AccessAsUser.All",
            "offline_access",
        ],
    },
    // Yahoo / AOL / ATT / Verizon
    Provider {
        host_suffixes: &[
            "imap.mail.yahoo.com",
            ".yahoo.com",
            ".yahoodns.net",
            ".aol.com",
            "imap.aol.com",
        ],
        issuer: "https://login.yahoo.com",
        scopes: &["mail-w"],
    },
    // Yandex
    Provider {
        host_suffixes: &["imap.yandex.ru", "imap.yandex.com", ".yandex.ru"],
        issuer: "https://oauth.yandex.com",
        scopes: &["imap:all"],
    },
    // Mail.ru
    Provider {
        host_suffixes: &["imap.mail.ru", ".mail.ru", ".bk.ru", ".list.ru", ".inbox.ru"],
        issuer: "https://o2.mail.ru",
        scopes: &["imap"],
    },
    // Fastmail
    Provider {
        host_suffixes: &["imap.fastmail.com", ".fastmail.com"],
        issuer: "https://www.fastmail.com",
        scopes: &[
            "https://www.fastmail.com/dev/imap",
            "offline_access",
        ],
    },
    // Comcast
    Provider {
        host_suffixes: &["imap.comcast.net", ".comcast.net"],
        issuer: "https://oauth.xfinity.com",
        scopes: &["https://email.comcast.net/"],
    },
];

/// Try to find an OAuth2 provider that matches the given IMAP hostname.
///
/// Matching is case-insensitive and done by suffix: a hostname "imap.gmail.com"
/// matches the suffix ".gmail.com".
pub fn lookup_oauth2(hostname: &str) -> Option<OAuth2Config> {
    let host = hostname.to_ascii_lowercase();
    for provider in PROVIDERS {
        if provider
            .host_suffixes
            .iter()
            .any(|suffix| host.ends_with(&suffix.to_ascii_lowercase()))
        {
            return Some(OAuth2Config {
                issuer: provider.issuer.to_string(),
                scope: provider.scopes.iter().map(|s| s.to_string()).collect(),
                auth_url: String::new(),
                token_url: String::new(),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_providers() {
        let cases = [
            ("imap.gmail.com", Some("https://accounts.google.com")),
            ("imap.gmail.com", Some("https://accounts.google.com")),
            ("outlook.office365.com", Some("https://login.microsoftonline.com/common/v2.0")),
            ("imap.mail.yahoo.com", Some("https://login.yahoo.com")),
            ("imap.aol.com", Some("https://login.yahoo.com")),
            ("imap.yandex.ru", Some("https://oauth.yandex.com")),
            ("imap.mail.ru", Some("https://o2.mail.ru")),
            ("imap.fastmail.com", Some("https://www.fastmail.com")),
            ("imap.comcast.net", Some("https://oauth.xfinity.com")),
        ];
        for (hostname, expected_issuer) in &cases {
            let result = lookup_oauth2(hostname);
            assert_eq!(
                result.map(|c| c.issuer),
                expected_issuer.map(|s| s.to_string()),
                "failed for hostname: {hostname}"
            );
        }
    }

    #[test]
    fn test_unknown_provider() {
        assert!(lookup_oauth2("mail.my-company.example").is_none());
    }
}
