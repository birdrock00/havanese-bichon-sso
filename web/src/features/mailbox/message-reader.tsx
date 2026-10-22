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
import { MailOpen } from 'lucide-react'

import { EmailEnvelope } from '@/api'
import { ScrollArea } from '@/components/ui/scroll-area'
import { MailMessageView } from '@/features/search/mail-message-view'
import { cn } from '@/lib/utils'

interface Props {
  envelope?: EmailEnvelope
  className?: string
}

/** Right-hand reading pane of the mailbox view. */
export function MessageReader({ envelope, className }: Props) {
  const { t } = useTranslation()

  return (
    <section
      className={cn(
        'flex min-h-0 flex-col overflow-hidden rounded-md border bg-background',
        className
      )}
    >
      <div className="flex shrink-0 items-center border-b px-3 py-2">
        <span className="text-xs font-medium text-muted-foreground">
          {t('mailboxView.message')}
        </span>
      </div>
      <ScrollArea
        // Remount so switching messages starts at the top of the new body.
        key={envelope ? `${envelope.account_id}:${envelope.id}` : 'empty'}
        // Same Radix table-wrapper fix as the message list: keep the body
        // constrained to the pane instead of growing to its widest element.
        className="min-h-0 flex-1 [&_[data-radix-scroll-area-viewport]>div]:!block"
      >
        <div className="p-4">
          {envelope ? (
            <MailMessageView envelope={envelope} />
          ) : (
            <div className="flex h-40 flex-col items-center justify-center gap-2 text-center text-xs text-muted-foreground">
              <MailOpen className="h-6 w-6 opacity-40" />
              {t('mailboxView.selectMessage')}
            </div>
          )}
        </div>
      </ScrollArea>
    </section>
  )
}
