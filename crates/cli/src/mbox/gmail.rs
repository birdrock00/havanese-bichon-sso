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
