use std::path::{Path, PathBuf};

use bichon_core::migrate::{
    do_migrate, is_tantivy_index_dir,
    store::{LegacyDirs, NewDirs},
};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use indicatif::{ProgressBar, ProgressStyle};

pub fn handle_migration(theme: &ColorfulTheme) {
    println!(
        "\n{}",
        style("MIGRATION: Bichon v0.3.7 Storage Architecture → v1.0.0")
            .bold()
            .yellow()
    );

    println!(
        "{}",
        style(
            "This tool migrates data from the legacy v0.3.7 Tantivy-based storage \
            architecture to the new v1.0.0 \
            separated index and Fjall-backed storage format."
        )
        .dim()
    );

    println!(
        "{}",
        style(
            "Legacy v0.3.7 architecture:\n\
            • envelope metadata stored in Tantivy\n\
            • message data stored in Tantivy\n\n\
                New v1.0.0 architecture:\n\
            • mail indexes stored in Tantivy\n\
            • attachment indexes stored in Tantivy\n\
            • raw message data stored in Fjall\n\
            • attachment blobs stored in Fjall"
        )
        .dim()
    );

    println!(
        "\n{} {}",
        style("IMPORTANT:").yellow().bold(),
        style(
            "The paths below must exactly match what your old bichon server was configured with."
        )
        .yellow()
    );

    // --- bichon-root-dir ---
    let root_dir_str: String = Input::with_theme(theme)
        .with_prompt("Enter --bichon-root-dir (same value used by the old server)")
        .validate_with(|input: &String| -> Result<(), &str> {
            let path = Path::new(input);
            if !path.is_absolute() {
                return Err("Path must be absolute.");
            }
            if !path.exists() {
                return Err("Directory does not exist.");
            }
            Ok(())
        })
        .interact_text()
        .unwrap();

    let root_path = PathBuf::from(&root_dir_str);

    // --- bichon-index-dir ---
    let default_index = root_path.join("envelope");
    let default_new_index = root_path.join("bichon-indices");
    let index_dir_str: String = Input::with_theme(theme)
        .with_prompt(format!(
            "Enter --bichon-index-dir (leave blank to use default: {})",
            style(default_index.display()).cyan()
        ))
        .allow_empty(true)
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.is_empty() {
                return Ok(());
            }
            let path = Path::new(input);
            if !path.is_absolute() {
                return Err("Path must be absolute.");
            }

            if !path.exists() {
                return Err("Directory does not exist.");
            }
            Ok(())
        })
        .interact_text()
        .unwrap();

    let index_path = if index_dir_str.is_empty() {
        default_index
    } else {
        PathBuf::from(&index_dir_str)
    };

    let new_index_path = if index_dir_str.is_empty() {
        default_new_index
    } else {
        PathBuf::from(&index_dir_str).join("bichon-indices")
    };

    // --- bichon-data-dir ---
    let default_data = root_path.join("eml");
    let default_new_data = root_path.join("bichon-storage");
    let data_dir_str: String = Input::with_theme(theme)
        .with_prompt(format!(
            "Enter --bichon-data-dir (leave blank to use default: {})",
            style(default_data.display()).cyan()
        ))
        .allow_empty(true)
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.is_empty() {
                return Ok(());
            }
            let path = Path::new(input);
            if !path.is_absolute() {
                return Err("Path must be absolute.");
            }
            if !path.exists() {
                return Err("Directory does not exist.");
            }
            Ok(())
        })
        .interact_text()
        .unwrap();

    let data_path = if data_dir_str.is_empty() {
        default_data
    } else {
        PathBuf::from(&data_dir_str)
    };

    let new_data_path = if data_dir_str.is_empty() {
        default_new_data
    } else {
        PathBuf::from(&data_dir_str).join("bichon-storage")
    };

    println!("\n{}", style("Paths to be migrated:").bold());
    println!("----------------------------------------");
    println!(
        "{:<20} : {}",
        "bichon-root-dir",
        style(root_path.display()).cyan()
    );
    println!(
        "{:<20} : {}",
        "bichon-index-dir",
        style(index_path.display()).cyan()
    );
    println!(
        "{:<20} : {}",
        "bichon-data-dir",
        style(data_path.display()).cyan()
    );
    println!("----------------------------------------");

    println!(
        "\n{} Checking legacy v0.x storage layout...",
        style("⌛").yellow()
    );

    match is_legacy_data_layout_with_paths(&index_path, &data_path) {
        Ok(true) => {
            println!(
                "{} {}",
                style("✔").green(),
                style("Legacy v0.x Tantivy-based storage detected. Migration to v1.0 is required.")
                    .yellow()
            );
        }
        Ok(false) => {
            println!(
                "{} {}",
                style("✔").green(),
                style("No legacy v0.x storage layout was detected at the specified paths.").green()
            );

            println!(
                "{}",
                style(
                    "The selected directories may already be using the v1.0 storage architecture."
                )
                .dim()
            );

            return;
        }
        Err(e) => {
            eprintln!(
                "{} Failed to verify legacy storage layout: {:?}",
                style("ERROR:").red().bold(),
                e
            );

            std::process::exit(1);
        }
    }

    println!(
        "\n{} {}",
        style("⚠").yellow(),
        style(
            "This migration is non-destructive. Existing v0.x storage files will remain unchanged."
        )
        .yellow()
    );

    if !Confirm::with_theme(theme)
        .with_prompt("Ready to migrate?")
        .default(true)
        .interact()
        .unwrap()
    {
        println!("{}", style("Migration cancelled.").dim());
        return;
    }

    // Step 1: Migrate metadata (meta.db + mailbox.db → memdb)
    match crate::meta::migrate_metadata(&root_path) {
        Ok(()) => {}
        Err(e) => {
            eprintln!(
                "\n{} Metadata migration failed:\n{}",
                style("✘").red().bold(),
                style(e).red()
            );
            eprintln!(
                "{}",
                style("Aborting migration. No changes have been made to Tantivy data.")
                    .yellow()
            );
            return;
        }
    }

    println!(
        "\n{} {}",
        style("⌛").yellow(),
        style("Step 2: Migrating email index and blob data...").cyan()
    );
    let pb = ProgressBar::new(0);
    pb.set_style(ProgressStyle::default_bar()
    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
    .unwrap()
    .progress_chars("#>-"));
    let legacy = LegacyDirs::new(index_path, data_path);
    let new_dirs = NewDirs::new(new_index_path, new_data_path);
    if let Err(e) = do_migrate(legacy, new_dirs, |msg| {
        if let Some(data) = msg.strip_prefix("PROGRESS:") {
            let parts: Vec<&str> = data.split(':').collect();
            if parts.len() == 2 {
                let migrated = parts[0].parse::<u64>().unwrap_or(0);
                let skipped = parts[1].parse::<u64>().unwrap_or(0);

                pb.set_position(migrated + skipped);
                pb.set_message(format!(
                    "Migrated: {}, {} {}",
                    style(migrated).green(),
                    style(skipped).red(),
                    style("skipped").dim()
                ));
            }
        } else if let Some(total) = msg.strip_prefix("TOTAL:") {
            pb.set_length(total.parse().unwrap_or(0));
        } else if msg.starts_with("WARN:") {
            pb.println(format!("{} {}", style("⚠").yellow(), &msg[5..]));
        } else if let Some(done_data) = msg.strip_prefix("DONE:") {
            let parts: Vec<&str> = done_data.split(':').collect();
            pb.finish_with_message(format!(
                "Migration finished. Total: {}, Skipped: {}",
                parts.get(0).unwrap_or(&"0"),
                parts.get(1).unwrap_or(&"0")
            ));
        }
    }) {
        eprintln!(
            "\n{} Migration failed:\n{:?}",
            style("✘").red().bold(),
            style(e).red()
        );
        return;
    }
    println!(
        "{} {}",
        style("✔").green(),
        style("Migration completed successfully!").bold()
    );
}

pub fn is_legacy_data_layout_with_paths(
    envelope_dir: &PathBuf,
    eml_dir: &PathBuf,
) -> std::io::Result<bool> {
    let envelope_result = is_tantivy_index_dir(envelope_dir)?;
    let eml_result = is_tantivy_index_dir(eml_dir)?;

    Ok(envelope_result || eml_result)
}
