use crate::api::download::download_and_export_with_json_header;
use crate::api::search::search_messages;
use crate::api::stats::fetch_account_stats;
use crate::BichonCtlConfig;
use bichon_core::account::payload::MinimalAccount;
use console::style;
use dialoguer::Confirm;
use dialoguer::{theme::ColorfulTheme, Input};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::path::{Path, PathBuf};
use sysinfo::Disks;

pub async fn handle_account_export(
    config: &BichonCtlConfig,
    account: MinimalAccount,
    theme: &ColorfulTheme,
) {
    let client = Client::new();

    println!("Fetching account statistics...");
    let stats = match fetch_account_stats(&client, config, account.id).await {
        Some(s) => s,
        None => {
            eprintln!("{} Failed to fetch account statistics.", style("✘").red());
            return;
        }
    };

    println!("\n--- Account Statistics ---");
    println!("  Total Emails: {}", style(stats.total_count).cyan());
    println!(
        "  Total Size:   {}",
        style(format_bytes(stats.total_size)).cyan()
    );

    let path = loop {
        let input: String = Input::with_theme(theme)
            .with_prompt("Enter ABSOLUTE directory path for MBOX file")
            .interact_text()
            .unwrap();

        let p = PathBuf::from(&input);

        if !p.is_absolute() {
            eprintln!(
                " {} {}",
                style("✘").red(),
                style("Invalid path: Must be an absolute path.").red()
            );
            continue;
        }

        if !p.exists() {
            eprintln!(
                " {} {}",
                style("✘").red(),
                style("Invalid path: Directory does not exist.").red()
            );
            continue;
        }

        if !p.is_dir() {
            eprintln!(
                " {} {}",
                style("✘").red(),
                style("Invalid path: The path provided is not a directory.").red()
            );
            continue;
        }
        break p;
    };

    let disks = Disks::new_with_refreshed_list();
    let disk_result = disks
        .list()
        .iter()
        .find(|d| path.starts_with(d.mount_point()))
        .ok_or_else(|| "Could not identify the disk for the provided path.");

    match disk_result {
        Ok(disk) => {
            let free_space = disk.available_space();
            let required_space = (stats.total_size as f64 * 1.2) as u64;

            if free_space < required_space {
                eprintln!(
                " {} Insufficient disk space (including 10% safety buffer)!\n   Required:  {} (Base: {})\n   Available: {}",
                style("✘").red(),
                style(format_bytes(required_space)).yellow(),
                style(format_bytes(stats.total_size)).yellow(),
                style(format_bytes(free_space)).yellow()
            );
                return;
            }

            println!(
                " {} Disk space check passed. (Required: {}, Available: {})",
                style("✔").green(),
                style(format_bytes(required_space)).cyan(),
                style(format_bytes(free_space)).cyan()
            );
        }
        Err(e) => {
            eprintln!(" {} {}", style("✘").red(), style(e).red());
            return;
        }
    }
    let mbox_file = get_unique_mbox_path(&path, account.id, &account.email);
    if Confirm::with_theme(theme)
        .with_prompt(format!(
            "Export {} emails to '{}'?",
            stats.total_count,
            mbox_file.display()
        ))
        .default(true)
        .interact()
        .unwrap()
    {
        println!(" {} Starting export...", style("✔").green());

        let pb = ProgressBar::new(stats.total_count as u64);
        pb.set_style(ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}"
        ).unwrap());

        let mut file = match tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&mbox_file)
            .await
        {
            Ok(f) => f,
            Err(e) => {
                eprintln!(" ✘ Failed to open file '{}': {}", path.display(), e);
                return;
            }
        };

        let page_size = 100;
        let mut current_page = 1;
        let mut total_pages;

        loop {
            if let Some(batch) = search_messages(&client, config, current_page, page_size).await {
                total_pages = batch.total_pages.unwrap();

                println!(" → Processing page {}/{}", current_page, total_pages);

                for envelope in batch.items {
                    let success =
                        download_and_export_with_json_header(&client, config, envelope, &mut file)
                            .await;

                    if !success {
                        eprintln!(" ✘ Failed to export an email. Aborting process...");
                        return;
                    }
                    pb.inc(1);
                }
                if current_page >= total_pages {
                    break;
                }
                current_page += 1;
            } else {
                eprintln!(
                    " ✘ Failed to fetch page {}. Aborting process...",
                    current_page
                );
                return;
            }
        }
        pb.finish();
        println!(" {} Export complete!", style("✔").green());
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{:.2} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes / 1024)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes / 1024 / 1024)
    } else {
        format!("{:.2} GB", bytes / 1024 / 1024 / 1024)
    }
}

fn get_unique_mbox_path(base_dir: &Path, account_id: u64, email: &str) -> PathBuf {
    let email_part = email.replace(' ', "_");

    let mut base_name = format!("account_{}_{}", account_id, email_part);
    if base_name.starts_with('.') {
        base_name = format!("_{}", base_name);
    }

    let mut final_path = base_dir.join(format!("{}.mbox", base_name));

    let mut counter = 1;
    while final_path.exists() {
        final_path = base_dir.join(format!("{}_{}.mbox", base_name, counter));
        counter += 1;
    }
    final_path
}
