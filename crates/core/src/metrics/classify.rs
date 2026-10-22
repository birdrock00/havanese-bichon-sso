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

//! Classification of sync/download errors into Prometheus-friendly kinds.
//!
//! IMAP has no status codes, so the kind is inferred from the error message.
//! The original message is still persisted verbatim; the kind only feeds the
//! `bichon_sync_errors_total` counter (and the persisted `AccountError.kind`).
//! Classification is intentionally best effort: an unknown message stays
//! `Internal` rather than guessing.

use serde::{Deserialize, Serialize};

/// Classified kind of a download/sync error.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
#[cfg_attr(feature = "web-api", oai(rename_all = "lowercase"))]
pub enum ErrorKind {
    /// Login / OAuth2 failure (IMAP analogue of HTTP 401/403).
    Auth,
    /// Server-side throttling or over-quota (IMAP analogue of HTTP 429).
    Throttled,
    /// Connection-level failure (refused, reset, DNS, unreachable).
    Network,
    /// Operation exceeded its deadline.
    Timeout,
    /// Server violated or rejected the protocol (bad response, parse error).
    Protocol,
    #[default]
    /// Anything else; bugs included.
    Internal,
}

impl ErrorKind {
    /// Label value used by `bichon_sync_errors_total`.
    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorKind::Auth => "auth",
            ErrorKind::Throttled => "throttled",
            ErrorKind::Network => "network",
            ErrorKind::Timeout => "timeout",
            ErrorKind::Protocol => "protocol",
            ErrorKind::Internal => "internal",
        }
    }
}

/// Substring matchers, in precedence order: the first matching kind wins.
///
/// Auth failures must not be masked as throttling, and throttling must not be
/// masked as generic network noise, so an auth message that also mentions
/// `too many connections` still classifies as `auth`.
const MATCHERS: &[(ErrorKind, &[&str])] = &[
    (
        ErrorKind::Auth,
        &[
            "authenticationfailed",
            "invalidcredentials",
            "login failed",
            "authentication failed",
            "auth",
            "credentials",
            "unauthorized",
            "forbidden",
        ],
    ),
    (
        ErrorKind::Throttled,
        &[
            "throttl",
            "overquota",
            "quota exceeded",
            "unavailable",
            "too many simultaneous connections",
            "too many connections",
            "rate limit",
            "try again later",
        ],
    ),
    (ErrorKind::Timeout, &["timeout", "timed out"]),
    (
        ErrorKind::Network,
        &[
            "connection refused",
            "connection reset",
            "connection closed",
            "connection aborted",
            "broken pipe",
            "dns",
            "resolve",
            "name or service not known",
            "network",
            "unreachable",
            "io error",
        ],
    ),
    (
        ErrorKind::Protocol,
        &[
            "bad command",
            "invalid response",
            "parse error",
            "unexpected response",
            "protocol error",
            "malformed",
        ],
    ),
];

/// Classify an error message; case-insensitive substring matching, first
/// match wins, unknown or empty messages are `Internal`.
pub fn classify_error(message: &str) -> ErrorKind {
    let lower = message.to_lowercase();
    for (kind, needles) in MATCHERS {
        if needles.iter().any(|needle| lower.contains(needle)) {
            return *kind;
        }
    }
    ErrorKind::Internal
}

#[cfg(test)]
mod tests {
    use super::{classify_error, ErrorKind};

    #[test]
    fn classifies_each_kind() {
        assert_eq!(
            classify_error("IMAP error: authentication failed for user"),
            ErrorKind::Auth
        );
        assert_eq!(
            classify_error("Server replied: OVERQUOTA only 42 MB left"),
            ErrorKind::Throttled
        );
        assert_eq!(classify_error("operation timed out"), ErrorKind::Timeout);
        assert_eq!(
            classify_error("connection reset by peer"),
            ErrorKind::Network
        );
        assert_eq!(
            classify_error("bad command or sequence number"),
            ErrorKind::Protocol
        );
        assert_eq!(classify_error("unexpected internal bug"), ErrorKind::Internal);
    }

    #[test]
    fn auth_takes_precedence_over_throttling() {
        assert_eq!(
            classify_error("authentication failed after too many connections"),
            ErrorKind::Auth
        );
    }

    #[test]
    fn throttling_takes_precedence_over_network_noise() {
        assert_eq!(
            classify_error("service temporarily unavailable due to rate limit"),
            ErrorKind::Throttled
        );
    }

    #[test]
    fn unknown_and_empty_messages_are_internal() {
        assert_eq!(classify_error(""), ErrorKind::Internal);
        assert_eq!(classify_error("something odd happened"), ErrorKind::Internal);
    }

    #[test]
    fn label_values_match_the_metric_contract() {
        for (kind, label) in [
            (ErrorKind::Auth, "auth"),
            (ErrorKind::Throttled, "throttled"),
            (ErrorKind::Network, "network"),
            (ErrorKind::Timeout, "timeout"),
            (ErrorKind::Protocol, "protocol"),
            (ErrorKind::Internal, "internal"),
        ] {
            assert_eq!(kind.as_str(), label);
        }
    }

    #[test]
    fn error_kind_serializes_as_lowercase() {
        assert_eq!(
            serde_json::to_string(&ErrorKind::Throttled).unwrap(),
            "\"throttled\""
        );
        assert_eq!(
            serde_json::from_str::<ErrorKind>("\"auth\"").unwrap(),
            ErrorKind::Auth
        );
        assert_eq!(ErrorKind::default(), ErrorKind::Internal);
    }
}
