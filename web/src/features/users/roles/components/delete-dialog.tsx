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
//

import { useState } from 'react'
import { IconAlertTriangle } from '@tabler/icons-react'
import { toast } from '@/hooks/use-toast'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ToastAction } from '@/components/ui/toast'
import { AxiosError } from 'axios'
import { remove_role, UserRole } from '@/api/users/api'
import { useTranslation } from 'react-i18next'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentRow: UserRole
}

export function RoleDeleteDialog({ open, onOpenChange, currentRow }: Props) {
  const { t } = useTranslation()
  const [value, setValue] = useState(0)
  const queryClient = useQueryClient()

  function handleSuccess() {
    toast({
      title: t('roles.delete.success_title'),
      description: t('roles.delete.success_desc', { name: currentRow.name, id: currentRow.id }),
      action: <ToastAction altText={t('roles.actions.close')}>{t('roles.actions.close')}</ToastAction>,
    })

    queryClient.invalidateQueries({ queryKey: ['role-list'] })
    onOpenChange(false)
  }

  function handleError(error: AxiosError) {
    const errorMessage =
      (error.response?.data as { message?: string })?.message ||
      error.message ||
      t('roles.delete.error_fallback')

    toast({
      variant: 'destructive',
      title: t('roles.delete.error_title'),
      description: errorMessage,
      action: <ToastAction altText={t('roles.actions.retry')}>{t('roles.actions.retry')}</ToastAction>,
    })

    console.error(error)
  }

  const deleteMutation = useMutation({
    mutationFn: (id: number) => remove_role(id),
    onSuccess: handleSuccess,
    onError: handleError,
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
      disabled={value !== currentRow.id || deleteMutation.isPending}
      className="max-w-2xl"
      title={
        <span className="text-destructive">
          <IconAlertTriangle
            className="mr-1 inline-block stroke-destructive"
            size={18}
          />{' '}
          {t('roles.delete.dialog_title')}
        </span>
      }
      desc={
        <div className="space-y-4">
          <p className="mb-2">
            {t('roles.delete.warning_text', { name: currentRow.name, id: currentRow.id })}
            <br />
            <span className="font-bold">{t('roles.delete.irreversible')}</span>
          </p>

          <Label className="my-2">
            {t('roles.delete.confirm_label')}
            <Input
              type="number"
              value={value === 0 ? '' : value}
              onChange={(e) => {
                const numericValue = parseInt(e.target.value, 10)
                setValue(isNaN(numericValue) ? 0 : numericValue)
              }}
              placeholder={t('roles.delete.placeholder', { id: currentRow.id })}
              className="mt-2"
            />
          </Label>

          <Alert variant="destructive">
            <AlertTitle>{t('roles.delete.alert_title')}</AlertTitle>
            <AlertDescription>
              {t('roles.delete.alert_desc')}
            </AlertDescription>
          </Alert>
        </div>
      }
      confirmText={deleteMutation.isPending ? t('roles.delete.deleting') : t('roles.delete.confirm_btn')}
      destructive
    />
  )
}