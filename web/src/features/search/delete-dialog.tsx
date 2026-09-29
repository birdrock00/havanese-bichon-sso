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

import { IconAlertTriangle } from '@tabler/icons-react'
import { toast } from '@/hooks/use-toast'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { delete_messages } from '@/api/mailbox/envelope/api'
import { OP_MESSAGE_DELETE } from '@/api/approvals/api'
import { useGatedOp } from '@/features/approvals/use-gated-op'
import { useSearchContext } from './context'
import { mapToRecordOfArrays } from '@/lib/utils'
import { useTranslation } from 'react-i18next'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function EnvelopeDeleteDialog({ open, onOpenChange }: Props) {
  const queryClient = useQueryClient()
  const { toDelete, setToDelete, setSelected } = useSearchContext()
  const { t } = useTranslation()
  const { gated, announceIfQueued } = useGatedOp(OP_MESSAGE_DELETE)

  const deleteMutation = useMutation({
    mutationFn: ({ payload }: { payload: Record<number, string[]> }) =>
      delete_messages(payload),
    retry: false,
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['search-messages'], exact: false })
      queryClient.invalidateQueries({ queryKey: ['all-tags'] })
      queryClient.invalidateQueries({ queryKey: ['approvals'] })
      onOpenChange(false)
      setToDelete(new Map())
      setSelected(new Map())

      // A queued deletion must not report that the messages are gone. They are
      // still there — the list was cleared because the operator asked for it,
      // not because anything happened.
      if (announceIfQueued(data)) return

      toast({
        title: t('search.delete.successTitle'),
        description: t('search.delete.successDesc'),
      })
    },
    onError: (error: any) => {
      toast({
        title: t('search.delete.errorTitle'),
        description: `${error.message}`,
        variant: 'destructive',
      })
    },
  })

  const handleDelete = () => {
    const payload = mapToRecordOfArrays(toDelete)
    deleteMutation.mutate({ payload })
  }

  const isLoading = deleteMutation.isPending

  const emailCount = Array.from(toDelete.values()).reduce(
    (sum, set) => sum + set.size,
    0
  )

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      handleConfirm={handleDelete}
      className="max-w-xl"
      isLoading={isLoading}
      destructive
      title={
        <span className="text-destructive">
          <IconAlertTriangle
            className="mr-1 inline-block stroke-destructive"
            size={18}
          />{' '}
          {t('search.delete.title')}
        </span>
      }
      desc={
        <div className="space-y-4">
          <p className="mb-2">
            {t('search.delete.confirmPrefix')}{' '}
            <span className="font-bold">
              {t('search.delete.countLabel', { count: emailCount })}
            </span>
            ?
            <br />
            {t('search.delete.confirmDetail')}
          </p>

          {gated ? (
            <Alert>
              <AlertTitle>
                {t('approvals.gatedMessagesTitle', 'Dual control is on for this operation')}
              </AlertTitle>
              <AlertDescription>
                {t(
                  'approvals.gatedMessagesDesc',
                  'This will NOT delete these messages now. It submits a request that a second person must approve, and only then are they deleted. They stay in the archive until that happens.'
                )}
              </AlertDescription>
            </Alert>
          ) : (
            <Alert variant="destructive">
              <AlertTitle>{t('search.delete.warningTitle')}</AlertTitle>
              <AlertDescription>
                {t('search.delete.warningDesc')}
              </AlertDescription>
            </Alert>
          )}
        </div>
      }
      confirmText={
        gated
          ? t('approvals.submitForApproval', 'Submit for approval')
          : t('search.delete.confirmButton')
      }
    />
  )
}
