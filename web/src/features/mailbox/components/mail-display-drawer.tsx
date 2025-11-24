//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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



import {
  Sheet,
  SheetContent,
  SheetTitle
} from '@/components/ui/sheet'
import { useMailboxContext } from '../context'
import { MailMessageView } from './mail-message-view'
import { VisuallyHidden } from '@radix-ui/react-visually-hidden'
import { useTranslation } from 'react-i18next'


interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function MailDisplayDrawer({ open, onOpenChange }: Props) {
  const { currentEnvelope } = useMailboxContext();
  const { t } = useTranslation()
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <VisuallyHidden asChild>
        <SheetTitle />
      </VisuallyHidden>
      <SheetContent className="md:w-[80rem] h-full p-0">

          <div className='m-5'>
            {currentEnvelope ? (
              <MailMessageView envelope={currentEnvelope} />
            ) : (
              <div className="p-8 text-center text-muted-foreground">{t('mail.noMessageSelected')}</div>
            )}
          </div>

        </SheetContent>
    </Sheet>
  );
}
