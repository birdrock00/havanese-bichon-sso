use std::{collections::HashMap, path::PathBuf};

use crate::{
    error::{code::ErrorCode, BichonResult},
    migrate::{
        legacy::schema::SchemaTools,
        store::{LegacyDirs, NewIndexWriter},
    },
    raise_error,
    settings::cli::SETTINGS,
};
use tantivy::{
    collector::TopDocs,
    columnar::Column,
    query::TermQuery,
    schema::{IndexRecordOption, Value},
    DocAddress, Index, TantivyDocument, Term,
};

pub mod legacy;
pub mod store;

pub fn is_tantivy_index_dir(dir: &PathBuf) -> std::io::Result<bool> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(false);
    }

    let tantivy_extensions = [".store", ".term", ".idx", ".fieldnorm", ".pos"];
    let mut match_count = 0;
    let mut has_meta_json = false;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if name == "meta.json" {
            has_meta_json = true;
            continue;
        }

        if tantivy_extensions.iter().any(|ext| name.ends_with(ext)) {
            match_count += 1;
        }
    }

    Ok(has_meta_json && match_count >= 3)
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

pub fn check_data_status() -> std::io::Result<bool> {
    let root_dir = PathBuf::from(&SETTINGS.bichon_root_dir);

    let new_indices_base = SETTINGS
        .bichon_index_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.clone());
    let new_indices_path = new_indices_base.join("bichon-indices");

    let new_data_base = SETTINGS
        .bichon_data_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.clone());
    let new_storage_path = new_data_base.join("bichon-storage");

    let has_new_indices = is_tantivy_index_dir(&new_indices_path.join("attachment_metadata"))?
        && is_tantivy_index_dir(&new_indices_path.join("mail_metadata"))?;
    let has_new_storage = is_dir_not_empty(&new_storage_path)?;

    if has_new_indices && has_new_storage {
        return Ok(true);
    }

    let legacy_index_root = SETTINGS
        .bichon_index_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.join("envelope"));
    let legacy_data_root = SETTINGS
        .bichon_data_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root_dir.join("eml"));

    let has_legacy_index = is_tantivy_index_dir(&legacy_index_root)?;
    let has_legacy_data = is_tantivy_index_dir(&legacy_data_root)?;

    if has_legacy_index || has_legacy_data {
        Ok(false)
    } else {
        Ok(true)
    }
}

fn is_dir_not_empty(path: &PathBuf) -> std::io::Result<bool> {
    if !path.exists() || !path.is_dir() {
        return Ok(false);
    }
    let mut entries = std::fs::read_dir(path)?;
    Ok(entries.next().is_some())
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

    // Recreate the StoreReader periodically to bound any internal caches.
    //const CHUNK_SIZE: u32 = 3000;
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

        // Flush Fjall buffers via ingestion API — bypasses memtable/WAL.
        writer.flush_fjall_buffers()?;

        chunk_start = chunk_end;
    }

    on_progress(&format!("DONE:{}:{}", total_migrated, total_skipped));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_tantivy_dir() {
        let path = PathBuf::from(r"D:\test-data\envelope");
        let result = is_tantivy_index_dir(&path).unwrap();
        println!("is tantivy index dir: {}", result);
        assert!(result);
    }
}
