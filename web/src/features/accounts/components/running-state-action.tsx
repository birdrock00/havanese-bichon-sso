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
import { useAccountContext } from '../context';
import { useTranslation } from 'react-i18next';
import { useCurrentUser } from '@/hooks/use-current-user';
import { toast } from '@/hooks/use-toast';
import { ToastAction } from '@/components/ui/toast';
import { AccountModel } from '@/api/account/api';

interface Props {
  row: Row<AccountModel>
}

export function RunningStateCellAction({ row }: Props) {
  const { t } = useTranslation()
  const { setOpen, setCurrentRow } = useAccountContext()
  const { require_any_permission } = useCurrentUser()

  let account_type = row.original.account_type;
  if (account_type === "NoSync") {
    return <span className="text-xs text-muted-foreground">n/a</span>
  }
  const hasPermission = require_any_permission(['system:root', 'account:read_details'], row.original.id)

  return (
    <Button variant='ghost' className="h-auto p-1" onClick={() => {
      if (hasPermission) {
        setCurrentRow(row.original)
        setOpen('running-state')
      } else {
        toast({
          variant: 'destructive',
          title: 'Forbidden',
          description: 'You do not have permission to view this account.',
          action: (
            <ToastAction altText="Close">
              Close
            </ToastAction>
          ),
        })
      }
    }}>
      <span className="text-xs text-blue-500 cursor-pointer underline hover:text-blue-700">{t('accounts.viewDetails')}</span>
    </Button>
  )
}
