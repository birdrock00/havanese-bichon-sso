use std::path::PathBuf;

use crate::{
    error::{code::ErrorCode, BichonResult},
    migrate::{
        legacy::schema::SchemaTools,
        store::{LegacyDirs, NewDirs, NewIndexWriter},
    },
    raise_error,
    settings::cli::SETTINGS,
};
use tantivy::{
    collector::TopDocs, query::AllQuery, schema::Value, DocAddress, Index, TantivyDocument,
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

const PAGE_SIZE: usize = 100;

pub fn do_migrate<F>(legacy: LegacyDirs, new_dirs: NewDirs, mut on_progress: F) -> BichonResult<()>
where
    F: FnMut(&str),
{
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

    let total_count = envelope_searcher.num_docs();
    on_progress(&format!("TOTAL:{}", total_count));

    let ef = SchemaTools::envelope_fields();
    let mf = SchemaTools::eml_fields();

    let mut writer = NewIndexWriter::open(new_dirs)?;

    let mut offset = 0usize;
    let mut total_migrated = 0usize;
    let mut total_skipped = 0usize;

    loop {
        let page: Vec<(_, DocAddress)> = envelope_searcher
            .search(
                &AllQuery,
                &TopDocs::with_limit(PAGE_SIZE)
                    .and_offset(offset)
                    .order_by_score(),
            )
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

        if page.is_empty() {
            break;
        }
        let fetched = page.len();

        for (_, doc_address) in page {
            let doc: TantivyDocument = envelope_searcher
                .doc(doc_address)
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

            let eid = match doc.get_first(ef.f_id).and_then(|v| v.as_u64()) {
                Some(v) => v,
                None => {
                    total_skipped += 1;
                    continue;
                }
            };
            let account_id = match doc.get_first(ef.f_account_id).and_then(|v| v.as_u64()) {
                Some(v) => v,
                None => {
                    total_skipped += 1;
                    continue;
                }
            };
            let mailbox_id = doc
                .get_first(ef.f_mailbox_id)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let uid = doc
                .get_first(ef.f_uid)
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let internal_date = doc
                .get_first(ef.f_internal_date)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let eml_term = tantivy::Term::from_field_u64(mf.f_id, eid);
            let eml_query =
                tantivy::query::TermQuery::new(eml_term, tantivy::schema::IndexRecordOption::Basic);
            let eml_hits: Vec<(_, DocAddress)> = eml_searcher
                .search(&eml_query, &TopDocs::with_limit(1).order_by_score())
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

            let eml_bytes = match eml_hits.first() {
                Some((_, addr)) => {
                    let eml_doc: TantivyDocument = eml_searcher
                        .doc(*addr)
                        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
                    match eml_doc.get_first(mf.f_eml).and_then(|v| v.as_bytes()) {
                        Some(b) => b.to_vec(),
                        None => {
                            on_progress(&format!("WARN: Account {} ID {} eml field missing", account_id, eid));
                            total_skipped += 1;
                            continue;
                        }
                    }
                }
                None => {
                    on_progress(&format!("WARN:Account {} ID {} eml not found", account_id, eid));
                    total_skipped += 1;
                    continue;
                }
            };

            if let Err(e) = writer.ingest(&eml_bytes, account_id, mailbox_id, uid, internal_date) {
                on_progress(&format!("ERROR:Account {} ID {} ingest failed: {}", account_id, eid, e));
                total_skipped += 1;
                continue;
            }

            total_migrated += 1;

            if total_migrated % 100 == 0 || total_migrated == total_count as usize {
                on_progress(&format!("PROGRESS:{}:{}", total_migrated, total_skipped));
            }
        }

        offset += fetched;
        if fetched < PAGE_SIZE {
            break;
        }
    }

    writer.commit()?;

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
