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


import { Card, CardContent } from '@/components/ui/card';
import { FixedHeader } from '@/components/layout/fixed-header';
import { Main } from '@/components/layout/main';
import { useSearchMessages } from '@/hooks/use-search-messages';
import { EnvelopeListPagination } from '@/components/pagination';
import React from 'react';
import { EmailEnvelope } from '@/api';
import { MailDisplayDrawer } from './mail-display-dialog';
import { EnvelopeDeleteDialog } from './delete-dialog';
import SearchProvider, { SearchDialogType } from './context';
import useDialogState from '@/hooks/use-dialog-state';
import { EditTagsDialog } from './add-tag-dialog';
import { useTranslation } from 'react-i18next';
import { RestoreMessageDialog } from './restore-message-dialog';
import { MailListTable } from './mail-list-table';
import { SortingState } from '@tanstack/react-table';

export default function Search() {
  const { t } = useTranslation()
  const [selectedEnvelope, setSelectedEnvelope] = React.useState<EmailEnvelope | undefined>(undefined);
  const [open, setOpen] = useDialogState<SearchDialogType>(null)
  const [toDelete, setToDelete] = React.useState<Map<number, Set<number>>>(new Map());
  const [selected, setSelected] = React.useState<Map<number, Set<number>>>(new Map());
  const [selectedTags, setSelectedTags] = React.useState<string[]>([]);
  const [sorting, setSorting] = React.useState<SortingState>([{ id: "date", desc: true }]);

  const {
    emails,
    total,
    totalPages,
    isLoading,
    page,
    pageSize,
    setPage,
    setPageSize,
    setSortBy,
    setSortOrder,
    filter,
    setFilter
  } = useSearchMessages();

  const handleSetPageSize = (pageSize: number) => {
    setPage(1);
    setPageSize(pageSize)
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
        <SearchProvider
          value={{
            open,
            setOpen,
            currentEnvelope: selectedEnvelope,
            selectedTags,
            setCurrentEnvelope: setSelectedEnvelope,
            toDelete,
            setToDelete,
            selected,
            setSelected,
            sorting,
            setSorting,
            filter,
            setFilter,
            handleTagToggle
          }}
        >
          <div className="mx-auto w-full px-4">
            <div className="flex gap-6">
              {/* <aside className="hidden lg:block w-64 flex-shrink-0">
                <div className="rounded-lg border bg-card p-4">
                  <EnvelopeTags
                    selectedTags={selectedTags}
                    onTagToggle={handleTagToggle}
                  />
                </div>
              </aside> */}
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

                <MailListTable
                  isLoading={isLoading}
                  items={emails}
                  onEnvelopeChanged={(envelope) => {
                    setOpen('display');
                    setSelectedEnvelope(envelope);
                  }}
                  setSortBy={setSortBy}
                  setSortOrder={setSortOrder}
                />
                {total > 0 && <EnvelopeListPagination
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
            key='search-mail-display'
            open={open === 'display'}
            onOpenChange={() => setOpen('display')}
          />

          <EnvelopeDeleteDialog
            key='delete-envelope'
            open={open === 'delete'}
            onOpenChange={() => setOpen('delete')}
          />

          <EditTagsDialog
            key='edit-tags-dialog'
            open={open === 'edit-tags'}
            onOpenChange={() => setOpen('edit-tags')}
          />

          <RestoreMessageDialog
            key='restore-mail-dialog'
            open={open === 'restore'}
            onOpenChange={() => setOpen('restore')}
          />
        </SearchProvider>
      </Main>
    </>
  );
}
