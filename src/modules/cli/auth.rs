use std::process;

use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
use reqwest::Client;

use crate::modules::{
    account::payload::MinimalAccount,
    cli::BichonCtlConfig,
    users::{permissions::Permission, view::UserView},
};

pub async fn verify_user_and_get_account(config: &BichonCtlConfig, theme: &ColorfulTheme) -> u64 {
    let client = Client::new();
    let url = format!("{}/api/v1/current-user", config.base_url);

    let response = match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            eprintln!(
                "\n{} {}",
                style("✘ Network Error:").red().bold(),
                "Could not connect to Bichon service."
            );
            eprintln!("{} {}", style("Details:").dim(), e);
            eprintln!(
                "\n{} Please check if the Base URL is correct and the server is running.",
                style("Tip:").cyan()
            );
            process::exit(1);
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "No error detail provided".to_string());

        eprintln!(
            "\n{} Server returned an error (Status: {})",
            style("✘ API Error:").red().bold(),
            style(status).yellow()
        );

        if status == 401 {
            eprintln!(
                "{} Your API Token seems to be invalid or expired.",
                style("Context:").dim()
            );
        } else if status == 404 {
            eprintln!(
                "{} The endpoint was not found. Please check your Base URL.",
                style("Context:").dim()
            );
        }

        eprintln!("{} {}", style("Response:").dim(), error_body);
        process::exit(1);
    }

    let user: UserView = response.json().await.expect("Failed to parse user data");
    println!("Welcome, {}!", style(&user.username).cyan());

    let account_list_url = format!(
        "{}/api/v1/minimal-account-list?only_nosync=true",
        config.base_url
    );
    let acc_response = client
        .get(&account_list_url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .await
        .expect("Failed to fetch account list");

    if !acc_response.status().is_success() {
        panic!(
            "Failed to retrieve accounts. Status: {}",
            acc_response.status()
        );
    }

    let accounts: Vec<MinimalAccount> = acc_response
        .json()
        .await
        .expect("Failed to parse minimal account list");

    if accounts.is_empty() {
        println!(
            "\n{}",
            style("Error: No 'nosync' accounts found.").red().bold()
        );
        println!(
            "{}",
            style("Mail import is only supported for 'nosync' type accounts.").dim()
        );
        println!(
            "Please create a new {} account in the Bichon web interface first.",
            style("Nosync").bold().yellow()
        );
        process::exit(1);
    }
    let required_permission = Permission::DATA_IMPORT_BATCH;
    let mut selectable_accounts = Vec::new();
    let mut options = Vec::new();

    for acc in accounts {
        let has_permission = if let Some(perms) = user.account_permissions.get(&acc.id) {
            perms.iter().any(|p| p == required_permission)
        } else {
            user.global_permissions
                .iter()
                .any(|p| p == Permission::DATA_MANAGE_ALL || p == Permission::ROOT)
        };

        let status_prefix = if has_permission {
            style(" [READY] ").green()
        } else {
            style(" [NO PERMISSION] ").red()
        };

        options.push(format!(
            "{}{} - {}",
            status_prefix,
            style(&acc.email).bold(),
            style(format!("ID: {}", acc.id)).dim()
        ));

        selectable_accounts.push((acc, has_permission));
    }

    let selection = Select::with_theme(theme)
        .with_prompt("Select the target account for import")
        .items(&options)
        .default(0)
        .max_length(10)
        .interact()
        .unwrap();

    let (selected_acc, can_import) = &selectable_accounts[selection];

    if !*can_import {
        eprintln!(
            "\n{} You do not have '{}' permission for account {}.",
            style("✘ Permission Denied:").red().bold(),
            style(required_permission).yellow(),
            style(&selected_acc.email).cyan()
        );
        eprintln!(
            "{} Please contact your administrator to upgrade your role for this account.",
            style("Tip:").dim()
        );
        process::exit(1);
    }

    println!(
        "{} Targeting account: {}",
        style("✔").green(),
        style(&selected_acc.email).cyan().bold()
    );

    selected_acc.id
}
