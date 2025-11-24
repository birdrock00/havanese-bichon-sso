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


import { cn, formatBytes } from "@/lib/utils"
import { formatDistanceToNow } from "date-fns"
import { MailIcon, MoreVertical, Paperclip, TagIcon, Trash2 } from "lucide-react"
import { Skeleton } from "@/components/ui/skeleton"
import { Checkbox } from "@/components/ui/checkbox"   // shadcn Checkbox
import { EmailEnvelope } from "@/api"
import { useSearchContext } from "./context"
import { MailBulkActions } from "./bulk-actions"
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { useTranslation } from 'react-i18next'

interface MailListProps {
    items: EmailEnvelope[]
    isLoading: boolean
    onEnvelopeChanged: (envelope: EmailEnvelope) => void
}

export function MailList({
    items,
    isLoading,
    onEnvelopeChanged
}: MailListProps) {
    const { t } = useTranslation()
    const { setOpen, currentEnvelope, setCurrentEnvelope, selected, setSelected, setToDelete } = useSearchContext()

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
    const toggleToDelete = (accountId: number, mailId: number) => {
        setToDelete(prev => {
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
    };

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

    const hasSelected = (accountId: number, mailId: number) => {
        return selected.get(accountId)?.has(mailId) ?? false;
    }

    const handleDelete = (envelope: EmailEnvelope) => {
        setToDelete(new Map());
        toggleToDelete(envelope.account_id, envelope.id)
        setOpen("delete")
    }

    if (isLoading) {
        return (
            <div className="divide-y divide-border">
                {Array.from({ length: 8 }).map((_, i) => (
                    <div key={i} className="flex items-center gap-3 p-3">
                        <Skeleton className="h-4 w-4" />
                        <Skeleton className="h-4 w-4 rounded-full" />
                        <Skeleton className="h-4 flex-1" />
                        <Skeleton className="h-3 w-20" />
                    </div>
                ))}
            </div>
        )
    }

    return (
        <div className="divide-y divide-border">
            {items.length > 0 && (
                <div className="flex items-center gap-3 px-3 py-2 bg-muted/30">
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
                    <span className="text-xs text-muted-foreground">
                        {totalSelected > 0
                            ? `${totalSelected} ${t('common.selected')}`
                            : t('common.selectAll')}
                    </span>
                </div>
            )}

            {items.map((item, index) => {
                const hasAttachments = item.attachments && item.attachments.length > 0
                const isSelectedRow = currentEnvelope?.id === item.id
                const isChecked = hasSelected(item.account_id, item.id)

                return (
                    <div
                        key={index}
                        className={cn(
                            "flex items-center gap-3 px-3 py-2.5 cursor-pointer transition-colors",
                            "hover:bg-accent/50",
                            isSelectedRow && "bg-accent"
                        )}
                        onClick={(e) => {
                            const target = e.target as HTMLElement
                            if (target.closest('input[type="checkbox"], button')) return
                            onEnvelopeChanged(item)
                        }}
                    >
                        <Checkbox
                            checked={isChecked}
                            onCheckedChange={() => toggleSelected(item.account_id, item.id)}
                            onClick={(e) => e.stopPropagation()}
                            className="h-4 w-4 shrink-0"
                        />

                        <MailIcon className="h-4 w-4 text-muted-foreground shrink-0" />
                        <div className="flex-1 min-w-0 grid grid-cols-1 sm:grid-cols-12 gap-1 sm:gap-0">

                            {/* LEFT AREA: From + Subject + Tags */}
                            <div className="col-span-1 sm:col-span-8 flex flex-col min-w-0">

                                {/* from + subject (large screen side by side, small screen subject hidden) */}
                                <div className="flex items-center gap-2 min-w-0">
                                    <p className="text-sm font-medium truncate">{item.from}</p>

                                    {/* subject on large screens */}
                                    <h3 className="text-sm text-muted-foreground truncate hidden sm:block">
                                        {item.subject}
                                    </h3>
                                </div>

                                {/* subject on small screens */}
                                <h3 className="text-sm text-muted-foreground truncate sm:hidden">
                                    {item.subject}
                                </h3>

                                {/* TAGS (always below on small screen, inline on large screen) */}
                                <div className="flex flex-wrap gap-1 mt-0.5">
                                    {item.tags?.map((tag, index) => (
                                        <Badge className="px-1 py-0.5 text-[10px] h-auto leading-none" key={index}>{tag}</Badge>
                                    ))}
                                </div>
                            </div>

                            {/* RIGHT AREA – actions & meta */}
                            <div className="col-span-1 sm:col-span-4 flex items-center justify-end gap-2 text-xs text-muted-foreground">

                                {hasAttachments && (
                                    <div className="flex items-center gap-1">
                                        <Paperclip className="h-3.5 w-3.5" />
                                        <span>{item.attachments?.length}</span>
                                    </div>
                                )}

                                <span className="hidden md:inline">{formatBytes(item.size)}</span>

                                <span className={cn(isSelectedRow ? "text-foreground font-medium" : "text-muted-foreground")}>
                                    {item.date && formatDistanceToNow(new Date(item.date), { addSuffix: true })}
                                </span>

                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button
                                            variant="ghost"
                                            size="icon"
                                            className="h-7 w-7 p-0 hover:bg-muted rounded-md"
                                            onClick={(e) => e.stopPropagation()}
                                        >
                                            <MoreVertical className="h-3.5 w-3.5" />
                                        </Button>
                                    </DropdownMenuTrigger>

                                    <DropdownMenuContent align="end" className="w-48">
                                        <DropdownMenuItem
                                            onClick={(e) => e.stopPropagation()}
                                            onSelect={(e) => {
                                                e.stopPropagation();
                                                setCurrentEnvelope(item);
                                                setOpen("edit-tags");
                                            }}
                                        >
                                            <TagIcon className="ml-2 h-4 w-4" />
                                            {t('search.editTag')}
                                        </DropdownMenuItem>

                                        <DropdownMenuItem
                                            className="text-destructive focus:text-destructive"
                                            onClick={(e) => e.stopPropagation()}
                                            onSelect={(e) => {
                                                e.stopPropagation();
                                                handleDelete(item);
                                            }}
                                        >
                                            <Trash2 className="ml-2 h-4 w-4" />
                                            {t('common.delete')}
                                        </DropdownMenuItem>
                                    </DropdownMenuContent>
                                </DropdownMenu>
                            </div>
                        </div>
                    </div>
                )
            })}
            {totalSelected > 0 && (
                <MailBulkActions />
            )}
        </div>
    )
}