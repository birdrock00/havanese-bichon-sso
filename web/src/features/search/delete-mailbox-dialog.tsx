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

import { IconAlertTriangle } from '@tabler/icons-react';
import { toast } from '@/hooks/use-toast';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { ConfirmDialog } from '@/components/confirm-dialog';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { delete_mailbox } from '@/api/mailbox/api';
import { OP_MAILBOX_DELETE } from '@/api/approvals/api';
import { useGatedOp } from '@/features/approvals/use-gated-op';
import { useSearchContext } from './context';

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function MailBoxDeleteDialog({ open, onOpenChange }: Props) {
  const queryClient = useQueryClient();
  const { selectedAccountId, deleteMailboxId, setDeleteMailboxId } = useSearchContext();
  const { t } = useTranslation();
  const { gated, announceIfQueued } = useGatedOp(OP_MAILBOX_DELETE);

  const deleteMutation = useMutation({
    mutationFn: ({ accountId, mailboxId }: { accountId: number; mailboxId: string }) =>
      delete_mailbox(accountId, mailboxId),
    retry: false,
    onSuccess: (data) => {
      // Invalidate either way: a queued delete does not change the mailbox
      // list, but it does add a row to the approval queue.
      queryClient.invalidateQueries({ queryKey: ['search-mailboxes', selectedAccountId] });
      queryClient.invalidateQueries({ queryKey: ['approvals'] });
      onOpenChange(false);
      setDeleteMailboxId(undefined);
      if (announceIfQueued(data)) return;
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
          {gated
            ? t('approvals.gatedTitle', 'Submit for approval?')
            : t('mailbox.deleteMailboxDialog.title')}
        </span>
      }
      desc={
        <div className="space-y-4">
          {gated ? (
            <Alert>
              <AlertTitle>
                {t('approvals.gatedMailboxTitle', 'Dual control is on for this operation')}
              </AlertTitle>
              <AlertDescription>
                {t(
                  'approvals.gatedMailboxDesc',
                  'This will NOT delete the mailbox now. It submits a request that a second person must approve, and only then is the mailbox deleted. The mailbox and its subfolders stay exactly as they are until that happens.'
                )}
              </AlertDescription>
            </Alert>
          ) : (
            <>
              <p className="mb-2">
                {t('mailbox.deleteMailboxDialog.desc')}
              </p>
              <Alert variant="destructive">
                <AlertTitle>{t('mailbox.deleteMailboxDialog.warningTitle')}</AlertTitle>
                <AlertDescription>{t('mailbox.deleteMailboxDialog.warningDesc')}</AlertDescription>
              </Alert>
            </>
          )}
        </div>
      }
      confirmText={
        gated
          ? t('approvals.submitForApproval', 'Submit for approval')
          : t('mailbox.deleteMailboxDialog.confirm')
      }
      destructive
    />
  );
}
