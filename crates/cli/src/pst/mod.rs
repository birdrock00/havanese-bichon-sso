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

use crate::api::sender::send_batch_request;
use crate::BichonCliConfig;
use bichon_core::import::pst::build_eml_base64;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input};
use outlook_pst::messaging::folder::Folder;
use outlook_pst::ndb::node_id::NodeId;
use reqwest::Client;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;

pub async fn handle_pst_import(config: &BichonCliConfig, account_id: u64, theme: &ColorfulTheme) {
    let path_str: String = Input::with_theme(theme)
        .with_prompt("Enter the path to your SINGLE .pst file")
        .validate_with(|input: &String| {
            let p = std::path::Path::new(input);
            if !p.exists() {
                return Err("The specified path does not exist.");
            }

            if !p.is_file() {
                return Err("PST mode requires a SINGLE file, not a directory.");
            }
            let is_pst = p
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("pst"))
                .unwrap_or(false);

            if !is_pst {
                return Err("The selected file must have a .pst extension.");
            }

            Ok(())
        })
        .interact_text()
        .unwrap();

    let pst_path = std::path::PathBuf::from(path_str);

    println!(
        "\n{} Ready to process PST file: {}",
        console::style("✔").green(),
        console::style(pst_path.display()).cyan()
    );

    if let Ok(meta) = std::fs::metadata(&pst_path) {
        let size_mb = meta.len() as f64 / 1024.0 / 1024.0;
        println!(
            "{}",
            console::style(format!("PST File Size: {:.1} MB", size_mb)).dim()
        );
    }

    if Confirm::with_theme(theme)
        .with_prompt("Start importing emails from this PST?")
        .default(true)
        .interact()
        .unwrap()
    {
        parse_pst(pst_path, config, account_id).await;
    } else {
        println!("{}", console::style("Operation cancelled by user.").red());
    }
}

async fn parse_pst(pst_path: PathBuf, config: &BichonCliConfig, account_id: u64) {
    let client = Client::new();

    let pst_store = match outlook_pst::open_store(&pst_path) {
        Ok(store) => store,
        Err(e) => {
            println!(
                "{} Failed to open PST file: {}",
                console::style("✘").red(),
                console::style(format!("{:#?}", e)).dim()
            );
            return;
        }
    };

    let ipm_sub_tree = match pst_store.properties().ipm_sub_tree_entry_id() {
        Ok(id) => id,
        Err(e) => {
            println!(
                "{} Could not find IPM_SUBTREE (Mailbox Root): {}",
                console::style("✘").red(),
                console::style(format!("{:#?}", e)).dim()
            );
            return;
        }
    };

    let ipm_subtree_folder = match pst_store.open_folder(&ipm_sub_tree) {
        Ok(folder) => folder,
        Err(e) => {
            println!(
                "{} Failed to open the root mailbox folder: {}",
                console::style("✘").red(),
                console::style(format!("{:#?}", e)).dim()
            );
            return;
        }
    };

    process_folder_recursively(&client, &ipm_subtree_folder, "", config, account_id).await;
}

fn process_folder_recursively<'a>(
    client: &'a Client,
    folder: &'a Rc<dyn Folder>,
    parent_path: &'a str,
    config: &'a BichonCliConfig,
    account_id: u64,
) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
    Box::pin(async move {
        let folder_name = folder
            .properties()
            .display_name()
            .unwrap_or_else(|_| "Unknown".to_string());

        let current_path = if parent_path.is_empty() {
            folder_name
        } else {
            format!("{}/{}", parent_path, folder_name)
        };

        println!(
            "{} {}",
            console::style("📁 Folder:").dim(),
            console::style(&current_path).cyan()
        );

        let mut emls_batch = Vec::new();

        if let Some(contents_table) = folder.contents_table() {
            for row in contents_table.rows_matrix() {
                let store = folder.store().clone();

                let entry_id = match store
                    .properties()
                    .make_entry_id(NodeId::from(u32::from(row.id())))
                {
                    Ok(id) => id,
                    Err(e) => {
                        eprintln!(
                            "  {} Skip row {}: {:?}",
                            console::style("⚠").yellow(),
                            row.unique(),
                            e
                        );
                        continue;
                    }
                };

                match store.open_message(&entry_id, None) {
                    Ok(message) => match build_eml_base64(message) {
                        Some(base64_eml) => emls_batch.push(base64_eml),
                        None => {}
                    },
                    Err(e) => eprintln!("  {} Open error: {:?}", console::style("⚠").yellow(), e),
                }

                if emls_batch.len() >= 50 {
                    let batch = emls_batch.clone();
                    emls_batch.clear();
                    send_to_bichon(client, config, account_id, &current_path, batch).await;
                }
            }
        }

        if !emls_batch.is_empty() {
            send_to_bichon(client, config, account_id, &current_path, emls_batch).await;
        }

        if let Some(hierarchy_table) = folder.hierarchy_table() {
            for row in hierarchy_table.rows_matrix() {
                let node = NodeId::from(u32::from(row.id()));
                if let Ok(entry_id) = folder.store().properties().make_entry_id(node) {
                    if let Ok(sub_folder) = folder.store().open_folder(&entry_id) {
                        process_folder_recursively(
                            client,
                            &sub_folder,
                            &current_path,
                            config,
                            account_id,
                        )
                        .await;
                    }
                }
            }
        }
    })
}

async fn send_to_bichon(
    client: &Client,
    config: &BichonCliConfig,
    account_id: u64,
    folder_path: &str,
    emls: Vec<String>,
) {
    send_batch_request(client, config, account_id, folder_path, emls).await;
}
