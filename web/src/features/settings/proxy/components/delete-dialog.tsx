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
import { Proxy } from '../data/schema'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ToastAction } from '@/components/ui/toast'
import { AxiosError } from 'axios'
import { delete_proxy } from '@/api/system/api'
import { useTranslation } from 'react-i18next'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentRow: Proxy
}

export function ProxyDeleteDialog({ open, onOpenChange, currentRow }: Props) {
  const { t } = useTranslation()
  const [value, setValue] = useState(0)
  const queryClient = useQueryClient();

  function handleSuccess() {
    toast({
      title: t('proxyDelete.successTitle'),
      description: t('proxyDelete.successDesc'),
      action: <ToastAction altText={t('proxyDelete.close')}>{t('proxyDelete.close')}</ToastAction>,
    });
    queryClient.invalidateQueries({ queryKey: ['proxy-list'] });
    onOpenChange(false);
  }

  function handleError(error: AxiosError) {
    const errorMessage = error.response?.data ||
      error.message ||
      t('proxyDelete.failedDesc');

    toast({
      variant: "destructive",
      title: t('proxyDelete.failedTitle'),
      description: errorMessage as string,
      action: <ToastAction altText={t('proxyDelete.tryAgain')}>{t('proxyDelete.tryAgain')}</ToastAction>,
    });
    console.error(error);
  }

  const deleteMutation = useMutation({
    mutationFn: (id: number) => delete_proxy(id),
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
          {t('proxyDelete.title')}
        </span>
      }
      desc={
        <div className='space-y-4'>
          <p className='mb-2'>
            {t('proxyDelete.confirmText')} <span className='font-bold'>{`${currentRow.id}`}</span>?
            <br />
            {t('proxyDelete.permanent')}
          </p>

          <Label className='my-2'>
            {t('proxyDelete.proxyIdLabel')}
            <Input
              type="number"
              value={`${value}`}
              onChange={(e) => setValue(parseInt(e.target.value, 10))}
              placeholder={t('proxyDelete.proxyIdPlaceholder')}
              className="mt-2"
            />
          </Label>

          <Alert variant='destructive'>
            <AlertTitle>{t('proxyDelete.warningTitle', 'Warning!')}</AlertTitle>
            <AlertDescription>
              {t('proxyDelete.warningDesc')}
            </AlertDescription>
          </Alert>
        </div>
      }
      confirmText={t('proxyDelete.deleteBtn')}
      destructive
    />
  )
}

