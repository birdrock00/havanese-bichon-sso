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
import { useTranslation } from 'react-i18next'
import { AlertTriangle } from 'lucide-react'
import type { RemoteCleanupSkippedMessage } from '@/api/account/api'
import { skipReasonLabel } from './cleanup-skip-reason'

interface CleanupUnprocessedListProps {
  messages?: RemoteCleanupSkippedMessage[] | null
  /** Exact failure count, so a truncated list can say how many are not shown. */
  failed: number
}

/**
 * Names the messages a run failed to move.
 *
 * Routine policy skips (too recently archived, gone from the source, flagged)
 * are expected, need no operator action and are not listed: a report that
 * repeats them just tells the user something they cannot act on. Only failures
 * — the archive could not be proved, or the server refused the move — are
 * shown.
 *
 * The server caps the list, so when the counter exceeds it this says how many
 * remain unnamed instead of quietly showing a shorter list than the total.
 * `null`/absent means the report was written before the list existed; that is
 * not the same as "nothing failed", so it gets its own wording.
 */
export function CleanupUnprocessedList({
  messages,
  failed,
}: CleanupUnprocessedListProps) {
  const { t } = useTranslation()

  if (failed === 0) return null

  if (!messages) {
    return (
      <p className='text-muted-foreground'>
        {t(
          'accounts.cleanup.unprocessedUnavailable',
          'This report was recorded before message details were kept. Run cleanup again to see which messages failed.'
        )}
      </p>
    )
  }

  const failedMessages = messages.filter((message) => message.failed)
  const unnamed = failed - failedMessages.length

  return (
    <div className='space-y-1'>
      <div className='font-semibold'>
        {t('accounts.cleanup.unprocessedTitle', 'Failed messages')}
      </div>
      <div className='space-y-0.5'>
        {failedMessages.map((message) => (
          <div
            key={`${message.folder}:${message.uid}:${message.message_id}`}
            className='flex items-start gap-1.5'
          >
            <AlertTriangle className='mt-0.5 h-3 w-3 shrink-0 text-destructive' />
            <span className='min-w-0'>
              <span className='font-medium break-words'>
                {message.subject || (
                  <span className='italic text-muted-foreground'>
                    {t('accounts.cleanup.noSubject', '(no subject)')}
                  </span>
                )}
              </span>
              <span className='text-muted-foreground'>
                {' · '}
                {skipReasonLabel(t, message.reason)} · {message.folder} · UID{' '}
                {message.uid}
              </span>
            </span>
          </div>
        ))}
      </div>
      {unnamed > 0 && (
        <p className='text-muted-foreground'>
          {t('accounts.cleanup.unprocessedTruncated', {
            count: unnamed,
            defaultValue:
              '{{count}} more message(s) were not recorded (the list is capped).',
          })}
        </p>
      )}
    </div>
  )
}
