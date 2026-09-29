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



import { useRef } from 'react'
import { X, Trash2, Upload, TagIcon } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import {
    Tooltip,
    TooltipTrigger,
    TooltipContent,
} from '@/components/ui/tooltip'
import { useSearchContext } from './context'
import { useTranslation } from 'react-i18next'
import { toast } from '@/hooks/use-toast'
import { OP_MESSAGE_DELETE } from '@/api/approvals/api'
import { usePendingApprovals } from '@/features/approvals/use-pending-approvals'

type MailBulkActionsProps = {
    children?: React.ReactNode
}

export function MailBulkActions({ children }: MailBulkActionsProps) {
    const { selected, setSelected, setOpen, setToDelete, setCurrentEnvelope } = useSearchContext()
    const toolbarRef = useRef<HTMLDivElement>(null)
    const { t } = useTranslation()
    const { countPendingMessages, hasPendingMessage } = usePendingApprovals()

    const selectedCount = Array.from(selected.values())
        .reduce((sum, set) => sum + set.size, 0)

    // Part of the selection may already be queued. The server has no dedup —
    // it would happily queue a message twice — so those are filtered out
    // before the dialog opens, and this count explains the difference between
    // what was selected and what will actually be submitted.
    const queuedCount = countPendingMessages(OP_MESSAGE_DELETE, selected)

    const handleClearSelection = () => {
        setSelected(new Map())
    }

    const handleDelete = () => {
        setToDelete(new Map())
        selected.forEach((ids, accountId) => {
            setToDelete(prev => {
                const next = new Map(prev)
                next.set(accountId, new Set(ids))
                return next
            })
        })
        setOpen('delete')
    }

    /** Open the delete dialog, leaving out anything already queued.
     *
     *  Skipping rather than submitting the lot: the server has no dedup, so a
     *  message that already awaits approval would be queued a second time and
     *  two approvers would each execute a deletion of the same message. */
    const handleDeleteSkippingQueued = () => {
        if (queuedCount === 0) {
            handleDelete()
            return
        }
        const remaining = new Map<number, Set<string>>()
        selected.forEach((ids, accountId) => {
            const keep = new Set(
                Array.from(ids).filter(
                    (id) => !hasPendingMessage(OP_MESSAGE_DELETE, accountId, id)
                )
            )
            if (keep.size > 0) remaining.set(accountId, keep)
        })
        setToDelete(remaining)
        if (remaining.size === 0) {
            toast({
                title: t(
                    'approvals.allQueuedTitle',
                    'Nothing to submit — every selected message already awaits approval'
                ),
                description: t(
                    'approvals.allQueuedDesc',
                    'A second person has to decide on those requests before they can be asked for again.'
                ),
            })
            return
        }
        setOpen('delete')
    }


    const handleRestore = () => {
        setCurrentEnvelope(undefined);
        setOpen('restore')
    }


    const handleUpdateTags = () => {
        setOpen('update-tags')
    }

    const handleKeyDown = (e: React.KeyboardEvent) => {
        const buttons = toolbarRef.current?.querySelectorAll('button')
        if (!buttons || buttons.length === 0) return

        const currentIndex = Array.from(buttons).findIndex(
            btn => btn === document.activeElement
        )

        switch (e.key) {
            case 'ArrowRight': {
                e.preventDefault()
                const next = (currentIndex + 1) % buttons.length
                buttons[next]?.focus()
                break
            }
            case 'ArrowLeft': {
                e.preventDefault()
                const prev = currentIndex === 0 ? buttons.length - 1 : currentIndex - 1
                buttons[prev]?.focus()
                break
            }
            case 'Home':
                e.preventDefault()
                buttons[0]?.focus()
                break
            case 'End':
                e.preventDefault()
                buttons[buttons.length - 1]?.focus()
                break
            case 'Escape': {
                const target = e.target as HTMLElement
                const active = document.activeElement as HTMLElement
                const isFromDropdown =
                    target.closest('[data-slot="dropdown-menu-trigger"]') ||
                    active.closest('[data-slot="dropdown-menu-trigger"]') ||
                    target.closest('[data-slot="dropdown-menu-content"]') ||
                    active.closest('[data-slot="dropdown-menu-content"]')

                if (!isFromDropdown) {
                    e.preventDefault()
                    handleClearSelection()
                }
                break
            }
        }
    }

    if (selectedCount === 0) return null

    return (
        <>
            <div
                ref={toolbarRef}
                role="toolbar"
                aria-label={t('search.bulkActions.ariaLabel', {
                    count: selectedCount,
                })}
                tabIndex={-1}
                onKeyDown={handleKeyDown}
                className={cn(
                    'fixed bottom-6 left-1/2 z-50 -translate-x-1/2 rounded-xl',
                    'transition-all delay-100 duration-300 ease-out hover:scale-105',
                    'focus-visible:ring-ring/50 focus-visible:ring-2 focus-visible:outline-none'
                )}
            >
                <div
                    className={cn(
                        'p-2 shadow-xl rounded-xl border',
                        'bg-background/95 supports-[backdrop-filter]:bg-background/60 backdrop-blur-lg',
                        'flex items-center gap-x-2'
                    )}
                >
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <Button
                                variant="outline"
                                size="icon"
                                onClick={handleClearSelection}
                                className="size-6 rounded-full"
                                aria-label={t('search.bulkActions.clear')}
                            >
                                <X className="h-3 w-3" />
                                <span className="sr-only">{t('search.bulkActions.clear')}</span>
                            </Button>
                        </TooltipTrigger>
                    </Tooltip>
                    <Separator orientation="vertical" className="h-5" />
                    <div className="flex items-center gap-x-1 text-sm">
                        <Badge variant="default" className="min-w-8 rounded-lg">
                            {selectedCount}
                        </Badge>{' '}
                    </div>
                    <Separator orientation="vertical" className="h-5" />
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <Button
                                variant="secondary"
                                size="sm"
                                onClick={handleRestore}
                                className="gap-1"
                            >
                                <Upload className="h-3.5 w-3.5" />
                            </Button>
                        </TooltipTrigger>
                        <TooltipContent>
                            {t('restore_message.restore_to_imap', 'Restore Mail')}
                        </TooltipContent>
                    </Tooltip>
                    <Separator orientation="vertical" className="h-5" />
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <Button
                                variant="secondary"
                                size="sm"
                                onClick={handleUpdateTags}
                                className="gap-1"
                            >
                                <TagIcon className="h-3.5 w-3.5" />
                            </Button>
                        </TooltipTrigger>
                        <TooltipContent>
                            {t('search.bulkActions.manageTags')}
                        </TooltipContent>
                    </Tooltip>
                    <Separator orientation="vertical" className="h-5" />
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <Button
                                variant="destructive"
                                size="sm"
                                onClick={handleDeleteSkippingQueued}
                                className="gap-1"
                            >
                                <Trash2 className="h-3.5 w-3.5" />
                                {queuedCount > 0 && (
                                    <span className="text-[10px] leading-none">
                                        {selectedCount - queuedCount}
                                    </span>
                                )}
                            </Button>
                        </TooltipTrigger>
                        <TooltipContent>
                            {queuedCount > 0
                                ? t(
                                      'approvals.bulkPartlyQueued',
                                      '{{queued}} of {{total}} selected message(s) already await approval and will be skipped',
                                      { queued: queuedCount, total: selectedCount }
                                  )
                                : t('search.bulkActions.deleteDesc')}
                        </TooltipContent>
                    </Tooltip>

                    {children}
                </div>
            </div>
        </>
    )
}

