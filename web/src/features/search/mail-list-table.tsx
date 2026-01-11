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


import { dateFnsLocaleMap, formatBytes } from "@/lib/utils"
import { format, formatDistanceToNow } from "date-fns"
import { Paperclip } from "lucide-react"
import { Skeleton } from "@/components/ui/skeleton"
import { Checkbox } from "@/components/ui/checkbox"
import { EmailEnvelope } from "@/api"
import { useSearchContext } from "./context"
import { MailBulkActions } from "./bulk-actions"
import { useTranslation } from 'react-i18next'
import { enUS } from "date-fns/locale"
import { ColumnDef } from "@tanstack/react-table"
import LongText from "@/components/long-text"
import { DataTableColumnHeader } from "./table/data-table-column-header"
import { SearchTable } from "./table/table"
import { DataTableRowActions } from "./table/data-table-row-actions"
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';

interface MailListProps {
    items: EmailEnvelope[]
    isLoading: boolean
    onEnvelopeChanged: (envelope: EmailEnvelope) => void
    setSortBy: (sortBy: "DATE" | "SIZE") => void
    setSortOrder: (value: "desc" | "asc") => void
}

export function MailListTable({
    items,
    isLoading,
    onEnvelopeChanged,
    setSortBy,
    setSortOrder
}: MailListProps) {
    const { t, i18n } = useTranslation()

    const locale = dateFnsLocaleMap[i18n.language.toLowerCase()] ?? enUS;
    const { selected, setSelected } = useSearchContext()

    const columns: ColumnDef<EmailEnvelope>[] = [
      {
        accessorKey: "id",
        header: () => (
            <Checkbox
              checked={
                totalSelected === items.length && items.length > 0
                  ? true
                  : totalSelected > 0
                    ? "indeterminate"
                    : false
              }
              onCheckedChange={handleToggleAll}
              className="h-4 w-4"
            />
        ),
        cell: ({ row }) => (
            <Checkbox
              checked={hasSelected(row.original.account_id, row.original.id)}
              onCheckedChange={() => toggleSelected(row.original.account_id, row.original.id)}
              onClick={(e) => e.stopPropagation()}
              className="h-4 w-4 shrink-0"
            />
        ),
        meta: { className: 'text-left text-sm' },
        minSize: 25,
        maxSize: 25,
      },
      {
        accessorKey: "account_email",
        header: t('search.account'),
        cell: ({ row }) => <LongText className='text-xs max-w-[150px]'>{row.original.account_email}</LongText>,
        meta: { className: 'text-left text-sm' },
        minSize: 166
      },
      {
        accessorKey: "mailbox_name",
        header: t('search.mailbox'),
        cell: ({ row }) => <LongText className='text-xs max-w-[100px]'>{row.original.mailbox_name}</LongText>,
        meta: { className: 'text-left text-sm' },
        minSize: 116,
        maxSize: 116,
      },
      {
        accessorKey: "from",
        header: t('search.from'),
        cell: ({ row }) => <LongText className='text-xs max-w-[134px]'>{row.original.from}</LongText>,
        meta: { className: 'text-left text-sm' },
        minSize: 150,
      },
      {
        accessorKey: "to",
        header: t('search.to'),
        cell: ({ row }) => <LongText className='text-xs max-w-[180px]'>{row.original.to.join(", ")}</LongText>,
        meta: { className: 'text-left text-sm' },
      },
      {
        accessorKey: "subject",
        header: t('search.subject'),
        cell: ({ row }) => <LongText className='text-xs max-w-[500px]'>{row.original.subject}</LongText>,
        meta: { className: 'text-left text-sm' },
        size: 1000
      },
      {
        id: "attachment_count",
        header: () => <Paperclip size={16} />,
        cell: ({ row }) => <span className='text-xs'>{(row.original.attachments ?? []).length}</span>,
        meta: { className: 'text-left text-sm' },
        minSize: 40,
        maxSize: 40
      },
      {
        accessorKey: 'size',
        header: ({ column }) => (
          <DataTableColumnHeader column={column} title={t('search.size')} />
        ),
        cell: ({ row }) => <span className='text-xs max-w-[40px]'>{formatBytes(row.original.size)}</span>,
        meta: { className: 'text-left text-sm' },
        minSize: 100,
        maxSize: 100,
      },
      {
        accessorKey: 'date',
        header: ({ column }) => (
          <DataTableColumnHeader column={column} title={t('search.date')} />
        ),
        cell: ({ row }) => {
          const date = new Date(row.original.date);
          const title = format(date, 'yyyy-MM-dd HH:mm:ss');
          return (
            <Tooltip>
              <TooltipTrigger asChild>
                <span className='text-xs whitespace-nowrap'>
                  {formatDistanceToNow(date, { addSuffix: true, locale })}
                </span>
              </TooltipTrigger>
              <TooltipContent>{title}</TooltipContent>
            </Tooltip>
          )
        },
        meta: { className: 'text-left text-sm' },
        minSize: 100,
      },
      {
        id: 'actions',
        header: t('users.columns.actions'),
        cell: DataTableRowActions,
        minSize: 70,
        maxSize: 70,
      },
    ]

    const handleToggleAll = () => {
        const total = Array.from(selected.values())
            .reduce((sum, set) => sum + set.size, 0);

        if (total === items.length && items.length > 0) {
            setSelected(new Map());
        } else {
            setSelected(prev => {
                const next = new Map(prev);
                for (const item of items) {
                    const set = new Set(next.get(item.account_id) || []);
                    set.add(item.id);
                    next.set(item.account_id, set);
                }
                return next;
            });
        }
    }

    const toggleSelected = (accountId: number, mailId: number) => {
        setSelected(prev => {
            const next = new Map(prev);
            const set = new Set(next.get(accountId) || []);

            if (set.has(mailId)) {
                set.delete(mailId);
                if (set.size === 0) next.delete(accountId);
                else next.set(accountId, set);
            } else {
                set.add(mailId);
                next.set(accountId, set);
            }

            return next;
        });
    }

    const totalSelected = Array.from(selected.values())
        .reduce((sum, set) => sum + set.size, 0);

    const hasSelected = (accountId: number, mailId: number) => selected.get(accountId)?.has(mailId) ?? false;

    if (isLoading) {
        return (
            <div className="divide-y divide-border">
                {Array.from({ length: 8 }).map((_, i) => (
                    <div key={i} className="flex items-center gap-2 px-2 py-1.5">
                        <Skeleton className="h-3 w-3" />
                        <Skeleton className="h-3 w-3 rounded-full" />
                        <Skeleton className="h-3 flex-1" />
                        <Skeleton className="h-2.5 w-16" />
                    </div>
                ))}
            </div>
        )
    }

    return (
        <>
          <SearchTable
            data={items}
            columns={columns}
            onRowClick={(e, row) => {
              const target = e.target as HTMLElement
              if (target.closest('input[type="checkbox"], button')) return
              onEnvelopeChanged(row.original)
            }}
            setSortBy={setSortBy}
            setSortOrder={setSortOrder}
          />
          {totalSelected > 0 && <MailBulkActions />}
        </>
    )
}
