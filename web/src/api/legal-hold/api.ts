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
// Legal Hold API client (Enterprise). Accounts under a legal hold are frozen:
// the retention sweep skips them and bulk deletion refuses to purge them.
import axiosInstance from '@/api/axiosInstance'

/** Account summary as returned by the legal-hold console endpoints. */
export interface HoldAccount {
  id: number
  email: string
  name?: string | null
  reason?: string | null
  placed_by?: number | null
  placed_by_name?: string | null
  placed_at?: number | null
}

export async function list_legal_holds(): Promise<HoldAccount[]> {
  const { data } = await axiosInstance.get<HoldAccount[]>('api/v1/legal-hold')
  return data
}

export async function place_legal_hold(
  account_id: number,
  reason: string
): Promise<HoldAccount> {
  const { data } = await axiosInstance.post<HoldAccount>(
    `api/v1/legal-hold/${account_id}`,
    { reason }
  )
  return data
}

export async function release_legal_hold(
  account_id: number,
  reason?: string
): Promise<{ ok: boolean; account_id: number }> {
  // Always send a JSON body ({ reason: null } when none) — a bodyless DELETE
  // used to hit the backend's required Json extractor and come back 415
  // Unsupported Media Type without a readable message.
  const { data } = await axiosInstance.delete<{
    ok: boolean
    account_id: number
  }>(`api/v1/legal-hold/${account_id}`, {
    data: { reason: reason ?? null },
  })
  return data
}

/** Per-account outcome of a batch hold operation. */
export interface BatchHoldResult {
  account_id: number
  email: string
  ok: boolean
  error?: string | null
}

/** Place a hold on multiple accounts at once with a shared reason. */
export async function place_legal_holds_batch(
  account_ids: number[],
  reason: string
): Promise<BatchHoldResult[]> {
  const { data } = await axiosInstance.post<BatchHoldResult[]>(
    'api/v1/legal-hold/batch',
    { account_ids, reason }
  )
  return data
}

/** Release holds on multiple accounts at once (reason optional). */
export async function release_legal_holds_batch(
  account_ids: number[],
  reason?: string
): Promise<BatchHoldResult[]> {
  const { data } = await axiosInstance.delete<BatchHoldResult[]>(
    'api/v1/legal-hold/batch',
    { data: { account_ids, reason: reason ?? null } }
  )
  return data
}
