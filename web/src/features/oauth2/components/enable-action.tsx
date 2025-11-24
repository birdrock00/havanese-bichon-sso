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
import { Switch } from '@/components/ui/switch'
import { useState } from 'react'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { ToastAction } from '@/components/ui/toast'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from '@/hooks/use-toast'
import { AxiosError } from 'axios'
import { OAuth2Entity } from '../data/schema'
import { update_oauth2 } from '@/api/oauth2/api'
import { useTranslation } from 'react-i18next'

interface DataTableRowActionsProps {
  row: Row<OAuth2Entity>
}

export function EnableAction({ row }: DataTableRowActionsProps) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false);
  const queryClient = useQueryClient();

  const updateMutation = useMutation({
    mutationFn: (enabled: boolean) =>
      update_oauth2(row.original.id, { enabled }),
    onSuccess: () => {
      setOpen(false);
      toast({
        title: t('oauth2.oauth2ClientUpdated'),
        description: t('oauth2.oauth2ClientHasBeenSuccessfully', { action: row.original.enabled ? t('oauth2.disabled').toLowerCase() : t('oauth2.enabled').toLowerCase() }),
        action: <ToastAction altText={t('common.close')}>{t('common.close')}</ToastAction>,
      })
      queryClient.invalidateQueries({ queryKey: ['oauth2-list'] })
    },
    onError: (error: AxiosError) => {
      const errorMessage =
        (error.response?.data as { message?: string })?.message ||
        error.message ||
        'Status update failed, please try again later'

      toast({
        variant: "destructive",
        title: t('oauth2.updateFailed'),
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
        title={row.original.enabled ? t('oauth2.disableOAuth2Client') : t('oauth2.enableOAuth2Client')}
        desc={
          t('oauth2.areYouSureYouWantTo', { action: row.original.enabled ? t('oauth2.disable').toLowerCase() : t('oauth2.enable').toLowerCase() }) +
          (row.original.enabled ? ' ' + t('oauth2.thisWillPreventTheOAuth2ClientFromBeingUsed') : '')
        }
        destructive={row.original.enabled}
        confirmText={row.original.enabled ? t('oauth2.disable') : t('oauth2.enable')}
        isLoading={updateMutation.isPending}
        handleConfirm={handleConfirm}
      />
    </>
  )
}
