// Analytics API client - unified explore query + saved views.
//
// NOTE: multi-valued filters are JSON arrays. Comma separated strings are
// intentionally not supported anymore.

import axiosInstance from '@/api/axiosInstance'

export type AnalyticsLevel = 'email' | 'attachment'
export type AnalyticsTimeField = 'date' | 'internal_date' | 'ingest_at'
export type AnalyticsBucket = 'day' | 'week' | 'month' | 'quarter' | 'year'
export type AnalyticsDirection = 'all' | 'received' | 'sent'
export type AnalyticsDimension =
  | 'account'
  | 'mailbox'
  | 'sender'
  | 'tag'
  | 'attachment_ext'
  | 'attachment_category'
  | 'attachment_content_type'
  | 'size_range'
  | 'attachment_count_range'
  | 'attachment_size_range'
  | 'page_count_range'
  | 'thread'
  | 'content_hash'
export type AnalyticsMetric =
  | 'count'
  | 'size_sum'
  | 'size_avg'
  | 'size_min'
  | 'size_max'
  | 'size_p50'
  | 'size_p95'
  | 'size_p99'
  | 'attachment_sum'
  | 'attachment_avg'
  | 'attachment_max'
  | 'with_attachment'
  | 'page_avg'
  | 'page_max'
  | 'unique_senders'
  | 'unique_threads'
  | 'unique_hashes'
  | 'text_count'
  | 'ocr_count'

export interface AnalyticsFilters {
  account_ids?: number[]
  mailbox_ids?: number[]
  tags?: string[]
  direction?: AnalyticsDirection
  since?: number
  before?: number
  internal_date_since?: number
  internal_date_before?: number
  ingest_since?: number
  ingest_before?: number
  text?: string
  subject?: string
  body?: string
  from?: string
  to?: string
  cc?: string
  bcc?: string
  any_recipient?: string
  any_participant?: string
  min_size?: number
  max_size?: number
  has_attachment?: boolean
  attachment_name?: string
  attachment_extension?: string
  attachment_category?: string
  attachment_content_type?: string
  message_id?: string
  content_hash?: string
  is_ocr?: boolean
  is_message?: boolean
  has_text?: boolean
  min_page_count?: number
  max_page_count?: number
}

export interface AnalyticsQuery {
  level: AnalyticsLevel
  filters: AnalyticsFilters
  time_field: AnalyticsTimeField
  bucket?: AnalyticsBucket | null
  dimension?: AnalyticsDimension | null
  metrics: AnalyticsMetric[]
  top: number
  min_doc_count: number
  /**
   * Email level only, requires dimension + bucket: every row and period is
   * counted separately for all / sent / received.
   */
  direction_split?: boolean
}

export interface AnalyticsPeriod {
  start_ms: number
  end_ms: number
}

export interface SeriesPoint {
  count: number
  metrics: Record<string, number>
}

export interface AnalyticsRow {
  key: string
  label?: string | null
  count: number
  metrics: Record<string, number>
  series: SeriesPoint[]
}

export interface AnalyticsDimensionBlock {
  by: AnalyticsDimension
  rows: AnalyticsRow[]
  other_count: number
}

export interface AnalyticsTimeBlock {
  field: AnalyticsTimeField
  bucket: AnalyticsBucket
  periods: AnalyticsPeriod[]
  defaulted: boolean
}

export interface SplitCounts {
  all: number
  sent: number
  received: number
}

export interface SplitRow {
  key: string
  label?: string | null
  totals: SplitCounts
  series: SplitCounts[]
}

export interface DirectionSplitBlock {
  bucket: AnalyticsBucket
  periods: AnalyticsPeriod[]
  rows: SplitRow[]
  totals: SplitCounts
  series_totals: SplitCounts[]
  other_count: number
}

export interface AnalyticsResult {
  level: AnalyticsLevel
  time: AnalyticsTimeBlock | null
  dimension: AnalyticsDimensionBlock | null
  totals: {
    count: number
    metrics: Record<string, number>
    series: SeriesPoint[]
  }
  applied_metrics: AnalyticsMetric[]
  direction_split?: DirectionSplitBlock | null
}

export interface SavedView {
  id: string
  user_id: number
  name: string
  query: AnalyticsQuery
  created_at_ms: number
  updated_at_ms: number
}

export async function post_analytics_query(
  query: AnalyticsQuery,
): Promise<AnalyticsResult> {
  const { data } = await axiosInstance.post<AnalyticsResult>(
    'api/v1/analytics/query',
    query,
  )
  return data
}

export async function list_analytics_views(): Promise<SavedView[]> {
  const { data } = await axiosInstance.get<{ items: SavedView[] }>(
    'api/v1/analytics/views',
  )
  return data.items
}

export async function create_analytics_view(
  name: string,
  query: AnalyticsQuery,
): Promise<SavedView> {
  const { data } = await axiosInstance.post<SavedView>(
    'api/v1/analytics/views',
    { name, query },
  )
  return data
}

export async function update_analytics_view(
  id: string,
  name: string,
  query: AnalyticsQuery,
): Promise<SavedView> {
  const { data } = await axiosInstance.put<SavedView>(
    `api/v1/analytics/views/${id}`,
    { name, query },
  )
  return data
}

export async function delete_analytics_view(id: string): Promise<void> {
  await axiosInstance.delete(`api/v1/analytics/views/${id}`)
}
