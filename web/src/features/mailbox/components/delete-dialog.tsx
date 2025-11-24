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

import { IconAlertTriangle } from '@tabler/icons-react';
import { toast } from '@/hooks/use-toast';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { ConfirmDialog } from '@/components/confirm-dialog';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { delete_messages } from '@/api/mailbox/envelope/api';
import { useMailboxContext } from '../context';
import { mapToRecordOfArrays } from '@/lib/utils';
import { useTranslation } from 'react-i18next';

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function EnvelopeDeleteDialog({ open, onOpenChange }: Props) {
  const queryClient = useQueryClient();
  const { selectedAccountId, deleteIds, setDeleteIds } = useMailboxContext();
  const { t } = useTranslation();

  const deleteMutation = useMutation({
    mutationFn: ({ payload }: { payload: Record<string, number[]> }) => delete_messages(payload),
    retry: false,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mailbox-list-messages'] });
      onOpenChange(false);
      setDeleteIds(new Set());
      toast({
        title: t('mailbox.deleteDialog.successTitle'),
        description: t('mailbox.deleteDialog.successDesc'),
      });
    },
    onError: (error: any) => {
      toast({
        title: t('mailbox.deleteDialog.errorTitle'),
        description: `${error.message}`,
        variant: 'destructive',
      });
    },
  });

  const handleDelete = () => {
    if (selectedAccountId) {
      const body = new Map<number, Set<number>>();
      body.set(selectedAccountId, deleteIds);
      const payload = mapToRecordOfArrays(body);
      deleteMutation.mutate({ payload });
    }
  };

  const isLoading = deleteMutation.isPending;

  const emailCount = deleteIds.size;
  const countText =
    emailCount > 1
      ? t('mailbox.deleteDialog.descCountMultiple', { count: emailCount })
      : t('mailbox.deleteDialog.descCountSingle');

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      handleConfirm={handleDelete}
      className="max-w-xl"
      isLoading={isLoading}
      title={
        <span className="text-destructive">
          <IconAlertTriangle
            className="mr-1 inline-block stroke-destructive"
            size={18}
          />{' '}
          {t('mailbox.deleteDialog.title')}
        </span>
      }
      desc={
        <div className="space-y-4">
          <p className="mb-2">
            {t('mailbox.deleteDialog.desc', { countText })}
          </p>

          <Alert variant="destructive">
            <AlertTitle>{t('mailbox.deleteDialog.warningTitle')}</AlertTitle>
            <AlertDescription>{t('mailbox.deleteDialog.warningDesc')}</AlertDescription>
          </Alert>
        </div>
      }
      confirmText={t('mailbox.deleteDialog.confirm')}
      destructive
    />
  );
}
