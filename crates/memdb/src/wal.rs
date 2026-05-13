use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::path::Path;

/// A single WAL operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum WalOp {
    Insert {
        collection: String,
        key: String,
        value: serde_json::Value,
    },
    Upsert {
        collection: String,
        key: String,
        value: serde_json::Value,
    },
    Delete {
        collection: String,
        key: String,
    },
}

/// A WAL entry carrying a monotonically increasing sequence number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    pub seq: u64,
    /// One entry may carry multiple ops (transaction batch).
    pub ops: Vec<WalOp>,
}

/// Write a WAL entry line to the file (no fsync — caller decides when to
/// sync for durability).
pub fn write_entry(file: &mut std::fs::File, entry: &WalEntry) -> Result<()> {
    let line = serde_json::to_string(entry)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

/// Force buffered WAL data to disk.
pub fn sync_wal(file: &std::fs::File) -> Result<()> {
    file.sync_data()?;
    Ok(())
}

/// Read all valid WAL entries whose seq > `after_seq`.
/// Corrupted lines (e.g. partial write after power loss) are skipped
/// with a warning and do not prevent startup.
pub fn read_after(path: &Path, after_seq: u64) -> Result<Vec<WalEntry>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut entries = vec![];
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<WalEntry>(trimmed) {
            Ok(entry) if entry.seq > after_seq => entries.push(entry),
            // seq <= after_seq — already covered by snapshot, skip.
            Ok(_) => {}
            Err(e) => {
                eprintln!("[wal] skipping corrupted entry: {e}");
            }
        }
    }
    Ok(entries)
}
