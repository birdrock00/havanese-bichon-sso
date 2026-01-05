use bichon::modules::cli::{
    auth::verify_user_and_get_account, eml::handle_eml_directory_import,
    mbox::handle_mbox_single_file_import, thunderbird::handle_thunderbird_import, BichonCli,
    BichonCtlConfig,
};
use clap::Parser;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::fs;

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

    let target_account_id = verify_user_and_get_account(&final_config, &theme).await;

    let import_modes = &[
        "EML: Scan directory recursively (Maintains folder structure)",
        "MBOX: Single archive file (Stream from one file)",
        "Thunderbird: Import from local profile directory",
    ];

    let mode_idx = Select::with_theme(&theme)
        .with_prompt("Select import method")
        .items(import_modes)
        .default(0)
        .interact()
        .unwrap();

    match mode_idx {
        0 => handle_eml_directory_import(&final_config, target_account_id, &theme).await,
        1 => handle_mbox_single_file_import(&final_config, target_account_id, &theme).await,
        2 => handle_thunderbird_import(&final_config, target_account_id, &theme).await,
        _ => unreachable!(),
    }
}
