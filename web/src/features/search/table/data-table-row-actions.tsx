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


import { Row } from '@tanstack/react-table'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { useTranslation } from 'react-i18next'
import { MoreVertical, TagIcon, Trash2 } from 'lucide-react'
import { EmailEnvelope } from '@/api'
import { useSearchContext } from '../context'

interface DataTableRowActionsProps {
  row: Row<EmailEnvelope>
}

export function DataTableRowActions({ row }: DataTableRowActionsProps) {
  const { setOpen, setCurrentEnvelope, setToDelete } = useSearchContext()
  const { t } = useTranslation()

  const toggleToDelete = (accountId: number, mailId: number) => {
    setToDelete(prev => {
      const next = new Map(prev);
      const set = new Set(next.get(accountId) || []);

      if (set.has(mailId)) {
        set.delete(mailId);
        if (set.size === 0) next.delete(accountId);
        else next.set(accountId, set);
      } else {
        set.add(mailId);
        next.set(accountId, set);
      }

      return next;
    });
  };

  const handleDelete = (envelope: EmailEnvelope) => {
    setToDelete(new Map());
    toggleToDelete(envelope.account_id, envelope.id)
    setOpen("delete")
  }

  return (
    <>
      <DropdownMenu modal={false}>
        <DropdownMenuTrigger asChild>
          <Button
            variant='ghost'
            className='flex h-8 w-8 p-0 data-[state=open]:bg-muted'
          >
            <MoreVertical size={10} />
            <span className='sr-only'>Open menu</span>
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align='end' className='w-[160px]'>
          <DropdownMenuItem
            onClick={(e) => {
              e.stopPropagation();  
              setCurrentEnvelope(row.original);
              setOpen("edit-tags");
            }}
          >
            {t('search.editTag')}
            <DropdownMenuShortcut>
              <TagIcon size={16} />
            </DropdownMenuShortcut>
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            onClick={(e) => {
              e.stopPropagation();
              setCurrentEnvelope(row.original);
              setOpen("restore");
            }}
          >
            {t('restore_message.restore_to_imap', 'Restore Mail')}
            <DropdownMenuShortcut>
              <TagIcon size={16} />
            </DropdownMenuShortcut>
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            onClick={(e) => {
              e.stopPropagation();
              handleDelete(row.original);
              //setCurrentRow(row.original)
              //setOpen('delete')
            }}
            className='!text-red-500'
          >
            {t('common.delete')}
            <DropdownMenuShortcut>
              <Trash2 size={16} />
            </DropdownMenuShortcut>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </>
  )
}
