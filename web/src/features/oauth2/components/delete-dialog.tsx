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
import { OAuth2Entity } from '../data/schema'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { delete_oauth2 } from '@/api/oauth2/api'
import { ToastAction } from '@/components/ui/toast'
import { AxiosError } from 'axios'
import { useTranslation } from 'react-i18next'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentRow: OAuth2Entity
}

export function TokenDeleteDialog({ open, onOpenChange, currentRow }: Props) {
  const { t } = useTranslation()
  const [value, setValue] = useState(0)
  const queryClient = useQueryClient();

  function handleSuccess() {
    toast({
      title: t('oauth2.deleteSuccess'),
      description: t('oauth2.yourOAuth2ApplicationHasBeenSuccessfullyDeleted'),
      action: <ToastAction altText={t('common.close')}>{t('common.close')}</ToastAction>,
    });

    queryClient.invalidateQueries({ queryKey: ['oauth2-list'] });
    onOpenChange(false);
  }

  function handleError(error: AxiosError) {
    const errorMessage = error.response?.data ||
      error.message ||
      t('oauth2.deleteFailed');

    toast({
      variant: "destructive",
      title: t('oauth2.oauth2DeleteFailed'),
      description: errorMessage as string,
      action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
    });
    console.error(error);
  }

  const deleteMutation = useMutation({
    mutationFn: (id: number) => delete_oauth2(id),
    onSuccess: handleSuccess,
    onError: handleError
  })

  const handleDelete = () => {
    if (value !== currentRow.id) return
    deleteMutation.mutate(currentRow.id)
  }

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      handleConfirm={handleDelete}
      disabled={value !== currentRow.id}
      className="max-w-2xl"
      title={
        <span className='text-destructive'>
          <IconAlertTriangle
            className='mr-1 inline-block stroke-destructive'
            size={18}
          />{' '}
          {t('oauth2.deleteToken')}
        </span>
      }
      desc={
        <div className='space-y-4'>
          <p className='mb-2'>
            {t('oauth2.areYouSureYouWantToDelete')}{' '}
            <span className='font-bold'>{currentRow.id}</span>?
            <br />
            {t('oauth2.thisActionWillPermanentlyRemoveTheOauth2Record')}
          </p>

          <Label className='my-2'>
            OAuth2:
            <Input
              type="number"
              value={value}
              onChange={(e) => setValue(parseInt(e.target.value, 10))}
              placeholder={t('oauth2.enterIdToConfirmDeletion')}
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
