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

use std::collections::HashMap;
use std::path::PathBuf;

use crate::api::sender::send_batch_request;
use crate::mbox::gmail::determine_folder;
use crate::mbox::reader::MboxFile;
use crate::BichonCtlConfig;
use bichon_core::base64_encode_url_safe;
use bichon_core::envelope::meta::{parse_bichon_metadata, BichonMetadata};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input};
use dialoguer::{Confirm, Select};
use mail_parser::parsers::MessageStream;
use mail_parser::MessageParser;
use reqwest::Client;

pub mod gmail;
pub mod reader;

pub async fn handle_mbox_single_file_import(
    config: &BichonCtlConfig,
    account_id: u64,
    theme: &ColorfulTheme,
) {
    let path_str: String = Input::with_theme(theme)
        .with_prompt("Enter the path to your SINGLE .mbox file")
        .validate_with(|input: &String| {
            let p = std::path::Path::new(input);
            if !p.exists() {
                return Err("The specified path does not exist.");
            }
            if !p.is_file() {
                return Err("MBOX mode requires a SINGLE file, not a directory.");
            }
            Ok(())
        })
        .interact_text()
        .unwrap();

    let mbox_path = PathBuf::from(path_str);

    let options = vec![
        "Use labels from mail headers (X-Gmail-Labels)",
        "Specify a single target folder for all emails",
        "Use X-Bichon-Metadata header (Automatic)",
    ];

    let selection = Select::with_theme(theme)
        .with_prompt("How should we determine the target folder?")
        .items(&options)
        .default(0)
        .interact()
        .unwrap();

    let target_folder: Option<String> = match selection {
        0 => None,
        1 => {
            let folder: String = Input::with_theme(theme)
                .with_prompt("Target folder name")
                .default("INBOX".into())
                .interact_text()
                .unwrap();
            Some(folder)
        }
        2 => None,
        _ => unreachable!(),
    };

    if let Some(ref folder) = target_folder {
        println!(
            "{}",
            style(format!("Mode: Fixed folder ({})", folder)).dim()
        );
    } else {
        println!("{}", style("Mode: Dynamic (header-based)").dim());
    }

    println!(
        "\n{} Ready to process MBOX file: {}",
        style("✔").green(),
        style(mbox_path.display()).cyan()
    );

    if let Ok(meta) = std::fs::metadata(&mbox_path) {
        let size_mb = meta.len() as f64 / 1024.0 / 1024.0;
        println!(
            "{}",
            style(format!("Processing file: {:.1} MB", size_mb)).dim()
        );
    }

    if Confirm::with_theme(theme)
        .with_prompt("Start importing?")
        .default(true)
        .interact()
        .unwrap()
    {
        run_import(account_id, &mbox_path, config, target_folder).await
    }
}

pub async fn run_import(
    account_id: u64,
    mbox_path: &PathBuf,
    config: &BichonCtlConfig,
    target_folder: Option<String>,
) {
    let client = Client::new();
    let mbox = match MboxFile::from_file(mbox_path) {
        Ok(mbox) => mbox,
        Err(err) => {
            println!("Skipping invalid MBOX: {} ({})", mbox_path.display(), err);
            return;
        }
    };

    let mut folder_buffers: HashMap<String, Vec<String>> = HashMap::new();
    let batch_limit = 50;

    println!("Starting import process...");

    for (index, e) in mbox.iter().enumerate() {
        let msg_num = index + 1;
        let body = e.data;
        let message = match MessageParser::new().parse(body) {
            Some(msg) => msg,
            None => {
                eprintln!(
                    "{} {}: {}",
                    style("Warning").yellow().bold(),
                    style(format!("at message #{}", msg_num)).dim(),
                    "Failed to parse email structure. Skipping..."
                );
                continue;
            }
        };

        let mut metadata: Option<BichonMetadata> = None;
        if let Some(meta_header) = message.header_raw("X-Bichon-Metadata") {
            metadata = parse_bichon_metadata(meta_header);
        }

        let get_default_folder = || {
            let gmail_labels = message.header_raw("X-Gmail-Labels").unwrap_or("INBOX");
            let text_cow = MessageStream::new(gmail_labels.as_bytes())
                .parse_unstructured()
                .into_text();
            let data: &str = match &text_cow {
                Some(c) => c.as_ref(),
                None => "INBOX",
            };
            determine_folder(data)
        };

        let folder_name = if let Some(ref folder) = target_folder {
            folder.clone()
        } else if let Some(ref meta) = metadata {
            meta.mailbox_name.clone().unwrap_or_else(get_default_folder)
        } else {
            get_default_folder()
        };

        let b64_eml = base64_encode_url_safe!(&body);
        let buffer = folder_buffers
            .entry(folder_name.clone())
            .or_insert_with(|| Vec::new());
        buffer.push(b64_eml);

        if buffer.len() >= batch_limit {
            let emls_to_send = folder_buffers.remove(&folder_name).unwrap();
            send_batch_request(&client, config, account_id, &folder_name, emls_to_send).await;
        }
    }

    for (folder_name, emls) in folder_buffers {
        if !emls.is_empty() {
            send_batch_request(&client, config, account_id, &folder_name, emls).await;
        }
    }

    println!("{}", style("Import completed successfully!").green().bold());
}
