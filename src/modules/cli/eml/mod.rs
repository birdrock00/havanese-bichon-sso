use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use console::style;
use dialoguer::{theme::ColorfulTheme, Input};
use mail_parser::MessageParser;
use reqwest::Client;

use crate::{
    base64_encode_url_safe,
    modules::cli::{sender::send_batch_request, BichonCtlConfig},
};

pub async fn handle_eml_directory_import(
    config: &BichonCtlConfig,
    account_id: u64,
    theme: &ColorfulTheme,
) {
    let root_str: String = Input::with_theme(theme)
        .with_prompt("Enter the ROOT directory to scan for .eml files")
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

    let root_path = std::path::PathBuf::from(root_str);
    let mut tasks: HashMap<String, Vec<PathBuf>> = HashMap::new();
    println!(
        "{}",
        style("🔍 Scanning recursively using std::fs...").dim()
    );
    if let Err(e) = scan_dir(&root_path, &root_path, &mut tasks) {
        eprintln!("Error scanning directory: {}", e);
        return;
    }

    if tasks.is_empty() {
        println!("{}", style("No .eml files found.").yellow());
    } else {
        process_and_upload(config, account_id, tasks).await;
    }
}

fn scan_dir(
    root: &Path,
    current: &Path,
    tasks: &mut HashMap<String, Vec<PathBuf>>,
) -> std::io::Result<()> {
    if current.is_dir() {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                scan_dir(root, &path, tasks)?;
            } else if path.is_file() {
                if path.extension().and_then(|s| s.to_str()) == Some("eml") {
                    let rel_path = path.strip_prefix(root).unwrap_or(Path::new(""));
                    let mailbox_name = rel_path
                        .parent()
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default();
                    let folder = if mailbox_name.is_empty() {
                        "Inbox".to_string()
                    } else {
                        mailbox_name
                    };
                    tasks.entry(folder).or_insert_with(|| Vec::new()).push(path);
                }
            }
        }
    }
    Ok(())
}

async fn process_and_upload(
    config: &BichonCtlConfig,
    account_id: u64,
    tasks: HashMap<String, Vec<PathBuf>>,
) {
    let client = Client::new();
    let batch_size = 50;

    for (mailbox, files) in tasks {
        println!("\n🚀 Processing mailbox: {}", style(&mailbox).cyan().bold());

        let mut current_batch = Vec::new();

        for file_path in files {
            let body = match fs::read(&file_path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!(
                        "  {} Failed to read file {:?}: {}",
                        style("✘").red(),
                        file_path,
                        e
                    );
                    continue;
                }
            };

            if MessageParser::new().parse(&body).is_some() {
                let b64_content = base64_encode_url_safe!(&body);
                current_batch.push(b64_content);

                if current_batch.len() >= batch_size {
                    let to_send = current_batch;
                    current_batch = Vec::with_capacity(batch_size);
                    send_batch_request(&client, config, account_id, &mailbox, to_send).await;
                }
            } else {
                eprintln!(
                    "  {} Invalid format, skipping: {:?}",
                    style("⚠").yellow(),
                    file_path
                );
            }
        }
        if !current_batch.is_empty() {
            send_batch_request(&client, config, account_id, &mailbox, current_batch).await;
        }
    }
}
