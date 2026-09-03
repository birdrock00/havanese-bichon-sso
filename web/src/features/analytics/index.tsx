// Analytics workbench - a unified explore page on top of the new
// POST /api/v1/analytics/query engine (email + attachment levels).
//
// Overview mode presets the axes; Explore mode exposes the full engine.
// Every chart cell / table row drills into the message or attachment search
// pages. Multi-valued filters are JSON arrays - comma separated strings are
// intentionally not used anywhere.

import { useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useTranslation } from 'react-i18next'
import { format } from 'date-fns'
import {
  AlertCircle,
  BarChart3,
  Bookmark,
  BookmarkPlus,
  CalendarRange,
  Check,
  ChevronDown,
  Download,
  Loader2,
  Mail,
  RotateCcw,
  Search,
  SlidersHorizontal,
  Trash2,
  X,
} from 'lucide-react'
import {
  type AnalyticsBucket,
  type AnalyticsDirection,
  type AnalyticsDimension,
  type AnalyticsFilters,
  type AnalyticsLevel,
  type AnalyticsMetric,
  type AnalyticsQuery,
  type AnalyticsResult,
  type AnalyticsRow,
  type AnalyticsTimeField,
  type SavedView,
  type SplitRow,
  create_analytics_view,
  delete_analytics_view,
  list_analytics_views,
  post_analytics_query,
  update_analytics_view,
} from '@/api/analytics/api'
import {
  DAY_MS,
  dimensionLabel,
  dimensionRangeBounds,
  dimensionsForLevel,
  directionLabel,
  displayTag,
  metricLabel,
  metricsForDimension,
  metricsForLevel,
  resultToCsv,
  splitResultToCsv,
  suggestBucket,
  timeFieldLabel,
  timeFieldsForLevel,
} from './format'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { DatePicker } from '@/components/date-picker'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { FixedHeader } from '@/components/layout/fixed-header'
import { Input } from '@/components/ui/input'
import { Main } from '@/components/layout/main'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/hooks/use-toast'
import { useAvailableAttachmentTags } from '@/hooks/use-available-attachment-tags'
import { useAvailableTags } from '@/hooks/use-available-tags'
import useMinimalAccountList from '@/hooks/use-minimal-account-list'
import {
  AnalyticsKpis,
  DirectionSplitTable,
  MatrixPanel,
  RankTable,
  TrendChart,
  type SplitDrillDirection,
} from './results'

type BucketValue = AnalyticsBucket | 'off'
type DimensionValue = AnalyticsDimension | 'none'
type Mode = 'overview' | 'explore' | 'tags'

interface UiFilters {
  accountIds: number[]
  tags: string[]
  direction: AnalyticsDirection
  textDraft: string
  text: string
  rangeKey: string
  since?: number
  before?: number
}

interface AxesState {
  timeField: AnalyticsTimeField
  bucket: BucketValue
  dimension: DimensionValue
  metrics: AnalyticsMetric[]
  top: number
  minDocCount: number
  matrixMode: 'heatmap' | 'stacked'
}

export function bucketLabel(
  bucket: AnalyticsBucket,
  t: (key: string, defaultValue: string) => string,
): string {
  switch (bucket) {
    case 'day':
      return t('analytics.bucketDay', 'Day')
    case 'week':
      return t('analytics.bucketWeek', 'Week')
    case 'month':
      return t('analytics.bucketMonth', 'Month')
    case 'quarter':
      return t('analytics.bucketQuarter', 'Quarter')
    case 'year':
      return t('analytics.bucketYear', 'Year')
  }
}

const RANGE_PRESETS: { key: string; days?: number }[] = [
  { key: '7d', days: 7 },
  { key: '30d', days: 30 },
  { key: '90d', days: 90 },
  { key: '180d', days: 180 },
  { key: '365d', days: 365 },
]
function defaultUiFilters(): UiFilters {
  const before = Date.now()
  const since = before - 365 * DAY_MS
  return {
    accountIds: [],
    tags: [],
    direction: 'all',
    textDraft: '',
    text: '',
    rangeKey: '365d',
    since,
    before,
  }
}

function overviewDimension(level: AnalyticsLevel): AnalyticsDimension {
  return level === 'email' ? 'sender' : 'attachment_category'
}

function defaultAxes(level: AnalyticsLevel): AxesState {
  const dimension = overviewDimension(level)
  return {
    // Trust the server (IMAP INTERNALDATE) by default instead of the
    // sender-declared Date header: spoofed / badly written future dates
    // (e.g. year 2611) would otherwise silently exclude messages from the
    // default time window. Users can still switch to 'date' explicitly.
    timeField: level === 'email' ? 'internal_date' : 'date',
    bucket: 'month',
    dimension,
    metrics: metricsForDimension(level, dimension, true),
    top: 20,
    minDocCount: 1,
    matrixMode: 'heatmap',
  }
}

/** Effective metrics for a dimension choice (respects per-dimension limits). */
function availableMetrics(
  level: AnalyticsLevel,
  dimension: DimensionValue,
): AnalyticsMetric[] {
  if (dimension === 'none') return metricsForLevel(level)
  return metricsForDimension(level, dimension, true)
}

/** Filters in the UI keep one window and apply it to the active time field. */
function applyWindow(
  filters: AnalyticsFilters,
  level: AnalyticsLevel,
  timeField: AnalyticsTimeField,
  since?: number,
  before?: number,
): void {
  if (level === 'attachment' && timeField === 'internal_date') return
  if (timeField === 'date') {
    filters.since = since
    filters.before = before
  } else if (timeField === 'internal_date') {
    filters.internal_date_since = since
    filters.internal_date_before = before
  } else {
    filters.ingest_since = since
    filters.ingest_before = before
  }
}

function buildRequest(
  level: AnalyticsLevel,
  ui: UiFilters,
  axes: AxesState,
  override?: Partial<AxesState>,
  split = false,
): AnalyticsQuery {
  const timeField = override?.timeField ?? axes.timeField
  const filters: AnalyticsFilters = {}
  if (ui.accountIds.length > 0) {
    filters.account_ids = [...ui.accountIds].sort((a, b) => a - b)
  }
  if (ui.tags.length > 0) filters.tags = [...ui.tags]
  if (ui.text.trim()) filters.text = ui.text.trim()
  if (level === 'email') filters.direction = ui.direction

  const hasWindow = ui.rangeKey !== 'all' && ui.since !== undefined
  const since = hasWindow ? ui.since : undefined
  const before = hasWindow ? ui.before : undefined
  applyWindow(filters, level, timeField, since, before)

  const metrics: AnalyticsMetric[] = [
    'count',
    ...(override?.metrics ?? axes.metrics).filter((m) => m !== 'count'),
  ]
  const bucket = override?.bucket ?? axes.bucket
  const dimension = override?.dimension ?? axes.dimension
  return {
    level,
    filters,
    time_field: timeField,
    bucket: bucket === 'off' ? null : bucket,
    dimension: dimension === 'none' ? null : dimension,
    metrics,
    top: override?.top ?? axes.top,
    min_doc_count: override?.minDocCount ?? axes.minDocCount,
    ...(split ? { direction_split: true } : {}),
  }
}

/** Overview axes: trend + a top-N breakdown across the whole time window. */
function overviewRequest(
  level: AnalyticsLevel,
  ui: UiFilters,
  axes: AxesState,
  breakdown: AnalyticsDimension,
): AnalyticsQuery {
  return buildRequest(level, ui, axes, {
    timeField: axes.timeField,
    bucket: suggestBucket(ui.since, ui.before),
    dimension: breakdown,
    metrics: metricsForDimension(level, breakdown, true),
    top: 10,
    minDocCount: 1,
  })
}

/**
 * Tag volume report: rows are tags, columns are time buckets and each cell
 * carries all / sent / received counts (server side direction split).
 */
function tagsRequest(ui: UiFilters, axes: AxesState): AnalyticsQuery {
  return buildRequest(
    'email',
    ui,
    axes,
    {
      timeField: 'internal_date',
      bucket: axes.bucket === 'off' || axes.bucket === 'day' ? 'month' : axes.bucket,
      dimension: 'tag',
      metrics: ['count'],
      top: 50,
      minDocCount: 1,
    },
    true,
  )
}
function extractWindow(req: AnalyticsQuery): {
  rangeKey: string
  since?: number
  before?: number
} {
  const f = req.filters
  let since: number | undefined
  let before: number | undefined
  if (req.time_field === 'date') {
    since = f.since
    before = f.before
  } else if (req.time_field === 'internal_date') {
    since = f.internal_date_since
    before = f.internal_date_before
  } else {
    since = f.ingest_since
    before = f.ingest_before
  }
  if (since === undefined && before === undefined) return { rangeKey: 'all' }
  return { rangeKey: since !== undefined ? 'custom' : 'all', since, before }
}

function uiFiltersFromRequest(req: AnalyticsQuery): UiFilters {
  const f = req.filters
  const win = extractWindow(req)
  return {
    accountIds: f.account_ids ?? [],
    tags: f.tags ?? [],
    direction: (f.direction as AnalyticsDirection) ?? 'all',
    textDraft: f.text ?? '',
    text: f.text ?? '',
    rangeKey: win.rangeKey,
    since: win.since,
    before: win.before,
  }
}

function axesFromRequest(req: AnalyticsQuery): AxesState {
  const base = defaultAxes(req.level)
  return {
    ...base,
    timeField: req.time_field,
    bucket: req.bucket ?? 'off',
    dimension: (req.dimension ?? 'none') as DimensionValue,
    metrics:
      req.metrics.length > 0
        ? ['count', ...req.metrics.filter((m) => m !== 'count')]
        : base.metrics,
    top: req.top,
    minDocCount: req.min_doc_count,
  }
}

function serverErrorText(error: unknown): string {
  if (error && typeof error === 'object' && 'response' in error) {
    const response = (error as { response?: { data?: unknown } }).response
    const data = response?.data as
      | { error?: string }
      | string
      | undefined
    if (typeof data === 'string') return data
    if (data && typeof data === 'object' && typeof data.error === 'string') {
      return data.error
    }
  }
  return error instanceof Error ? error.message : String(error)
}
function rangeLabel(
  rangeKey: string,
  since: number | undefined,
  before: number | undefined,
  t: (key: string, defaultValue: string, options?: Record<string, unknown>) => string
): string {
  if (rangeKey === 'all') {
    return t('analytics.rangeAllTime', 'All time')
  }

  if (rangeKey === '365d') {
    return t('analytics.rangeLast12Months', 'Last 12 months')
  }

  if (rangeKey.endsWith('d')) {
    const days = Number(rangeKey.slice(0, -1))
    return t('analytics.rangeLastDays', `Last ${days} days`, { days })
  }

  const start = since !== undefined ? format(since, 'yyyy-MM-dd') : '?'
  const end = before !== undefined ? format(before, 'yyyy-MM-dd') : '?'

  return `${start} ~ ${end}`
}
/**
 * Builds the JSON search filter used by the message/attachment search pages
 * when a user drills into a cell. Only filters the search page understands
 * are emitted; unsupported axes keep the rest of the scope intact.
 */
function buildDrillFilter(
  level: AnalyticsLevel,
  ui: UiFilters,
  axes: AxesState,
  row: AnalyticsRow | undefined,
  periodIndex: number | undefined,
  periods: { start_ms: number; end_ms: number }[] | undefined,
  getEmailById: (id: number | string) => string | null,
): Record<string, unknown> {
  const filter: Record<string, unknown> = {}
  if (ui.accountIds.length > 0) {
    filter.account_ids = [...ui.accountIds].sort((a, b) => a - b)
  }
  if (ui.tags.length > 0) filter.tags = [...ui.tags]
  if (ui.text.trim()) filter.text = ui.text.trim()
  // Direction is only meaningful for email searches and only when the scope
  // narrows down to one account (we need that account's own address).
  if (level === 'email' && ui.direction !== 'all') {
    const email = ui.accountIds.length === 1 ? getEmailById(ui.accountIds[0]) : null
    if (email) {
      if (ui.direction === 'received') filter.any_recipient = email
      else filter.from = email
    }
  }

  const dimension: AnalyticsDimension | 'none' =
    axes.dimension === 'none' ? 'none' : axes.dimension
  if (dimension !== 'none' && row) {
    switch (dimension) {
      case 'account': {
        const id = Number(row.key)
        if (Number.isFinite(id)) filter.account_ids = [id]
        break
      }
      case 'mailbox': {
        const id = Number(row.key)
        if (Number.isFinite(id)) filter.mailbox_ids = [id]
        break
      }
      case 'sender':
        filter.from = row.key
        break
      case 'tag':
        filter.tags = [row.key]
        break
      case 'attachment_ext':
        filter.attachment_extension = row.key
        break
      case 'attachment_category':
        filter.attachment_category = row.key
        break
      case 'attachment_content_type':
        filter.attachment_content_type = row.key
        break
      case 'content_hash':
        if (level === 'attachment') filter.content_hash = row.key
        break
      case 'size_range':
      case 'attachment_size_range':
      case 'page_count_range': {
        const bounds = dimensionRangeBounds(dimension, row.key)
        if (bounds.min !== undefined) {
          if (dimension === 'page_count_range') filter.min_page_count = bounds.min
          else filter.min_size = bounds.min
        }
        if (bounds.max !== undefined) {
          if (dimension === 'page_count_range') filter.max_page_count = bounds.max
          else filter.max_size = bounds.max
        }
        break
      }
      default:
        break
    }
  }

  if (periodIndex !== undefined && periods && periods[periodIndex]) {
    const period = periods[periodIndex]
    if (axes.timeField === 'date') {
      filter.since = period.start_ms
      filter.before = period.end_ms - 1
    } else if (level === 'email' && axes.timeField === 'internal_date') {
      filter.internal_date_since = period.start_ms
      filter.internal_date_before = period.end_ms - 1
    } else if (level === 'email' && axes.timeField === 'ingest_at') {
      filter.ingest_since = period.start_ms
      filter.ingest_before = period.end_ms - 1
    }
    // The attachment search page only exposes a date-header window; when the
    // analytics axis used ingest time we keep the row scope only.
  }

  return filter
}
function Segmented<T extends string>({
  options,
  value,
  onChange,
  disabled,
}: {
  options: { value: T; label: string }[]
  value: T
  onChange: (value: T) => void
  disabled?: boolean
}) {
  return (
    <div className='inline-flex items-center rounded-md border bg-background p-0.5'>
      {options.map((option) => {
        const active = option.value === value
        return (
          <button
            key={option.value}
            type='button'
            disabled={disabled}
            onClick={() => onChange(option.value)}
            className={
              'rounded px-2.5 py-1 text-xs font-medium transition-colors ' +
              (active
                ? 'bg-primary text-primary-foreground'
                : 'text-muted-foreground hover:text-foreground') +
              (disabled ? ' cursor-not-allowed opacity-50' : '')
            }
          >
            {option.label}
          </button>
        )
      })}
    </div>
  )
}

function AccountFilterPopover({
  selected,
  onChange,
}: {
  selected: number[]
  onChange: (next: number[]) => void
}) {
  const { t } = useTranslation()
  const [search, setSearch] = useState('')
  const { minimalList = [] } = useMinimalAccountList()

  const toggle = (id: number) => {
    const next = new Set(selected)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    onChange(Array.from(next).sort((a, b) => a - b))
  }

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase()
    return minimalList
      .filter(
        (account) =>
          !q ||
          account.email.toLowerCase().includes(q) ||
          (account.name ?? '').toLowerCase().includes(q),
      )
      .sort((a, b) => {
        const aSel = selected.includes(a.id)
        const bSel = selected.includes(b.id)
        if (aSel !== bSel) return aSel ? -1 : 1
        return a.id - b.id
      })
  }, [minimalList, search, selected])

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button size='sm' variant='outline' className='gap-1.5 px-3'>
          <Mail className='h-3.5 w-3.5 text-muted-foreground' />
          <span className='font-normal'>
            {t('analytics.accounts', 'Accounts')}
          </span>
          {selected.length > 0 && (
            <Badge variant='secondary' className='h-5 px-1.5 text-xs'>
              {selected.length}
            </Badge>
          )}
          <ChevronDown className='h-3.5 w-3.5 opacity-60' />
        </Button>
      </PopoverTrigger>
      <PopoverContent align='start' className='w-80 p-0'>
        <div className='p-2'>
          <Input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder={t('analytics.searchAccounts', 'Search accounts')}
            className='h-8 text-xs'
          />
        </div>
        <div className='max-h-72 overflow-y-auto border-t'>
          {filtered.map((account) => (
            <label
              key={account.id}
              className='flex cursor-pointer items-center gap-2 px-3 py-1.5 text-xs hover:bg-accent'
            >
              <Checkbox
                checked={selected.includes(account.id)}
                onCheckedChange={() => toggle(account.id)}
              />
              <span className='truncate font-medium'>
                {account.name?.trim() || account.email}
              </span>
              <span className='ml-auto truncate text-xs text-muted-foreground'>
                {account.name ? account.email : `#${account.id}`}
              </span>
            </label>
          ))}
          {filtered.length === 0 && (
            <p className='px-3 py-4 text-center text-xs text-muted-foreground'>
              {t('analytics.noAccounts', 'No accounts found')}
            </p>
          )}
        </div>
        {selected.length > 0 && (
          <button
            type='button'
            onClick={() => onChange([])}
            className='flex w-full items-center gap-2 border-t px-3 py-2 text-xs font-medium text-destructive hover:bg-destructive/10'
          >
            <X className='h-3.5 w-3.5' />
            {t('analytics.clearAccounts', 'Clear account filter')}
          </button>
        )}
      </PopoverContent>
    </Popover>
  )
}
function TagFilterPopover({
  level,
  selected,
  onChange,
}: {
  level: AnalyticsLevel
  selected: string[]
  onChange: (next: string[]) => void
}) {
  const { t } = useTranslation()
  const [search, setSearch] = useState('')
  const emailTags = useAvailableTags()
  const attachmentTags = useAvailableAttachmentTags()
  const tagsHook = level === 'email' ? emailTags : attachmentTags

  const toggle = (tag: string) => {
    const next = new Set(selected)
    if (next.has(tag)) next.delete(tag)
    else next.add(tag)
    onChange(Array.from(next).sort())
  }

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase()
    return tagsHook.tags.filter(
      (tag) =>
        !q ||
        tag.toLowerCase().includes(q) ||
        displayTag(tag).toLowerCase().includes(q),
    )
  }, [tagsHook.tags, search])

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button size='sm' variant='outline' className='gap-1.5 px-3'>
          <BarChart3 className='h-3.5 w-3.5 text-muted-foreground' />
          <span className='font-normal'>{t('analytics.tags', 'Tags')}</span>
          {selected.length > 0 && (
            <Badge variant='secondary' className='h-5 px-1.5 text-xs'>
              {selected.length}
            </Badge>
          )}
          <ChevronDown className='h-3.5 w-3.5 opacity-60' />
        </Button>
      </PopoverTrigger>
      <PopoverContent align='start' className='w-80 p-0'>
        <div className='p-2'>
          <Input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder={t('analytics.searchTags', 'Search tags')}
            className='h-8 text-xs'
          />
        </div>
        <ScrollArea className='max-h-72 border-t'>
          <div className='p-1'>
            {filtered.map((tag) => (
              <label
                key={tag}
                className='flex cursor-pointer items-center gap-2 px-2 py-1.5 text-xs hover:bg-accent'
              >
                <Checkbox
                  checked={selected.includes(tag)}
                  onCheckedChange={() => toggle(tag)}
                />
                <span className='truncate'>{displayTag(tag)}</span>
              </label>
            ))}
            {filtered.length === 0 && (
              <p className='px-3 py-4 text-center text-xs text-muted-foreground'>
                {t('analytics.noTags', 'No tags found')}
              </p>
            )}
          </div>
        </ScrollArea>
        {selected.length > 0 && (
          <button
            type='button'
            onClick={() => onChange([])}
            className='flex w-full items-center gap-2 border-t px-3 py-2 text-xs font-medium text-destructive hover:bg-destructive/10'
          >
            <X className='h-3.5 w-3.5' />
            {t('analytics.clearTags', 'Clear tag filter')}
          </button>
        )}
      </PopoverContent>
    </Popover>
  )
}

function RangePopover({
  level,
  timeField,
  onTimeFieldChange,
  rangeKey,
  since,
  before,
  onRangeChange,
}: {
  level: AnalyticsLevel
  timeField: AnalyticsTimeField
  onTimeFieldChange: (field: AnalyticsTimeField) => void
  rangeKey: string
  since?: number
  before?: number
  onRangeChange: (range: {
    rangeKey: string
    since?: number
    before?: number
  }) => void
}) {
  const { t } = useTranslation()
  const applyPreset = (days?: number) => {
    if (days === undefined) {
      onRangeChange({ rangeKey: 'all' })
      return
    }
    const end = Date.now()
    onRangeChange({
      rangeKey: `${days}d`,
      since: end - days * DAY_MS,
      before: end,
    })
  }

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button size='sm' variant='outline' className='gap-1.5 px-3'>
          <CalendarRange className='h-3.5 w-3.5 text-muted-foreground' />
          <span className='font-normal'>
            {timeFieldLabel(timeField, t)} ·{' '}
            {rangeLabel(rangeKey, since, before, t)}
          </span>
          <ChevronDown className='h-3.5 w-3.5 opacity-60' />
        </Button>
      </PopoverTrigger>
      <PopoverContent align='start' className='w-72 p-3'>
        <p className='mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground'>
          {t('analytics.timeField', 'Time field')}
        </p>
        <div className='mb-3 flex flex-wrap gap-1.5'>
          {timeFieldsForLevel(level).map((field) => (
            <button
              key={field}
              type='button'
              onClick={() => onTimeFieldChange(field)}
              className={
                'rounded-md border px-2 py-1 text-xs font-medium transition-colors ' +
                (field === timeField
                  ? 'border-primary bg-primary/10 text-primary'
                  : 'text-muted-foreground hover:bg-accent')
              }
            >
              {timeFieldLabel(field, t)}
            </button>
          ))}
        </div>
        <div className='mb-3 grid grid-cols-2 gap-1.5'>
          {RANGE_PRESETS.map((preset) => (
            <button
              key={preset.key}
              type='button'
              onClick={() => applyPreset(preset.days)}
              className={
                'rounded-md border px-2 py-1.5 text-xs font-medium transition-colors ' +
                (rangeKey === preset.key
                  ? 'border-primary bg-primary/10 text-primary'
                  : 'text-muted-foreground hover:bg-accent')
              }
            >
              {preset.days === 365
                ? t('analytics.rangeLast12Months', 'Last 12 months')
                : t('analytics.rangeLastDays', 'Last {{days}} days', {
                  days: preset.days,
                })}
            </button>
          ))}

          <button
            type='button'
            onClick={() => applyPreset(undefined)}
            className={
              'rounded-md border px-2 py-1.5 text-xs font-medium transition-colors ' +
              (rangeKey === 'all'
                ? 'border-primary bg-primary/10 text-primary'
                : 'text-muted-foreground hover:bg-accent')
            }
          >
            {t('analytics.rangeAllTime', 'All time')}
          </button>
        </div>
        <div className='flex flex-col gap-2'>
          <div className='flex items-center gap-2'>
            <span className='w-8 shrink-0 text-xs text-muted-foreground'>From</span>
            <div className='min-w-0 flex-1'>
              <DatePicker
                selected={since !== undefined ? new Date(since) : undefined}
                className='w-full text-xs'
                onSelect={(date) =>
                  onRangeChange({
                    rangeKey: 'custom',
                    since: date ? date.getTime() : since,
                    before,
                  })
                }
              />
            </div>
          </div>

          <div className='flex items-center gap-2'>
            <span className='w-8 shrink-0 text-xs text-muted-foreground'>To</span>
            <div className='min-w-0 flex-1'>
              <DatePicker
                selected={before !== undefined ? new Date(before) : undefined}
                className='w-full text-xs'
                onSelect={(date) =>
                  onRangeChange({
                    rangeKey: 'custom',
                    since,
                    before: date ? date.getTime() : before,
                  })
                }
              />
            </div>
          </div>
        </div>
        {rangeKey === 'all' && (
          <p className='mt-2 text-xs text-muted-foreground'>
            {t(
              'analytics.trendFallbackHint',
              'Tip: without a date window the trend axis falls back to the last 30 days.',
            )}
          </p>
        )}
      </PopoverContent>
    </Popover>
  )
}
export default function AnalyticsPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const [level, setLevel] = useState<AnalyticsLevel>('email')
  const [mode, setMode] = useState<Mode>('overview')
  const [filters, setFilters] = useState<UiFilters>(defaultUiFilters)
  const [axes, setAxes] = useState<AxesState>(() => defaultAxes('email'))
  const [breakdown, setBreakdown] = useState<AnalyticsDimension>(() =>
    overviewDimension('email'),
  )
  const [splitDirection, setSplitDirection] = useState<SplitDrillDirection>('all')
  const [saveOpen, setSaveOpen] = useState(false)
  const [viewName, setViewName] = useState('')

  const { getEmailById } = useMinimalAccountList()

  const patchFilters = (patch: Partial<UiFilters>) =>
    setFilters((prev) => ({ ...prev, ...patch }))

  const switchLevel = (next: AnalyticsLevel) => {
    if (next === level) return
    setLevel(next)
    setAxes(defaultAxes(next))
    setBreakdown(overviewDimension(next))
  }

  const patchAxes = (patch: Partial<AxesState>) =>
    setAxes((prev) => {
      const next = { ...prev, ...patch }
      if (patch.dimension !== undefined && patch.dimension !== prev.dimension) {
        next.metrics = availableMetrics(level, patch.dimension)
      }
      return next
    })

  const request = useMemo(() => {
    if (mode === 'overview') {
      return overviewRequest(level, filters, axes, breakdown)
    }
    if (mode === 'tags') {
      return tagsRequest(filters, axes)
    }
    return buildRequest(level, filters, axes)
  }, [mode, level, filters, axes, breakdown])

  const requestKey = JSON.stringify(request)

  const resultQuery = useQuery<AnalyticsResult>({
    queryKey: ['analytics-query', requestKey],
    queryFn: () => post_analytics_query(request),
    placeholderData: (prev) => prev,
    staleTime: 20_000,
    retry: 1,
  })
  const result = resultQuery.data
  const errorText = resultQuery.error ? serverErrorText(resultQuery.error) : null

  const viewsQuery = useQuery<SavedView[]>({
    queryKey: ['analytics-views'],
    queryFn: list_analytics_views,
    staleTime: 30_000,
  })
  const views = viewsQuery.data ?? []
  const activeViewId = views.find((view) => JSON.stringify(view.query) === requestKey)?.id

  const invalidateViews = () => {
    void queryClient.invalidateQueries({ queryKey: ['analytics-views'] })
  }

  const saveViewMutation = useMutation({
    mutationFn: async (name: string) => {
      const existing = views.find((view) => view.name === name)
      if (existing) return update_analytics_view(existing.id, name, request)
      return create_analytics_view(name, request)
    },
    onSuccess: () => {
      invalidateViews()
      setSaveOpen(false)
      toast({ title: t('analytics.viewSaved', 'View saved') })
    },
    onError: (error) => {
      toast({
        title: t('analytics.saveFailed', 'Could not save view'),
        description: serverErrorText(error),
        variant: 'destructive',
      })
    },
  })

  const deleteViewMutation = useMutation({
    mutationFn: delete_analytics_view,
    onSuccess: () => {
      invalidateViews()
      toast({ title: t('analytics.viewDeleted', 'View deleted') })
    },
    onError: (error) => {
      toast({
        title: t('analytics.deleteFailed', 'Could not delete view'),
        description: serverErrorText(error),
        variant: 'destructive',
      })
    },
  })

  const drillAxes = useMemo<AxesState>(
    () => ({
      timeField: request.time_field,
      dimension: (request.dimension ?? 'none') as DimensionValue,
      bucket: request.bucket ?? 'off',
      metrics: request.metrics,
      top: request.top,
      minDocCount: request.min_doc_count,
      matrixMode: 'heatmap',
    }),
    [request],
  )

  const drill = (row?: AnalyticsRow, periodIndex?: number) => {
    const filter = buildDrillFilter(
      request.level,
      filters,
      drillAxes,
      row,
      periodIndex,
      result?.time?.periods,
      getEmailById,
    )
    const q = Object.keys(filter).length > 0 ? JSON.stringify(filter) : undefined
    const to = request.level === 'email' ? '/search' : '/attachment'
    void navigate({
      to,
      search: (prev: any) => ({ ...prev, page: 1, pageSize: prev?.pageSize ?? 50, q }),
    })
  }

  const handleModeChange = (next: Mode) => {
    setMode(next)
    if (next === 'tags') {
      setLevel('email')
      // Direction split runs all/sent/received internally, so a leftover
      // sent/received filter would be rejected by the server.
      setFilters((prev) => (prev.direction === 'all' ? prev : { ...prev, direction: 'all' }))
      setAxes((prev) =>
        prev.bucket === 'off' || prev.bucket === 'day'
          ? { ...prev, bucket: 'month' }
          : prev,
      )
    }
  }

  const exportResult = () => {
    if (!result) return
    if (result.direction_split) splitResultToCsv(result)
    else resultToCsv(result)
  }

  const drillSplit = (
    row: SplitRow,
    direction: 'all' | 'sent' | 'received',
    periodIndex?: number,
  ) => {
    const filter: Record<string, unknown> = {}
    if (filters.accountIds.length > 0) {
      filter.account_ids = [...filters.accountIds].sort((a, b) => a - b)
    }
    if (filters.text.trim()) filter.text = filters.text.trim()
    filter.tags = [row.key]
    if (direction !== 'all' && filters.accountIds.length === 1) {
      const email = getEmailById(filters.accountIds[0])
      if (email) {
        if (direction === 'sent') filter.from = email
        else filter.any_recipient = email
      }
    }
    const periods = result?.direction_split?.periods
    if (periodIndex !== undefined && periods && periods[periodIndex]) {
      const period = periods[periodIndex]
      filter.internal_date_since = period.start_ms
      filter.internal_date_before = period.end_ms - 1
    }
    const q = Object.keys(filter).length > 0 ? JSON.stringify(filter) : undefined
    void navigate({
      to: '/search',
      search: (prev: any) => ({ ...prev, page: 1, pageSize: prev?.pageSize ?? 50, q }),
    })
  }

  const reset = () => {
    setFilters(defaultUiFilters())
    setBreakdown(overviewDimension(level))
  }

  const loadView = (view: SavedView) => {
    if (view.query.direction_split) {
      setMode('tags')
      setLevel('email')
    } else if (mode === 'tags') {
      // A non-split view cannot render in tag volume mode.
      setMode('explore')
    }
    setLevel(view.query.level)
    setFilters(uiFiltersFromRequest(view.query))
    setAxes(axesFromRequest(view.query))
    setBreakdown(
      (view.query.dimension as AnalyticsDimension | null) ??
      overviewDimension(view.query.level),
    )
  }
  const levelOptions = (
    ['email', 'attachment'] as AnalyticsLevel[]
  ).map((value) => ({
    value,
    label:
      value === 'email'
        ? t('analytics.levelEmail', 'Emails')
        : t('analytics.levelAttachment', 'Attachments'),
  }))
  const directionOptions = (
    ['all', 'received', 'sent'] as AnalyticsDirection[]
  ).map((value) => ({ value, label: directionLabel(value, t) }))

  const isLoadingFirst = resultQuery.isLoading && !result

  return (
    <>
      <FixedHeader />
      <Main>
        <div className='mx-auto w-full max-w-[1400px] px-4 pb-8'>
          {/* Header */}
          <div className='mb-4 flex flex-col gap-3'>
            <div>
              <h1 className='flex items-center gap-2 text-lg font-semibold'>
                <BarChart3 className='h-5 w-5 text-primary' />
                {t('analytics.title', 'Analytics')}
              </h1>
              <p className='mt-1 text-xs text-muted-foreground'>
                {t(
                  'analytics.description',
                  'Explore archived emails and attachments: trends, top groups, size and attachment breakdowns - every cell drills into the search results.',
                )}
              </p>
            </div>
            <div className='flex flex-wrap items-center gap-2'>
              <Segmented<Mode>
                options={[
                  {
                    value: 'overview',
                    label: t('analytics.modeOverview', 'Overview'),
                  },
                  {
                    value: 'tags',
                    label: t('analytics.modeTags', 'Tag volume'),
                  },
                  {
                    value: 'explore',
                    label: t('analytics.modeExplore', 'Explore'),
                  },
                ]}
                value={mode}
                onChange={handleModeChange}
              />
              {mode !== 'tags' && (
                <Segmented<AnalyticsLevel>
                  options={levelOptions}
                  value={level}
                  onChange={switchLevel}
                />
              )}
              <Button
                variant='outline'
                size='sm'
                onClick={() => {
                  setViewName(activeViewId ? views.find((v) => v.id === activeViewId)?.name ?? '' : '')
                  setSaveOpen(true)
                }}
              >
                <BookmarkPlus className='mr-1.5 h-3.5 w-3.5' />
                {t('analytics.saveView', 'Save view')}
              </Button>
              <Popover>
                <PopoverTrigger asChild>
                  <Button variant='outline' size='sm'>
                    <Bookmark className='mr-1.5 h-3.5 w-3.5' />
                    {t('analytics.views', 'Views')}
                    {views.length > 0 && (
                      <Badge variant='secondary' className='ml-1 h-5 px-1.5 text-xs'>
                        {views.length}
                      </Badge>
                    )}
                  </Button>
                </PopoverTrigger>
                <PopoverContent align='end' className='w-72 p-1'>
                  <p className='px-3 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground'>
                    {t('analytics.savedViews', 'Saved views')}
                  </p>
                  <ScrollArea className='max-h-80'>
                    {views.length === 0 ? (
                      <p className='px-3 py-3 text-xs text-muted-foreground'>
                        {t('analytics.noViews', 'No saved views yet')}
                      </p>
                    ) : (
                      views.map((view) => (
                        <div
                          key={view.id}
                          className='group flex items-center gap-1 rounded-md px-2 py-1.5 hover:bg-accent'
                        >
                          <button
                            type='button'
                            onClick={() => loadView(view)}
                            className='flex flex-1 items-center gap-2 truncate text-left text-xs'
                          >
                            {view.id === activeViewId && (
                              <Check className='h-3.5 w-3.5 shrink-0 text-primary' />
                            )}
                            <span className='truncate'>{view.name}</span>
                          </button>
                          <button
                            type='button'
                            title={t('analytics.deleteView', 'Delete view')}
                            onClick={() => deleteViewMutation.mutate(view.id)}
                            className='rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100'
                          >
                            <Trash2 className='h-3.5 w-3.5' />
                          </button>
                        </div>
                      ))
                    )}
                  </ScrollArea>
                </PopoverContent>
              </Popover>
              <Button
                variant='outline'
                size='sm'
                disabled={!result}
                onClick={exportResult}
              >
                <Download className='mr-1.5 h-3.5 w-3.5' />
                {t('analytics.export', 'CSV')}
              </Button>
              <Button variant='outline' size='sm' onClick={reset}>
                <RotateCcw className='mr-1.5 h-3.5 w-3.5' />
                {t('analytics.reset', 'Reset')}
              </Button>
            </div>
          </div>

          {/* Filter bar */}
          <Card className='mb-4'>
            <CardContent className='flex flex-wrap items-center gap-2 py-2.5'>
              <AccountFilterPopover
                selected={filters.accountIds}
                onChange={(ids) => patchFilters({ accountIds: ids })}
              />
              <TagFilterPopover
                level={level}
                selected={filters.tags}
                onChange={(tags) => patchFilters({ tags })}
              />
              {mode !== 'tags' && level === 'email' && (
                <div className='flex items-center gap-2'>
                  <span className='text-xs font-medium text-muted-foreground'>
                    {t('analytics.direction', 'Direction')}
                  </span>
                  <Segmented<AnalyticsDirection>
                    options={directionOptions}
                    value={filters.direction}
                    onChange={(direction) => patchFilters({ direction })}
                  />
                </div>
              )}
              <RangePopover
                level={level}
                timeField={axes.timeField}
                onTimeFieldChange={(timeField) => patchAxes({ timeField })}
                rangeKey={filters.rangeKey}
                since={filters.since}
                before={filters.before}
                onRangeChange={(range) => patchFilters(range)}
              />
              <form
                className='relative ml-auto w-full max-w-xs'
                onSubmit={(event) => {
                  event.preventDefault()
                  patchFilters({ text: filters.textDraft.trim() })
                }}
              >
                <Search className='pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground' />
                <Input
                  value={filters.textDraft}
                  onChange={(event) =>
                    patchFilters({ textDraft: event.target.value })
                  }
                  placeholder={t('analytics.searchText', 'Full-text search…')}
                  className='h-8 pl-8 pr-8 text-xs'
                />
                {filters.textDraft && (
                  <button
                    type='button'
                    onClick={() =>
                      patchFilters({ textDraft: '', text: '' })
                    }
                    className='absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-muted-foreground hover:text-foreground'
                  >
                    <X className='h-3.5 w-3.5' />
                  </button>
                )}
              </form>
            </CardContent>
          </Card>
          {/* Body */}
          {errorText && !result && (
            <div className='mb-4 flex items-start gap-2 rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-xs text-destructive'>
              <AlertCircle className='mt-0.5 h-4 w-4 shrink-0' />
              <div>
                <p className='font-medium'>
                  {t('analytics.queryFailed', 'Query failed')}
                </p>
                <p className='mt-0.5 break-all text-xs opacity-90'>
                  {errorText}
                </p>
              </div>
            </div>
          )}

          {isLoadingFirst && (
            <div className='space-y-4'>
              <div className='grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5'>
                {Array.from({ length: 5 }).map((_, index) => (
                  <Skeleton key={index} className='h-24 rounded-lg' />
                ))}
              </div>
              <Skeleton className='h-72 rounded-lg' />
              <Skeleton className='h-64 rounded-lg' />
            </div>
          )}

          {result && mode === 'overview' && (
            <div className='space-y-4'>
              <AnalyticsKpis result={result} />
              <Card>
                <CardHeader className='pb-2'>
                  <CardTitle className='text-sm'>
                    {t('analytics.volumeTrend', 'Volume over time')}
                  </CardTitle>
                  <CardDescription className='text-xs'>
                    {metricLabel('count', t)}
                    {result.time
                      ? ` · ${timeFieldLabel(result.time.field, t)} · ${bucketLabel(result.time.bucket, t)}`
                      : ''}
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  {result.time && result.time.periods.length > 0 ? (
                    <TrendChart
                      result={result}
                      heightClass='h-60'
                      onDrill={(target) =>
                        drill(undefined, target.periodIndex)
                      }
                    />
                  ) : (
                    <p className='py-16 text-center text-xs text-muted-foreground'>
                      {t('analytics.noTrend', 'No time buckets to plot')}
                    </p>
                  )}
                </CardContent>
              </Card>
              <Card>
                <CardHeader className='space-y-2 pb-2'>
                  <div className='flex items-center justify-between gap-2'>
                    <CardTitle className='text-sm'>
                      {t('analytics.topGroups', 'Top groups')}
                    </CardTitle>
                    <Select
                      value={breakdown}
                      onValueChange={(value) =>
                        setBreakdown(value as AnalyticsDimension)
                      }
                    >
                      <SelectTrigger className='h-8 w-44 text-xs'>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {dimensionsForLevel(level).map((dimension) => (
                          <SelectItem key={dimension} value={dimension}>
                            {dimensionLabel(dimension, t)}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <CardDescription className='text-xs'>
                    {t(
                      'analytics.topGroupsHint',
                      'Click a row to open the matching messages.',
                    )}
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  {result.dimension && result.dimension.rows.length > 0 ? (
                    <RankTable
                      result={result}
                      onRowDrill={(row) => drill(row)}
                    />
                  ) : (
                    <p className='py-14 text-center text-xs text-muted-foreground'>
                      {t('analytics.noRows', 'No groups in this result')}
                    </p>
                  )}
                </CardContent>
              </Card>

              {result.time &&
                result.time.periods.length > 1 &&
                result.dimension &&
                result.dimension.rows.length > 1 && (
                  <Card>
                    <CardHeader className='flex-row items-center justify-between space-y-0 pb-2'>
                      <CardTitle className='text-sm'>
                        {t('analytics.breakdownOverTime', 'Breakdown over time')}
                      </CardTitle>
                      <Segmented<'heatmap' | 'stacked'>
                        options={[
                          {
                            value: 'heatmap',
                            label: t('analytics.heatmap', 'Heatmap'),
                          },
                          {
                            value: 'stacked',
                            label: t('analytics.stacked', 'Stacked'),
                          },
                        ]}
                        value={axes.matrixMode}
                        onChange={(matrixMode) => patchAxes({ matrixMode })}
                      />
                    </CardHeader>
                    <CardContent>
                      <MatrixPanel
                        result={result}
                        mode={axes.matrixMode}
                        onCellDrill={(row, index) => drill(row, index)}
                      />
                    </CardContent>
                  </Card>
                )}
            </div>
          )}
          {result && mode === 'explore' && (
            <div className='space-y-4'>
              {/* Explore axes */}
              <Card>
                <CardContent className='flex flex-wrap items-end gap-x-6 gap-y-3 py-3'>
                  <div>
                    <p className='mb-1.5 text-xs font-medium text-muted-foreground'>
                      {t('analytics.timeAxis', 'Time buckets')}
                    </p>
                    <Segmented<BucketValue>
                      options={[
                        { value: 'off', label: t('analytics.bucketOff', 'None') },
                        ...(['day', 'week', 'month', 'quarter', 'year'] as AnalyticsBucket[]).map(
                          (bucket) => ({
                            value: bucket,
                            label: bucketLabel(bucket, t),
                          }),
                        ),
                      ]}
                      value={axes.bucket}
                      onChange={(bucket) => patchAxes({ bucket })}
                    />
                  </div>
                  <div>
                    <p className='mb-1.5 text-xs font-medium text-muted-foreground'>
                      {t('analytics.groupBy', 'Group by')}
                    </p>
                    <Select
                      value={axes.dimension}
                      onValueChange={(value) =>
                        patchAxes({ dimension: value as DimensionValue })
                      }
                    >
                      <SelectTrigger className='h-8 w-48 text-xs'>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value='none'>
                          {t('analytics.noGroup', 'No grouping')}
                        </SelectItem>
                        {dimensionsForLevel(level).map((dimension) => (
                          <SelectItem key={dimension} value={dimension}>
                            {dimensionLabel(dimension, t)}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <div>
                    <p className='mb-1.5 text-xs font-medium text-muted-foreground'>
                      {t('analytics.metrics', 'Metrics')}
                    </p>
                    <Popover>
                      <PopoverTrigger asChild>
                        <Button size='sm' variant='outline' className='gap-1.5'>
                          <SlidersHorizontal className='h-3.5 w-3.5 text-muted-foreground' />
                          {axes.metrics.length}
                          <ChevronDown className='h-3.5 w-3.5 opacity-60' />
                        </Button>
                      </PopoverTrigger>
                      <PopoverContent align='start' className='w-64 p-2'>
                        <div className='max-h-80 overflow-y-auto'>
                          {availableMetrics(level, axes.dimension).map(
                            (metric) => {
                              const checked =
                                metric === 'count' || axes.metrics.includes(metric)
                              return (
                                <label
                                  key={metric}
                                  className='flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-xs hover:bg-accent'
                                >
                                  <Checkbox
                                    checked={checked}
                                    disabled={metric === 'count'}
                                    onCheckedChange={() => {
                                      if (metric === 'count') return
                                      const next = axes.metrics.includes(metric)
                                        ? axes.metrics.filter(
                                          (item) => item !== metric,
                                        )
                                        : [...axes.metrics, metric]
                                      patchAxes({
                                        metrics:
                                          next.length > 0 ? next : ['count'],
                                      })
                                    }}
                                  />
                                  <span className='truncate'>
                                    {metricLabel(metric, t)}
                                  </span>
                                </label>
                              )
                            },
                          )}
                        </div>
                      </PopoverContent>
                    </Popover>
                  </div>
                  <div>
                    <p className='mb-1.5 text-xs font-medium text-muted-foreground'>
                      {t('analytics.topN', 'Top rows')}
                    </p>
                    <Select
                      value={String(axes.top)}
                      onValueChange={(value) =>
                        patchAxes({ top: Number(value) })
                      }
                    >
                      <SelectTrigger className='h-8 w-24 text-xs'>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {[10, 20, 50, 100, 200].map((value) => (
                          <SelectItem key={value} value={String(value)}>
                            {value}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <div>
                    <p className='mb-1.5 text-xs font-medium text-muted-foreground'>
                      {t('analytics.minDocCount', 'Min documents')}
                    </p>
                    <Select
                      value={String(axes.minDocCount)}
                      onValueChange={(value) =>
                        patchAxes({ minDocCount: Number(value) })
                      }
                    >
                      <SelectTrigger className='h-8 w-20 text-xs'>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {[1, 2, 5, 10, 20].map((value) => (
                          <SelectItem key={value} value={String(value)}>
                            {value}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                </CardContent>
              </Card>

              <AnalyticsKpis result={result} />

              {(() => {
                const hasTime = (result.time?.periods.length ?? 0) > 0
                const hasDim = !!result.dimension
                if (!hasTime && !hasDim) {
                  return (
                    <Card>
                      <CardContent className='py-16 text-center'>
                        <SlidersHorizontal className='mx-auto mb-2 h-8 w-8 text-muted-foreground/50' />
                        <p className='text-xs text-muted-foreground'>
                          {t(
                            'analytics.exploreHint',
                            'Turn on a time bucket or a group-by to visualize the data. Totals above are computed for the whole scope.',
                          )}
                        </p>
                      </CardContent>
                    </Card>
                  )
                }
                if (hasDim && !hasTime) {
                  return (
                    <Card>
                      <CardHeader className='pb-2'>
                        <CardTitle className='text-sm'>
                          {dimensionLabel(result.dimension!.by, t)}
                        </CardTitle>
                      </CardHeader>
                      <CardContent>
                        <RankTable
                          result={result}
                          onRowDrill={(row) => drill(row)}
                        />
                      </CardContent>
                    </Card>
                  )
                }
                if (hasTime && !hasDim) {
                  return (
                    <Card>
                      <CardHeader className='pb-2'>
                        <CardTitle className='text-sm'>
                          {t('analytics.trend', 'Trend')}
                        </CardTitle>
                      </CardHeader>
                      <CardContent>
                        <TrendChart
                          result={result}
                          onDrill={(target) =>
                            drill(undefined, target.periodIndex)
                          }
                        />
                      </CardContent>
                    </Card>
                  )
                }
                return (
                  <Card>
                    <CardHeader className='flex-row items-center justify-between space-y-0 pb-2'>
                      <CardTitle className='text-sm'>
                        {dimensionLabel(result.dimension!.by, t)}{' '}
                        {t('analytics.byTime', 'over time')}
                      </CardTitle>
                      <Segmented<'heatmap' | 'stacked'>
                        options={[
                          {
                            value: 'heatmap',
                            label: t('analytics.heatmap', 'Heatmap'),
                          },
                          {
                            value: 'stacked',
                            label: t('analytics.stacked', 'Stacked'),
                          },
                        ]}
                        value={axes.matrixMode}
                        onChange={(matrixMode) => patchAxes({ matrixMode })}
                      />
                    </CardHeader>
                    <CardContent>
                      <MatrixPanel
                        result={result}
                        mode={axes.matrixMode}
                        onCellDrill={(row, index) => drill(row, index)}
                      />
                    </CardContent>
                  </Card>
                )
              })()}
            </div>
          )}
          {result && mode === 'tags' && result.direction_split && (
            <div className='space-y-4'>
              <Card>
                <CardHeader className='space-y-1 pb-2'>
                  <CardTitle className='text-sm'>
                    {t('analytics.tagVolume', 'Tag volume')}
                  </CardTitle>
                  <CardDescription className='text-xs'>
                    {t(
                      'analytics.tagVolumeHint',
                      'Rows are tags, columns are time buckets. Pick a direction to see one number per cell.',
                    )}
                  </CardDescription>
                </CardHeader>
                <CardContent className='flex flex-wrap items-end gap-x-6 gap-y-3'>
                  <div>
                    <p className='mb-1.5 text-xs font-medium text-muted-foreground'>
                      {t('analytics.timeAxis', 'Time buckets')}
                    </p>
                    <Segmented<AnalyticsBucket>
                      options={(
                        ['week', 'month', 'quarter', 'year'] as AnalyticsBucket[]
                      ).map((bucket) => ({
                        value: bucket,
                        label: bucketLabel(bucket, t),
                      }))}
                      value={
                        axes.bucket === 'off' || axes.bucket === 'day'
                          ? 'month'
                          : axes.bucket
                      }
                      onChange={(bucket) => patchAxes({ bucket })}
                    />
                  </div>
                  <div>
                    <p className='mb-1.5 text-xs font-medium text-muted-foreground'>
                      {t('analytics.direction', 'Direction')}
                    </p>
                    <Segmented<SplitDrillDirection>
                      options={(
                        ['all', 'sent', 'received'] as SplitDrillDirection[]
                      ).map((value) => ({
                        value,
                        label: directionLabel(value, t),
                      }))}
                      value={splitDirection}
                      onChange={setSplitDirection}
                    />
                  </div>
                  <p className='ml-auto text-xs text-muted-foreground'>
                    {t('analytics.tagVolumeScope', 'Scope: {{count}} account(s)', {
                      count: filters.accountIds.length,
                    })}
                  </p>
                </CardContent>
              </Card>
              <Card>
                <CardContent className='pt-4'>
                  {result.direction_split.rows.length > 0 ? (
                    <DirectionSplitTable
                      result={result}
                      direction={splitDirection}
                      onCellDrill={(row, direction, periodIndex) =>
                        drillSplit(row, direction, periodIndex)
                      }
                    />
                  ) : (
                    <p className='py-14 text-center text-xs text-muted-foreground'>
                      {t('analytics.noRows', 'No tags in this result')}
                    </p>
                  )}
                </CardContent>
              </Card>
            </div>
          )}

          {/* Save view dialog */}
          <Dialog open={saveOpen} onOpenChange={setSaveOpen}>
            <DialogContent className='sm:max-w-md'>
              <DialogHeader>
                <DialogTitle>
                  {t('analytics.saveViewTitle', 'Save analytics view')}
                </DialogTitle>
                <DialogDescription>
                  {t(
                    'analytics.saveViewDescription',
                    'Stores the current level, filters, time axis and grouping. You can restore it any time from the Views menu.',
                  )}
                </DialogDescription>
              </DialogHeader>
              <form
                onSubmit={(event) => {
                  event.preventDefault()
                  const name = viewName.trim()
                  if (name) saveViewMutation.mutate(name)
                }}
              >
                <Input
                  autoFocus
                  value={viewName}
                  onChange={(event) => setViewName(event.target.value)}
                  placeholder={t('analytics.viewNamePlaceholder', 'e.g. Q3 senders')}
                  className='mb-4'
                />
                <DialogFooter>
                  <Button
                    type='button'
                    variant='ghost'
                    onClick={() => setSaveOpen(false)}
                  >
                    {t('common.cancel', 'Cancel')}
                  </Button>
                  <Button
                    type='submit'
                    disabled={!viewName.trim() || saveViewMutation.isPending}
                  >
                    {saveViewMutation.isPending && (
                      <Loader2 className='mr-2 h-4 w-4 animate-spin' />
                    )}
                    {t('analytics.saveView', 'Save view')}
                  </Button>
                </DialogFooter>
              </form>
            </DialogContent>
          </Dialog>
        </div>
      </Main>
    </>
  )
}
