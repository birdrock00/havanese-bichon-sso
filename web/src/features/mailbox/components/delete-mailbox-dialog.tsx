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
import { useMailboxContext } from '../context';
import { useTranslation } from 'react-i18next';
import { delete_mailbox } from '@/api/mailbox/api';

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function MailBoxDeleteDialog({ open, onOpenChange }: Props) {
  const queryClient = useQueryClient();
  const { selectedAccountId, deleteMailboxId, setDeleteMailboxId } = useMailboxContext();
  const { t } = useTranslation();

  const deleteMutation = useMutation({
    mutationFn: ({ accountId, mailboxId }: { accountId: number; mailboxId: string }) =>
      delete_mailbox(accountId, mailboxId),
    retry: false,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['account-mailboxes', `${selectedAccountId}`] });
      onOpenChange(false);
      setDeleteMailboxId(undefined);
      toast({
        title: t('mailbox.deleteMailboxDialog.successTitle'),
        description: t('mailbox.deleteMailboxDialog.successDesc'),
      });
    },
    onError: (error: any) => {
      toast({
        title: t('mailbox.deleteMailboxDialog.errorTitle'),
        description: error.message || "Delete failed",
        variant: 'destructive',
      });
    },
  });

  const handleDelete = () => {
    if (selectedAccountId && deleteMailboxId) {
      deleteMutation.mutate({
        accountId: selectedAccountId,
        mailboxId: deleteMailboxId
      });
    }
  };

  const isLoading = deleteMutation.isPending;

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={(isOpen) => {
        onOpenChange(isOpen);
        if (!isOpen) setDeleteMailboxId(undefined);
      }}
      handleConfirm={handleDelete}
      className="max-w-xl"
      isLoading={isLoading}
      title={
        <span className="text-destructive">
          <IconAlertTriangle
            className="mr-1 inline-block stroke-destructive"
            size={18}
          />{' '}
          {t('mailbox.deleteMailboxDialog.title')}
        </span>
      }
      desc={
        <div className="space-y-4">
          <p className="mb-2">
            {t('mailbox.deleteMailboxDialog.desc')}
          </p>
          <Alert variant="destructive">
            <AlertTitle>{t('mailbox.deleteMailboxDialog.warningTitle')}</AlertTitle>
            <AlertDescription>{t('mailbox.deleteMailboxDialog.warningDesc')}</AlertDescription>
          </Alert>
        </div>
      }
      confirmText={t('mailbox.deleteMailboxDialog.confirm')}
      destructive
    />
  );
}
