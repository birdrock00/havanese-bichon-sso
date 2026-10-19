//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Path comparison and free-space helpers shared by restore and the backup
//! targets.
//!
//! These started life private to [`crate::backup::restore`] (the R7 live-root
//! guard). The local backup target needs the same containment test to keep a
//! backup directory out of the data root, so they live here instead of being
//! copied — a second copy would drift on the Windows verbatim-path handling,
//! which is the whole reason [`normalize`] exists.

use std::path::Path;

use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::raise_error;

// ── path safety helpers (Windows-tolerant) ─────────────────────────────────

pub(crate) fn paths_equal(a: &Path, b: &Path) -> bool {
    normalize(a) == normalize(b)
}

/// `a` is `b` or inside it (case-insensitive on Windows).
pub(crate) fn path_contains(container: &Path, inside: &Path) -> bool {
    let c = normalize(container);
    let i = normalize(inside);
    i == c || i.starts_with(&c)
}

/// Canonicalized, separator-normalized, trailing-slash-terminated form used
/// for every path comparison in this module.
///
/// A path that cannot be canonicalized (it does not exist yet) falls back to
/// its raw spelling — comparisons then stay best-effort rather than panicking.
/// macOS is case-insensitive by default but only Windows is lowercased here,
/// so two paths differing solely in case compare unequal on APFS. That is
/// acceptable: it can only produce a false *negative* on a containment guard
/// that is a safety net, never a false rejection of a legitimate target.
pub(crate) fn normalize(p: &Path) -> String {
    let canon = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let mut s = canon.to_string_lossy().replace('\\', "/");
    // Windows `canonicalize` returns verbatim paths (`\\?\C:\...`), which
    // would break prefix comparison against a raw path for the same
    // directory; strip the prefix so both sides compare on the same form.
    if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string();
    }
    if !s.ends_with('/') {
        s.push('/');
    }
    #[cfg(windows)]
    {
        s = s.to_lowercase();
    }
    s
}

// ── free space ─────────────────────────────────────────────────────────────

/// Safety margin applied to the bytes a restore needs before comparing
/// against free space (mirrors the export path's `DISK_SAFETY_FACTOR`).
pub(crate) const DISK_SAFETY_FACTOR: f64 = 1.2;

/// Refuse the operation when the filesystem holding `dir` cannot take
/// `needed` bytes plus [`DISK_SAFETY_FACTOR`]. `prefix` labels the caller
/// (`"restore"` / `"backup"`) so the message points at the right subsystem.
pub(crate) fn check_free_space(
    prefix: &str,
    name: &str,
    dir: &Path,
    needed: u64,
) -> BichonResult<()> {
    if needed == 0 {
        return Ok(());
    }
    check_free_space_against(prefix, name, dir, needed, available_space_on(dir))
}

/// The threshold decision, split out from the disk lookup so it can be
/// tested without depending on which mounts the host happens to have.
fn check_free_space_against(
    prefix: &str,
    name: &str,
    dir: &Path,
    needed: u64,
    free: u64,
) -> BichonResult<()> {
    let required = (needed as f64 * DISK_SAFETY_FACTOR) as u64;
    if free < required {
        return Err(raise_error!(
            format!(
                "{prefix}: insufficient free space on the {name} target {}: {} bytes required (with safety buffer), {} bytes available",
                dir.display(),
                required,
                free
            ),
            ErrorCode::InternalError
        ));
    }
    Ok(())
}

/// Free bytes on the filesystem containing `dir`, matched by mount-point
/// prefix (mirrors `crate::import::check_temp_disk_space`). Returns
/// `u64::MAX` when no mount point matches, so an unmatchable target never
/// blocks on a guess.
pub(crate) fn available_space_on(dir: &Path) -> u64 {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    for disk in disks.list() {
        if canonical.starts_with(disk.mount_point()) {
            return disk.available_space();
        }
    }
    u64::MAX
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A unique empty directory under the system temp dir. The directory is
    /// created (so `canonicalize` succeeds) and named per call, because
    /// parallel tests must never share a path.
    fn temp_dir(tag: &str) -> PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "bichon-path-test-{}-{tag}-{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn normalize_always_ends_with_a_separator() {
        let dir = temp_dir("norm");
        assert!(normalize(&dir).ends_with('/'));
    }

    #[test]
    fn normalize_falls_back_to_the_raw_path_when_it_does_not_exist() {
        let missing = temp_dir("fallback").join("definitely-not-here");
        // No panic, and the trailing separator is still applied so the
        // prefix comparison in `path_contains` stays sound.
        let s = normalize(&missing);
        assert!(s.ends_with('/'));
        assert!(s.contains("definitely-not-here"));
    }

    #[test]
    fn paths_equal_ignores_trailing_separators_and_dot_segments() {
        let dir = temp_dir("equal");
        let roundabout = dir.join(".").join("");
        assert!(paths_equal(&dir, &roundabout));
        assert!(paths_equal(&dir, &dir));
    }

    #[test]
    fn path_contains_is_bidirectional_for_nested_paths() {
        let outer = temp_dir("outer");
        let inner = outer.join("inner");
        std::fs::create_dir_all(&inner).unwrap();

        assert!(path_contains(&outer, &inner), "outer contains inner");
        assert!(
            !path_contains(&inner, &outer),
            "inner must not be reported as containing outer"
        );
        // A path contains itself.
        assert!(path_contains(&outer, &outer));
    }

    #[test]
    fn path_contains_does_not_match_a_sibling_with_a_shared_prefix() {
        // `/tmp/x/a` must not "contain" `/tmp/x/ab` — the trailing separator
        // added by `normalize` is what prevents this.
        let parent = temp_dir("sibling");
        let a = parent.join("a");
        let ab = parent.join("ab");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&ab).unwrap();

        assert!(!path_contains(&a, &ab));
        assert!(!path_contains(&ab, &a));
    }

    #[test]
    fn check_free_space_skips_a_zero_byte_requirement() {
        let dir = temp_dir("space");
        // Must not consult the disk list at all for a no-op request.
        assert!(check_free_space("backup", "local", &dir, 0).is_ok());
    }

    #[test]
    fn check_free_space_rejects_a_target_that_is_short() {
        let dir = temp_dir("space-short");
        let err = check_free_space_against("backup", "local", &dir, 1000, 10).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("insufficient free space"), "{msg}");
        assert!(msg.contains("backup:"), "prefix must label the caller: {msg}");
    }

    #[test]
    fn check_free_space_applies_the_safety_factor() {
        let dir = temp_dir("space-factor");
        // 100 bytes needed, factor 1.2 → 120 required. Exactly 120 fits;
        // 119 does not.
        assert!(check_free_space_against("backup", "local", &dir, 100, 120).is_ok());
        assert!(check_free_space_against("backup", "local", &dir, 100, 119).is_err());
    }

    #[test]
    fn check_free_space_never_blocks_on_an_unknown_free_space() {
        // `available_space_on` returns `u64::MAX` when no mount point matches
        // (see its doc), which must read as "unknown, do not block".
        let dir = temp_dir("space-unknown");
        assert!(check_free_space_against("backup", "local", &dir, u64::MAX, u64::MAX).is_ok());
    }
}
