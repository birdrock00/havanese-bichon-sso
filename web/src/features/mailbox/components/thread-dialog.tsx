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

import { useState } from 'react';
import { useInfiniteQuery } from '@tanstack/react-query';
import { format } from 'date-fns';
import { ChevronDown, ChevronUp, Loader2, MessageSquareText } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { useMailboxContext } from '../context';
import { get_thread_messages } from '@/api/mailbox/envelope/api';
import { MailMessageView } from './mail-message-view';

interface MailThreadDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function MailThreadDialog({ open, onOpenChange }: MailThreadDialogProps) {
  const { t } = useTranslation();
  const { selectedAccountId, currentEnvelope } = useMailboxContext();
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set());

  const threadId = currentEnvelope?.thread_id;
  const accountId = selectedAccountId;

  const {
    data,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    isLoading,
    isError,
    error,
  } = useInfiniteQuery({
    queryKey: ['thread', accountId, threadId],
    queryFn: ({ pageParam = 1 }) =>
      get_thread_messages(accountId!, threadId!, pageParam, 10),
    getNextPageParam: (lastPage) =>
      lastPage.current_page && lastPage.total_pages
        ? lastPage.current_page < lastPage.total_pages
          ? lastPage.current_page + 1
          : undefined
        : undefined,
    enabled: open && !!accountId && !!threadId,
    initialPageParam: 1,
  });

  const allMessages = data?.pages.flatMap((page) => page.items) ?? [];
  const totalCount = data?.pages[0]?.total_items ?? 0;

  const toggleExpand = (id: number) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-full max-w-full p-0 max-h-full flex flex-col md:max-w-3xl lg:max-w-4xl">
        {/* Header */}
        <DialogHeader className="p-4 pb-3 border-b shrink-0">
          <div className="flex items-center justify-between">
            <DialogTitle className="flex items-center gap-2">
              <MessageSquareText className="w-5 h-5" />
              <div className="text-sm">
                {t('mailbox.thread.title', {
                  count: totalCount,
                  messageLabel: totalCount === 1 ? t('mailbox.thread.message') : t('mailbox.thread.messages')
                })}
              </div>
            </DialogTitle>
          </div>
        </DialogHeader>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {isLoading && <ThreadSkeleton />}

          {isError && (
            <div className="text-center text-destructive text-sm">
              {t('mailbox.thread.loadError')}: {(error as Error)?.message}
            </div>
          )}

          {!isLoading && allMessages.length === 0 && (
            <div className="text-center text-muted-foreground text-sm">
              {t('mailbox.thread.empty')}
            </div>
          )}

          {allMessages
            .sort((a, b) => a.internal_date - b.internal_date)
            .map((msg) => {
              const isExpanded = expandedIds.has(msg.id);
              const preview = msg.text?.slice(0, 120) + (msg.text?.length > 120 ? '...' : '');
              const date = new Date(msg.internal_date);
              const formattedDate = isNaN(date.getTime())
                ? t('mailbox.thread.invalidDate')
                : format(date, 'yyyy-MM-dd HH:mm:ss');

              return (
                <Card
                  key={msg.id}
                  className={`transition-all ${isExpanded ? 'ring-2 ring-primary' : ''}`}
                >
                  <CardHeader
                    className="cursor-pointer pb-3"
                    onClick={() => toggleExpand(msg.id)}
                  >
                    <div className="flex items-start justify-between gap-4">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 text-sm">
                          <span className="font-medium truncate">{msg.from}</span>
                          <span className="text-muted-foreground">→</span>
                          <span className="text-muted-foreground truncate">
                            {msg.to.join(', ')}
                          </span>
                        </div>
                        <p className="font-medium mt-1 text-sm">
                          {msg.subject || t('mailbox.thread.noSubject')}
                        </p>
                        {!isExpanded && preview && (
                          <p className="text-xs text-muted-foreground mt-1 line-clamp-2">
                            {preview}
                          </p>
                        )}
                      </div>
                      <div className="flex items-center gap-2 text-xs text-muted-foreground">
                        <span>{formattedDate}</span>
                        {isExpanded ? (
                          <ChevronUp className="w-4 h-4" />
                        ) : (
                          <ChevronDown className="w-4 h-4" />
                        )}
                      </div>
                    </div>
                  </CardHeader>

                  {isExpanded && (
                    <CardContent className="p-0">
                      <div className="h-96 border-t m-5">
                        <MailMessageView
                          envelope={msg}
                          showActions={false}
                          showAttachments={false}
                          showHeader={false}
                        />
                      </div>
                    </CardContent>
                  )}
                </Card>
              );
            })}

          {hasNextPage && (
            <div className="flex justify-center py-3">
              <Button
                onClick={() => fetchNextPage()}
                disabled={isFetchingNextPage}
                variant="outline"
                size="sm"
              >
                {isFetchingNextPage ? (
                  <>
                    <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                    {t('mailbox.thread.loadingMore')}
                  </>
                ) : (
                  t('mailbox.thread.loadMore')
                )}
              </Button>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

// Skeleton
function ThreadSkeleton() {
  return (
    <div className="space-y-4">
      {[...Array(3)].map((_, i) => (
        <Card key={i}>
          <CardHeader>
            <Skeleton className="h-4 w-48 mb-2" />
            <Skeleton className="h-5 w-64 mb-1" />
            <Skeleton className="h-4 w-full" />
            <Skeleton className="h-4 w-32 mt-2" />
          </CardHeader>
        </Card>
      ))}
    </div>
  );
}
