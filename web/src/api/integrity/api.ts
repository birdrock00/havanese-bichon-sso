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
// Integrity check API client (Pro edition). Manual full/quick verification of
// the archive: re-attach detached attachments, recompute content hashes and
// compare against the envelope index. Reports are exposed per run with CSV
// downloads.
import { saveAs } from 'file-saver'
import axiosInstance from '@/api/axiosInstance'

export type IntegrityMode = 'full' | 'quick'
export type IntegrityStatus = 'running' | 'finished' | 'cancelled' | 'failed'

export interface IntegrityRunRequest {
  account_ids?: number[]
  mode?: IntegrityMode
}

export interface RunStarted {
  run_id: string
  status: string
}

/** Live progress of the currently active run (running only). */
export interface JobProgress {
  run_id: string
  status: string
  triggered_by: string
  scope: number[]
  mode: string
  total: number
  processed: number
  ok: number
  failed: number
  current_account_id?: number | null
  current_account_name?: string | null
  started_at: number
  finished_at?: number | null
  failed_by_type: Record<string, number>
  message?: string | null
}

/** Persisted summary of a finished (or running) run. */
export interface RunSummary {
  run_id: string
  triggered_by: string
  scope: number[]
  mode: string
  status: string
  started_at: number
  finished_at?: number | null
  total: number
  ok: number
  failed: number
  failed_by_type: Record<string, number>
  message?: string | null
}

export interface AccountStat {
  account_id: number
  account_name: string
  total: number
  ok: number
  failed: number
  integrity_pct: number
  corruption_pct: number
}

export interface FailureRow {
  account_id: number
  account_name?: string | null
  mailbox_id?: number | null
  mailbox_name?: string | null
  envelope_id: string
  message_id?: string | null
  subject?: string | null
  uid?: number | null
  internal_date?: number | null
  size?: number | null
  expected_hash?: string | null
  actual_hash?: string | null
  failure_type: string
  detail?: string | null
}

export interface RunPage {
  items: RunSummary[]
  total: number
  page: number
  page_size: number
}

export interface FailurePage {
  items: FailureRow[]
  total: number
  page: number
  page_size: number
}

export interface IntegrityReport {
  run_id: string
  triggered_by: string
  scope: number[]
  mode: string
  status: string
  started_at: number
  finished_at?: number | null
  total: number
  ok: number
  failed: number
  integrity_pct: number
  corruption_pct: number
  failed_by_type: Record<string, number>
  message?: string | null
  accounts: AccountStat[]
  failures: FailurePage
}

export async function start_integrity_run(
  body: IntegrityRunRequest
): Promise<RunStarted> {
  const { data } = await axiosInstance.post<RunStarted>(
    'api/v1/integrity/run',
    body
  )
  return data
}

export async function get_active_integrity_run(): Promise<
  JobProgress | { active: false }
> {
  const { data } = await axiosInstance.get<JobProgress | { active: false }>(
    'api/v1/integrity/active'
  )
  return data
}

export async function list_integrity_runs(
  page: number,
  page_size: number
): Promise<RunPage> {
  const { data } = await axiosInstance.get<RunPage>('api/v1/integrity/jobs', {
    params: { page, page_size },
  })
  return data
}

export async function get_integrity_run(
  run_id: string
): Promise<JobProgress | RunSummary> {
  const { data } = await axiosInstance.get<JobProgress | RunSummary>(
    `api/v1/integrity/jobs/${run_id}`
  )
  return data
}

export async function cancel_integrity_run(
  run_id: string
): Promise<{ cancelled: boolean }> {
  const { data } = await axiosInstance.post<{ cancelled: boolean }>(
    `api/v1/integrity/jobs/${run_id}/cancel`
  )
  return data
}

export async function get_integrity_report(
  run_id: string,
  page: number,
  page_size: number
): Promise<IntegrityReport> {
  const { data } = await axiosInstance.get<IntegrityReport>(
    `api/v1/integrity/jobs/${run_id}/report`,
    { params: { page, page_size } }
  )
  return data
}

export async function download_integrity_report(
  run_id: string,
  kind: 'summary' | 'failures'
): Promise<void> {
  const response = await axiosInstance.get(
    `api/v1/integrity/jobs/${run_id}/report/${kind}.csv`,
    { responseType: 'blob' }
  )
  const blob = new Blob([response.data])
  saveAs(blob, `integrity-${kind}-${run_id}.csv`)
}
