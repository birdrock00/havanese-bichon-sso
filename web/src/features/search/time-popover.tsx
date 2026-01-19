import * as React from 'react'
import { CalendarRange, ChevronDown, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { format } from 'date-fns'
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
    const { t } = useTranslation()
    const { filter, setFilter } = useSearchContext()
    const [customDays, setCustomDays] = React.useState<string>('')

    const since = filter.since
    const before = filter.before

    const toDate = (ts: number) => {
        return format(ts, t('time.format'))
    }

    const label = (s?: number, b?: number) => {
        if (!s && !b) return t('time.label')
        if (s && b) return `${toDate(s)} → ${toDate(b)}`
        if (s) return `${t('time.since')} ${toDate(s)}`
        return `${t('time.before')} ${toDate(b!)}`
    }

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
                <Section title={t('time.recent_range')}>
                    <div className="space-y-4 w-full">
                        <div className="flex flex-wrap gap-2">
                            {[1, 7, 30].map(d => (
                                <Quick key={d} onClick={() => setRange(Date.now() - d * DAY, undefined)}>
                                    {d === 1 ? t('time.last_day') : t('time.last_days', { count: d })}
                                </Quick>
                            ))}
                            {[3, 6].map(m => (
                                <Quick key={m} onClick={() => setRange(Date.now() - m * 30 * DAY, undefined)}>
                                    {t('time.last_months', { count: m })}
                                </Quick>
                            ))}
                        </div>

                        <div className="flex items-center gap-2 pt-3 border-t border-border/50">
                            <span className="text-[10px] uppercase font-bold opacity-40 shrink-0">{t('time.recent_prefix')}</span>
                            <Input
                                type="number"
                                min={1}
                                placeholder="10"
                                className="h-8 w-20 text-xs"
                                value={customDays}
                                onChange={e => setCustomDays(e.target.value)}
                                onKeyDown={e => e.key === 'Enter' && handleApplyRecent()}
                            />
                            <span className="text-xs text-muted-foreground shrink-0">{t('time.days_ago_to_now')}</span>
                            <Button
                                size="sm"
                                variant="secondary"
                                className="h-8 px-3 ml-auto text-xs"
                                onClick={handleApplyRecent}
                            >
                                {t('time.apply')}
                            </Button>
                        </div>
                    </div>
                </Section>
                <Section title={t('time.historical')}>
                    <div className="flex flex-wrap gap-2 w-full">
                        {[1, 2, 3, 5, 10].map(y => (
                            <Quick
                                key={y}
                                onClick={() => setRange(undefined, Date.now() - y * 365 * DAY)}
                                className="border-orange-200 hover:border-orange-400 hover:text-orange-600"
                            >
                                {t('time.over_years_ago', {
                                    count: y,
                                    unit: y === 1 ? t('time.year') : t('time.years')
                                })}
                            </Quick>
                        ))}
                    </div>
                </Section>
                <Section title={t('time.absolute_range')}>
                    <div className="flex gap-3 w-full">
                        <div className="flex-1 min-w-0 space-y-1.5">
                            <span className="text-[10px] pl-1 opacity-50 font-medium">{t('time.since').toUpperCase()}</span>
                            <DatePicker
                                placeholder={t('time.start_date')}
                                selected={since ? new Date(since) : undefined}
                                onSelect={(date) => setSince(date?.getTime())}
                            />
                        </div>
                        <div className="flex-1 min-w-0 space-y-1.5">
                            <span className="text-[10px] pl-1 opacity-50 font-medium">{t('time.before').toUpperCase()}</span>
                            <DatePicker
                                placeholder={t('time.end_date')}
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
                            {t('time.clear_filters')}
                        </Button>
                    </div>
                )}
            </PopoverContent>
        </Popover>
    )
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