//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use crate::settings::dir::DATA_DIR_MANAGER;
use bichon_memdb::{Durability, MemDb};
use std::sync::LazyLock;
use std::time::Duration;

pub static DB_MANAGER: LazyLock<DatabaseManager> = LazyLock::new(DatabaseManager::new);

pub struct DatabaseManager {
    db: MemDb,
}

impl DatabaseManager {
    fn new() -> Self {
        let db_path = &DATA_DIR_MANAGER.memdb_dir;
        std::fs::create_dir_all(db_path).expect("Failed to create memdb data directory");

        let db = MemDb::open_with(db_path, Durability::Batch { max_ops: 100 })
            .expect("Failed to open memdb database");

        // Start periodic snapshot worker (every 5 minutes)
        db.start_snapshot_worker(Duration::from_secs(300));

        // Start periodic flush worker (every 10 seconds) so buffered writes
        // are flushed regularly and not only at the batch threshold.
        db.start_flush_worker(Duration::from_secs(10));

        DatabaseManager { db }
    }

    /// Get a reference to the MemDb instance.
    pub fn db(&self) -> &MemDb {
        &self.db
    }

    /// Flush any buffered WAL entries to disk. Must be called before shutdown
    /// to avoid losing writes that haven't hit the batch threshold yet.
    pub fn flush(&self) {
        if let Err(e) = self.db.flush() {
            eprintln!("[memdb] flush error on shutdown: {e}");
        }
    }
}
