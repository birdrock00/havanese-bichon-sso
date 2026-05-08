use std::path::PathBuf;

use crate::settings::cli::SETTINGS;

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

pub fn is_legacy_data_layout() -> std::io::Result<bool> {
    let root_dir = PathBuf::from(&SETTINGS.bichon_root_dir);
    let envelope_dir = if let Some(ref index_dir) = SETTINGS.bichon_index_dir {
        PathBuf::from(index_dir)
    } else {
        root_dir.join("envelope")
    };

    let eml_dir = if let Some(ref data_dir) = SETTINGS.bichon_data_dir {
        PathBuf::from(data_dir)
    } else {
        root_dir.join("eml")
    };

    let envelope_result = is_tantivy_index_dir(&envelope_dir)?;
    let eml_result = is_tantivy_index_dir(&eml_dir)?;
    Ok(envelope_result || eml_result)
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
