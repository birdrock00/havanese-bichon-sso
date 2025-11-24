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


import { useState } from 'react'
import { IconAlertTriangle } from '@tabler/icons-react'
import { toast } from '@/hooks/use-toast'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { AccessToken } from '../data/schema'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { delete_access_token } from '@/api/access-tokens/api'
import { ToastAction } from '@/components/ui/toast'
import { AxiosError } from 'axios'
import { useTranslation } from 'react-i18next'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentRow: AccessToken
}

export function TokenDeleteDialog({ open, onOpenChange, currentRow }: Props) {
  const { t } = useTranslation()
  const [value, setValue] = useState('')
  const queryClient = useQueryClient();

  const deleteMutation = useMutation({
    mutationFn: () => delete_access_token(currentRow.token),
    onSuccess: handleSuccess,
    onError: handleError
  });


  function handleSuccess() {
    toast({
      title: t('accessTokens.accessTokenDeleted'),
      description: t('accessTokens.yourAccessTokenHasBeenSuccessfullyDeleted'),
      action: <ToastAction altText={t('common.close')}>{t('common.close')}</ToastAction>,
    });

    queryClient.invalidateQueries({ queryKey: ['access-tokens'] });
    onOpenChange(false);
  }

  function handleError(error: AxiosError) {
    const errorMessage = (error.response?.data as { message?: string })?.message ||
      error.message ||
      t('dialogs.deleteFailed');

    toast({
      variant: "destructive",
      title: t('accessTokens.accessTokenDeleteFailed'),
      description: errorMessage as string,
      action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
    });
    console.error(error);
  }


  const handleDelete = () => {
    if (value.trim() !== currentRow.token) return
    deleteMutation.mutate()
  }

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      handleConfirm={handleDelete}
      disabled={value.trim() !== currentRow.token}
      className="max-w-2xl"
      title={
        <span className='text-destructive'>
          <IconAlertTriangle
            className='mr-1 inline-block stroke-destructive'
            size={18}
          />{' '}
          {t('accessTokens.deleteToken')}
        </span>
      }
      desc={
        <div className='space-y-4'>
          <p className='mb-2'>
            {t('dialogs.confirmDeleteDesc')}
            <span className='font-bold'>{currentRow.token}</span>?
            <br />
            {t('accessTokens.thisActionWillPermanentlyRemoveTheToken')}
          </p>

          <Label className='my-2'>
            {t('settings.token')}:
            <Input
              value={value}
              onChange={(e) => setValue(e.target.value)}
              placeholder={t('accessTokens.enterTokenToConfirmDeletion')}
              className="mt-2"
            />
          </Label>

          <Alert variant='destructive'>
            <AlertTitle>{t('oauth2.warning')}</AlertTitle>
            <AlertDescription>
              {t('oauth2.pleaseBeCareful')}
            </AlertDescription>
          </Alert>
        </div>
      }
      confirmText={t('common.delete')}
      destructive
    />
  )
}
