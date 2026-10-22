pub mod account;
pub mod backup;
pub mod ext;
pub mod admin;
pub mod autoconfig;
pub mod archive;
pub mod common;
pub mod context;
pub mod dashboard;
pub mod database;
pub mod envelope;
pub mod error;
pub mod export;
pub mod imap;
pub mod import;
pub mod logger;
pub mod mailbox;
pub mod message;
pub mod metrics;
pub mod migrate;
pub mod oauth2;
pub mod oidc;
pub mod retention;
pub mod saved_search;
pub mod settings;
pub mod store;
pub mod system_config;
pub mod tasks;
pub mod token;
pub mod users;
pub mod utils;
pub mod version;

#[cfg(test)]
pub(crate) mod test_support {
    /// Serializes test code that **writes** the process-global envelope /
    /// attachment index.
    ///
    /// Test modules run in parallel and several of them share the same global
    /// `ENVELOPE_MANAGER` (tag updates, retention sweeps, ingest). Concurrent
    /// commits to one tantivy directory make Windows refuse the files with
    /// `PermissionDenied` ("拒绝访问"), which surfaces as rare failures in
    /// unrelated-looking tests. Any test that writes the global index must hold
    /// this lock for the duration of its writes.
    pub(crate) static GLOBAL_INDEX_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));
    /// A fresh data root for one test process.
    ///
    /// Deliberately **not** keyed on the pid alone: Windows recycles pids, so a
    /// later run can land on a previous run's directory and inherit its data —
    /// including an index built by an older schema generation, which then fails
    /// in unrelated tests with a confusing "field not found". Modules add a
    /// per-process nonce so a recycled pid always starts from an empty root.
    pub(crate) fn unique_test_root(prefix: &str) -> std::path::PathBuf {
        // Stable per (process, prefix): callers hold the result for the whole
        // test, and a second call must not mint a second root.
        static ROOTS: std::sync::LazyLock<
            std::sync::Mutex<std::collections::HashMap<String, std::path::PathBuf>>,
        > = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

        let mut roots = ROOTS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = roots.get(prefix) {
            return existing.clone();
        }

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root =
            std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&root).expect("create the test data root");
        roots.insert(prefix.to_string(), root.clone());
        root
    }
}
