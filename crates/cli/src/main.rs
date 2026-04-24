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

use bichon_core::bichon_version;
use clap::Parser;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::{
    auth::verify_user_and_get_account, eml::handle_eml_directory_import,
    export::handle_account_export, mbox::handle_mbox_single_file_import, pst::handle_pst_import,
    thunderbird::handle_thunderbird_import,
};

pub mod api;
pub mod auth;
pub mod eml;
pub mod export;
pub mod mbox;
pub mod pst;
pub mod thunderbird;

#[derive(Parser, Debug)]
#[command(
    name = "bichonctl",
    author = "rustmailer",
    version = bichon_version!(),
    about = "A CLI tool to import email data into Bichon service"
)]
pub struct BichonCli {
    /// Path to the configuration file
    #[arg(
        short,
        long,
        default_value = "config.toml",
        value_name = "FILE",
        help = "Sets a custom config file"
    )]
    pub config: std::path::PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BichonCtlConfig {
    pub base_url: String,
    pub api_token: String,
}

#[tokio::main]
async fn main() {
    let cli = BichonCli::parse();
    let theme = ColorfulTheme::default();
    let config_path = &cli.config;
    let mut current_config: Option<BichonCtlConfig> = None;

    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(config_path) {
            if let Ok(config) = toml::from_str::<BichonCtlConfig>(&content) {
                println!("{}", style("✔ Existing configuration found:").green());
                println!("  Base URL: {}", style(&config.base_url).yellow());
                println!(" API Token: {}", style(&config.api_token).yellow());

                // Confirm with user
                if Confirm::with_theme(&theme)
                    .with_prompt("Do you want to use this configuration?")
                    .default(true)
                    .interact()
                    .unwrap()
                {
                    current_config = Some(config);
                }
            }
        }
    }

    let final_config = match current_config {
        Some(conf) => conf,
        None => {
            println!("\n{}", style("Please enter Bichon service details:").bold());

            let url: String = Input::with_theme(&theme)
                .with_prompt("Bichon Base URL")
                .default("http://localhost:15630".into())
                .interact_text()
                .unwrap();

            let token: String = Input::with_theme(&theme)
                .with_prompt("API Token")
                .interact_text()
                .unwrap();

            let conf = BichonCtlConfig {
                base_url: url,
                api_token: token,
            };

            // 3. Offer to save the new configuration
            if Confirm::with_theme(&theme)
                .with_prompt("Save this configuration for future use?")
                .default(true)
                .interact()
                .unwrap()
            {
                let toml_str = toml::to_string(&conf).unwrap();
                fs::write(config_path, toml_str).expect("Failed to save config file");
                println!("{}", style("Configuration saved successfully!").green());
            }
            conf
        }
    };

    let operations = &[
        "1. Import: Upload email data to Bichon",
        "2. Export: Download account data as MBOX file",
    ];

    let op_idx = Select::with_theme(&theme)
        .with_prompt("Select operation")
        .items(operations)
        .default(0)
        .interact()
        .unwrap();

    match op_idx {
        0 => {
            let target_account = verify_user_and_get_account(&final_config, &theme, true).await;

            let import_modes = &[
                "1. EML: Scan directory recursively (Maintains folder structure)",
                "2. MBOX: Single archive file (Stream from one file)",
                "3. Thunderbird: Import from local profile directory",
                "4. PST: Outlook Personal Storage (Single .pst file)",
            ];

            let mode_idx = Select::with_theme(&theme)
                .with_prompt("Select import method")
                .items(import_modes)
                .default(0)
                .interact()
                .unwrap();

            match mode_idx {
                0 => handle_eml_directory_import(&final_config, target_account.id, &theme).await,
                1 => handle_mbox_single_file_import(&final_config, target_account.id, &theme).await,
                2 => handle_thunderbird_import(&final_config, target_account.id, &theme).await,
                3 => handle_pst_import(&final_config, target_account.id, &theme).await,
                _ => unreachable!(),
            }
        }
        1 => {
            let target_account = verify_user_and_get_account(&final_config, &theme, false).await;
            handle_account_export(&final_config, target_account, &theme).await;
        }
        _ => unreachable!(),
    }
}
