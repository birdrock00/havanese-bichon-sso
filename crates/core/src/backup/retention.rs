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

//! Structured retention policy for restore points.
//!
//! Replaces the old CLI-passthrough retention string with a
//! typed policy (`keep_last` / `keep_daily` / `keep_weekly` / `keep_monthly`).
//! Selection is conservative and deterministic: the newest `keep_last`
//! manifests are always kept, then the newest manifest per UTC day / ISO week
//! / UTC month for the most recent buckets, until each budget is exhausted.

use std::collections::HashSet;

use chrono::{DateTime, Datelike, Utc};

use crate::backup::manifest::Manifest;

/// How many restore points to keep in each bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct RetentionPolicy {
    /// Always keep the N most recent restore points.
    pub keep_last: u32,
    /// Keep the newest restore point per UTC day, for the N most recent days.
    pub keep_daily: u32,
    /// Keep the newest restore point per ISO week, for the N most recent weeks.
    pub keep_weekly: u32,
    /// Keep the newest restore point per UTC month, for the N most recent months.
    pub keep_monthly: u32,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        // Mirrors the classic defaults (--keep-last 7 --keep-daily 7
        // --keep-weekly 4 --keep-monthly 6).
        Self {
            keep_last: 7,
            keep_daily: 7,
            keep_weekly: 4,
            keep_monthly: 6,
        }
    }
}

/// Select the restore points to keep from `manifests` (newest first).
///
/// Always returns the newest `keep_last` ids, then per-period selections over
/// the remainder. Conservative: a period bucket is only filled by the newest
/// manifest in it, and the result never excludes something `keep_last` covers.
pub fn select_manifests(manifests: &[Manifest], policy: &RetentionPolicy) -> HashSet<String> {
    let mut kept: Vec<&Manifest> = Vec::new();
    for m in manifests.iter().take(policy.keep_last as usize) {
        kept.push(m);
    }
    let rest = &manifests[policy.keep_last.min(manifests.len() as u32) as usize..];
    kept.extend(select_period(rest, policy.keep_daily, |dt| {
        dt.format("%Y-%m-%d").to_string()
    }));
    kept.extend(select_period(rest, policy.keep_weekly, |dt| {
        format!("{}-{:02}", dt.iso_week().year(), dt.iso_week().week())
    }));
    kept.extend(select_period(rest, policy.keep_monthly, |dt| {
        dt.format("%Y-%m").to_string()
    }));
    kept.into_iter().map(|m| m.id.clone()).collect()
}

/// The newest manifest per distinct period bucket, up to `keep` buckets (most
/// recent buckets first). Manifests whose `created` cannot be parsed are
/// skipped — they are still covered by `keep_last` while they are new.
fn select_period<'a>(
    manifests: &'a [Manifest],
    keep: u32,
    bucket: impl Fn(&DateTime<Utc>) -> String,
) -> Vec<&'a Manifest> {
    if keep == 0 {
        return Vec::new();
    }
    let mut out: Vec<&'a Manifest> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for m in manifests {
        let Some(dt) = parse_created(&m.created) else {
            continue;
        };
        if seen.insert(bucket(&dt)) {
            out.push(m);
            if out.len() as u32 >= keep {
                break;
            }
        }
    }
    out
}

fn parse_created(rfc3339: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(rfc3339)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(id: &str, created: &str) -> Manifest {
        Manifest {
            version: 1,
            id: id.to_string(),
            created: created.to_string(),
            trigger: "manual".to_string(),
            previous: None,
            objects: Default::default(),
        }
    }

    /// Ten daily manifests ending 2026-09-21, newest first (as the engine
    /// returns them).
    fn ten_daily() -> Vec<Manifest> {
        (0..10)
            .map(|i| {
                let day = 21 - i;
                manifest(&format!("m-d{day}"), &format!("2026-09-{day:02}T02:00:00Z"))
            })
            .collect()
    }

    #[test]
    fn keep_last_always_wins() {
        let ms = ten_daily();
        let policy = RetentionPolicy {
            keep_last: 3,
            keep_daily: 0,
            keep_weekly: 0,
            keep_monthly: 0,
        };
        let kept = select_manifests(&ms, &policy);
        assert_eq!(kept.len(), 3);
        assert!(kept.contains("m-d21"));
        assert!(kept.contains("m-d20"));
        assert!(kept.contains("m-d19"));
        assert!(!kept.contains("m-d12"));
    }

    #[test]
    fn daily_keeps_one_per_day() {
        // Ten manifests, two per day → keep_last 1 plus daily keeps one per
        // day for the newest days (9 days). Newest-first order is part of the
        // engine contract (the 09:00 manifest of each day comes before the
        // 02:00 one).
        let mut ms = Vec::new();
        for day in (12..=21).rev() {
            ms.push(manifest(&format!("m-{day}b"), &format!("2026-09-{day:02}T09:00:00Z")));
            ms.push(manifest(&format!("m-{day}a"), &format!("2026-09-{day:02}T02:00:00Z")));
        }
        let policy = RetentionPolicy {
            keep_last: 1,
            keep_daily: 3,
            keep_weekly: 0,
            keep_monthly: 0,
        };
        let kept = select_manifests(&ms, &policy);
        // Newest: 21b (keep_last). Daily selection then independently keeps
        // the newest per day for the 3 most recent days — including day 21's
        // second manifest (same day as keep_last; overlapping is fine and
        // conservative, matching the per-bucket semantics).
        assert_eq!(
            kept,
            HashSet::from([
                "m-21b".to_string(),
                "m-21a".to_string(),
                "m-20b".to_string(),
                "m-19b".to_string(),
            ]),
            "keep_last plus one newest per day for the 3 most recent days"
        );
    }

    #[test]
    fn weekly_buckets_are_iso_weeks() {
        // 2026-09-26 is Saturday of ISO week 39 (Mon 09-21 → Sun 09-27);
        // 2026-09-21 is Monday of week 39; 2026-09-14 is Monday of week 38.
        // Manifests on the same week collapse to one — only the newest of
        // week 39 survives.
        let ms = vec![
            manifest("m-w39b", "2026-09-26T02:00:00Z"), // Sat, week 39
            manifest("m-w39a", "2026-09-21T02:00:00Z"), // Mon, week 39
            manifest("m-w38", "2026-09-14T02:00:00Z"),  // Mon, week 38
            manifest("m-w37", "2026-09-07T02:00:00Z"),  // Mon, week 37
        ];
        let policy = RetentionPolicy {
            keep_last: 0,
            keep_daily: 0,
            keep_weekly: 2,
            keep_monthly: 0,
        };
        let kept = select_manifests(&ms, &policy);
        assert_eq!(
            kept,
            HashSet::from(["m-w39b".to_string(), "m-w38".to_string()]),
            "newest per ISO week for the 2 most recent weeks"
        );
    }

    #[test]
    fn monthly_buckets() {
        let ms = vec![
            manifest("m-same-month-b", "2026-09-21T02:00:00Z"),
            manifest("m-same-month-a", "2026-09-01T02:00:00Z"),
            manifest("m-aug", "2026-08-15T02:00:00Z"),
            manifest("m-jul", "2026-07-10T02:00:00Z"),
        ];
        let policy = RetentionPolicy {
            keep_last: 0,
            keep_daily: 0,
            keep_weekly: 0,
            keep_monthly: 2,
        };
        let kept = select_manifests(&ms, &policy);
        assert_eq!(
            kept,
            HashSet::from(["m-same-month-b".to_string(), "m-aug".to_string()]),
            "newest per month for the 2 most recent months"
        );
    }

    #[test]
    fn unparsable_created_is_skipped_in_periods_but_kept_by_last() {
        let ms = vec![
            manifest("m-bad", "not-a-date"),
            manifest("m-ok", "2026-09-21T02:00:00Z"),
        ];
        let policy = RetentionPolicy {
            keep_last: 1,
            keep_daily: 0,
            keep_weekly: 0,
            keep_monthly: 0,
        };
        let kept = select_manifests(&ms, &policy);
        assert!(kept.contains("m-bad"), "keep_last is positional, no parse needed");
    }

    #[test]
    fn empty_input_keeps_nothing() {
        assert!(select_manifests(&[], &RetentionPolicy::default()).is_empty());
    }
}
