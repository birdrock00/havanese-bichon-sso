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

pub fn determine_folder(labels_raw: &str) -> String {
    let mut status_blacklist = HashSet::new();
    status_blacklist.insert("Opened");
    status_blacklist.insert("Unread");
    status_blacklist.insert("Archived");

    let all_labels: Vec<&str> = labels_raw
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if all_labels.is_empty() {
        return "Unknown".to_string();
    }

    let filtered: Vec<&str> = all_labels
        .iter()
        .filter(|&&l| !status_blacklist.contains(l))
        .cloned()
        .collect();

    match filtered.len() {
        // Case A: If all labels were status labels, fallback to the first original label
        0 => all_labels[0].to_string(),
        // Case B: If only one label remains, that's our target destination
        1 => filtered[0].to_string(),
        // Case C: Multiple labels remain (e.g., ["Inbox", "medium"])
        _ => {
            // Prioritize custom business labels by excluding generic locations like "Inbox" or "Sent"
            let business_label = filtered.iter().find(|&&l| l != "Inbox" && l != "Sent");

            match business_label {
                // Return the first non-generic label found
                Some(label) => label.to_string(),
                // If only generic labels remain (e.g., ["Sent", "Inbox"]), pick the first available
                None => filtered[0].to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mail_parser::{HeaderValue, MessageParser};

    use super::*;

    fn parse_x_gmail_labels(raw_message: &[u8]) -> Option<String> {
        // MessageParser::new() has an empty header_map so the hardcoded match at
        // parsers/header.rs:76 treats ALL unknown headers as raw (no RFC 2047
        // decoding).  We need three things to get decoding:
        // 1. A non-empty header_map (so the else branch runs)
        // 2. default_header_text() so the fallback fn is parse_unstructured
        // 3. OR register X-Gmail-Labels explicitly via header_text()
        let message = MessageParser::new()
            .with_minimal_headers()
            .default_header_text()
            .parse(raw_message)?;
        let value: &HeaderValue<'_> = message.header("X-Gmail-Labels")?;
        value.as_text().map(|s| s.to_string())
    }

    /// Construct a raw MIME message with RFC 2047 encoded X-Gmail-Labels,
    /// parse it, and verify the header is correctly decoded.
    fn build_email(x_gmail_labels: &str) -> Vec<u8> {
        format!(
            "From: sender@example.com\r\n\
             To: recipient@example.com\r\n\
             Subject: Test\r\n\
             X-Gmail-Labels: {}\r\n\
             \r\n\
             Body text here.\r\n",
            x_gmail_labels
        )
        .into_bytes()
    }

    #[test]
    fn rfc2047_encoded_labels_are_decoded() {
        // Exactly the format the user reported: French Gmail labels
        let raw = build_email("=?UTF-8?Q?Corbeille?=, =?UTF-8?Q?Messages_archiv=C3=A9s?=");
        let labels = parse_x_gmail_labels(&raw).expect("failed to parse X-Gmail-Labels");

        // mail-parser decodes RFC 2047 header values during initial parsing.
        // The decoded text should NOT contain raw =?UTF-8?Q?... sequences.
        assert!(!labels.contains("=?UTF-8"), "labels still encoded: {labels:?}");
        assert!(labels.contains("Corbeille"), "missing 'Corbeille': {labels:?}");
        assert!(
            labels.contains("archivés"),
            "missing decoded 'archivés': {labels:?}",
        );

        // Full pipeline: decoded labels → determine_folder
        let folder = determine_folder(&labels);
        assert_eq!(folder, "Corbeille");
    }

    #[test]
    fn plain_ascii_labels_passthrough() {
        let raw = build_email("Inbox, Important");
        let labels = parse_x_gmail_labels(&raw).expect("failed to parse X-Gmail-Labels");
        assert_eq!(labels, "Inbox, Important");
        assert_eq!(determine_folder(&labels), "Important");
    }

    #[test]
    fn missing_x_gmail_labels_header() {
        let raw = b"From: sender@example.com\r\nTo: r@example.com\r\n\r\nBody.\r\n";
        let message = MessageParser::new().parse(raw.as_slice()).unwrap();
        assert!(message.header("X-Gmail-Labels").is_none());
    }
}
