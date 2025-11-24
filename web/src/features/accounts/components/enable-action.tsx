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
import { AccountModel } from '../data/schema'
import { Switch } from '@/components/ui/switch'
import { useState } from 'react'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { ToastAction } from '@/components/ui/toast'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { update_account } from '@/api/account/api'
import { toast } from '@/hooks/use-toast'
import { AxiosError } from 'axios'
import { useTranslation } from 'react-i18next'

interface DataTableRowActionsProps {
  row: Row<AccountModel>
}

export function EnableAction({ row }: DataTableRowActionsProps) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false);
  const queryClient = useQueryClient();

  const updateMutation = useMutation({
    mutationFn: (enabled: boolean) =>
      update_account(row.original.id, { enabled }),
    onSuccess: () => {
      setOpen(false);
      toast({
        title: t('accounts.accountUpdated'),
        description: t('accounts.accountHasBeenSuccessfully', { action: row.original.enabled ? t('accounts.disabled').toLowerCase() : t('accounts.enabled').toLowerCase() }),
        action: <ToastAction altText={t('common.close')}>{t('common.close')}</ToastAction>,
      })
      queryClient.invalidateQueries({ queryKey: ['account-list'] })
    },
    onError: (error: AxiosError) => {
      const errorMessage =
        (error.response?.data as { message?: string })?.message ||
        error.message ||
        'Status update failed, please try again later'

      toast({
        variant: "destructive",
        title: t('accounts.accountUpdateFailed'),
        description: errorMessage,
        action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
      })
    }
  })

  const handleConfirm = () => {
    updateMutation.mutate(!row.original.enabled)
  }

  return (
    <>
      <Switch
        checked={row.original.enabled}
        onCheckedChange={() => setOpen(true)}
        disabled={updateMutation.isPending}
      />
      <ConfirmDialog
        open={open}
        onOpenChange={setOpen}
        title={row.original.enabled ? t('accounts.disableAccount') : t('accounts.enableAccount')}
        desc={
          t('accounts.areYouSureYouWantTo', { action: row.original.enabled ? t('accounts.disable').toLowerCase() : t('accounts.enable').toLowerCase() }) +
          (row.original.enabled ? ' ' + t('accounts.thisWillPreventTheAccountFromBeingUsed') : '')
        }
        destructive={row.original.enabled}
        confirmText={row.original.enabled ? t('accounts.disable') : t('accounts.enable')}
        isLoading={updateMutation.isPending}
        handleConfirm={handleConfirm}
      />
    </>
  )
}
