use std::{
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use bichon::modules::{
    cli::admin::meta::{find_admin, init_meta_database, update_admin_password},
    error::BichonError,
    utils::encrypt::internal_decrypt_string,
};
use console::{style, Emoji};
use dialoguer::Confirm;
use dialoguer::{theme::ColorfulTheme, Input, Password, Select};
use native_db::Database;

#[tokio::main]
async fn main() {
    let theme = ColorfulTheme::default();
    println!(
        "\n{}\n",
        style("BICHON ADMINISTRATIVE TOOL").bold().bright().cyan()
    );

    let main_options = vec!["Reset Admin Password", "Exit"];
    let selection = Select::with_theme(&theme)
        .with_prompt("Select an operation")
        .default(0)
        .items(&main_options)
        .interact()
        .unwrap();

    if selection == 1 {
        println!("{}", style("Exiting...").dim());
        return;
    }

    let root_dir_str: String = Input::with_theme(&theme)
        .with_prompt("Enter the absolute path for 'bichon_root_dir'")
        .validate_with(|input: &String| -> Result<(), &str> {
            let path = Path::new(input);
            if !path.is_absolute() {
                return Err("Path must be absolute.");
            }
            if !path.exists() {
                return Err("Directory does not exist.");
            }
            let has_metadata = path.join("meta.db").exists();
            if !has_metadata {
                return Err("Invalid directory: 'meta.db' not found.");
            }
            Ok(())
        })
        .interact_text()
        .unwrap();

    let root_path = PathBuf::from(&root_dir_str);

    let database: Rc<Database<'static>> = match init_meta_database(&root_path.join("meta.db")) {
        Ok(database) => database,
        Err(e) => match e {
            BichonError::Generic {
                message,
                location,
                code,
            } => {
                if message.contains("RedbDatabaseError(DatabaseAlreadyOpen") {
                    println!("\n{}", style("ERROR: Database is locked.").red().bold());
                    println!(
                        "{}",
                        style("The Bichon service is likely still running.").yellow()
                    );
                    println!(
                        "Since the database cannot be shared between multiple instances, \n\
                     you must {} the Bichon service before proceeding.",
                        style("STOP").underlined().bold()
                    );
                    std::process::exit(1);
                } else {
                    eprintln!(
                        "\n{} (Code: {:#?})\nLocation: {}\nMessage: {}",
                        style("A database error occurred:").red().bold(),
                        code,
                        location,
                        message
                    );
                    std::process::exit(1);
                }
            }
        },
    };

    let admin = find_admin(&database);

    match admin {
        Ok(Some(user)) => {
            println!("\n{}", style("Admin user found:").green().bold());
            println!("----------------------------------------");
            println!("{:<12} : {}", "Username", style(&user.username).cyan());
            println!("{:<12} : {}", "Email", style(&user.email).cyan());
        }
        Ok(None) => {
            println!(
                "\n{}",
                style("ERROR: No admin user found in the database.")
                    .red()
                    .bold()
            );
            println!("Please ensure the system has been initialized correctly.");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!(
                "\n{} Failed to query admin user.",
                style("ERROR:").red().bold()
            );
            eprintln!("Details: {:?}", e);
            std::process::exit(1);
        }
    }

    let encryption_key = loop {
        let auth_methods = vec![
            "Enter encryption password manually",
            "Read from password file",
        ];
        let method = Select::with_theme(&theme)
            .with_prompt("How would you like to provide the database encryption key?")
            .items(&auth_methods)
            .interact()
            .unwrap();

        let raw_key = if method == 0 {
            Password::with_theme(&theme)
                .with_prompt("Enter Encryption Password")
                .interact()
                .unwrap()
        } else {
            let file_path: String = Input::with_theme(&theme)
                .with_prompt("Enter path to encryption password file")
                .interact_text()
                .unwrap();

            match fs::read_to_string(&file_path) {
                Ok(content) => content.trim().to_string(),
                Err(e) => {
                    println!("{}: {}", style("Failed to read file").red(), e);
                    continue;
                }
            }
        };

        if raw_key.is_empty() {
            println!("{}", style("Key cannot be empty.").red());
            continue;
        }

        let prompt_message = format!(
            "Encryption key loaded: [ {} ]\n\n  \
                {}: This key must match the database encryption key used by the server.\n  \
                It corresponds to these settings in your service:\n    \
                - Arguments: {} or {}\n    \
                - Envs:      {} or {}\n\n  \
                Do you want to continue?",
            style(&raw_key).cyan().bold(),
            style("IMPORTANT").yellow().bold(),
            style("--bichon_encrypt_password").italic(),
            style("--bichon_encrypt_password_file").italic(),
            style("BICHON_ENCRYPT_PASSWORD").green(),
            style("BICHON_ENCRYPT_PASSWORD_FILE").green()
        );

        if Confirm::with_theme(&theme)
            .with_prompt(prompt_message)
            .default(true)
            .interact()
            .unwrap()
        {
            break raw_key;
        }
    };

    let admin = find_admin(&database);

    match admin {
        Ok(Some(user)) => {
            println!("----------------------------------------");
            println!("{:<12} : {}", "Username", style(&user.username).cyan());
            println!("{:<12} : {}", "Email", style(&user.email).cyan());

            let pwd_display = match &user.password {
                Some(p) => {
                    let password = match internal_decrypt_string(&encryption_key, p) {
                        Ok(p) => p,
                        Err(e) => {
                            println!("\n{}", style("ERROR: Decryption Failed").red().bold());
                            println!(
                                "{}",
                                style("The provided encryption key is incorrect or invalid for this database.").yellow()
                            );
                            println!(
                                "{} Please verify your {} or the {} you provided.",
                                style("➔").cyan(),
                                style("encryption password").bold(),
                                style("key file").bold()
                            );
                            eprintln!("\nTechnical details: {:?}", e);
                            std::process::exit(1);
                        }
                    };
                    style(password).yellow().to_string()
                }
                None => style("None (No password set)").dim().italic().to_string(),
            };

            println!("{:<12} : {}", "Password", pwd_display);
            println!("----------------------------------------");

            if !dialoguer::Confirm::with_theme(&theme)
                .with_prompt(format!(
                    "Do you want to reset the password for '{}'?",
                    user.username
                ))
                .interact()
                .unwrap()
            {
                println!("Operation cancelled.");
                return;
            }
        }
        Ok(None) => {
            println!(
                "\n{}",
                style("ERROR: No admin user found in the database.")
                    .red()
                    .bold()
            );
            println!("Please ensure the system has been initialized correctly.");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!(
                "\n{} Failed to query admin user.",
                style("ERROR:").red().bold()
            );
            eprintln!("Details: {:?}", e);
            std::process::exit(1);
        }
    }

    println!(
        "\n{}",
        style("TARGET: Reset password for user 'admin'")
            .yellow()
            .bold()
    );

    let new_login_password = Password::with_theme(&theme)
        .with_prompt("Enter new Admin Login Password")
        .with_confirmation("Repeat password to confirm", "Passwords do not match!")
        .interact()
        .unwrap();

    if !Confirm::with_theme(&theme)
        .with_prompt("Proceed with database update?")
        .interact()
        .unwrap()
    {
        return;
    }

    println!("\n{} {}", style("⌛").yellow(), "Updating database...");

    match update_admin_password(&database, new_login_password, &encryption_key) {
        Ok(_) => {
            println!(
                "\n{} {}",
                Emoji("✨", "*"),
                style("Success! Admin password has been updated.")
                    .green()
                    .bold()
            );
            println!(
                "{}",
                style("You can now log in with the new password.").dim()
            );
        }
        Err(e) => {
            println!(
                "\n{}",
                style("ERROR: Failed to update database").red().bold()
            );
            eprintln!(
                "{} Could not save the new password to the database.",
                style("➔").cyan()
            );
            eprintln!("\nDetails: {:?}", e);
            std::process::exit(1);
        }
    }
}
