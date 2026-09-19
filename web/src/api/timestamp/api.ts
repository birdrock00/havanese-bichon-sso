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
// Timestamp anchoring API client (Enterprise). Periodically the Merkle-tree
// root over archived content hashes is sent to an external RFC 3161 TSA, and a
// signed token is stored. `verify_email` returns a counter-proof bundle a third
// party can check independently with `openssl ts -verify`.
import axiosInstance from '@/api/axiosInstance'

/** A single anchored Merkle-tree root. Mirrors the server `AnchorRow`. */
export interface TimestampAnchor {
  id: number
  root_hash: string
  gen_time: number
  tsa_url?: string | null
  serial?: string | null
  status: string
  leaf_count: number
  created_at: number
  /** Base64 of the RFC 3161 `.tsr` token, when a third-party TSA was used. */
  token_b64?: string | null
}

/** One step of a Merkle inclusion path. */
export interface ProofStep {
  hash: string
  side: 'left' | 'right'
}

/** Counter-proof bundle for a single envelope. Mirrors server `ProofBundle`. */
export interface EmailProof {
  found: boolean
  anchored: boolean
  envelope_id: string
  leaf_hash: string
  ingest_at: number
  deleted_at?: number | null
  anchor_id?: number | null
  root_hash?: string | null
  gen_time?: number | null
  tsa_token_b64?: string | null
  merkle_path: ProofStep[]
}

export async function list_timestamps(): Promise<TimestampAnchor[]> {
  const { data } = await axiosInstance.get<TimestampAnchor[]>('api/v1/timestamps')
  return data
}

export async function anchor_now(): Promise<TimestampAnchor> {
  const { data } = await axiosInstance.post<TimestampAnchor>(
    'api/v1/timestamps/anchor-now'
  )
  return data
}

export async function verify_email(
  envelope_id: string
): Promise<EmailProof> {
  const { data } = await axiosInstance.post<EmailProof>(
    `api/v1/timestamps/verify-email/${encodeURIComponent(envelope_id)}`,
    {}
  )
  return data
}
