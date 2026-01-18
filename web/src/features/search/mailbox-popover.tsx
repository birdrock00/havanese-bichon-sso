import * as React from 'react'
import { ChevronDown, Folders, X } from 'lucide-react'
import { useQueries } from '@tanstack/react-query'

import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '@/components/ui/popover'
import {
    Accordion,
    AccordionContent,
    AccordionItem,
    AccordionTrigger,
} from '@/components/ui/accordion'

import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'

import { list_mailboxes, MailboxData } from '@/api/mailbox/api'
import useMinimalAccountList from '@/hooks/use-minimal-account-list'
import { useSearchContext } from './context'

export function MailboxPopover() {
    const { filter, setFilter } = useSearchContext()
    const { minimalList = [] } = useMinimalAccountList()

    const [search, setSearch] = React.useState('')

    const accountIds: number[] = filter.account_ids ?? []
    const selectedMailboxIds: number[] = filter.mailbox_ids ?? []

    const { mailboxes, isLoading } = useQueries({
        queries: accountIds.map(id => ({
            queryKey: ['search-mailboxes', id],
            queryFn: () => list_mailboxes(id, false),
            enabled: accountIds.length > 0,
        })),
        combine: results => ({
            mailboxes: results.flatMap(r => r.data ?? []),
            isLoading: results.some(r => r.isLoading),
        }),
    })

    const toggleMailbox = (id: number) => {
        setFilter(prev => {
            const next = { ...prev }
            const set = new Set<number>(next.mailbox_ids ?? [])

            set.has(id) ? set.delete(id) : set.add(id)

            const ids = Array.from(set)

            if (ids.length === 0) delete next.mailbox_ids
            else next.mailbox_ids = ids

            return next
        })
    }

    const clearAllMailboxes = () => {
        setFilter(prev => {
            const next = { ...prev }
            delete next.mailbox_ids
            return next
        })
    }

    const grouped = React.useMemo(() => {
        const q = search.trim().toLowerCase()
        const map = new Map<number, MailboxData[]>()

        for (const mb of mailboxes) {
            if (q && !mb.name.toLowerCase().includes(q)) continue
            if (!map.has(mb.account_id)) map.set(mb.account_id, [])
            map.get(mb.account_id)!.push(mb)
        }

        for (const list of map.values()) {
            list.sort((a, b) => {
                const aSel = selectedMailboxIds.includes(a.id)
                const bSel = selectedMailboxIds.includes(b.id)
                if (aSel && !bSel) return -1
                if (!aSel && bSel) return 1
                return a.name.localeCompare(b.name)
            })
        }

        return Array.from(map.entries())
    }, [mailboxes, search, selectedMailboxIds])

    const defaultOpen = grouped
        .filter(([, boxes]) =>
            boxes.some(m => selectedMailboxIds.includes(m.id))
        )
        .map(([id]) => id.toString())

    const getAccountEmail = (id: number) =>
        minimalList.find(a => a.id === id)?.email ?? ''

    const disabled = accountIds.length === 0

    return (
        <Popover>
            <PopoverTrigger asChild>
                <Button
                    size="sm"
                    variant="outline"
                    disabled={disabled}
                    className={cn(
                        'h-8 rounded-none px-3 gap-1.5',
                        selectedMailboxIds.length > 0 &&
                        'bg-primary/10 text-primary'
                    )}
                >
                    <Folders className="h-4 w-4" />
                    Mailbox
                    {selectedMailboxIds.length > 0 && (
                        <span className="ml-1 text-xs opacity-70">
                            {selectedMailboxIds.length}
                        </span>
                    )}
                    <ChevronDown className="h-3.5 w-3.5 opacity-60" />
                </Button>
            </PopoverTrigger>

            <PopoverContent align="start" className="min-w-[260px] w-fit max-w-[620px] p-1">
                <div className="p-1 pb-2">
                    <Input
                        value={search}
                        onChange={e => setSearch(e.target.value)}
                        placeholder="Search mailbox"
                        className="h-8 text-sm"
                    />
                </div>
                {selectedMailboxIds.length > 0 && (
                    <div className="px-1 pb-2">
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={clearAllMailboxes}
                            className="h-7 w-full justify-start text-xs text-muted-foreground hover:text-destructive transition-colors"
                        >
                            <X className="mr-2 h-3.5 w-3.5" />
                            Clear Mailboxes ({selectedMailboxIds.length})
                        </Button>
                    </div>
                )}
                <ScrollArea className="h-96 p-1">
                    {disabled ? (
                        <p className="px-3 py-2 text-xs text-muted-foreground">
                            Please select account first
                        </p>
                    ) : isLoading ? (
                        <div className="space-y-2 p-2">
                            {Array.from({ length: 6 }).map((_, i) => (
                                <div
                                    key={i}
                                    className="h-4 rounded bg-muted animate-pulse"
                                />
                            ))}
                        </div>
                    ) : grouped.length === 0 ? (
                        <p className="px-3 py-2 text-xs text-muted-foreground">
                            No mailbox found
                        </p>
                    ) : (
                        <Accordion
                            type="multiple"
                            defaultValue={defaultOpen}
                            className="space-y-1"
                        >
                            {grouped.map(([accountId, boxes]) => {
                                const selectedCount = boxes.filter(b =>
                                    selectedMailboxIds.includes(b.id)
                                ).length

                                return (
                                    <AccordionItem
                                        key={accountId}
                                        value={accountId.toString()}
                                    >
                                        <AccordionTrigger className="text-xs px-2 py-1.5">
                                            <span className="truncate">
                                                {getAccountEmail(accountId)}
                                            </span>

                                            {selectedCount > 0 && (
                                                <span className="ml-2 text-[10px] text-primary">
                                                    {selectedCount}
                                                </span>
                                            )}
                                        </AccordionTrigger>

                                        <AccordionContent>
                                            <div className="space-y-0.5">
                                                {boxes.map(mailbox => {
                                                    const checked =
                                                        selectedMailboxIds.includes(mailbox.id)

                                                    return (
                                                        <TooltipProvider key={mailbox.id}>
                                                            <Tooltip>
                                                                <TooltipTrigger asChild>
                                                                    <div
                                                                        onClick={() =>
                                                                            toggleMailbox(mailbox.id)
                                                                        }
                                                                        className={cn(
                                                                            'flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer',
                                                                            'hover:bg-accent transition-colors',
                                                                            checked &&
                                                                            'bg-primary/10 text-primary'
                                                                        )}
                                                                    >
                                                                        <Checkbox
                                                                            checked={checked}
                                                                            onCheckedChange={() =>
                                                                                toggleMailbox(mailbox.id)
                                                                            }
                                                                            onClick={e =>
                                                                                e.stopPropagation()
                                                                            }
                                                                        />

                                                                        <span className="text-xs truncate">
                                                                            {mailbox.name}
                                                                        </span>
                                                                    </div>
                                                                </TooltipTrigger>

                                                                <TooltipContent side="right">
                                                                    <div className="text-sm break-all">
                                                                        {mailbox.name}
                                                                    </div>
                                                                </TooltipContent>
                                                            </Tooltip>
                                                        </TooltipProvider>
                                                    )
                                                })}
                                            </div>
                                        </AccordionContent>
                                    </AccordionItem>
                                )
                            })}
                        </Accordion>
                    )}
                </ScrollArea>
            </PopoverContent>
        </Popover>
    )
}
