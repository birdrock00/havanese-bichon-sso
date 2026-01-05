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
