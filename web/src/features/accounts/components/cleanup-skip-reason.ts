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
import type { TFunction } from 'i18next'

/**
 * Maps the stable reason keys the server writes (`skip_reasons`,
 * `unprocessed[].reason`) to translation keys.
 *
 * The server keeps the codes fine-grained for diagnosis; several of them mean
 * the same thing to an operator, so they share one label here. An unknown code
 * falls back to itself rather than disappearing — a newer server must not make
 * a skip reason invisible.
 */
const SKIP_REASON_TO_KEY: Record<string, string> = {
  flagged: 'skipReason.flagged',

  too_recently_archived: 'skipReason.archivedTooRecently',
  message_too_recent: 'skipReason.messageTooRecent',

  missing_uid: 'skipReason.incompleteMetadata',
  missing_message_date: 'skipReason.incompleteMetadata',
  unknown_uid_validity: 'skipReason.incompleteMetadata',

  source_uid_missing: 'skipReason.sourceGone',

  local_envelope_missing: 'skipReason.localMissing',
  local_blob_missing: 'skipReason.localMissing',
  local_blob_empty: 'skipReason.localMissing',
  local_blob_unreadable: 'skipReason.localUnreadable',
  local_hash_mismatch: 'skipReason.localHashMismatch',
  local_uidvalidity_missing: 'skipReason.localUidvalidityMissing',

  uidvalidity_changed: 'skipReason.uidvalidityChanged',
  size_mismatch: 'skipReason.sizeMismatch',
  internal_date_mismatch: 'skipReason.internalDateMismatch',
  message_id_mismatch: 'skipReason.messageIdMismatch',
  remote_message_id_missing: 'skipReason.remoteMessageIdMissing',
  body_hash_mismatch: 'skipReason.bodyHashMismatch',
  message_id_ambiguous: 'skipReason.messageIdAmbiguous',
  source_uid_survives_move: 'skipReason.sourceUidSurvives',

  examine_failed: 'skipReason.serverError',
  flag_lookup_failed: 'skipReason.serverError',
  header_fetch_failed: 'skipReason.serverError',
  body_fetch_failed: 'skipReason.serverError',
  message_id_search_failed: 'skipReason.serverError',
  source_select_failed: 'skipReason.serverError',
  source_check_failed: 'skipReason.serverError',
  move_failed: 'skipReason.serverError',
}

export function skipReasonLabel(
  t: TFunction,
  reason: string | null | undefined
): string {
  if (!reason) return ''
  const key = SKIP_REASON_TO_KEY[reason]
  if (!key) return reason
  return t(`accounts.cleanup.${key}`, reason)
}
