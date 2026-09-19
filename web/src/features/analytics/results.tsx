// Result renderers for the analytics workbench.
//
// All of them operate on the uniform AnalyticsResult returned by
// POST /api/v1/analytics/query.

import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Legend,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Tooltip as UiTooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import {
  type AnalyticsMetric,
  type AnalyticsResult,
  type AnalyticsRow,
  type SplitRow,
} from '@/api/analytics/api'
import {
  displayTag,
  directionLabel,
  formatInteger,
  formatMetricValue,
  metricLabel,
  periodLabel,
  rowDisplayLabel,
} from './format'

const PALETTE = [
  '#6366f1',
  '#10b981',
  '#f59e0b',
  '#ef4444',
  '#06b6d4',
  '#a855f7',
  '#84cc16',
  '#ec4899',
  '#64748b',
  '#f97316',
  '#14b8a6',
  '#8b5cf6',
]

export interface DrillTarget {
  row?: AnalyticsRow
  periodIndex?: number
}

function KpiCard({
  title,
  value,
  sub,
}: {
  title: string
  value: string
  sub?: string
}) {
  return (
    <div className='flex flex-col justify-between rounded-lg border bg-card p-4 shadow-sm'>
      <div className='text-xs font-medium uppercase tracking-wide text-muted-foreground'>
        {title}
      </div>
      <div className='mt-2 text-xl font-semibold tabular-nums'>{value}</div>
      {sub ? <div className='mt-1 text-xs text-muted-foreground'>{sub}</div> : null}
    </div>
  )
}

export function AnalyticsKpis({ result }: { result: AnalyticsResult }) {
  const { t } = useTranslation()
  const countMetric: AnalyticsMetric = 'count'
  const primary = result.applied_metrics.find((m) => m !== 'count')
  const cards = [
    <KpiCard
      key='count'
      title={t('analytics.count', '{{label}}', { label: metricLabel('count', t) })}
      value={formatInteger(result.totals.count)}
    />,
  ]
  if (primary) {
    cards.push(
      <KpiCard
        key={primary}
        title={metricLabel(primary, t)}
        value={formatMetricValue(primary, result.totals.metrics[primary] ?? 0)}
      />,
    )
  }
  // Additional applied metrics (non-count) as small cards.
  for (const m of result.applied_metrics) {
    if (m === countMetric || m === primary) continue
    cards.push(
      <KpiCard
        key={m}
        title={metricLabel(m, t)}
        value={formatMetricValue(m, result.totals.metrics[m] ?? 0)}
      />,
    )
  }
  return <div className='grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5'>{cards}</div>
}

export function TrendChart({
  result,
  metric = 'count',
  onDrill,
  heightClass = 'h-72',
}: {
  result: AnalyticsResult
  metric?: AnalyticsMetric
  onDrill?: (target: DrillTarget) => void
  /** Tailwind height class for the chart wrapper. */
  heightClass?: string
}) {
  const { t } = useTranslation()
  const points = result.totals.series
  const data = useMemo(
    () =>
      points.map((point, i) => {
        const period = result.time?.periods[i]
        return {
          label: period
            ? periodLabel(period.start_ms, result.time!.bucket)
            : String(i),
          count: point.count,
          metric: metric === 'count' ? point.count : point.metrics[metric] ?? 0,
        }
      }),
    [points, metric, result.time],
  )
  const manyPoints = points.length > 40
  return (
    <div className={`${heightClass} w-full`}>
      <ResponsiveContainer width='100%' height='100%'>
        <AreaChart
          data={data}
          margin={{ top: 8, right: 8, bottom: 0, left: 0 }}
          onClick={(state: any) => {
            if (onDrill && state?.activeTooltipIndex !== undefined) {
              onDrill({ periodIndex: state.activeTooltipIndex })
            }
          }}
        >
          <defs>
            <linearGradient id='analyticsTrend' x1='0' y1='0' x2='0' y2='1'>
              <stop offset='5%' stopColor='#6366f1' stopOpacity={0.35} />
              <stop offset='95%' stopColor='#6366f1' stopOpacity={0.02} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray='3 3' className='stroke-muted' vertical={false} />
          <XAxis
            dataKey='label'
            tick={{ fontSize: 10 }}
            interval={manyPoints ? 'preserveStartEnd' : 0}
            tickMargin={8}
          />
          <YAxis
            tick={{ fontSize: 10 }}
            width={56}
            tickFormatter={(v: number) => formatInteger(v)}
          />
          <Tooltip
            formatter={(value: number | string, name: string) => [
              name === 'metric'
                ? formatMetricValue(metric, Number(value))
                : formatInteger(Number(value)),
              metric === 'count' ? t('analytics.metricCount', 'Count') : metricLabel(metric, t),
            ]}
          />
          <Area
            type='monotone'
            dataKey='metric'
            stroke='#6366f1'
            strokeWidth={2}
            fill='url(#analyticsTrend)'
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  )
}

export function RankTable({
  result,
  onRowDrill,
}: {
  result: AnalyticsResult
  onRowDrill?: (row: AnalyticsRow) => void
}) {
  const { t } = useTranslation()
  const rows = result.dimension?.rows ?? []
  const other = result.dimension?.other_count ?? 0
  const metricCols = result.applied_metrics.filter((m) => m !== 'count')
  const max = Math.max(1, ...rows.map((r) => r.count))
  return (
    <ScrollArea orientation='horizontal' className='w-full rounded-md'>
      <table className='w-full text-xs' style={{ minWidth: 'max-content' }}>
        <thead>
          <tr className='border-b text-left text-muted-foreground'>
            <th className='px-2 py-2 font-medium'>#</th>
            <th className='px-2 py-2 font-medium'>{t('analytics.group', 'Group')}</th>
            <th className='px-2 py-2 text-right font-medium'>
              {t('analytics.metricCount', 'Count')}
            </th>
            {metricCols.map((m) => (
              <th key={m} className='px-2 py-2 text-right font-medium'>
                {metricLabel(m, t)}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr
              key={`${row.key}-${i}`}
              onClick={() => onRowDrill?.(row)}
              className={
                onRowDrill
                  ? 'cursor-pointer border-b hover:bg-accent hover:text-accent-foreground'
                  : 'border-b'
              }
            >
              <td className='px-2 py-1.5 text-muted-foreground'>{i + 1}</td>
              <td className='max-w-[18rem] truncate px-2 py-1.5'>
                <div className='truncate font-medium' title={rowDisplayLabel(row)}>
                  {rowDisplayLabel(row)}
                </div>
                <div className='mt-0.5 h-1 w-32 overflow-hidden rounded-full bg-muted'>
                  <div
                    className='h-full rounded-full bg-primary/70'
                    style={{ width: `${(row.count / max) * 100}%` }}
                  />
                </div>
              </td>
              <td className='px-2 py-1.5 text-right tabular-nums'>
                {formatInteger(row.count)}
              </td>
              {metricCols.map((m) => (
                <td key={m} className='px-2 py-1.5 text-right tabular-nums'>
                  {formatMetricValue(m, row.metrics[m] ?? 0)}
                </td>
              ))}
            </tr>
          ))}
          {other > 0 ? (
            <tr className='border-b text-muted-foreground'>
              <td className='px-2 py-1.5'>…</td>
              <td className='px-2 py-1.5 italic'>
                {t('analytics.others', 'Others')}
              </td>
              <td className='px-2 py-1.5 text-right tabular-nums'>
                {formatInteger(other)}
              </td>
              {metricCols.map((m) => (
                <td key={m} className='px-2 py-1.5' />
              ))}
            </tr>
          ) : null}
          <tr className='font-semibold'>
            <td className='px-2 py-2' colSpan={2}>
              {t('analytics.total', 'Total')}
            </td>
            <td className='px-2 py-2 text-right tabular-nums'>
              {formatInteger(result.totals.count)}
            </td>
            {metricCols.map((m) => (
              <td key={m} className='px-2 py-2 text-right tabular-nums'>
                {formatMetricValue(m, result.totals.metrics[m] ?? 0)}
              </td>
            ))}
          </tr>
        </tbody>
      </table>
    </ScrollArea>
  )
}

/** Row x period matrix: stacked bars or a heatmap table. */
export function MatrixPanel({
  result,
  mode = 'heatmap',
  onCellDrill,
}: {
  result: AnalyticsResult
  mode?: 'stacked' | 'heatmap'
  onCellDrill?: (row: AnalyticsRow, periodIndex: number) => void
}) {
  const { t } = useTranslation()
  const periods = result.time?.periods ?? []
  const rows = (result.dimension?.rows ?? []).slice(0, 24)
  const columnMax = useMemo(() => {
    return periods.map((_, i) =>
      Math.max(1, ...rows.map((r) => r.series[i]?.count ?? 0)),
    )
  }, [periods, rows])

  if (mode === 'stacked') {
    const visible = rows.slice(0, 10)
    const data = periods.map((period, i) => {
      const point: Record<string, number | string> = {
        label: periodLabel(period.start_ms, result.time!.bucket),
      }
      visible.forEach((row, rIdx) => {
        point[`s${rIdx}`] = row.series[i]?.count ?? 0
      })
      return point
    })
    return (
      <div className='h-80 w-full'>
        <ResponsiveContainer width='100%' height='100%'>
          <BarChart data={data} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
            <CartesianGrid strokeDasharray='3 3' className='stroke-muted' vertical={false} />
            <XAxis dataKey='label' tick={{ fontSize: 10 }} tickMargin={6} />
            <YAxis tick={{ fontSize: 10 }} width={50} tickFormatter={(v: number) => formatInteger(v)} />
            <Tooltip />
            <Legend wrapperStyle={{ fontSize: 10 }} />
            {visible.map((row, rIdx) => (
              <Bar
                key={row.key}
                dataKey={`s${rIdx}`}
                stackId='a'
                fill={PALETTE[rIdx % PALETTE.length]}
                name={rowDisplayLabel(row)}
              />
            ))}
          </BarChart>
        </ResponsiveContainer>
      </div>
    )
  }

  return (
    <ScrollArea orientation='horizontal' className='w-full rounded-md'>
      <table className='w-full border-collapse text-xs' style={{ minWidth: 'max-content' }}>
        <thead>
          <tr className='border-b text-muted-foreground'>
            <th className='sticky left-0 bg-background px-3 py-2 text-left font-medium'>
              {t('analytics.group', 'Group')}
            </th>
            {periods.map((period, i) => (
              <th key={period.start_ms} className='px-2 py-2 text-right font-medium'>
                {periodLabel(period.start_ms, result.time!.bucket)}
                {i === periods.length - 1 ? '' : ''}
              </th>
            ))}
            <th className='sticky right-0 bg-background px-3 py-2 text-right font-medium'>
              {t('analytics.total', 'Total')}
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row, rIdx) => (
            <tr key={`${row.key}-${rIdx}`} className='border-b'>
              <td className='sticky left-0 max-w-[16rem] truncate bg-background px-3 py-1.5 font-medium'>
                {rowDisplayLabel(row)}
              </td>
              {periods.map((_, i) => {
                const count = row.series[i]?.count ?? 0
                const alpha =
                  count > 0 ? Math.min(0.38, (count / columnMax[i]) * 0.38) : 0
                return (
                  <td
                    key={i}
                    className='cursor-pointer px-2 py-1.5 text-right tabular-nums hover:bg-accent hover:text-accent-foreground'
                    style={
                      alpha > 0
                        ? { backgroundColor: `hsla(var(--primary) / ${alpha})` }
                        : undefined
                    }
                    title={`${rowDisplayLabel(row)} · ${count}`}
                    onClick={() => onCellDrill?.(row, i)}
                  >
                    {count > 0 ? formatInteger(count) : ''}
                  </td>
                )
              })}
              <td className='sticky right-0 bg-background px-3 py-1.5 text-right font-medium tabular-nums'>
                {formatInteger(row.count)}
              </td>
            </tr>
          ))}
          <tr className='font-semibold'>
            <td className='sticky left-0 bg-background px-3 py-2'>
              {t('analytics.total', 'Total')}
            </td>
            {result.totals.series.map((point, i) => (
              <td key={i} className='px-2 py-2 text-right tabular-nums'>
                {formatInteger(point.count)}
              </td>
            ))}
            <td className='sticky right-0 bg-background px-3 py-2 text-right tabular-nums'>
              {formatInteger(result.totals.count)}
            </td>
          </tr>
        </tbody>
      </table>
    </ScrollArea>
  )
}
export type SplitDrillDirection = 'all' | 'sent' | 'received'

/** Row (tag) x period matrix showing a single direction at a time. */
export function DirectionSplitTable({
  result,
  direction,
  onCellDrill,
}: {
  result: AnalyticsResult
  direction: SplitDrillDirection
  onCellDrill?: (
    row: SplitRow,
    direction: SplitDrillDirection,
    periodIndex?: number,
  ) => void
}) {
  const { t } = useTranslation()
  const split = result.direction_split
  if (!split) return null
  const periods = split.periods
  const rows = split.rows

  const cellButton = (key: string, value: number, row: SplitRow, periodIndex?: number) => (
    <td key={key} className='px-2 py-1.5 text-right tabular-nums'>
      {value > 0 ? (
        <button
          type='button'
          className='rounded px-1 hover:bg-accent hover:text-accent-foreground'
          title={`${displayTag(row.key)} · ${periodIndex !== undefined
            ? periodLabel(periods[periodIndex].start_ms, split.bucket)
            : t('analytics.total', 'Total')
            } · ${directionLabel(direction, t)}`}
          onClick={() => onCellDrill?.(row, direction, periodIndex)}
        >
          {formatInteger(value)}
        </button>
      ) : (
        <span className='text-muted-foreground/40'>{formatInteger(value)}</span>
      )}
    </td>
  )

  return (
    <TooltipProvider delayDuration={200}>
      <div>
        <ScrollArea orientation='horizontal' className='w-full rounded-md'>
          <table
            className='w-full border-collapse text-xs'
            style={{ minWidth: 'max-content' }}
          >
            <thead>
              <tr className='border-b text-muted-foreground'>
                <th className='sticky left-0 bg-background px-3 py-2 text-left font-medium'>
                  {t('analytics.tag', 'Tag')}
                </th>
                {periods.map((period) => (
                  <th
                    key={`p-${period.start_ms}`}
                    className='px-2 py-2 text-right font-medium'
                  >
                    {periodLabel(period.start_ms, split.bucket)}
                  </th>
                ))}
                <th className='px-2 py-2 text-right font-medium'>
                  {t('analytics.total', 'Total')}
                </th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.key} className='border-b'>
                  <td className='sticky left-0 max-w-[14rem] truncate bg-background px-3 py-1.5 font-medium'>
                    {row.label ?? displayTag(row.key)}
                  </td>
                  {periods.map((_, i) =>
                    cellButton(
                      `c-${row.key}-${i}`,
                      row.series[i]?.[direction] ?? 0,
                      row,
                      i,
                    ),
                  )}
                  {cellButton(`t-${row.key}`, row.totals[direction], row)}
                </tr>
              ))}
            </tbody>
            <tfoot>
              <tr className='border-t font-semibold'>
                <td className='sticky left-0 bg-background px-3 py-2'>
                  {t('analytics.scopeTotal', 'All emails in scope')}
                  <UiTooltip>
                    <TooltipTrigger asChild>
                      <sup className='ml-1 cursor-help text-primary'>*</sup>
                    </TooltipTrigger>
                    <TooltipContent className='max-w-xs whitespace-normal'>
                      {t(
                        'analytics.scopeTotalNote',
                        'All counts every email in this filter and time range. Sent only matches emails whose From is the account; Received only matches emails whose To/Cc contains the account. Emails sent to yourself (or to yourself and others), received via Bcc, or with no resolvable direction are only in All or on both sides - so All is not simply Sent + Received.',
                      )}
                    </TooltipContent>
                  </UiTooltip>
                </td>
                {periods.map((_, i) => (
                  <td key={`ct-${i}`} className='px-2 py-2 text-right tabular-nums'>
                    {formatInteger(split.series_totals[i]?.[direction] ?? 0)}
                  </td>
                ))}
                <td className='px-2 py-2 text-right tabular-nums'>
                  {formatInteger(split.totals[direction])}
                </td>
              </tr>
            </tfoot>
          </table>
        </ScrollArea>

        {split.other_count > 0 && (
          <p className='mt-1 text-xs text-muted-foreground'>
            {t('analytics.otherTags', '+{{count}} more tags', { count: split.other_count })}
          </p>
        )}
      </div>
    </TooltipProvider>
  )
}