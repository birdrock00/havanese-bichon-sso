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

use std::{collections::HashMap, path::PathBuf};

use crate::{mbox::run_import, BichonCtlConfig};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};

pub async fn handle_thunderbird_import(
    config: &BichonCtlConfig,
    account_id: u64,
    theme: &ColorfulTheme,
) {
    let root_str: String = Input::with_theme(theme)
        .with_prompt("Enter your Thunderbird Mail/ImapMail directory")
        .validate_with(|input: &String| {
            let p = std::path::Path::new(input);
            if p.exists() && p.is_dir() {
                Ok(())
            } else {
                Err("Directory not found.")
            }
        })
        .interact_text()
        .unwrap();

    let root_path = std::path::PathBuf::from(&root_str);
    println!("{}", style("🔍 Scanning Thunderbird structure...").dim());

    let mut mbox_tasks: HashMap<String, PathBuf> = HashMap::new();

    fn scan_thunderbird_dir(
        root: &std::path::Path,
        current: &std::path::Path,
        tasks: &mut HashMap<String, PathBuf>,
    ) {
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                if path.is_dir() {
                    scan_thunderbird_dir(root, &path, tasks);
                } else {
                    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    match extension {
                        "msf" | "dat" | "html" | "json" | "txt" | "sqlite" => continue,
                        _ => {}
                    }

                    if file_name == "filterlog.html" || file_name == "msgFilterRules.dat" {
                        continue;
                    }

                    if !extension.is_empty() {
                        continue;
                    }

                    if let Ok(rel) = path.strip_prefix(root) {
                        let mailbox = rel.to_string_lossy().replace(".sbd", "").replace('\\', "/");
                        tasks.insert(mailbox, path);
                    }
                }
            }
        }
    }

    scan_thunderbird_dir(&root_path, &root_path, &mut mbox_tasks);
    if mbox_tasks.is_empty() {
        println!(
            "{}",
            style("No mailboxes found in the specified directory.").yellow()
        );
        return;
    }

    println!("\n{}", style("🔍 Scanned Mailboxes:").bold().underlined());
    let mut sorted_keys: Vec<_> = mbox_tasks.keys().collect();
    sorted_keys.sort();

    for name in &sorted_keys {
        let path = &mbox_tasks[*name];
        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let size_mb = file_size as f64 / 1024.0 / 1024.0;

        println!(
            "  {} {} ({:.2} MB)",
            style("•").dim(),
            style(name).cyan(),
            size_mb
        );
    }

    println!();
    let prompt = format!("Ready to import {} mailboxes. Proceed?", mbox_tasks.len());
    if Confirm::with_theme(theme)
        .with_prompt(prompt)
        .default(true)
        .interact()
        .unwrap()
    {
        for (mailbox_name, mbox_file) in mbox_tasks {
            println!("\n🚀 Importing: {}", style(&mailbox_name).cyan().bold());
            run_import(account_id, &mbox_file, config, Some(mailbox_name)).await;
        }
        println!(
            "\n{}",
            style("✨ All mailboxes imported successfully!")
                .green()
                .bold()
        );
    } else {
        println!("{}", style("Import cancelled.").yellow());
    }
}
