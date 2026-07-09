use std::{collections::HashMap, path::PathBuf};

use bichon_core::{
    error::{code::ErrorCode, BichonResult},
    migrate::is_tantivy_index_dir,
    raise_error,
};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use indicatif::{ProgressBar, ProgressStyle};
use tantivy::{
    collector::TopDocs,
    columnar::Column,
    query::TermQuery,
    schema::{IndexRecordOption, Value},
    DocAddress, Index, TantivyDocument, Term,
};

use crate::legacy::schema::SchemaTools;
use crate::migrate_store::{LegacyDirs, NewDirs, NewIndexWriter};

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
            separated index and blob-backed storage format."
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
            • raw message data stored in blob engine\n\
            • attachment blobs stored in blob engine"
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
            let path = PathBuf::from(input);
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
            let path = PathBuf::from(input);
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
            let path = PathBuf::from(input);
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

    let mut writer = match NewIndexWriter::open(NewDirs::new(
        new_index_path.clone(),
        new_data_path.clone(),
    )) {
        Ok(w) => w,
        Err(e) => {
            pb.finish_with_message(format!("{}", style("Migration failed.").red()));
            eprintln!("\n{} {:?}", style("✘").red().bold(), e);
            return;
        }
    };

    let mut grand_total_migrated: usize = 0;
    let mut grand_total_skipped: usize = 0;

    for seg_idx in 0..total_segments {
        let seg_total: std::cell::Cell<usize> = std::cell::Cell::new(0);

        pb.set_message(format!("Segment {}/{}", seg_idx + 1, total_segments));
        let legacy = LegacyDirs::new(index_path.clone(), data_path.clone());
        match do_migrate_segment(
            batch_size,
            legacy,
            &mut writer,
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

    pb.set_message(style("Finalizing indexes...").dim().to_string());
    if let Err(e) = writer.finish_writers() {
        pb.finish_with_message(format!("{}", style("Migration failed.").red()));
        eprintln!("\n{} {:?}", style("✘").red().bold(), e);
        return;
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

/// Return the number of segments in the legacy EML Tantivy index.
/// Each segment can be passed to `do_migrate_segment` for bounded-memory batch migration.
pub fn count_eml_segments(legacy: &LegacyDirs) -> BichonResult<usize> {
    let eml_index = Index::open_in_dir(&legacy.eml_dir)
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    let reader = eml_index
        .reader()
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    let searcher = reader.searcher();
    Ok(searcher.segment_readers().len())
}

/// Migrate all documents from a single EML segment to the new storage layout.
///
/// This is the core of the batch migration strategy: each Process B invocation
/// handles exactly one EML segment, so peak memory is bounded by that segment's
/// size regardless of the total archive size.
pub fn do_migrate_segment<F>(
    batch_size: u32,
    legacy: LegacyDirs,
    writer: &mut NewIndexWriter,
    segment_index: usize,
    mut on_progress: F,
) -> BichonResult<()>
where
    F: FnMut(&str),
{
    // ── open legacy indices ────────────────────────────────────────────
    let envelope_index = Index::open_in_dir(&legacy.envelope_dir)
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    let eml_index = Index::open_in_dir(&legacy.eml_dir)
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

    let envelope_reader = envelope_index
        .reader()
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    let eml_reader = eml_index
        .reader()
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

    let envelope_searcher = envelope_reader.searcher();
    let eml_searcher = eml_reader.searcher();

    let ef = SchemaTools::envelope_fields();
    let mf = SchemaTools::eml_fields();

    let eml_segments = eml_searcher.segment_readers();
    let eml_segment = eml_segments.get(segment_index).ok_or_else(|| {
        raise_error!(
            format!(
                "segment index {} out of range ({} segments)",
                segment_index,
                eml_segments.len()
            ),
            ErrorCode::InternalError
        )
    })?;

    let num_docs = eml_segment.num_docs();
    if num_docs == 0 {
        on_progress("TOTAL:0");
        on_progress("DONE:0:0");
        return Ok(());
    }

    on_progress(&format!("TOTAL:{}", num_docs));

    let max_doc = eml_segment.max_doc();
    let ff = eml_segment.fast_fields();
    let f_id_col: Column<u64> = ff.u64("id").map_err(|e| {
        raise_error!(
            format!("failed to open f_id fast field: {e:#?}"),
            ErrorCode::InternalError
        )
    })?;

    // ── Phase 1: build eid → (uid, internal_date) from envelope, then drop it ──
    let mut envelope_map: HashMap<u64, (u32, i64)> = HashMap::with_capacity(num_docs as usize);

    let mut env_scanned = 0u32;
    let mut env_skipped = 0u32;
    for doc_id in 0..max_doc {
        if eml_segment.is_deleted(doc_id) {
            continue;
        }
        let eid = f_id_col.values.get_val(doc_id);

        let term = Term::from_field_u64(ef.f_id, eid);
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let hits: Vec<(_, DocAddress)> = envelope_searcher
            .search(&query, &TopDocs::with_limit(1).order_by_score())
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

        if let Some((_, addr)) = hits.first() {
            let env_doc: TantivyDocument = envelope_searcher
                .doc(*addr)
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
            let uid = env_doc
                .get_first(ef.f_uid)
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let internal_date = env_doc
                .get_first(ef.f_internal_date)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            envelope_map.insert(eid, (uid, internal_date));
            env_scanned += 1;
        } else {
            env_skipped += 1;
        }

        if env_scanned % 10 == 0 {
            on_progress(&format!(
                "PHASE1:{}/{} skipped:{}",
                env_scanned, max_doc, env_skipped
            ));
        }
    }

    // Free the envelope index before the heavy EML processing.
    drop(envelope_searcher);
    drop(envelope_reader);
    drop(envelope_index);

    // ── Phase 2: process EML docs, streaming one at a time ─────────────
    let mut total_migrated = 0usize;
    let mut total_skipped = 0usize;

    let mut chunk_start = 0u32;

    while chunk_start < max_doc {
        let chunk_end = (chunk_start + batch_size).min(max_doc);
        let store_reader = eml_segment
            .get_store_reader(2)
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

        for doc_id in chunk_start..chunk_end {
            if eml_segment.is_deleted(doc_id) {
                continue;
            }

            let eid = f_id_col.values.get_val(doc_id);

            let (uid, internal_date) = match envelope_map.get(&eid) {
                Some(v) => *v,
                None => {
                    on_progress(&format!("WARN: eid {} envelope not found", eid));
                    total_skipped += 1;
                    continue;
                }
            };

            let eml_doc: TantivyDocument = store_reader
                .get(doc_id)
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

            let account_id = match eml_doc.get_first(mf.f_account_id).and_then(|v| v.as_u64()) {
                Some(v) => v,
                None => {
                    on_progress(&format!("WARN: eid {} account_id missing", eid));
                    total_skipped += 1;
                    continue;
                }
            };
            let mailbox_id = eml_doc
                .get_first(mf.f_mailbox_id)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            // Borrow directly from eml_doc — no .to_vec() clone.
            let eml_bytes = match eml_doc.get_first(mf.f_eml).and_then(|v| v.as_bytes()) {
                Some(b) => b,
                None => {
                    on_progress(&format!("WARN: eid {} eml bytes missing", eid));
                    total_skipped += 1;
                    continue;
                }
            };

            if let Err(e) = writer.ingest(eml_bytes, account_id, mailbox_id, uid, internal_date) {
                on_progress(&format!(
                    "ERROR: Account {} eid {} ingest failed: {}",
                    account_id, eid, e
                ));
                total_skipped += 1;
                continue;
            }

            total_migrated += 1;

            if total_migrated % 10 == 0 || total_migrated as u32 == num_docs {
                on_progress(&format!("PROGRESS:{}:{}", total_migrated, num_docs));
            }
        }

        drop(store_reader);

        // Flush blob buffers via ingestion API — bypasses memtable/WAL.
        writer.flush_fjall_buffers()?;

        chunk_start = chunk_end;
    }

    on_progress(&format!("DONE:{}:{}", total_migrated, total_skipped));
    Ok(())
}
