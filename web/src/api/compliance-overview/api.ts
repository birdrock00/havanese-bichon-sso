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
//
// Compliance dashboard API client (Enterprise). One read that aggregates the
// six compliance subsystems into a single governance view: integrity, legal
// hold, exports, dual control, timestamp anchoring, and audit-log health.
//
// Read-only by design — the dashboard adds no write path. Everything it shows
// is changed from the page that owns it.
import axiosInstance from '@/api/axiosInstance'

/** The most recent integrity run, or `null` if none has ever run. */
export interface IntegrityLastRun {
  run_id: string
  status: string
  mode: string
  started_at: number
  finished_at?: number | null
  total: number
  ok: number
  failed: number
}

export interface IntegritySection {
  last_run?: IntegrityLastRun | null
  total_runs: number
}

/** How much of the archive is frozen. `covered_pct` is a raw ratio — the
 *  server does not round, so display precision stays a client decision. */
export interface HoldSection {
  held_accounts: number
  total_accounts: number
  covered_pct: number
  latest_placed_at?: number | null
  latest_placed_reason?: string | null
}

/** Export and verification state.
 *
 *  The server aggregates across all users because the export registry is
 *  per-user: asking for the caller's own exports would show a compliance
 *  officer nothing, since they are not the one who runs exports. The registry
 *  is also in memory and only `finished` jobs survive a restart, so this is
 *  "exports this process knows about" — the durable record is the audit log. */
export interface ExportSection {
  total: number
  verified_ok: number
  verified_mismatched: number
  unverified: number
  verifying: number
  /** Verification ran and errored out. Worse than `unverified`: the export's
   *  integrity is unknown rather than merely unchecked, and someone already
   *  tried. */
  verification_failed: number
  latest_finished_at?: number | null
}

export interface ApprovalSection {
  enabled: boolean
  pending: number
  /** Decisions taken through break-glass — each one is a second signature
   *  that was waived, which is why it is surfaced rather than buried. */
  break_glass_total: number
  /** Hours a request must wait before break-glass applies, or `-1` when the
   *  override is switched off entirely. */
  break_glass_min_hours: number
  gated_ops: number
}

export interface TimestampLatest {
  id: number
  gen_time: number
  status: string
  tsa_url?: string | null
  leaf_count: number
}

export interface TimestampSection {
  anchor_count: number
  latest?: TimestampLatest | null
}

/** Audit-log health.
 *
 *  `dropped` counts events the bounded channel refused because it was full.
 *  A non-zero value means the audit chain has a hole in it — the one number
 *  on this page that an auditor must never have to go to the server log to
 *  find. A long-standing zero is the reassuring reading. */
export interface AuditSection {
  dropped: number
  pending: number
  /** `0` means retention cleanup is disabled. */
  retention_days: number
}

export interface ComplianceOverview {
  generated_at: number
  integrity: IntegritySection
  legal_hold: HoldSection
  exports: ExportSection
  approvals: ApprovalSection
  timestamps: TimestampSection
  audit: AuditSection
}

export async function get_compliance_overview(): Promise<ComplianceOverview> {
  const { data } = await axiosInstance.get<ComplianceOverview>(
    'api/v1/compliance/overview'
  )
  return data
}
