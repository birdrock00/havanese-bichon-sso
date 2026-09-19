// Formatting / CSV / drill-down helpers for the analytics workbench.

import { format } from 'date-fns'
import {
  type AnalyticsBucket,
  type AnalyticsDirection,
  type AnalyticsDimension,
  type AnalyticsLevel,
  type AnalyticsMetric,
  type AnalyticsResult,
  type AnalyticsRow,
  type AnalyticsTimeField,
} from '@/api/analytics/api'
import { useTranslation } from 'react-i18next'

export const DAY_MS = 86_400_000

export function formatNumber(n: number | undefined | null): string {
  if (n === undefined || n === null || !Number.isFinite(n)) return '-'
  if (Math.abs(n) >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(2)}B`
  if (Math.abs(n) >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (Math.abs(n) >= 10_000) return `${(n / 1000).toFixed(1)}K`
  return new Intl.NumberFormat().format(n)
}

export function formatInteger(n: number | undefined | null): string {
  if (n === undefined || n === null || !Number.isFinite(n)) return '-'
  return new Intl.NumberFormat().format(Math.round(n))
}

export function formatBytes(n: number | undefined | null): string {
  if (n === undefined || n === null || !Number.isFinite(n)) return '-'
  if (n === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.min(
    units.length - 1,
    Math.floor(Math.log(n) / Math.log(1024)),
  )
  const v = n / 1024 ** i
  return `${v >= 100 ? v.toFixed(0) : v.toFixed(1)} ${units[i]}`
}

/** Labels for grouping dimensions (row.label fallback is resolved server-side). */
export function rowDisplayLabel(row: AnalyticsRow): string {
  if (row.label) return row.label
  return row.key.replace(/^\/+/, '')
}

export function displayTag(tag: string): string {
  return tag.replace(/^\/+/, '')
}

export function periodLabel(startMs: number, bucket: AnalyticsBucket): string {
  const date = new Date(startMs)
  switch (bucket) {
    case 'day':
      return format(date, 'MM-dd')
    case 'week':
      return format(date, 'yyyy-MM-dd')
    case 'month':
      return format(date, 'yyyy-MM')
    case 'quarter':
      return `${date.getFullYear()}-Q${Math.floor(date.getMonth() / 3) + 1}`
    case 'year':
      return `${date.getFullYear()}`
  }
}

export function dimensionLabel(
  dimension: AnalyticsDimension | null,
  t: (key: string, defaultValue: string) => string,
): string {
  switch (dimension) {
    case 'account':
      return t('analytics.dimensionAccount', 'Account')
    case 'mailbox':
      return t('analytics.dimensionFolder', 'Folder')
    case 'sender':
      return t('analytics.dimensionSender', 'Sender')
    case 'tag':
      return t('analytics.dimensionTag', 'Tag')
    case 'attachment_ext':
      return t('analytics.dimensionAttachmentType', 'Attachment type (ext)')
    case 'attachment_category':
      return t('analytics.dimensionAttachmentCategory', 'Attachment category')
    case 'attachment_content_type':
      return t('analytics.dimensionMimeType', 'MIME type')
    case 'size_range':
      return t('analytics.dimensionEmailSize', 'Email size')
    case 'attachment_count_range':
      return t('analytics.dimensionAttachmentsPerEmail', 'Attachments per email')
    case 'attachment_size_range':
      return t('analytics.dimensionAttachmentSize', 'Attachment size')
    case 'page_count_range':
      return t('analytics.dimensionPdfPages', 'Pages (PDF)')
    default:
      return ''
  }
}

export function metricLabel(
  metric: AnalyticsMetric,
  t: (key: string, defaultValue: string) => string,
): string {
  switch (metric) {
    case 'count':
      return t('analytics.metricMessages', 'Messages')
    case 'size_sum':
      return t('analytics.metricTotalSize', 'Total size')
    case 'size_avg':
      return t('analytics.metricAvgSize', 'Avg size')
    case 'size_min':
      return t('analytics.metricMinSize', 'Min size')
    case 'size_max':
      return t('analytics.metricMaxSize', 'Max size')
    case 'size_p50':
      return t('analytics.metricMedianSize', 'Median size')
    case 'size_p95':
      return t('analytics.metricP95Size', 'P95 size')
    case 'size_p99':
      return t('analytics.metricP99Size', 'P99 size')
    case 'attachment_sum':
      return t('analytics.metricAttachments', 'Attachments')
    case 'attachment_avg':
      return t('analytics.metricAvgAttachments', 'Avg attachments')
    case 'attachment_max':
      return t('analytics.metricMaxAttachments', 'Max attachments')
    case 'with_attachment':
      return t('analytics.metricWithAttachments', 'With attachments')
    case 'page_avg':
      return t('analytics.metricAvgPages', 'Avg pages')
    case 'page_max':
      return t('analytics.metricMaxPages', 'Max pages')
    case 'unique_senders':
      return t('analytics.metricUniqueSenders', 'Unique senders')
    case 'unique_threads':
      return t('analytics.metricUniqueConversations', 'Unique conversations')
    case 'unique_hashes':
      return t('analytics.metricUniqueContent', 'Unique content')
    case 'text_count':
      return t('analytics.metricTextExtracted', 'Text extracted')
    case 'ocr_count':
      return t('analytics.metricOcrExtracted', 'OCR extracted')
  }
}

export function formatMetricValue(metric: AnalyticsMetric, v: number): string {
  switch (metric) {
    case 'count':
    case 'attachment_sum':
    case 'attachment_avg':
    case 'attachment_max':
    case 'with_attachment':
    case 'page_avg':
    case 'page_max':
    case 'unique_senders':
    case 'unique_threads':
    case 'unique_hashes':
    case 'text_count':
    case 'ocr_count':
      return formatNumber(v)
    case 'size_sum':
    case 'size_avg':
    case 'size_min':
    case 'size_max':
    case 'size_p50':
    case 'size_p95':
    case 'size_p99':
      return formatBytes(v)
  }
}

export function suggestBucket(since?: number, before?: number): AnalyticsBucket {
  const days = Math.max(
    1,
    Math.round(((before ?? Date.now()) - (since ?? Date.now())) / DAY_MS),
  )
  if (days <= 45) return 'day'
  if (days <= 400) return 'month'
  if (days <= 1500) return 'quarter'
  return 'year'
}

/** Metrics available for a level (used by the UI pickers). */
export function metricsForLevel(level: AnalyticsLevel): AnalyticsMetric[] {
  const email: AnalyticsMetric[] = [
    'count',
    'size_sum',
    'size_avg',
    'size_max',
    'size_p95',
    'size_p99',
    'attachment_sum',
    'attachment_avg',
    'with_attachment',
    'unique_senders',
    'unique_threads',
    'unique_hashes',
  ]
  const attachment: AnalyticsMetric[] = [
    'count',
    'size_sum',
    'size_avg',
    'size_max',
    'size_p95',
    'size_p99',
    'page_avg',
    'page_max',
    'unique_hashes',
    'text_count',
    'ocr_count',
  ]
  return level === 'email' ? email : attachment
}

/** Dimensions per level (order controls the picker). */
export function dimensionsForLevel(level: AnalyticsLevel): AnalyticsDimension[] {
  if (level === 'email') {
    return [
      'account',
      'mailbox',
      'sender',
      'tag',
      'attachment_ext',
      'attachment_category',
      'size_range',
      'attachment_count_range',
    ]
  }
  return [
    'account',
    'mailbox',
    'sender',
    'tag',
    'attachment_ext',
    'attachment_category',
    'attachment_size_range',
    'page_count_range',
  ]
}

/** Mirrors the server rules: dimensions only support stats-family metrics. */
export function metricsForDimension(
  level: AnalyticsLevel,
  dimension: AnalyticsDimension | null,
  _time: boolean,
): AnalyticsMetric[] {
  if (!dimension) return metricsForLevel(level)
  if (dimension === 'tag') return ['count']
  const stats: AnalyticsMetric[] = ['count', 'size_sum', 'size_avg', 'size_max']
  if (level === 'email') {
    return [...stats, 'attachment_sum', 'attachment_avg']
  }
  return [...stats, 'page_avg', 'page_max']
}

export function timeFieldsForLevel(level: AnalyticsLevel): AnalyticsTimeField[] {
  return level === 'email' ? ['date', 'internal_date', 'ingest_at'] : ['date', 'ingest_at']
}

export function timeFieldLabel(field: AnalyticsTimeField, t: (key: string, defaultValue: string) => string): string {
  switch (field) {
    case 'date':
      return t('analytics.timeFieldMessageDate', 'Message date')
    case 'internal_date':
      return t('analytics.timeFieldServerReceive', 'Server receive')
    case 'ingest_at':
      return t('analytics.timeFieldArchivedAt', 'Archived at')
  }
}

export function directionLabel(direction: AnalyticsDirection, t: (key: string, defaultValue: string) => string): string {
  switch (direction) {
    case 'all':
      return t('analytics.directionAll', 'All')
    case 'received':
      return t('analytics.directionReceived', 'Received')
    case 'sent':
      return t('analytics.directionSent', 'Sent')
  }
}

// ---------------------------------------------------------------------------
// CSV export
// ---------------------------------------------------------------------------

function escapeCsv(value: string | number): string {
  const s = String(value)
  return `"${s.replace(/"/g, '""')}"`
}

export function resultToCsv(result: AnalyticsResult): void {
  const { t } = useTranslation();

  const lines: string[] = []
  if (result.time && result.time.periods.length > 0) {
    if (result.dimension) {
      // Heatmap/stacked: rows x periods.
      const headers = [
        dimensionLabel(result.dimension.by, t),
        ...result.time.periods.map((p) => periodLabel(p.start_ms, result.time!.bucket)),
        'Total',
      ]
      lines.push(headers.map(escapeCsv).join(','))
      for (const row of result.dimension.rows) {
        const seriesSum = row.series.reduce((acc, s) => acc + s.count, 0)
        lines.push(
          [rowDisplayLabel(row), ...row.series.map((s) => s.count), seriesSum]
            .map(escapeCsv)
            .join(','),
        )
      }
      lines.push(
        ['Total', ...result.totals.series.map((s) => s.count), result.totals.count]
          .map(escapeCsv)
          .join(','),
      )
    } else {
      const headers = [
        'Period',
        'Count',
        ...result.applied_metrics
          .filter((m) => m !== 'count')
          .map((m) => metricLabel(m, t)),
      ]
      lines.push(headers.map(escapeCsv).join(','))
      result.totals.series.forEach((point, i) => {
        const period = result.time!.periods[i]
        if (!period) return
        const values = [
          periodLabel(period.start_ms, result.time!.bucket),
          point.count,
          ...result.applied_metrics
            .filter((m) => m !== 'count')
            .map((m) => point.metrics[m] ?? 0),
        ]
        lines.push(values.map(escapeCsv).join(','))
      })
    }
  } else if (result.dimension) {
    const headers = [
      dimensionLabel(result.dimension.by, t),
      'Count',
      ...result.applied_metrics
        .filter((m) => m !== 'count')
        .map((m) => metricLabel(m, t)),
    ]
    lines.push(headers.map(escapeCsv).join(','))
    for (const row of result.dimension.rows) {
      lines.push(
        [
          rowDisplayLabel(row),
          row.count,
          ...result.applied_metrics
            .filter((m) => m !== 'count')
            .map((m) => row.metrics[m] ?? 0),
        ]
          .map(escapeCsv)
          .join(','),
      )
    }
    if (result.dimension.other_count > 0) {
      lines.push(
        ['Others', result.dimension.other_count, ...Array(Math.max(0, headers.length - 2)).fill('')]
          .map(escapeCsv)
          .join(','),
      )
    }
    lines.push(['Total', result.totals.count].map(escapeCsv).join(','))
  } else {
    const headers = [
      'Count',
      ...result.applied_metrics
        .filter((m) => m !== 'count')
        .map((m) => metricLabel(m, t)),
    ]
    lines.push(headers.map(escapeCsv).join(','))
    lines.push(
      [
        result.totals.count,
        ...result.applied_metrics
          .filter((m) => m !== 'count')
          .map((m) => result.totals.metrics[m] ?? 0),
      ]
        .map(escapeCsv)
        .join(','),
    )
  }
  const blob = new Blob([`\uFEFF${lines.join('\n')}`], {
    type: 'text/csv;charset=utf-8',
  })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = 'analytics.csv'
  link.click()
  URL.revokeObjectURL(url)
}

// ---------------------------------------------------------------------------
// Range mapping (drill-down uses exact min/max sizes)
// ---------------------------------------------------------------------------

export const EMAIL_SIZE_RANGE_BOUNDS: Record<string, { min?: number; max?: number }> = {
  '0-10KB': { max: 10_000 },
  '10KB-100KB': { min: 10_000, max: 100_000 },
  '100KB-1MB': { min: 100_000, max: 1_000_000 },
  '1MB-5MB': { min: 1_000_000, max: 5_000_000 },
  '5MB-20MB': { min: 5_000_000, max: 20_000_000 },
  '20MB+': { min: 20_000_000 },
}

export const ATTACHMENT_SIZE_RANGE_BOUNDS: Record<string, { min?: number; max?: number }> = {
  '0-100KB': { max: 100_000 },
  '100KB-1MB': { min: 100_000, max: 1_000_000 },
  '1MB-10MB': { min: 1_000_000, max: 10_000_000 },
  '10MB-50MB': { min: 10_000_000, max: 50_000_000 },
  '50MB+': { min: 50_000_000 },
}

export function attachmentRangeBounds(key: string): { min?: number; max?: number } {
  if (key.includes('page')) {
    return { min: Number(key.split('-')[0]) || undefined }
  }
  if (key === '11+' || key === '6-10' || key === '3-5' || key === '2' || key === '1' || key === '0') {
    // attachment count buckets handled via has_attachment/min? They map onto
    // the regular_attachment_count field which the search UI has no direct
    // filter for; keep drill-down to the whole scope instead.
    return {}
  }
  return ATTACHMENT_SIZE_RANGE_BOUNDS[key] ?? EMAIL_SIZE_RANGE_BOUNDS[key] ?? {}
}
// ---------------------------------------------------------------------------
// Range -> exact numeric bounds used when drilling into the search page
// ---------------------------------------------------------------------------

export const PAGE_RANGE_BOUNDS: Record<string, { min?: number; max?: number }> = {
  '1-2': { min: 1, max: 2 },
  '3-9': { min: 3, max: 9 },
  '10-49': { min: 10, max: 49 },
  '50+': { min: 50 },
}

/** Maps an analytics range-bucket key to exact search-filter bounds, if any. */
export function dimensionRangeBounds(
  dimension: AnalyticsDimension | 'none' | null,
  key: string,
): { min?: number; max?: number } {
  if (dimension === 'size_range') return EMAIL_SIZE_RANGE_BOUNDS[key] ?? {}
  if (dimension === 'attachment_size_range') return ATTACHMENT_SIZE_RANGE_BOUNDS[key] ?? {}
  if (dimension === 'page_count_range') return PAGE_RANGE_BOUNDS[key] ?? {}
  return {}
}

/**
 * CSV export for a direction-split (tag volume) result: one row per
 * tag x period, with all / sent / received columns.
 */
export function splitResultToCsv(result: AnalyticsResult): void {
  const split = result.direction_split
  if (!split) return
  const lines: string[] = []
  const headers = ['Tag', 'Period', 'All', 'Sent', 'Received']
  lines.push(headers.map(escapeCsv).join(','))
  for (const row of split.rows) {
    split.periods.forEach((period, i) => {
      const cell = row.series[i] ?? { all: 0, sent: 0, received: 0 }
      lines.push(
        [
          displayTag(row.key),
          periodLabel(period.start_ms, split.bucket),
          cell.all,
          cell.sent,
          cell.received,
        ]
          .map(escapeCsv)
          .join(','),
      )
    })
    lines.push(
      [displayTag(row.key), 'Total', row.totals.all, row.totals.sent, row.totals.received]
        .map(escapeCsv)
        .join(','),
    )
  }
  if (split.other_count > 0) {
    lines.push([`Others (${split.other_count})`, '', '', '', ''].map(escapeCsv).join(','))
  }
  split.periods.forEach((period, i) => {
    const cell = split.series_totals[i] ?? { all: 0, sent: 0, received: 0 }
    lines.push(
      ['Total', periodLabel(period.start_ms, split.bucket), cell.all, cell.sent, cell.received]
        .map(escapeCsv)
        .join(','),
    )
  })
  lines.push(
    ['Total', 'Total', split.totals.all, split.totals.sent, split.totals.received]
      .map(escapeCsv)
      .join(','),
  )
  const blob = new Blob([`\uFEFF${lines.join('\n')}`], {
    type: 'text/csv;charset=utf-8',
  })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = 'analytics.csv'
  link.click()
  URL.revokeObjectURL(url)
}