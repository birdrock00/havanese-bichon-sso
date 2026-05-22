use std::path::{Path, PathBuf};

use bichon_core::migrate::{
    count_eml_segments, do_migrate_segment, is_tantivy_index_dir,
    store::{LegacyDirs, NewDirs},
};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use indicatif::{ProgressBar, ProgressStyle};

pub fn handle_migration(theme: &ColorfulTheme) {
    println!(
        "\n{}",
        style("MIGRATION: Bichon v0.3.7 Storage Architecture → v1.x")
            .bold()
            .yellow()
    );

    println!(
        "{}",
        style(
            "This tool migrates data from the legacy v0.3.7 Tantivy-based storage \
            architecture to the new v1.x \
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
                New v1.x architecture:\n\
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
        "\n{} Checking legacy v0.3.7 storage layout...",
        style("⌛").yellow()
    );

    match is_legacy_data_layout_with_paths(&index_path, &data_path) {
        Ok(true) => {
            println!(
                "{} {}",
                style("✔").green(),
                style("Legacy v0.3.7 Tantivy-based storage detected. Migration to v1.x is required.")
                    .yellow()
            );
        }
        Ok(false) => {
            println!(
                "{} {}",
                style("✔").green(),
                style("No legacy v0.3.7 storage layout was detected at the specified paths.").green()
            );

            println!(
                "{}",
                style(
                    "The selected directories may already be using the v1.x storage architecture."
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
                style("Aborting migration. No changes have been made to Tantivy data.").yellow()
            );
            return;
        }
    }

    println!(
        "\n{} {}",
        style("⌛").yellow(),
        style("Step 2: Migrating email index and blob data...").cyan()
    );

    println!(
        "\n{} {}",
        style("ℹ").blue(),
        style("Batch size controls memory usage during migration:").dim()
    );
    println!(
        "  {} 1000  — ~500MB RAM  (slower, low memory)",
        style("•").dim()
    );
    println!("  {} 3000  — ~1GB RAM    (recommended)", style("•").dim());
    println!(
        "  {} 5000  — ~2GB RAM    (faster, high memory)",
        style("•").dim()
    );
    println!(
        "  {} Note: actual memory usage depends on your average email size.",
        style("•").yellow()
    );
    println!(
        "  {}       If your mailbox contains many large attachments, use a smaller batch size.\n",
        style(" ").dim()
    );

    let batch_size: u32 = {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter batch size (affects memory usage, see notes above)")
            .default("3000".to_string())
            .validate_with(|s: &String| match s.trim().parse::<usize>() {
                Ok(n) if n > 0 => Ok(()),
                _ => Err("Please enter a valid positive number"),
            })
            .interact_text()
            .unwrap_or("3000".to_string());
        input.trim().parse::<u32>().unwrap_or(3000)
    };

    println!(
        "{} Using batch size: {}\n",
        style("✓").green(),
        style(batch_size).cyan().bold()
    );

    let legacy = LegacyDirs::new(index_path.clone(), data_path.clone());
    let total_segments = match count_eml_segments(&legacy) {
        Ok(n) => n,
        Err(e) => {
            eprintln!(
                "\n{} Failed to count EML segments:\n{:?}",
                style("✘").red().bold(),
                e
            );
            return;
        }
    };

    if total_segments == 0 {
        println!(
            "{} {}",
            style("✔").green(),
            style("No EML segments found. Nothing to migrate.").bold()
        );
        return;
    }

    println!(
        "{} EML segments to migrate: {}",
        style("⌛").yellow(),
        style(total_segments).cyan()
    );

    let pb = ProgressBar::new(total_segments as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut grand_total_migrated: usize = 0;
    let mut grand_total_skipped: usize = 0;

    for seg_idx in 0..total_segments {
        let seg_total: std::cell::Cell<usize> = std::cell::Cell::new(0);

        pb.set_message(format!("Segment {}/{}", seg_idx + 1, total_segments));
        let legacy = LegacyDirs::new(index_path.clone(), data_path.clone());
        match do_migrate_segment(
            batch_size,
            legacy,
            NewDirs::new(new_index_path.clone(), new_data_path.clone()),
            seg_idx,
            |msg| {
                if let Some(data) = msg.strip_prefix("TOTAL:") {
                    seg_total.set(data.parse().unwrap_or(0));
                } else if let Some(data) = msg.strip_prefix("PHASE1:") {
                    let parts: Vec<&str> = data.split('/').collect();
                    let scanned: usize = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
                    let total: usize = parts
                        .get(1)
                        .and_then(|s| s.split_once(" skipped:").map(|(n, _)| n))
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    let skipped: usize = data
                        .split_once("skipped:")
                        .and_then(|(_, s)| s.parse().ok())
                        .unwrap_or(0);
                    let pct = if total > 0 {
                        (scanned * 100) / total
                    } else {
                        0
                    };
                    pb.set_message(format!(
                        "Segment {}/{} [scanning {}/{} skipped:{} {}%]",
                        seg_idx + 1,
                        total_segments,
                        scanned,
                        total,
                        skipped,
                        pct,
                    ));
                } else if let Some(data) = msg.strip_prefix("PROGRESS:") {
                    let parts: Vec<&str> = data.split(':').collect();
                    let migrated: usize = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
                    let total = seg_total.get();
                    let pct = if total > 0 {
                        (migrated * 100) / total
                    } else {
                        0
                    };
                    pb.set_message(format!(
                        "Segment {}/{} [migrating {}/{} {}%]",
                        seg_idx + 1,
                        total_segments,
                        migrated,
                        total,
                        pct,
                    ));
                } else if let Some(warn) = msg.strip_prefix("WARN:") {
                    pb.println(format!("{} {}", style("⚠").yellow(), warn));
                } else if let Some(done_data) = msg.strip_prefix("DONE:") {
                    let parts: Vec<&str> = done_data.split(':').collect();
                    let migrated: usize = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
                    let skipped: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                    grand_total_migrated += migrated;
                    grand_total_skipped += skipped;
                }
            },
        ) {
            Ok(()) => {}
            Err(e) => {
                pb.finish_with_message(format!("{}", style("Migration failed.").red()));
                eprintln!("\n{} {:?}", style("✘").red().bold(), e);
                return;
            }
        }

        pb.set_position((seg_idx + 1) as u64);
    }

    pb.finish_with_message(format!(
        "Migration finished. Total: {}, Skipped: {}",
        grand_total_migrated, grand_total_skipped
    ));

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
