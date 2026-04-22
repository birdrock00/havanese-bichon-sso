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


import { Card, CardContent } from '@/components/ui/card';
import { FixedHeader } from '@/components/layout/fixed-header';
import { Main } from '@/components/layout/main';
import { AttachmentListPagination } from '@/components/pagination';
import React from 'react';
import AttachmentProvider, { AttachmentDialogType } from './context';
import useDialogState from '@/hooks/use-dialog-state';
import { useTranslation } from 'react-i18next';
import { AttachmentListTable } from './mail-list-table';
import { SortingState } from '@tanstack/react-table';
import { useSearchAttachments } from '@/hooks/use-search-attachments';
import { AttachmentModel } from '@/api/attachment/api';
import { MailDisplayDrawer } from './mail-display-dialog';
import { EnvelopeDeleteDialog } from './delete-dialog';
import { RestoreMessageDialog } from './restore-message-dialog';
import { NestedEmailDialog } from './nested-email-dialog';

export default function AttachmentSearch() {
  const { t } = useTranslation()
  const [currentAttachment, setCurrentAttachment] = React.useState<AttachmentModel | undefined>(undefined);
  const [open, setOpen] = useDialogState<AttachmentDialogType>(null)
  const [toDelete, setToDelete] = React.useState<Map<number, Set<string>>>(new Map());
  const [selected, setSelected] = React.useState<Map<number, Set<string>>>(new Map());
  const [selectedTags, setSelectedTags] = React.useState<string[]>([]);
  const [sorting, setSorting] = React.useState<SortingState>([{ id: "date", desc: true }]);
  const [deleteMailboxId, setDeleteMailboxId] = React.useState<string | undefined>(undefined);
  const [selectedAccountId, setSelectedAccountId] = React.useState<number | undefined>(undefined);

  const {
    attachments,
    total,
    totalPages,
    isLoading,
    page,
    pageSize,
    setPage,
    setSearchPageSize,
    setSortBy,
    setSortOrder,
    filter,
    setFilter
  } = useSearchAttachments();

  const handleSetPageSize = (pageSize: number) => {
    setPage(1);
    setSearchPageSize(pageSize)
  }

  const handleTagToggle = (tag: string) => {
    setSelectedTags(prev =>
      prev.includes(tag)
        ? prev.filter(t => t !== tag)
        : [...prev, tag]
    );
  };

  return (
    <>
      <FixedHeader />
      <Main>
        <AttachmentProvider
          value={{
            open,
            setOpen,
            currentAttachment,
            selectedTags,
            setCurrentAttachment,
            toDelete,
            setToDelete,
            selected,
            setSelected,
            sorting,
            setSorting,
            filter,
            setFilter,
            deleteMailboxId,
            setDeleteMailboxId,
            selectedAccountId,
            setSelectedAccountId,
            handleTagToggle
          }}
        >
          <div className="mx-auto w-full px-4">
            <div className="flex gap-6">
              <div className="flex-1 min-w-0 space-y-4">
                {isLoading && (
                  <Card>
                    <CardContent className="py-12">
                      <div className="flex flex-col items-center gap-2 text-muted-foreground">
                        <div className="animate-spin rounded-full h-6 w-6 border-2 border-primary border-t-transparent"></div>
                        <p className="text-sm">{t('search.searching')}</p>
                      </div>
                    </CardContent>
                  </Card>
                )}

                <AttachmentListTable
                  isLoading={isLoading}
                  items={attachments}
                  setSortBy={setSortBy}
                  setSortOrder={setSortOrder}
                />
                {total > 0 && <AttachmentListPagination
                  totalItems={total}
                  hasNextPage={() => page < totalPages}
                  pageIndex={page - 1}
                  pageSize={pageSize}
                  setPageIndex={(index) => setPage(index + 1)}
                  setPageSize={handleSetPageSize}
                />}
              </div>
            </div>
          </div>

          <MailDisplayDrawer
            key='attachment-mail-display'
            open={open === 'display'}
            onOpenChange={() => setOpen('display')}
          />

          <EnvelopeDeleteDialog
            key='delete-attachment-envelope'
            open={open === 'delete'}
            onOpenChange={() => setOpen('delete')}
          />

          <RestoreMessageDialog
            key='attachment-restore-mail-dialog'
            open={open === 'restore'}
            onOpenChange={() => setOpen('restore')}
          />

          <NestedEmailDialog
            key="nested-eml-attachment-dialog"
            open={open === 'nested-eml'}
            onOpenChange={() => setOpen('nested-eml')}
          />
        </AttachmentProvider>
      </Main>
    </>
  );
}
