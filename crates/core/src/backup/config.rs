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

//! Runtime backup configuration, configured through the WebUI.
//!
//! The whole configuration — enabled, schedule, S3 target, prefix,
//! retention — is **one** [`BackupConfig`] struct stored as
//! a single JSON document in its own memdb `backup_config` collection. Only
//! the two S3 credentials are sensitive: `s3_access_key` / `s3_secret_key`
//! are encrypted per-field on serialization ([`encrypt_secret`]) and
//! decrypted on load, while the remaining (non-secret) fields stay plaintext
//! JSON so a config save costs one JSON encode plus two credential
//! encryptions instead of a per-field encrypt/decrypt pass.
//!
//! Read-time precedence is *stored wins, else code default*: there are no
//! `BICHON_BACKUP_*` environment variables, the S3 form is the product's
//! configuration surface (design doc §9). Editing takes effect without a
//! restart. A warm in-memory cache ([`load`]) keeps the hot accessors
//! (`enabled`, `schedule` for the status card / scheduler) decrypt-free.
//!
//! Historical `backup.*` `system_config` overrides are migrated on first
//! load ([`migrate_legacy`]). Secrets never round-trip through the API in
//! plaintext (the WebUI sees only `*_set` flags).

use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, LazyLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::backup::engine::S3Options;
use crate::backup::retention::RetentionPolicy;
use crate::database::manager::DB_MANAGER;
use crate::database::{delete_impl, find_impl, upsert_impl, MemDbModel};
use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::raise_error;
use crate::system_config;
use crate::utc_now;
use crate::utils::encrypt::{decrypt_string, encrypt_string};

// ── document layout ────────────────────────────────────────────────────────

/// Entry key of the single config document inside the `backup_config`
/// collection.
pub const ENTRY_KEY: &str = "config";

/// Default schedule (cron, UTC): daily at 03:00.
pub const DEFAULT_SCHEDULE: &str = "0 0 3 * * *";

/// Default prefix; the design-doc layout lives under `<prefix>/v1/`.
pub const DEFAULT_PREFIX: &str = "bichon-backup";

/// One-shot "rebase the audit chain on the next run" request, set by the
/// Pro WebUI's rebase button (`backup.rebase`). A transient flag, so it stays
/// in `system_config` rather than the config document.
pub const KEY_REBASE: &str = "backup.rebase";

/// The complete backup configuration. In memory every field is plaintext; on
/// disk the document is JSON with only the two S3 credentials individually
/// encrypted ([`encrypt_secret`]).
///
/// `#[serde(default)]` fills fields added in later releases from
/// [`Default`], so old documents keep working after an upgrade.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BackupConfig {
    /// Whether the backup feature is enabled (default: off — enable it in
    /// the WebUI Backup page).
    pub enabled: bool,
    /// Cron expression (UTC) for scheduled backups.
    pub schedule: String,
    /// Object-store prefix under the bucket.
    pub prefix: String,
    /// Structured retention policy.
    pub retention: RetentionPolicy,
    /// S3 endpoint (`http://…` for MinIO/R2/Wasabi style, empty for AWS).
    pub s3_endpoint: Option<String>,
    /// S3 region.
    pub s3_region: Option<String>,
    /// S3 bucket name.
    pub s3_bucket: Option<String>,
    /// S3 access key. **Secret**: encrypted at rest in the document.
    #[serde(serialize_with = "encrypt_secret", deserialize_with = "decrypt_secret")]
    pub s3_access_key: Option<String>,
    /// S3 secret key. **Secret**: encrypted at rest in the document.
    #[serde(serialize_with = "encrypt_secret", deserialize_with = "decrypt_secret")]
    pub s3_secret_key: Option<String>,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            schedule: DEFAULT_SCHEDULE.to_string(),
            prefix: DEFAULT_PREFIX.to_string(),
            retention: RetentionPolicy::default(),
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_access_key: None,
            s3_secret_key: None,
        }
    }
}

/// Serialize an S3 credential as its encrypted ciphertext, so the stored
/// document never contains the plaintext secret.
fn encrypt_secret<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        None => serializer.serialize_none(),
        Some(plain) => {
            let cipher = encrypt_string(plain).map_err(serde::ser::Error::custom)?;
            serializer.serialize_some(&cipher)
        }
    }
}

/// Deserialize an S3 credential back from its encrypted ciphertext. A value
/// that cannot be decrypted (changed encryption password / corruption) is
/// treated as unset, matching the `system_config` read behavior — the
/// accessor then reports the secret as not configured rather than surfacing
/// the failure.
fn decrypt_secret<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let cipher = Option::<String>::deserialize(deserializer)?;
    match cipher {
        None => Ok(None),
        Some(c) => match decrypt_string(&c) {
            Ok(plain) => Ok(Some(plain)),
            Err(e) => {
                tracing::warn!("backup: stored s3 secret cannot be decrypted ({e}); treating as unset");
                Ok(None)
            }
        },
    }
}

/// One row of the `backup_config` collection: the config document. Unlike
/// `system_config` entries the document is not whole-value encrypted — only
/// the two S3 credentials inside it are.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupConfigEntry {
    /// Document key ([`ENTRY_KEY`]).
    pub key: String,
    /// JSON document; `s3_access_key` / `s3_secret_key` are individually
    /// encrypted within it.
    pub value: String,
    /// UTC millis of the last write.
    pub updated_at: i64,
}

impl MemDbModel for BackupConfigEntry {
    fn collection() -> &'static str {
        "backup_config"
    }
    fn key(&self) -> String {
        self.key.clone()
    }
}

// ── load / save (with a warm in-memory cache) ──────────────────────────────

/// Decrypted, deserialized config cache so hot accessors (`enabled`,
/// `schedule` for the status card and scheduler) never re-decrypt the
/// credentials on every call.
static CACHED: LazyLock<RwLock<Option<Arc<BackupConfig>>>> =
    LazyLock::new(|| RwLock::new(None));

/// The effective configuration, stored document winning over code defaults.
///
/// First call decrypts the document once and warms the cache; afterwards
/// accessors return the cached struct until the next [`save`].
pub fn load() -> BackupConfig {
    if let Some(cfg) = read_cached() {
        return cfg;
    }
    if let Some(cfg) = load_from_store() {
        *CACHED.write().unwrap() = Some(Arc::new(cfg.clone()));
        return cfg;
    }
    // No stored document: fall back to the legacy `backup.*` overrides (and
    // migrate them if present). Do NOT cache a pure code-default result —
    // defaults are recomputable, and warming the cache with them would let a
    // concurrent read shadow a later legacy-key write (the config cache is a
    // process-global that out-of-module reads like the backup scheduler also
    // consult). `migrate_legacy` caches itself when it actually migrates.
    migrate_legacy()
}

/// Read + decrypt + deserialize the stored document, if any.
fn load_from_store() -> Option<BackupConfig> {
    let entry: BackupConfigEntry = match find_impl(DB_MANAGER.db(), ENTRY_KEY) {
        Ok(Some(e)) => e,
        _ => return None,
    };
    match serde_json::from_str::<BackupConfig>(&entry.value) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            // Loud, not silent: falling back to defaults turns the whole
            // feature off (backups stop with no user-visible error).
            tracing::error!("backup: stored config document is invalid ({e}); using defaults");
            None
        }
    }
}

/// Validate a configuration document before it is persisted. A value that
/// fails here would otherwise fail *silently* at run time: an unparsable cron
/// disables scheduled backups with no user-visible error, an unusable prefix
/// redirects the runs to the engine's fallback location, and a keep-nothing
/// retention policy would delete every restore point (the engine clamps, but
/// the user should hear about it at save time).
pub fn validate(cfg: &BackupConfig) -> BichonResult<()> {
    // `Schedule::from_str` requires seconds-first 6/7-field expressions; an
    // empty string means "no schedule" (manual runs only).
    if !cfg.schedule.trim().is_empty() {
        if let Err(e) = cron::Schedule::from_str(&cfg.schedule) {
            return Err(raise_error!(
                format!(
                    "backup: invalid cron expression '{}': {e} (expected a 6-field UTC \
                     expression like '{}')",
                    cfg.schedule, DEFAULT_SCHEDULE
                ),
                ErrorCode::InvalidParameter
            ));
        }
    }
    // The engine derives the object-store base path from the prefix.
    if cfg.prefix.trim().is_empty() {
        return Err(raise_error!(
            "backup: prefix must not be empty".into(),
            ErrorCode::InvalidParameter
        ));
    }
    if object_store::path::Path::parse(format!("{}/v1", cfg.prefix)).is_err() {
        return Err(raise_error!(
            format!(
                "backup: prefix '{}' is not a valid object-store path prefix",
                cfg.prefix
            ),
            ErrorCode::InvalidParameter
        ));
    }
    // Retention: at least one budget must be non-zero.
    let r = &cfg.retention;
    if r.keep_last == 0 && r.keep_daily == 0 && r.keep_weekly == 0 && r.keep_monthly == 0 {
        return Err(raise_error!(
            "backup: retention policy must keep at least one restore point".into(),
            ErrorCode::InvalidParameter
        ));
    }
    Ok(())
}

/// Serialize, encrypt the two credentials and persist the document, then
/// refresh the cache. Rejects with `429` while a backup window is open (the
/// memdb write gate is paused).
pub fn save(cfg: &BackupConfig) -> BichonResult<()> {
    validate(cfg)?;
    let json = serde_json::to_string(cfg).map_err(|e| {
        raise_error!(
            format!("backup: cannot encode config: {e}"),
            ErrorCode::InternalError
        )
    })?;
    let entry = BackupConfigEntry {
        key: ENTRY_KEY.to_string(),
        value: json,
        updated_at: utc_now!(),
    };
    upsert_impl(DB_MANAGER.db(), entry)?;
    *CACHED.write().unwrap() = Some(Arc::new(cfg.clone()));
    Ok(())
}

/// Remove the stored config document and drop the cache, returning the
/// install to the code defaults. Idempotent. Used by tests and as a reset
/// escape hatch.
pub fn clear() -> BichonResult<()> {
    *CACHED.write().unwrap() = None;
    match delete_impl::<BackupConfigEntry>(DB_MANAGER.db(), ENTRY_KEY) {
        Ok(()) => Ok(()),
        Err(e) if e.code() == ErrorCode::ResourceNotFound => Ok(()),
        Err(e) => Err(e),
    }
}

fn read_cached() -> Option<BackupConfig> {
    CACHED.read().unwrap().as_ref().map(|c| (**c).clone())
}

// ── legacy `backup.*` system_config overrides ──────────────────────────────

/// Historical per-key overrides, stored in `system_config` before the config
/// document existed (≤ the 2026-09 config layout). Migrated into the
/// document on first load.
const LEGACY_ENABLED: &str = "backup.enabled";
const LEGACY_SCHEDULE: &str = "backup.schedule";
const LEGACY_S3_ENDPOINT: &str = "backup.s3_endpoint";
const LEGACY_S3_REGION: &str = "backup.s3_region";
const LEGACY_S3_BUCKET: &str = "backup.s3_bucket";
const LEGACY_S3_ACCESS_KEY: &str = "backup.s3_access_key";
const LEGACY_S3_SECRET_KEY: &str = "backup.s3_secret_key";
const LEGACY_PREFIX: &str = "backup.prefix";
const LEGACY_RETENTION_JSON: &str = "backup.retention_json";

const LEGACY_KEYS: [&str; 9] = [
    LEGACY_ENABLED,
    LEGACY_SCHEDULE,
    LEGACY_S3_ENDPOINT,
    LEGACY_S3_REGION,
    LEGACY_S3_BUCKET,
    LEGACY_S3_ACCESS_KEY,
    LEGACY_S3_SECRET_KEY,
    LEGACY_PREFIX,
    LEGACY_RETENTION_JSON,
];

/// Build the config from legacy `backup.*` overrides (if any), persist it as
/// the document and delete the old keys. Idempotent: runs only when no
/// document exists yet; a failed persist just leaves the legacy keys for the
/// next start.
fn migrate_legacy() -> BackupConfig {
    let mut cfg = BackupConfig::default();
    let mut touched = false;

    if let Some(v) = system_config::get(LEGACY_ENABLED) {
        cfg.enabled = v == "true";
        touched = true;
    }
    if let Some(v) = system_config::get(LEGACY_SCHEDULE) {
        cfg.schedule = v;
        touched = true;
    }
    if let Some(v) = system_config::get(LEGACY_S3_ENDPOINT) {
        cfg.s3_endpoint = Some(v);
        touched = true;
    }
    if let Some(v) = system_config::get(LEGACY_S3_REGION) {
        cfg.s3_region = Some(v);
        touched = true;
    }
    if let Some(v) = system_config::get(LEGACY_S3_BUCKET) {
        cfg.s3_bucket = Some(v);
        touched = true;
    }
    if let Some(v) = system_config::get(LEGACY_S3_ACCESS_KEY) {
        cfg.s3_access_key = Some(v);
        touched = true;
    }
    if let Some(v) = system_config::get(LEGACY_S3_SECRET_KEY) {
        cfg.s3_secret_key = Some(v);
        touched = true;
    }
    if let Some(v) = system_config::get(LEGACY_PREFIX) {
        cfg.prefix = v;
        touched = true;
    }
    if let Some(v) = system_config::get(LEGACY_RETENTION_JSON) {
        match serde_json::from_str::<RetentionPolicy>(&v) {
            Ok(policy) => {
                cfg.retention = policy;
                touched = true;
            }
            Err(e) => tracing::warn!(
                "backup: legacy retention policy is invalid ({e}); keeping defaults"
            ),
        }
    }

    if touched {
        if let Err(e) = save(&cfg) {
            tracing::warn!(
                "backup: legacy config migration could not be saved ({e}); it will retry next start"
            );
        } else {
            for key in LEGACY_KEYS {
                let _ = system_config::delete(key);
            }
        }
    }
    cfg
}

// ── effective-value accessors (stored document, code defaults) ─────────────

/// Whether the backup feature is enabled (default: off — enable it in the
/// WebUI Backup page).
pub fn enabled() -> bool {
    load().enabled
}

/// Cron expression (UTC) for scheduled backups.
pub fn schedule() -> String {
    load().schedule
}

pub fn s3_endpoint() -> Option<String> {
    load().s3_endpoint
}

pub fn s3_region() -> Option<String> {
    load().s3_region
}

pub fn s3_bucket() -> Option<String> {
    load().s3_bucket
}

pub fn s3_access_key() -> Option<String> {
    load().s3_access_key
}

pub fn s3_secret_key() -> Option<String> {
    load().s3_secret_key
}

/// Whether S3 credentials are configured in the WebUI (page only).
pub fn s3_access_key_set() -> bool {
    load().s3_access_key.is_some()
}

pub fn s3_secret_key_set() -> bool {
    load().s3_secret_key.is_some()
}

/// The object-store prefix for this install's backups.
pub fn prefix() -> String {
    load().prefix
}

/// The structured retention policy for the native engine.
pub fn retention_policy_struct() -> RetentionPolicy {
    load().retention
}

// ── native S3 engine options ───────────────────────────────────────────────

/// The effective S3 connection options for the native engine. Page-configured
/// (WebUI) only — the S3 form is the product's configuration surface (design
/// doc §9). Errors when the bucket is missing, so a run fails up front with a
/// clear message instead of silently using a half-configured target (R11).
pub fn s3_options() -> BichonResult<S3Options> {
    let cfg = load();
    let bucket = cfg.s3_bucket.unwrap_or_default();
    if bucket.is_empty() {
        return Err(raise_error!(
            "backup: s3 bucket is not configured (set it in the backup settings)".to_string(),
            ErrorCode::MissingConfiguration
        ));
    }
    Ok(S3Options {
        endpoint: cfg.s3_endpoint.unwrap_or_default(),
        bucket,
        region: cfg.s3_region.unwrap_or_default(),
        access_key: cfg.s3_access_key.unwrap_or_default(),
        secret_key: cfg.s3_secret_key.unwrap_or_default(),
        // Custom endpoints (MinIO/R2/Wasabi style) always use path-style
        // addressing; AWS (no endpoint) keeps the SDK default.
        force_path_style: true,
    })
}

// ── restore config file ────────────────────────────────────────────────────

/// The S3 backup backend configuration for `bichon-admin restore`, read from
/// a plain JSON file (`--config <path>`). Restore is a one-shot
/// disaster-recovery operation that must not depend on the local install's
/// metadata store, so the connection details are supplied out-of-band.
///
/// Field names mirror the WebUI Backup page / [`BackupConfig`] document, so
/// the file can be authored by copying the configured values.
#[derive(Clone, Debug, Deserialize)]
pub struct RestoreFileConfig {
    /// Object-store endpoint, with or without scheme
    /// (`http://minio.internal:9000`, bare host for AWS).
    pub s3_endpoint: String,
    pub s3_bucket: String,
    /// Key prefix of the backup objects. Defaults to [`DEFAULT_PREFIX`].
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub s3_region: Option<String>,
    #[serde(default)]
    pub s3_access_key: Option<String>,
    #[serde(default)]
    pub s3_secret_key: Option<String>,
}

impl RestoreFileConfig {
    /// Read and parse the restore config file.
    pub fn load(path: &Path) -> BichonResult<Self> {
        let raw = std::fs::read_to_string(path).map_err(|e| {
            raise_error!(
                format!(
                    "restore: cannot read config file {}: {e}",
                    path.display()
                ),
                ErrorCode::InvalidParameter
            )
        })?;
        let cfg: RestoreFileConfig = serde_json::from_str(&raw).map_err(|e| {
            raise_error!(
                format!(
                    "restore: invalid restore config {}: {e} (expected fields: \
                     s3_endpoint, s3_bucket, prefix?, s3_region?, s3_access_key?, s3_secret_key?)",
                    path.display()
                ),
                ErrorCode::InvalidParameter
            )
        })?;
        if cfg.s3_endpoint.trim().is_empty() {
            return Err(raise_error!(
                format!(
                    "restore: config file {} is missing s3_endpoint",
                    path.display()
                ),
                ErrorCode::InvalidParameter
            ));
        }
        if cfg.s3_bucket.trim().is_empty() {
            return Err(raise_error!(
                format!(
                    "restore: config file {} is missing s3_bucket",
                    path.display()
                ),
                ErrorCode::InvalidParameter
            ));
        }
        Ok(cfg)
    }

    /// The backup target as `s3://<endpoint>/<bucket>/<prefix>`.
    pub fn s3_uri(&self) -> String {
        let prefix = if self.prefix.trim().is_empty() {
            DEFAULT_PREFIX.to_string()
        } else {
            self.prefix.trim().trim_end_matches('/').to_string()
        };
        format!(
            "s3://{}/{}/{}",
            self.s3_endpoint.trim(),
            self.s3_bucket.trim(),
            prefix
        )
    }
}

// ── Pro rebase flag (stays in system_config) ───────────────────────────────

/// Whether the Pro WebUI requested a forced audit rebase on the next run.
pub fn rebase_requested() -> bool {
    system_config::get(KEY_REBASE).map_or(false, |v| v == "true")
}

/// Clear a consumed rebase request (called by the audit contributor after it
/// produced the new baseline).
pub fn clear_rebase_request() {
    let _ = system_config::delete(KEY_REBASE);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::gate::WRITE_GATE;
    use crate::database::manager::DB_MANAGER;
    use crate::settings::cli::SETTINGS;
    use std::sync::Once;

    static TEST_ENV: Once = Once::new();
    /// The config tests mutate the same process-global document and cache, and
    /// write through the write gate — so they serialize with each other AND
    /// with the manager machine tests (which pause that gate), via the shared
    /// `BACKUP_STATE_TESTS` lock.
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

    /// Wipe the document + cache and any leftover legacy overrides, and make
    /// sure the write gate is open.
    fn cleanup() {
        WRITE_GATE.resume();
        let _ = clear();
        for key in LEGACY_KEYS {
            let _ = system_config::delete(key);
        }
    }

    #[tokio::test]
    async fn defaults_and_saved_roundtrip_win() {
        let _run_guard = crate::backup::BACKUP_STATE_TESTS.lock().await;
        init_test_env();
        cleanup();

        // No document → the code defaults.
        assert!(!enabled());
        assert_eq!(schedule(), DEFAULT_SCHEDULE);
        assert_eq!(prefix(), DEFAULT_PREFIX);
        assert_eq!(retention_policy_struct(), RetentionPolicy::default());

        // A saved document wins over the defaults.
        let cfg = BackupConfig {
            enabled: true,
            schedule: "0 0 * * * *".to_string(),
            prefix: "prod-archive".to_string(),
            retention: RetentionPolicy {
                keep_last: 14,
                keep_daily: 10,
                keep_weekly: 4,
                keep_monthly: 6,
            },
            s3_endpoint: Some("http://localhost:9000".to_string()),
            s3_region: Some("us-east-1".to_string()),
            s3_bucket: Some("bichon".to_string()),
            s3_access_key: Some("minioadmin".to_string()),
            s3_secret_key: Some("minioadmin123".to_string()),
        };
        save(&cfg).unwrap();
        assert!(enabled());
        assert_eq!(schedule(), "0 0 * * * *");
        assert_eq!(prefix(), "prod-archive");
        assert_eq!(retention_policy_struct().keep_last, 14);
        assert_eq!(s3_endpoint(), Some("http://localhost:9000".to_string()));
        assert_eq!(s3_bucket(), Some("bichon".to_string()));
        assert_eq!(s3_access_key(), Some("minioadmin".to_string()));
        assert_eq!(s3_secret_key(), Some("minioadmin123".to_string()));
        assert!(s3_access_key_set() && s3_secret_key_set());

        // Overwrite with a defaults-based document (save is a full-document
        // replace) — cache + document both follow.
        save(&BackupConfig {
            enabled: false,
            ..Default::default()
        })
        .unwrap();
        assert!(!enabled());
        assert_eq!(schedule(), DEFAULT_SCHEDULE);

        cleanup();
    }

    #[tokio::test]
    async fn secrets_are_stored_encrypted_inside_the_document() {
        let _run_guard = crate::backup::BACKUP_STATE_TESTS.lock().await;
        init_test_env();
        cleanup();

        save(&BackupConfig {
            prefix: "prod-archive".to_string(),
            s3_access_key: Some("minioadmin".to_string()),
            s3_secret_key: Some("minioadmin123".to_string()),
            ..Default::default()
        })
        .unwrap();

        // The raw document must never contain the plaintext credentials —
        // but the non-secret fields stay readable.
        let entry: BackupConfigEntry = DB_MANAGER
            .db()
            .collection(BackupConfigEntry::collection())
            .get(ENTRY_KEY)
            .unwrap()
            .expect("document must exist");
        assert!(
            !entry.value.contains("minioadmin123") && !entry.value.contains("minioadmin"),
            "credentials must never be stored in plaintext: {}",
            entry.value
        );
        assert!(entry.value.contains("prod-archive"), "non-secret fields stay plaintext");

        cleanup();
    }

    #[tokio::test]
    async fn s3_options_require_a_bucket_and_carry_page_overrides() {
        let _run_guard = crate::backup::BACKUP_STATE_TESTS.lock().await;
        init_test_env();
        cleanup();

        // Missing bucket → up-front error (R11: no half-configured target).
        assert!(s3_options().is_err());

        save(&BackupConfig {
            s3_endpoint: Some("http://localhost:9000".to_string()),
            s3_bucket: Some("bichon".to_string()),
            s3_region: Some("us-east-1".to_string()),
            ..Default::default()
        })
        .unwrap();
        let opts = s3_options().expect("bucket configured");
        assert_eq!(opts.bucket, "bichon");
        assert_eq!(opts.endpoint, "http://localhost:9000");
        assert_eq!(opts.region, "us-east-1");
        assert!(!s3_access_key_set());
        assert!(!s3_secret_key_set());

        cleanup();
    }

    #[tokio::test]
    async fn legacy_keys_migrate_to_the_document_on_first_load() {
        let _run_guard = crate::backup::BACKUP_STATE_TESTS.lock().await;
        init_test_env();
        cleanup();

        // Simulate a pre-document install: per-key `backup.*` overrides.
        system_config::set(LEGACY_ENABLED, "true").unwrap();
        system_config::set(LEGACY_SCHEDULE, "0 0 2 * * *").unwrap();
        system_config::set(LEGACY_PREFIX, "legacy-archive").unwrap();
        system_config::set(LEGACY_S3_ENDPOINT, "http://localhost:9000").unwrap();
        system_config::set(LEGACY_S3_BUCKET, "bichon").unwrap();
        system_config::set(LEGACY_S3_ACCESS_KEY, "legacy-key").unwrap();
        system_config::set(LEGACY_S3_SECRET_KEY, "legacy-secret").unwrap();
        system_config::set(LEGACY_RETENTION_JSON, r#"{"keep_last":14,"keep_daily":10,"keep_weekly":4,"keep_monthly":6}"#)
            .unwrap();

        // First accessor call triggers the migration.
        assert!(enabled());
        assert_eq!(schedule(), "0 0 2 * * *");
        assert_eq!(prefix(), "legacy-archive");
        assert_eq!(s3_access_key(), Some("legacy-key".to_string()));
        assert_eq!(s3_secret_key(), Some("legacy-secret".to_string()));
        assert_eq!(retention_policy_struct().keep_last, 14);

        // The legacy keys are gone and the document now owns the config.
        for key in LEGACY_KEYS {
            assert!(
                system_config::get(key).is_none(),
                "legacy key {key} must be deleted after migration"
            );
        }
        assert!(find_impl::<BackupConfigEntry>(DB_MANAGER.db(), ENTRY_KEY)
            .unwrap()
            .is_some());

        cleanup();
    }

    #[tokio::test]
    async fn serde_roundtrip_and_forward_compatible_fields() {
        init_test_env();
        // Forward compatibility: a document written by an older release (or
        // hand-edited) that lacks new fields still deserializes.
        let cfg: BackupConfig = serde_json::from_str(r#"{"enabled":true}"#).unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.schedule, DEFAULT_SCHEDULE);
        assert_eq!(cfg.prefix, DEFAULT_PREFIX);
        assert_eq!(cfg.s3_access_key, None);
        assert_eq!(cfg.s3_secret_key, None);

        // Plaintext struct → JSON → back: the credentials survive the
        // encrypt-on-serialize / decrypt-on-deserialize round trip. Long
        // sentinels (the ciphertext is random base64url, so a short value
        // could collide with the plaintext by chance).
        let cfg = BackupConfig {
            s3_access_key: Some("test-access-key-0001".to_string()),
            s3_secret_key: Some("test-secret-key-0001".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(
            !json.contains("test-access-key-0001") && !json.contains("test-secret-key-0001"),
            "credentials must never be serialized in plaintext: {json}"
        );
        let back: BackupConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.s3_access_key.as_deref(), Some("test-access-key-0001"));
        assert_eq!(back.s3_secret_key.as_deref(), Some("test-secret-key-0001"));
    }
    #[test]
    fn validate_rejects_unusable_values() {
        let ok = || BackupConfig {
            schedule: "0 0 3 * * *".to_string(),
            prefix: "bichon-backup".to_string(),
            retention: RetentionPolicy {
                keep_last: 1,
                ..RetentionPolicy::default()
            },
            ..Default::default()
        };
        validate(&ok()).unwrap();

        // Cron: the classic 5-field form (no seconds) is invalid and would
        // otherwise silently disable scheduled backups.
        let mut bad = ok();
        bad.schedule = "0 3 * * *".to_string();
        let e = validate(&bad).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidParameter, "{e:?}");

        // An empty schedule is legal ("no schedule", manual runs only).
        let mut manual = ok();
        manual.schedule = String::new();
        validate(&manual).unwrap();

        // Prefix: empty or unusable as an object-store path.
        let mut bad = ok();
        bad.prefix = String::new();
        assert_eq!(validate(&bad).unwrap_err().code(), ErrorCode::InvalidParameter);
        let mut bad = ok();
        bad.prefix = "bad prefix/../with spaces".to_string();
        assert_eq!(validate(&bad).unwrap_err().code(), ErrorCode::InvalidParameter);

        // Retention: a keep-nothing policy would delete every restore point.
        let mut bad = ok();
        bad.retention = RetentionPolicy {
            keep_last: 0,
            keep_daily: 0,
            keep_weekly: 0,
            keep_monthly: 0,
        };
        assert_eq!(validate(&bad).unwrap_err().code(), ErrorCode::InvalidParameter);
    }
}
