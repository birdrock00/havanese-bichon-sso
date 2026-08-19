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


import { Row } from '@tanstack/react-table'
import { Button } from '@/components/ui/button'
import { useAccountContext } from '../context';
import { useTranslation } from 'react-i18next';
import { useCurrentUser } from '@/hooks/use-current-user';
import { toast } from '@/hooks/use-toast';
import { ToastAction } from '@/components/ui/toast';
import { useQuery } from '@tanstack/react-query'
import { download_state, AccountModel, DownloadStatus } from '@/api/account/api';
import { Loader2 } from 'lucide-react'

interface Props {
  row: Row<AccountModel>
}

export function RunningStateCellAction({ row }: Props) {
  const { t } = useTranslation()
  const { setOpen, setCurrentRow } = useAccountContext()
  const { require_any_permission } = useCurrentUser()

  const hasPermission = require_any_permission(['system:root', 'account:read_details'], row.original.id)


  const { data: state } = useQuery({
    queryKey: ['running-state', row.original.id],
    queryFn: () => download_state(row.original.id),
    refetchInterval: (query) => {
      const s = query.state.data?.active_session
      return s && s.status === DownloadStatus.Running ? 5000 : false
    },
  })

  if (row.original.deleting) {
    return <span className="text-xs text-muted-foreground italic">Deleting...</span>
  }


  if (row.original.account_type === "NoSync") {
    return <span className="text-xs text-muted-foreground">n/a</span>
  }

  const running = state?.active_session
  const isRunning = !!running && running.status === DownloadStatus.Running

  return (
    <div className="flex items-center justify-center gap-2">
      {isRunning && (
        <span className="inline-flex items-center gap-1.5 rounded-full bg-blue-500/10 text-blue-600 border border-blue-500/20 px-2 py-0.5 text-[11px] font-medium shrink-0">
          <Loader2 className="h-3 w-3 animate-spin" />
          {t('accounts.runningState.syncing')}
        </span>
      )}
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
        <span
          className="text-xs text-primary cursor-pointer underline underline-offset-2 hover:opacity-80 transition-opacity"
        >
          {t('accounts.viewDetails')}
        </span>
      </Button>
    </div>
  )
}
