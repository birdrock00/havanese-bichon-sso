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



import { useRef } from 'react'
import { X, Trash2 } from 'lucide-react'
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

type MailBulkActionsProps = {
    children?: React.ReactNode
}

export function MailBulkActions({ children }: MailBulkActionsProps) {
    const { selected, setSelected, setOpen, setToDelete } = useSearchContext()
    const toolbarRef = useRef<HTMLDivElement>(null)
    const { t } = useTranslation()

    const selectedCount = Array.from(selected.values())
        .reduce((sum, set) => sum + set.size, 0)

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
                    {/* Clear Selection */}
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
                        <TooltipContent>
                            {t('search.bulkActions.clearWithKey', { key: 'Escape' })}
                        </TooltipContent>
                    </Tooltip>

                    <Separator orientation="vertical" className="h-5" />

                    {/* Selected Count */}
                    <div className="flex items-center gap-x-1 text-sm">
                        <Badge variant="default" className="min-w-8 rounded-lg">
                            {selectedCount}
                        </Badge>{' '}
                        <span className="hidden sm:inline">
                            {t('search.bulkActions.selected', { count: selectedCount })}
                        </span>
                    </div>

                    <Separator orientation="vertical" className="h-5" />

                    {/* Delete */}
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <Button
                                variant="destructive"
                                size="sm"
                                onClick={handleDelete}
                                className="gap-1"
                            >
                                <Trash2 className="h-3.5 w-3.5" />
                                <span className="hidden sm:inline">
                                    {t('search.bulkActions.delete')}
                                </span>
                            </Button>
                        </TooltipTrigger>
                        <TooltipContent>
                            {t('search.bulkActions.deleteDesc')}
                        </TooltipContent>
                    </Tooltip>

                    {children}
                </div>
            </div>
        </>
    )
}

