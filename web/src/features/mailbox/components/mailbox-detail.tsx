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


import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

import AceEditor from '@/components/ace-editor'
import { useTheme } from '@/context/theme-context'
import { MailboxData } from '@/api/mailbox/api'
import { useMailboxContext } from '../context'
import { useTranslation } from 'react-i18next'


interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

function convertMailboxData(raw: MailboxData): any {

  const attributes: string[] = [];
  raw.attributes.forEach(item => {
    attributes.push(item.attr);
    if (item.attr.toLowerCase() === "Extension" && item.extension !== null) {
      attributes.push(item.extension);
    }
  });

  return {
    // ...raw,
    id: raw.id.toString(),
    attributes
  };
}


export function MailboxDialog({ open, onOpenChange }: Props) {
  const { theme } = useTheme()
  const { currentMailbox } = useMailboxContext()
  const { t } = useTranslation()
  return (
    <Dialog
      open={open}
      onOpenChange={(state) => {
        onOpenChange(state)
      }}
    >
      <DialogContent className='w-full md:max-w-xl'>
        <DialogHeader className='text-left mb-4'>
          <DialogTitle>{currentMailbox?.name}</DialogTitle>
        </DialogHeader>
        <AceEditor
          readOnly={true}
          value={currentMailbox
            ? JSON.stringify(convertMailboxData(currentMailbox), null, 2)
            : 'null'}
          className="h-[14rem]"
          mode='json'
          theme={theme === "dark" ? 'monokai' : 'kuroir'}
        />
        <DialogFooter>
          <DialogClose asChild>
            <Button variant='outline' className="px-2 py-1 text-sm h-auto">{t('common.close')}</Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
