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

//! The backup backend seam.
//!
//! The manager drives a [`BackupBackend`] — the seam between the capture
//! window and the native S3 engine (design doc §10). Where a whole-directory
//! copier would snapshot a tree, a backend now receives
//! the exact [`Artifact`]s captured during the write window and persists them
//! as a restore point. The manager never sees S3, content addressing, GC or
//! manifests — those are the engine's job ([`crate::backup::engine`]).
//!
//! [`EngineBackend`] is the production implementation. The trait stays
//! object-safe so the state machine can be exercised with fakes in tests and
//! so an alternative backend could be swapped in without touching the
//! manager.

use std::future::Future;
use std::pin::Pin;

use tracing::{info, warn};

use crate::backup::artifact::Artifact;
use crate::backup::config;
use crate::backup::engine::{self, ArtifactStat, BackupEngine, EngineSummary};
use crate::backup::manifest::Manifest;
use crate::backup::retention::RetentionPolicy;
use crate::error::BichonResult;

/// Audit-chain facts the backend stamps on a restore point's manifest,
/// forwarded verbatim from the audit contributor's upload-phase delivery. The
/// manager and backend treat them as opaque — only the Pro audit contributor
/// sets them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AuditStamp {
    /// Highest audit seq covered by this run's delivery.
    pub cursor: Option<i64>,
    /// Cumulative audit delta bytes since the current baseline, after this
    /// run.
    pub cumulative_delta_bytes: Option<u64>,
}

/// Per-run stats surfaced in the run record / status page.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct BackupSummary {
    /// Id of the committed restore point (empty when the run failed before
    /// commit).
    pub manifest_id: String,
    pub uploaded_bytes: u64,
    pub new_objects: u64,
    pub skipped_objects: u64,
    /// Per-component breakdown of the totals above, so the UI can say which
    /// part of the archive was new vs skipped (`#[serde(default)]` keeps
    /// records written before this field readable).
    #[serde(default)]
    pub by_kind: Vec<ArtifactStat>,
}

/// What the backup manager needs from a persistence backend.
///
/// * **preflight** — verify the target is reachable *and* writable before
///   anything is paused, so a misconfigured target never stalls the server.
/// * **upload** — persist the captured artifacts as a restore point and
///   apply retention. Runs with the write gate already reopened (R2); the
///   manager guarantees every artifact it hands over is an immutable file.
pub trait BackupBackend: Send + Sync {
    /// Human-readable name for logs and status (e.g. `"s3"`).
    fn name(&self) -> &'static str;

    /// Verify the target before the capture window opens.
    fn preflight<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = BichonResult<()>> + Send + 'a>>;

    /// The most recently committed restore point, for contributors that need
    /// chain state during the upload phase (`None` before the first commit).
    fn previous_manifest<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = BichonResult<Option<Manifest>>> + Send + 'a>>;

    /// Persist `artifacts` as one restore point and apply `retention`.
    /// `audit` carries audit-chain facts to stamp on the manifest (opaque to
    /// the backend). Runs with the write gate already reopened (R2); the
    /// manager guarantees every artifact it hands over is an immutable file.
    fn upload<'a>(
        &'a self,
        artifacts: &'a [Artifact],
        trigger: &'a str,
        retention: &'a RetentionPolicy,
        audit: &'a AuditStamp,
    ) -> Pin<Box<dyn Future<Output = BichonResult<BackupSummary>> + Send + 'a>>;
}

/// The native S3 backend: content-addressed upload, manifest commit, GC.
pub struct EngineBackend {
    engine: BackupEngine,
}

impl EngineBackend {
    /// Build the backend from the effective S3 configuration (WebUI
    /// overrides). Errors when the target is not fully configured.
    pub fn from_settings() -> BichonResult<Self> {
        let opts = config::s3_options()?;
        let store = engine::s3_store(&opts)?;
        let prefix = config::prefix();
        Ok(Self {
            engine: BackupEngine::new(store, &prefix),
        })
    }

    /// Wrap an existing engine (tests use an in-memory or local store).
    #[cfg(test)]
    pub fn with_engine(engine: BackupEngine) -> Self {
        Self { engine }
    }

    /// A read-only engine handle over the configured target (manifest
    /// browser: list/detail/audit download). Same store + prefix as the
    /// run-time backend, so what the browser shows is what a run wrote.
    pub fn browser_engine() -> BichonResult<BackupEngine> {
        let opts = config::s3_options()?;
        let store = engine::s3_store(&opts)?;
        Ok(BackupEngine::new(store, &config::prefix()))
    }
}

impl BackupBackend for EngineBackend {
    fn name(&self) -> &'static str {
        "s3"
    }

    fn preflight<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = BichonResult<()>> + Send + 'a>> {
        Box::pin(async move { self.engine.preflight().await })
    }

    fn previous_manifest<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = BichonResult<Option<Manifest>>> + Send + 'a>> {
        Box::pin(async move { self.engine.load_latest().await })
    }

    fn upload<'a>(
        &'a self,
        artifacts: &'a [Artifact],
        trigger: &'a str,
        retention: &'a RetentionPolicy,
        audit: &'a AuditStamp,
    ) -> Pin<Box<dyn Future<Output = BichonResult<BackupSummary>> + Send + 'a>> {
        Box::pin(async move {
            // Chain to the previously committed restore point, if any.
            let previous = self.engine.load_latest().await?;
            // v7: time-ordered, so manifest ids sort by creation.
            let id = uuid::Uuid::now_v7().to_string();
            let mut manifest = Manifest::new(
                id,
                trigger,
                previous.as_ref().map(|m| m.id.clone()),
            );

            // Upload first, commit last (R4): a crash here leaves only
            // unreferenced objects, reclaimed by the next GC pass.
            let mut esum = EngineSummary::default();
            for art in artifacts {
                self.engine.upload_artifact(art, &mut esum).await?;
                manifest.add_artifact(art)?;
            }
            // Stamp the audit-chain facts reported by the audit contributor
            // (cursor always; cumulative only when the contributor moved it).
            if let Some(cursor) = audit.cursor {
                manifest.set_audit_cursor(cursor)?;
            }
            if let Some(cumulative) = audit.cumulative_delta_bytes {
                if let Some(a) = manifest.objects.audit.as_mut() {
                    a.cumulative_delta_bytes = cumulative;
                }
            }
            // Manifests are fully self-describing: "each lists every object it
            // references" (design doc §4). A delta-only restore point therefore
            // inherits the baseline and the earlier deltas of the chain it
            // extends — otherwise pruning the manifest that held the baseline
            // would let GC delete the base object a newer restore point still
            // needs (R6: GC only deletes objects outside the union of the
            // retained manifests' references).
            if let Some(audit) = manifest.objects.audit.as_mut() {
                // A run that delivered its own baseline (first run / rebase)
                // already carries the fresh chain — inherit nothing.
                if audit.base.key.is_empty() {
                    if let Some(pa) = previous.as_ref().and_then(|p| p.objects.audit.as_ref()) {
                        audit.base = pa.base.clone();
                        // Inherit every earlier delta this run did not
                        // re-export. Delta ranges are half-open — they cover
                        // rows `seq_from < seq <= seq_to` — so a contiguous
                        // chain has `previous.last.seq_to == new.first.seq_from`
                        // (`<=`, not `<`, or the inherited chain silently
                        // loses the rows of the previous delta).
                        let first_new = audit.deltas.iter().map(|d| d.seq_from).min();
                        for d in &pa.deltas {
                            let keep = match first_new {
                                Some(f) => d.seq_to <= f,
                                None => true,
                            };
                            if keep && !audit.deltas.iter().any(|e| e.seq_from == d.seq_from) {
                                audit.deltas.push(d.clone());
                            }
                        }
                        audit.deltas.sort_by_key(|d| d.seq_from);
                    }
                }
            }
            self.engine.commit(&manifest).await?;

            // Retention sweep with the write gate open (R2). A failed sweep
            // must not fail the run — the restore point is committed and
            // intact — but it is logged loudly (R11).
            match self.engine.gc(retention).await {
                Ok(gc) => {
                    info!(
                        "backup: retention reclaimed {} expired manifests, {} objects ({} bytes)",
                        gc.manifests_removed, gc.objects_removed, gc.bytes_reclaimed
                    );
                }
                Err(e) => {
                    warn!("backup: retention gc failed (restore point kept): {e:?}");
                }
            }

            Ok(BackupSummary {
                manifest_id: manifest.id,
                uploaded_bytes: esum.uploaded_bytes,
                new_objects: esum.new_objects,
                skipped_objects: esum.skipped_objects,
                by_kind: esum.by_kind,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::artifact::ArtifactKey;
    use crate::backup::prepare::artifact_from_file;
    use std::path::PathBuf;

    fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bichon-backend-test-{}-{}",
            std::process::id(),
            name
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    /// Two `EngineBackend`s over the same in-memory store, so a run's LATEST
    /// is visible to the next run's upload (the real production sequence).
    fn backend_pair() -> (EngineBackend, EngineBackend) {
        let store = BackupEngine::memory_store();
        (
            EngineBackend::with_engine(BackupEngine::new(store.clone(), "test")),
            EngineBackend::with_engine(BackupEngine::new(store, "test")),
        )
    }

    #[tokio::test]
    async fn delta_only_restore_point_inherits_the_full_chain() {
        let (first, second) = backend_pair();
        let retention = RetentionPolicy::default();

        // Run 1: a baseline (rebase) — the manifest carries the base object.
        let base_file = write_temp("base.bin", b"audit baseline");
        let base_art = artifact_from_file(base_file, ArtifactKey::AuditBase).unwrap();
        let run1 = first
            .upload(
                &[base_art],
                "manual",
                &retention,
                &AuditStamp {
                    cursor: Some(100),
                    cumulative_delta_bytes: Some(0),
                },
            )
            .await
            .unwrap();

        // Run 2: a delta run — no base delivered, one new delta object.
        let delta_file = write_temp("delta.bin", b"audit delta 101..150");
        let delta_art = artifact_from_file(delta_file, ArtifactKey::AuditDelta(101, 150)).unwrap();
        let run2 = second
            .upload(
                &[delta_art],
                "manual",
                &retention,
                &AuditStamp {
                    cursor: Some(150),
                    cumulative_delta_bytes: Some(24),
                },
            )
            .await
            .unwrap();

        // The newer restore point must reference the baseline AND the earlier
        // chain state, not just its own delta — a single manifest alone must
        // be restorable and must keep the whole chain reachable for GC (R6).
        let m2 = first
            .engine
            .load_by_id(&run2.manifest_id)
            .await
            .unwrap()
            .expect("run 2 manifest must be committed");
        let audit = m2.objects.audit.expect("audit slot must exist");
        assert!(
            !audit.base.key.is_empty(),
            "a delta-only restore point must still reference the baseline"
        );
        assert_eq!(audit.base.key, run1_id_base_key(&first, &run1.manifest_id).await);
        assert_eq!(audit.cursor, 150);
        assert_eq!(audit.cumulative_delta_bytes, 24);
        // The inherited chain + this run's delta, in seq order.
        assert_eq!(audit.deltas.len(), 1, "one delta: this run's");
        assert_eq!(audit.deltas[0].seq_from, 101);
        assert_eq!(audit.deltas[0].seq_to, 150);
    }

    #[tokio::test]
    async fn no_change_run_still_references_the_chain() {
        let (first, second) = backend_pair();
        let retention = RetentionPolicy::default();

        let base_file = write_temp("base2.bin", b"audit baseline v2");
        let base_art = artifact_from_file(base_file, ArtifactKey::AuditBase).unwrap();
        first
            .upload(
                &[base_art],
                "manual",
                &retention,
                &AuditStamp {
                    cursor: Some(100),
                    cumulative_delta_bytes: Some(0),
                },
            )
            .await
            .unwrap();

        // Run 2: nothing new — the contributor still reports the unchanged
        // chain facts, so the restore point keeps referencing the chain.
        let run2 = second
            .upload(
                &[],
                "manual",
                &retention,
                &AuditStamp {
                    cursor: Some(100),
                    cumulative_delta_bytes: Some(0),
                },
            )
            .await
            .unwrap();
        let m2 = first
            .engine
            .load_by_id(&run2.manifest_id)
            .await
            .unwrap()
            .expect("run 2 manifest must be committed");
        let audit = m2.objects.audit.expect("audit slot must exist");
        assert!(
            !audit.base.key.is_empty(),
            "a no-change restore point must still reference the baseline"
        );
        assert!(audit.deltas.is_empty());
        assert_eq!(audit.cursor, 100);
    }

    /// Delta ranges are half-open (`seq_from < seq <= seq_to`), so a
    /// contiguous chain has `previous.last.seq_to == new.first.seq_from`.
    /// The inheritance must keep that previous delta — dropping it silently
    /// loses its rows from the chain (and GC then reclaims the object).
    #[tokio::test]
    async fn inherited_chain_keeps_the_delta_touching_the_new_one() {
        let (first, second) = backend_pair();
        let retention = RetentionPolicy::default();

        // Run 1: baseline plus one delta covering rows 51..=100.
        let base_file = write_temp("base3.bin", b"audit baseline v3");
        let base_art = artifact_from_file(base_file, ArtifactKey::AuditBase).unwrap();
        let d1_file = write_temp("delta3a.bin", b"audit delta 51..100");
        let d1 = artifact_from_file(d1_file, ArtifactKey::AuditDelta(51, 100)).unwrap();
        first
            .upload(
                &[base_art, d1],
                "manual",
                &retention,
                &AuditStamp {
                    cursor: Some(100),
                    cumulative_delta_bytes: Some(50),
                },
            )
            .await
            .unwrap();

        // Run 2: the contiguous next delta — starts exactly where the
        // previous one ended.
        let d2_file = write_temp("delta3b.bin", b"audit delta 101..150");
        let d2 = artifact_from_file(d2_file, ArtifactKey::AuditDelta(101, 150)).unwrap();
        let run2 = second
            .upload(
                &[d2],
                "manual",
                &retention,
                &AuditStamp {
                    cursor: Some(150),
                    cumulative_delta_bytes: Some(100),
                },
            )
            .await
            .unwrap();

        let m2 = first
            .engine
            .load_by_id(&run2.manifest_id)
            .await
            .unwrap()
            .expect("run 2 manifest must be committed");
        let audit = m2.objects.audit.expect("audit slot must exist");
        assert_eq!(audit.deltas.len(), 2, "inherited delta + this run's");
        assert_eq!(audit.deltas[0].seq_from, 51);
        assert_eq!(audit.deltas[0].seq_to, 100, "the previous delta is inherited");
        assert_eq!(audit.deltas[1].seq_from, 101);
        assert_eq!(audit.deltas[1].seq_to, 150);
    }

    /// The base object key recorded by run 1's manifest (uploaded once, so its
    /// key is the same object any delta-only manifest inherits).
    async fn run1_id_base_key(backend: &EngineBackend, id: &str) -> String {
        backend
            .engine
            .load_by_id(id)
            .await
            .unwrap()
            .expect("run 1 manifest must exist")
            .objects
            .audit
            .expect("run 1 must carry the baseline")
            .base
            .key
    }

    /// A failed run must never execute the retention sweep: cleanup is
    /// reserved for runs that committed a new restore point. Run 2 fails
    /// mid-upload under an aggressive keep_last=1 — the previous restore
    /// point, already expired under that policy, must survive (a *successful*
    /// run 2 would have reclaimed it).
    #[tokio::test]
    async fn failed_upload_skips_retention_cleanup() {
        let (first, second) = backend_pair();
        let retention = RetentionPolicy {
            keep_last: 1,
            keep_daily: 0,
            keep_weekly: 0,
            keep_monthly: 0,
        };

        let ok_file = write_temp("ok.bin", b"surviving restore point");
        let ok_art = artifact_from_file(ok_file, ArtifactKey::Memdb).unwrap();
        let run1 = first
            .upload(&[ok_art], "manual", &retention, &AuditStamp::default())
            .await
            .unwrap();

        // Run 2 points at a file that does not exist — the artifact loop
        // fails before any commit or cleanup can happen.
        let ghost_art = Artifact {
            local: std::env::temp_dir().join(format!(
                "bichon-backend-test-{}-missing.bin",
                std::process::id()
            )),
            logical: ArtifactKey::Memdb,
            sha256: [0u8; 32],
            bytes: 10,
        };
        assert!(
            second
                .upload(&[ghost_art], "manual", &retention, &AuditStamp::default())
                .await
                .is_err(),
            "the upload must fail"
        );

        assert!(
            first
                .engine
                .load_by_id(&run1.manifest_id)
                .await
                .unwrap()
                .is_some(),
            "a failed run must not execute retention cleanup"
        );
    }
}
