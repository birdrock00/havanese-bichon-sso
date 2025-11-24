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
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { AccessToken } from '../data/schema'
import { Button } from '@/components/ui/button'
import { AccountsDetailTable } from './accounts-detail-table'
import { getColumns } from './accounts-detail-columns'
import { useTranslation } from 'react-i18next'

interface Props {
  currentRow: AccessToken
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function AccountDetailDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const columns = getColumns(t)
  return (
    <Dialog
      open={open}
      onOpenChange={(state) => {
        onOpenChange(state)
      }}
    >
      <DialogContent className='sm:max-w-4xl'>
        <DialogHeader className='text-left'>
          <DialogTitle>{t('settings.accounts')}</DialogTitle>
          <DialogDescription>
            {t('accessTokens.theListOfAccountsThatCanBeQueried')}
          </DialogDescription>
        </DialogHeader>
        <div className="h-[33rem] overflow-x-auto overflow-y-auto">
          <AccountsDetailTable data={currentRow.accounts} columns={columns} />
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant='outline' className="px-2 py-1 text-sm h-auto">{t('common.close')}</Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
