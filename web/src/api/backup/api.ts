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
import axiosInstance from '@/api/axiosInstance'

/** Current backup state + schedule, mirror of `BackupStatusView`. */
export interface BackupStatusView {
  enabled: boolean
  running: boolean
  phase: string
  started_at: number | null
  current_record_id: string | null
  last_success_at: number | null
  last_error: string | null
  last_manifest_id: string | null
  last_uploaded_bytes: number | null
  last_new_objects: number | null
  last_skipped_objects: number | null
  schedule: string
  next_run_at: string | null
}

export type BackupRunStatus = 'running' | 'success' | 'failed'

/** Per-component transfer stats: which part of the archive was new vs skipped. */
export interface ArtifactStat {
  /** Component name: "memdb", "blob", "envelope-index", "attachment-index", … */
  kind: string
  new_count: number
  new_bytes: number
  skipped_count: number
  skipped_bytes: number
}

/** Per-run stats reported by the native S3 engine. */
export interface BackupRunSummary {
  manifest_id: string
  uploaded_bytes: number
  new_objects: number
  skipped_objects: number
  /** Per-component breakdown of the totals (empty on records written before
   *  this field existed). */
  by_kind: ArtifactStat[]
}

/** Effective backup configuration for the WebUI form. Credentials are exposed
 *  only as `*_set` flags — the plaintext never leaves the server.
 *  S3-only (design doc §9). */
export interface RetentionPolicy {
  keep_last: number
  keep_daily: number
  keep_weekly: number
  keep_monthly: number
}

export interface BackupConfigView {
  enabled: boolean
  schedule: string
  prefix: string
  retention: RetentionPolicy
  s3_endpoint: string | null
  s3_region: string | null
  s3_bucket: string | null
  s3_access_key_set: boolean
  s3_secret_key_set: boolean
}

/** Partial update. `undefined` leaves a field untouched. Secrets follow the
 *  SIEM convention: omit to keep, `'********'` to keep (same thing), `''` to
 *  clear (returning to the env fallback), anything else to replace. */
export interface BackupConfigUpdate {
  enabled?: boolean
  schedule?: string
  prefix?: string
  retention?: RetentionPolicy
  s3_endpoint?: string
  s3_region?: string
  s3_bucket?: string
  s3_access_key?: string
  s3_secret_key?: string
}

/** The effective backup configuration (masked credentials). */
export const getBackupTargetConfig = async () => {
  const response = await axiosInstance.get<BackupConfigView>(
    'api/v1/backup/config'
  )
  return response.data
}

/** Save a partial backup-configuration update. */
export const updateBackupTargetConfig = async (
  payload: BackupConfigUpdate
) => {
  const response = await axiosInstance.post<BackupConfigView>(
    'api/v1/backup/config',
    payload
  )
  return response.data
}

/** One persisted backup run record. */
export interface BackupRecord {
  id: string
  trigger: string
  started_at: number
  finished_at: number | null
  status: BackupRunStatus
  phase: string
  error: string | null
  snapshot_id: string | null
  summary: BackupRunSummary | null
  warnings: string[]
}

/** One referenced object (content-addressed, immutable). */
export interface ObjectView {
  key: string
  bytes: number
  /** For index objects: which index this is (`envelope` / `attachment`). */
  name?: string | null
  /** For blob segments: the segment id within the blob store. */
  segment_id?: number | null
}

/** The audit chain of a restore point. */
export interface AuditDeltaView extends ObjectView {
  seq_from: number
  seq_to: number
}

export interface AuditView {
  base: ObjectView | null
  deltas: AuditDeltaView[]
  cursor: number
  cumulative_delta_bytes: number
  base_ratio: number | null
}

/** One restore point in the manifest browser. */
export interface ManifestView {
  id: string
  /** RFC 3339 (UTC) creation time of the restore point. */
  created: string
  trigger: string
  bytes_total: number
  object_count: number
  memdb: ObjectView | null
  imap_uid: ObjectView | null
  segments: ObjectView[]
  audit: AuditView | null
  integrity: ObjectView | null
  timestamp: ObjectView | null
  tantivy: ObjectView[]
}

/** Triggers a backup run immediately. 202 when started. */
export const triggerBackup = async () => {
  const response =
    await axiosInstance.post<BackupStatusView>('api/v1/backup/run')
  return response.data
}

export const getBackupStatus = async () => {
  const response = await axiosInstance.get<BackupStatusView>(
    'api/v1/backup/status'
  )
  return response.data
}

export const listBackupRecords = async (limit = 20) => {
  const response = await axiosInstance.get<BackupRecord[]>(
    'api/v1/backup/records',
    {
      params: { limit },
    }
  )
  return response.data
}

/** Every committed restore point (manifest), newest first. */
export const listBackupManifests = async () => {
  const response = await axiosInstance.get<ManifestView[]>(
    'api/v1/backup/manifests'
  )
  return response.data
}

/** One restore point's full object composition. */
export const getBackupManifest = async (id: string) => {
  const response = await axiosInstance.get<ManifestView>(
    `api/v1/backup/manifests/${id}`
  )
  return response.data
}

/** Result of deleting one restore point. */
export interface ManifestDeleteResult {
  manifest_id: string
  /** Objects removed from the bucket (only ones no surviving restore point
   *  referenced). */
  objects_removed: number
  /** Bytes freed from the bucket. */
  bytes_reclaimed: number
}

/** Delete one restore point and reclaim the objects only it referenced.
 *  Rejected with 429 while a backup run is in progress. */
export const deleteBackupManifest = async (id: string) => {
  const response = await axiosInstance.delete<ManifestDeleteResult>(
    `api/v1/backup/manifests/${id}`
  )
  return response.data
}

/** One audit delta as gzip JSONL — the timestamped compliance export
 *  artifact. Covers rows `seq_from < seq <= seq_to`. */
export const downloadAuditDelta = async (
  manifestId: string,
  seqFrom: number,
  seqTo: number
) => {
  const response = await axiosInstance.get<Blob>(
    `api/v1/backup/manifests/${manifestId}/audit/${seqFrom}/${seqTo}`,
    { responseType: 'blob' }
  )
  return response.data
}

// ── Pro/Enterprise: rebase + restore drills ────────────────────────────────
// These endpoints only exist on the Pro/Enterprise server; the community
// pages never call them.

/** Outcome of one restore drill, persisted in the memdb. */
export interface DrillReport {
  id: string
  manifest_id: string
  created: string
  ran_at: number
  passed: boolean
  objects_total: number
  bytes_total: number
  bytes_checked: number
  note: string | null
  error: string | null
}

/** Request the next run to produce a fresh audit baseline (clears the delta
 *  chain). The request survives until it is applied. */
export const requestBackupRebase = async () => {
  const response = await axiosInstance.post<{
    requested: boolean
    applied_on_next_run: boolean
  }>('api/v1/backup/rebase')
  return response.data
}

/** Verify one restore point against the target and persist the report. */
export const runBackupDrill = async (manifestId: string) => {
  const response = await axiosInstance.post<DrillReport>(
    `api/v1/backup/manifests/${manifestId}/drill`
  )
  return response.data
}

/** The most recent drill report, if any. */
export const getLatestBackupDrill = async () => {
  const response = await axiosInstance.get<{ report: DrillReport | null }>(
    'api/v1/backup/drill'
  )
  return response.data
}
