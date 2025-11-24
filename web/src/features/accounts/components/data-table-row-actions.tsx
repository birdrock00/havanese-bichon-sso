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


import { DotsHorizontalIcon } from '@radix-ui/react-icons'
import { Row } from '@tanstack/react-table'
import { IconEdit, IconTrash } from '@tabler/icons-react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { useAccountContext } from '../context'
import { AccountModel } from '../data/schema'
import { Mailbox, MessageSquareMore } from 'lucide-react'
import { useTranslation } from 'react-i18next'

interface DataTableRowActionsProps {
  row: Row<AccountModel>
}

export function DataTableRowActions({ row }: DataTableRowActionsProps) {
  const { t } = useTranslation()
  const { setOpen, setCurrentRow } = useAccountContext()

  const account_type = row.original.account_type;

  return (
    <>
      <DropdownMenu modal={false}>
        <DropdownMenuTrigger asChild>
          <Button
            variant='ghost'
            className='flex h-8 w-8 p-0 data-[state=open]:bg-muted'
          >
            <DotsHorizontalIcon className='h-4 w-4' />
            <span className='sr-only'>{t('accounts.openMenu')}</span>
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align='end' className='w-[160px]'>
          <DropdownMenuItem
            onClick={() => {
              setCurrentRow(row.original)
              if (account_type === "IMAP") {
                setOpen("edit-imap");
              }
              if (account_type === "NoSync") {
                setOpen("edit-nosync");
              }
            }}
          >
            {t('accounts.edit')}
            <DropdownMenuShortcut>
              <IconEdit size={16} />
            </DropdownMenuShortcut>
          </DropdownMenuItem>
          {account_type === "IMAP" && <DropdownMenuItem
            onClick={() => {
              setCurrentRow(row.original)
              setOpen('sync-folders')
            }}
          >
            {t('accounts.syncFolders')}
            <DropdownMenuShortcut>
              <Mailbox size={16} />
            </DropdownMenuShortcut>
          </DropdownMenuItem>}
          {account_type === "IMAP" && <DropdownMenuItem
            onClick={() => {
              setCurrentRow(row.original)
              setOpen('detail')
            }}
          >
            {t('accounts.detail')}
            <DropdownMenuShortcut>
              <MessageSquareMore size={16} />
            </DropdownMenuShortcut>
          </DropdownMenuItem>}
          <DropdownMenuSeparator />
          <DropdownMenuItem
            onClick={() => {
              setCurrentRow(row.original)
              setOpen('delete')
            }}
            className='!text-red-500'
          >
            {t('accounts.delete')}
            <DropdownMenuShortcut>
              <IconTrash size={16} />
            </DropdownMenuShortcut>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </>
  )
}
