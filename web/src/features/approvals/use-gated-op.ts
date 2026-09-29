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
// Knowing whether a destructive action will actually happen, or be queued.
//
// A delete dialog has to say which of the two it is about to do. "Delete this
// account" and "ask a second person to delete this account" are different
// promises, and a dialog that says the first while the server does the second
// is a lie the operator only discovers afterwards.
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { get_approval_config } from '@/api/approvals/api'
import { useEdition } from '@/hooks/use-edition'
import { useCurrentUser } from '@/hooks/use-current-user'
import { toast } from '@/hooks/use-toast'

/** The shape the gated endpoints return when they queue instead of acting. */
export interface QueuedResponse {
  ok: false
  pending: true
  request_id: string
  expires_at?: number | null
}

/** True when a response is a queued acknowledgement rather than a result.
 *
 *  The gated endpoints answer 202 with this body; the ungated path answers 200
 *  with an empty body. Axios treats both as success, so the discriminator has
 *  to be the body itself. */
export function is_queued(data: unknown): data is QueuedResponse {
  return (
    !!data &&
    typeof data === 'object' &&
    (data as QueuedResponse).pending === true &&
    typeof (data as QueuedResponse).request_id === 'string'
  )
}

/**
 * Whether `op` is currently gated, and a toast for the queued case.
 *
 * The config is read from the server rather than inferred: the client cannot
 * know whether an administrator has this operation on, and guessing wrong in
 * either direction produces a dialog that misdescribes what the button does.
 */
export function useGatedOp(op: string) {
  const { t } = useTranslation()
  const { isEnterprise } = useEdition()
  const { require_any_permission } = useCurrentUser()

  // Only an administrator who could change the setting can read it; for anyone
  // else the query stays disabled and `gated` is false. That is the right
  // default: a non-admin's deletes are not gated by a policy they cannot see,
  // and if they are, the server queues them and the toast below still fires.
  const canRead = isEnterprise && require_any_permission(['system:root', 'user:manage'])

  const { data } = useQuery({
    queryKey: ['approvals', 'config'],
    queryFn: get_approval_config,
    enabled: canRead,
    staleTime: 60_000,
  })

  const gated = !!data?.enabled && (data.ops ?? []).includes(op)

  /** Announce a queued operation. Returns true when it was queued, so callers
   *  can branch their own success handling. */
  const announceIfQueued = (data: unknown): boolean => {
    if (!is_queued(data)) return false
    toast({
      title: t(
        'approvals.queuedTitle',
        'Submitted for approval — nothing has happened yet'
      ),
      description: t(
        'approvals.queuedDesc',
        'Dual control is on for this operation: a second person must approve it before it takes effect. It now appears on the Approvals page.'
      ),
    })
    return true
  }

  return { gated, announceIfQueued }
}
