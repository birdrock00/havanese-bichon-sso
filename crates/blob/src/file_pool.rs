use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::error::Result;

/// Simple LRU pool of open file handles, keyed by segment_id.
/// Uses Arc<Mutex<File>> to allow safe concurrent reads from the same segment.
pub struct FilePool {
    max_entries: usize,
    entries: Mutex<VecDeque<(u32, Arc<Mutex<File>>)>>,
}

impl FilePool {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries: max_entries.max(1),
            entries: Mutex::new(VecDeque::new()),
        }
    }

    /// Get an open File for the given segment. Reuses cached handle if available.
    pub fn get(&self, seg_id: u32, path: &Path) -> Result<Arc<Mutex<File>>> {
        let mut entries = self.entries.lock().unwrap();

        // Check for existing entry
        for (i, (id, _)) in entries.iter().enumerate() {
            if *id == seg_id {
                let (_, file) = entries.remove(i).unwrap();
                entries.push_front((seg_id, file.clone()));
                return Ok(file);
            }
        }

        // Open new file
        let file = Arc::new(Mutex::new(File::open(path)?));

        // Evict oldest if full
        if entries.len() >= self.max_entries {
            entries.pop_back();
        }

        entries.push_front((seg_id, file.clone()));
        Ok(file)
    }

    /// Remove a cached file handle (e.g. after GC rewrites a segment).
    pub fn invalidate(&self, seg_id: u32) {
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|(id, _)| *id != seg_id);
    }
}
