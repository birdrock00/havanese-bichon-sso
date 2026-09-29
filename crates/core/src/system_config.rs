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

//! memdb-backed runtime system configuration with uniform reversible
//! encryption.
//!
//! The static `BICHON_*` settings are frozen at startup, so anything that
//! must be changeable from the WebUI on a running install lives here instead:
//! a single `system_config` collection mapping a config key to a value. Every
//! value is stored **reversibly encrypted** at rest via
//! [`crate::utils::encrypt`] (AES-256-GCM keyed off `BICHON_ENCRYPT_PASSWORD`),
//! so secrets such as S3 keys and repository passwords never appear as
//! plaintext — not in the memdb snapshot that backups archive, nor in the
//! memdb WAL on disk.
//!
//! Read-time precedence is *override wins, else env/CLI*: consumers read
//! through a keyed accessor (e.g. [`crate::backup::config`]) that falls back
//! to `SETTINGS` when no override is stored, so an unconfigured install keeps
//! its existing env behavior untouched.
//!
//! Writes go through the gated memdb helpers, so saving configuration is
//! rejected while a backup window is open (surfaced as `429 Too Many
//! Requests` to the WebUI — retry once the backup finishes).

use serde::{Deserialize, Serialize};

use crate::database::manager::DB_MANAGER;
use crate::database::{delete_impl, find_impl, upsert_impl, MemDbModel};
use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::utc_now;
use crate::utils::encrypt::{decrypt_string, encrypt_string};

/// One key→value pair of the system configuration. Values are always stored
/// encrypted; the collection is included in the memdb backup snapshot and
/// restored with it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemConfigEntry {
    /// Config key, e.g. `backup.target_type`.
    pub key: String,
    /// Reversibly encrypted value (`encrypt_string`).
    pub value_encrypted: String,
    /// UTC millis of the last write.
    pub updated_at: i64,
}

impl MemDbModel for SystemConfigEntry {
    fn collection() -> &'static str {
        "system_config"
    }
    fn key(&self) -> String {
        self.key.clone()
    }
}

/// The decrypted value for `key`, if an override has been stored. `None` also
/// when the value cannot be decrypted (wrong encryption password / corruption)
/// — the caller falls back to its env/CLI default rather than surfacing the
/// secret's failure.
pub fn get(key: &str) -> Option<String> {
    let entry: SystemConfigEntry = match find_impl(DB_MANAGER.db(), key) {
        Ok(Some(e)) => e,
        _ => return None,
    };
    decrypt_string(&entry.value_encrypted).ok()
}

/// Store an encrypted override for `key`. Rejects with `429` while a backup
/// window is open (the memdb write gate is paused).
pub fn set(key: &str, value: &str) -> BichonResult<()> {
    let entry = SystemConfigEntry {
        key: key.to_string(),
        value_encrypted: encrypt_string(value)?,
        updated_at: utc_now!(),
    };
    upsert_impl(DB_MANAGER.db(), entry)
}

/// Remove the override for `key`, returning that config item to its env/CLI
/// fallback. Idempotent: deleting a key that has no override is a no-op.
pub fn delete(key: &str) -> BichonResult<()> {
    match delete_impl::<SystemConfigEntry>(DB_MANAGER.db(), key) {
        Ok(()) => Ok(()),
        Err(e) if e.code() == ErrorCode::ResourceNotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Whether an override is stored for `key`.
pub fn exists(key: &str) -> bool {
    get(key).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::gate::WRITE_GATE;
    use crate::database::manager::DB_MANAGER;
    use crate::settings::cli::SETTINGS;
    use std::sync::Once;

    static TEST_ENV: Once = Once::new();
    fn init_test_env() {
        TEST_ENV.call_once(|| {
            let root = std::env::temp_dir().join(format!(
                "bichon-core-test-{}",
                std::process::id()
            ));
            std::fs::create_dir_all(&root).unwrap();
            std::env::set_var("BICHON_ROOT_DIR", &root);
            std::env::set_var("BICHON_ENCRYPT_PASSWORD", "test-password");
            let _ = &*SETTINGS;
            let _ = &*DB_MANAGER;
        });
    }

    #[tokio::test]
    async fn set_get_roundtrips_through_encryption() {
        init_test_env();
        WRITE_GATE.resume(); // defensive: start from an open gate
        let key = format!("test-system-config-{}", std::process::id());
        set(&key, "s3:http://localhost:9000/bucket").unwrap();
        assert_eq!(
            get(&key),
            Some("s3:http://localhost:9000/bucket".to_string())
        );
        delete(&key).unwrap();
        assert_eq!(get(&key), None);
    }

    #[tokio::test]
    async fn values_are_stored_encrypted_at_rest() {
        init_test_env();
        WRITE_GATE.resume();
        let key = format!("test-system-config-secret-{}", std::process::id());
        set(&key, "super-secret-password").unwrap();

        // The raw document in the collection must not contain the plaintext.
        let raw: SystemConfigEntry = DB_MANAGER
            .db()
            .collection(SystemConfigEntry::collection())
            .get(&key)
            .unwrap()
            .expect("entry must exist");
        assert!(
            !raw.value_encrypted.contains("super-secret-password"),
            "plaintext must never be stored"
        );
        // And it must decrypt back to the original.
        assert_eq!(
            decrypt_string(&raw.value_encrypted).unwrap(),
            "super-secret-password"
        );

        delete(&key).unwrap();
    }

    #[tokio::test]
    async fn set_overwrites_and_exists_tracks() {
        init_test_env();
        WRITE_GATE.resume();
        let key = format!("test-system-config-overwrite-{}", std::process::id());
        assert!(!exists(&key));
        set(&key, "first").unwrap();
        set(&key, "second").unwrap();
        assert!(exists(&key));
        assert_eq!(get(&key), Some("second".to_string()));
        delete(&key).unwrap();
        assert!(!exists(&key));
    }

    #[tokio::test]
    async fn delete_is_idempotent() {
        init_test_env();
        WRITE_GATE.resume();
        let key = format!("test-system-config-delete-{}", std::process::id());
        // Deleting a key that was never set must not error.
        delete(&key).unwrap();
        set(&key, "x").unwrap();
        delete(&key).unwrap();
        delete(&key).unwrap(); // second delete: still ok
        assert_eq!(get(&key), None);
    }
}
