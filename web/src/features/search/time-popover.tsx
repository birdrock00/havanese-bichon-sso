import * as React from 'react'
import { CalendarRange, ChevronDown, X } from 'lucide-react'
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '@/components/ui/popover'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { useSearchContext } from './context'
import { DatePicker } from '@/components/date-picker'

const DAY = 86400000

export function TimePopover() {
    const { filter, setFilter } = useSearchContext()
    const [customDays, setCustomDays] = React.useState<string>('')

    const since = filter.since
    const before = filter.before

    const setRange = (s?: number, b?: number) => {
        setFilter(prev => {
            const next = { ...prev }
            s ? (next.since = s) : delete next.since
            b ? (next.before = b) : delete next.before
            return next
        })
    }

    const setSince = (s?: number) => setRange(s, before)
    const setBefore = (b?: number) => setRange(since, b)

    const handleApplyRecent = () => {
        const days = parseInt(customDays)
        if (!isNaN(days) && days > 0) {
            setRange(Date.now() - days * DAY, undefined)
        }
    }

    const clear = () => {
        setRange()
        setCustomDays('')
    }

    return (
        <Popover>
            <PopoverTrigger asChild>
                <Button
                    size="sm"
                    variant="outline"
                    className={cn(
                        'h-8 rounded-none px-3 gap-1.5 transition-colors',
                        (since || before) && 'bg-primary/10 text-primary hover:bg-primary/20'
                    )}
                >
                    <CalendarRange className="h-4 w-4" />
                    {label(since, before)}
                    <ChevronDown className="h-3.5 w-3.5 opacity-60" />
                </Button>
            </PopoverTrigger>

            <PopoverContent align="start" className="w-[530px] p-4 space-y-6">
                <Section title="Recent Range (Since...)">
                    <div className="space-y-4 w-full">
                        <div className="flex flex-wrap gap-2">
                            {[1, 7, 30].map(d => (
                                <Quick key={d} onClick={() => setRange(Date.now() - d * DAY, undefined)}>
                                    Last {d === 1 ? 'day' : `${d} days`}
                                </Quick>
                            ))}
                            {[3, 6].map(m => (
                                <Quick key={m} onClick={() => setRange(Date.now() - m * 30 * DAY, undefined)}>
                                    Last {m} months
                                </Quick>
                            ))}
                        </div>

                        <div className="flex items-center gap-2 pt-3 border-t border-border/50">
                            <span className="text-[10px] uppercase font-bold opacity-40 shrink-0">Recent:</span>
                            <Input
                                type="number"
                                min={1}
                                placeholder="10"
                                className="h-8 w-20 text-xs"
                                value={customDays}
                                onChange={e => setCustomDays(e.target.value)}
                                onKeyDown={e => e.key === 'Enter' && handleApplyRecent()}
                            />
                            <span className="text-xs text-muted-foreground shrink-0">days ago to now</span>
                            <Button
                                size="sm"
                                variant="secondary"
                                className="h-8 px-3 ml-auto text-xs"
                                onClick={handleApplyRecent}
                            >
                                Apply
                            </Button>
                        </div>
                    </div>
                </Section>
                <Section title="Historical (Older than...)">
                    <div className="flex flex-wrap gap-2 w-full">
                        {[1, 2, 3, 5, 10].map(y => (
                            <Quick
                                key={y}
                                onClick={() => setRange(undefined, Date.now() - y * 365 * DAY)}
                                className="border-orange-200 hover:border-orange-400 hover:text-orange-600"
                            >
                                Over {y} {y === 1 ? 'year' : 'years'} ago
                            </Quick>
                        ))}
                    </div>
                </Section>
                <Section title="Absolute Date Range">
                    <div className="flex gap-3 w-full">
                        <div className="flex-1 min-w-0 space-y-1.5">
                            <span className="text-[10px] pl-1 opacity-50 font-medium">SINCE</span>
                            <DatePicker
                                placeholder="Start date"
                                selected={since ? new Date(since) : undefined}
                                onSelect={(date) => setSince(date?.getTime())}
                            />
                        </div>
                        <div className="flex-1 min-w-0 space-y-1.5">
                            <span className="text-[10px] pl-1 opacity-50 font-medium">BEFORE</span>
                            <DatePicker
                                placeholder="End date"
                                selected={before ? new Date(before) : undefined}
                                onSelect={(date) => setBefore(date?.getTime())}
                            />
                        </div>
                    </div>
                </Section>

                {(since || before) && (
                    <div className="px-1 pb-2">
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={clear}
                            className="h-7 w-full justify-start text-xs text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
                        >
                            <X className="mr-2 h-3.5 w-3.5" />
                            Clear time filters
                        </Button>
                    </div>
                )}
            </PopoverContent>
        </Popover>
    )
}

function toDate(ts: number) {
    const d = new Date(ts)
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

function label(s?: number, b?: number) {
    if (!s && !b) return 'Time'
    if (s && b) return `${toDate(s)} → ${toDate(b)}`
    if (s) return `Since ${toDate(s)}`
    return `Older than ${toDate(b!)}`
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
    return (
        <div className="flex flex-col items-start w-full">
            <div className="text-[11px] font-semibold mb-2.5 text-muted-foreground uppercase tracking-wider">
                {title}
            </div>
            {children}
        </div>
    )
}

function Quick({
    children,
    onClick,
    className
}: {
    children: React.ReactNode;
    onClick: () => void;
    className?: string
}) {
    return (
        <Button
            size="sm"
            variant="outline"
            className={cn(
                "h-7 px-2.5 text-xs font-normal hover:bg-primary/5 hover:text-primary shrink-0",
                className
            )}
            onClick={onClick}
        >
            {children}
        </Button>
    )
}