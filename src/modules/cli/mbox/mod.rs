use std::collections::HashMap;
use std::path::PathBuf;

use crate::base64_encode_url_safe;
use crate::modules::cli::mbox::gmail::determine_folder;
use crate::modules::cli::mbox::reader::MboxFile;
use crate::modules::cli::sender::send_batch_request;
use crate::modules::cli::BichonCtlConfig;
use console::style;
use dialoguer::{theme::ColorfulTheme, Input};
use dialoguer::{Confirm, Select};
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
    let mbox = MboxFile::from_file(mbox_path).unwrap();

    let mut folder_buffers: HashMap<String, Vec<String>> = HashMap::new();
    let batch_limit = 50;

    println!("Starting import process...");

    for e in mbox.iter() {
        let body = e.data;
        let message = MessageParser::new().parse(body).unwrap();

        let folder_name = match target_folder {
            Some(ref folder_name) => folder_name.clone(),
            None => {
                let labels = message
                    .header("X-Gmail-Labels")
                    .and_then(|h| h.as_text())
                    .unwrap_or("Inbox");
                determine_folder(labels)
            }
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
