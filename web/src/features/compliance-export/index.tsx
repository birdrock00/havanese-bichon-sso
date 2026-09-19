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
//
// Compliance export page (Pro edition). Browse finished batch exports and
// their compliance verification results: artifact SHA-256, per-message
// content-hash cross-check, mismatch drill-down and re-verification.
import React from 'react'
import { AxiosError } from 'axios'
import {
  CheckCircle2,
  Clock,
  Loader2,
  RefreshCw,
  ShieldCheck,
  X,
  XCircle,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { TFunction } from 'i18next'
import {
  getExportVerifyProgress,
  listExports,
  startExportVerify,
  type ExportJobView,
  type ExportVerifyProgressView,
  type ExportVerifyView,
} from '@/api/export/api'
import { toast } from '@/hooks/use-toast'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Drawer, DrawerContent, DrawerTitle } from '@/components/ui/drawer'
import { Progress } from '@/components/ui/progress'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Separator } from '@/components/ui/separator'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { FixedHeader } from '@/components/layout/fixed-header'
import { Main } from '@/components/layout/main'
import { TableSkeleton } from '@/components/table-skeleton'

const VERIFY_POLL_INTERVAL_MS = 1000

const getErrorMessage = (error: unknown) => {
  if (error instanceof AxiosError) {
    return (
      (error.response?.data as { message?: string } | undefined)?.message ||
      error.message
    )
  }
  return error instanceof Error ? error.message : String(error)
}

const formatTime = (ts: number) => {
  const d = new Date(ts)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

const shortHash = (hash: string) =>
  hash.length > 16 ? `${hash.slice(0, 16)}…` : hash

const verifyReasonLabel = (
  t: TFunction,
  reason: string
) => {
  switch (reason) {
    case 'blob_missing':
      return t('export_tasks.verifyReasonBlobMissing', 'Message data missing in archive')
    case 'attachment_missing':
      return t('export_tasks.verifyReasonAttachmentMissing', 'Attachments missing in archive')
    case 'content_changed':
      return t('export_tasks.verifyReasonContentChanged', 'Message content changed since export')
    default:
      return reason
  }
}

export default function ComplianceExportPage() {
  const { t } = useTranslation()

  const [jobs, setJobs] = React.useState<ExportJobView[]>([])
  const [loading, setLoading] = React.useState(true)
  const [refreshing, setRefreshing] = React.useState(false)
  const [filter, setFilter] = React.useState('all')

  const [selected, setSelected] = React.useState<ExportJobView | null>(null)
  const [detailsProgress, setDetailsProgress] =
    React.useState<ExportVerifyProgressView | null>(null)
  const [detailsResult, setDetailsResult] =
    React.useState<ExportVerifyView | null>(null)
  const [detailsError, setDetailsError] = React.useState<string | null>(null)

  const [verifyJobId, setVerifyJobId] = React.useState<string | null>(null)
  const [verifyProgress, setVerifyProgress] =
    React.useState<ExportVerifyProgressView | null>(null)
  const [verifyError, setVerifyError] = React.useState<string | null>(null)
  const verifyPollRef = React.useRef<ReturnType<typeof setInterval> | null>(
    null
  )

  const stopVerifyPolling = () => {
    if (verifyPollRef.current) {
      clearInterval(verifyPollRef.current)
      verifyPollRef.current = null
    }
  }

  const pollVerify = React.useCallback(
    (jobId: string, onDone?: () => void) => {
      stopVerifyPolling()
      verifyPollRef.current = setInterval(() => {
        getExportVerifyProgress(jobId)
          .then((progress) => {
            setVerifyProgress(progress)
            if (progress.status === 'finished' || progress.status === 'failed') {
              stopVerifyPolling()
              setVerifyJobId(null)
              setVerifyProgress(null)
              onDone?.()
            }
          })
          .catch((error) => {
            stopVerifyPolling()
            setVerifyJobId(null)
            setVerifyProgress(null)
            setVerifyError(getErrorMessage(error))
          })
      }, VERIFY_POLL_INTERVAL_MS)
    },
    []
  )

  const refresh = React.useCallback(
    async (silent = false) => {
      if (silent) {
        setRefreshing(true)
      } else {
        setLoading(true)
      }
      try {
        const data = await listExports()
        setJobs(data)
        const running = data.find(
          (job) =>
            job.status === 'finished' && job.verify_status === 'running'
        )
        if (running) {
          setVerifyJobId(running.job_id)
          getExportVerifyProgress(running.job_id)
            .then((progress) => {
              setVerifyProgress(progress)
              if (progress.status === 'running') {
                pollVerify(running.job_id)
              }
            })
            .catch(() => {})
        } else {
          stopVerifyPolling()
          setVerifyJobId(null)
          setVerifyProgress(null)
        }
      } catch (error) {
        toast({
          title: t('compliance_export.loadFailed', 'Failed to load compliance records'),
          description: getErrorMessage(error),
          variant: 'destructive',
        })
      } finally {
        setLoading(false)
        setRefreshing(false)
      }
    },
    [pollVerify, t]
  )

  React.useEffect(() => {
    refresh()
    return stopVerifyPolling
  }, [refresh])

  const handleVerify = (job: ExportJobView) => {
    setVerifyJobId(job.job_id)
    setVerifyProgress(null)
    setVerifyError(null)
    startExportVerify(job.job_id)
      .then((progress) => {
        setVerifyProgress(progress)
        if (progress.status === 'running') {
          pollVerify(job.job_id, () => refresh(true))
        } else {
          setVerifyJobId(null)
          setVerifyProgress(null)
          refresh(true)
        }
      })
      .catch((error) => {
        setVerifyError(getErrorMessage(error))
        toast({
          title: t('compliance_export.verifyStartFailed', 'Failed to start verification'),
          description: getErrorMessage(error),
          variant: 'destructive',
        })
      })
  }

  const openDetails = (job: ExportJobView) => {
    setSelected(job)
    setDetailsProgress(null)
    setDetailsResult(null)
    setDetailsError(null)
  }

  const closeDetails = () => {
    setSelected(null)
    setDetailsProgress(null)
    setDetailsResult(null)
    setDetailsError(null)
  }

  React.useEffect(() => {
    if (!selected) return
    let disposed = false
    const load = () => {
      getExportVerifyProgress(selected.job_id)
        .then((progress) => {
          if (disposed) return
          setDetailsProgress(progress)
          setDetailsResult(progress.result)
        })
        .catch((error) => {
          if (disposed) return
          setDetailsError(getErrorMessage(error))
        })
    }
    load()
    const timer = setInterval(load, VERIFY_POLL_INTERVAL_MS)
    return () => {
      disposed = true
      clearInterval(timer)
    }
  }, [selected])

  const statusBadge = (job: ExportJobView) => {
    switch (job.verify_status) {
      case 'finished':
        return (
          <Badge variant='secondary'>
            <CheckCircle2 className='mr-1 h-3 w-3 text-emerald-600' />
            {t('compliance_export.statusVerified', 'Verified')}
          </Badge>
        )
      case 'running':
        return (
          <Badge variant='default'>
            <Loader2 className='mr-1 h-3 w-3 animate-spin' />
            {t('compliance_export.statusRunning', 'Running')}
          </Badge>
        )
      case 'failed':
        return (
          <Badge variant='destructive'>
            <XCircle className='mr-1 h-3 w-3' />
            {t('compliance_export.statusFailed', 'Failed')}
          </Badge>
        )
      default:
        return (
          <Badge variant='outline'>
            {t('compliance_export.statusUnverified', 'Unverified')}
          </Badge>
        )
    }
  }

  const matchesFilter = (job: ExportJobView) => {
    switch (filter) {
      case 'verified':
        return job.verify_status === 'finished'
      case 'unverified':
        return job.verify_status === 'idle' || job.verify_status === 'running'
      case 'failed':
        return job.verify_status === 'failed'
      default:
        return true
    }
  }

  const filtered = jobs.filter(matchesFilter)

  const counts = React.useMemo(
    () => ({
      verified: jobs.filter((j) => j.verify_status === 'finished').length,
      unverified: jobs.filter(
        (j) => j.verify_status === 'idle' || j.verify_status === 'running'
      ).length,
      failed: jobs.filter((j) => j.verify_status === 'failed').length,
    }),
    [jobs]
  )

  const runningJob = verifyJobId
    ? jobs.find((j) => j.job_id === verifyJobId)
    : undefined

  return (
    <>
      <FixedHeader />
      <Main>
        <div className='mx-auto w-full max-w-7xl px-4'>
          <div className='mb-1 flex items-start justify-between gap-4'>
            <div>
              <h1 className='text-lg font-semibold'>
                {t('compliance_export.title', 'Compliance Export')}
              </h1>
              <p className='mt-0.5 text-sm text-muted-foreground'>
                {t('compliance_export.description', 'Review verification results of finished batch exports.')}
              </p>
            </div>
            <Button
              variant='outline'
              size='sm'
              onClick={() => refresh(true)}
              disabled={refreshing}
            >
              {refreshing ? (
                <Loader2 className='mr-2 h-4 w-4 animate-spin' />
              ) : (
                <RefreshCw className='mr-2 h-4 w-4' />
              )}
              {t('compliance_export.refresh', 'Refresh')}
            </Button>
          </div>
          <Separator className='mt-2 mb-4 lg:mt-3 lg:mb-6' />

          <div className='mb-4 flex flex-wrap items-center gap-2'>
            <Select value={filter} onValueChange={(v) => setFilter(v)}>
              <SelectTrigger className='w-[200px]'>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value='all'>
                  {t('compliance_export.filterAll', 'All')} ({jobs.length})
                </SelectItem>
                <SelectItem value='verified'>
                  {t('compliance_export.filterVerified', 'Verified')} (
                  {counts.verified})
                </SelectItem>
                <SelectItem value='unverified'>
                  {t('compliance_export.filterUnverified', 'Unverified')} (
                  {counts.unverified})
                </SelectItem>
                <SelectItem value='failed'>
                  {t('compliance_export.filterFailed', 'Failed')} (
                  {counts.failed})
                </SelectItem>
              </SelectContent>
            </Select>
            {runningJob && verifyProgress?.status === 'running' && (
              <div className='flex items-center gap-2 rounded-md border bg-accent/40 px-3 py-1.5 text-xs text-muted-foreground'>
                <Loader2 className='h-3.5 w-3.5 animate-spin' />
                <span>
                  {t('export_tasks.verifyProgress', {
                    checked: verifyProgress.checked,
                    total: verifyProgress.total,
                  })}
                </span>
                <span className='font-medium text-foreground'>
                  {runningJob.saved_search_name || runningJob.job_id}
                </span>
              </div>
            )}
            {verifyError && (
              <div className='text-xs text-destructive'>{verifyError}</div>
            )}
          </div>

          {loading ? (
            <TableSkeleton columns={7} rows={5} showPagination={false} />
          ) : (
            <div className='overflow-x-auto rounded-md border'>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>
                      {t('compliance_export.exportName', 'Export name')}
                    </TableHead>
                    <TableHead>
                      {t('compliance_export.exportedAt', 'Exported at')}
                    </TableHead>
                    <TableHead>
                      {t('compliance_export.sha256', 'SHA-256')}
                    </TableHead>
                    <TableHead>
                      {t('compliance_export.status', 'Status')}
                    </TableHead>
                    <TableHead>
                      {t('compliance_export.verifiedAt', 'Verified at')}
                    </TableHead>
                    <TableHead className='text-right'>
                      {t('compliance_export.checked', 'Checked')} /{' '}
                      {t('compliance_export.matched', 'Matched')} /{' '}
                      {t('compliance_export.mismatched', 'Mismatched')}
                    </TableHead>
                    <TableHead className='text-right'>
                      {t('compliance_export.actions', 'Actions')}
                    </TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filtered.map((job) => (
                    <TableRow key={job.job_id}>
                      <TableCell>
                        <div className='max-w-[220px] truncate font-medium'>
                          {job.saved_search_name || job.job_id}
                        </div>
                        <div className='max-w-[220px] truncate font-mono text-xs text-muted-foreground'>
                          {job.job_id}
                        </div>
                      </TableCell>
                      <TableCell className='whitespace-nowrap text-xs text-muted-foreground'>
                        {formatTime(job.created_at)}
                      </TableCell>
                      <TableCell>
                        {job.artifact_hash ? (
                          <span
                            className='block max-w-[160px] truncate font-mono text-xs'
                            title={job.artifact_hash}
                          >
                            {shortHash(job.artifact_hash)}
                          </span>
                        ) : (
                          <span className='text-muted-foreground'>-</span>
                        )}
                      </TableCell>
                      <TableCell>{statusBadge(job)}</TableCell>
                      <TableCell className='whitespace-nowrap text-xs text-muted-foreground'>
                        {job.verify_finished_at
                          ? formatTime(job.verify_finished_at)
                          : '-'}
                      </TableCell>
                      <TableCell className='text-right text-xs'>
                        {job.verify_checked > 0 || job.verify_mismatched > 0 ? (
                          <span className='font-mono'>
                            {job.verify_checked} / {job.verify_matched} /{' '}
                            <span
                              className={
                                job.verify_mismatched > 0
                                  ? 'text-destructive'
                                  : 'text-muted-foreground'
                              }
                            >
                              {job.verify_mismatched}
                            </span>
                          </span>
                        ) : (
                          <span className='text-muted-foreground'>-</span>
                        )}
                      </TableCell>
                      <TableCell className='text-right'>
                        <div className='flex items-center justify-end gap-1'>
                          {job.verify_status === 'running' ? (
                            <Button variant='outline' size='sm' disabled>
                              <Loader2 className='mr-1 h-3 w-3 animate-spin' />
                              {t('compliance_export.statusRunning', 'Running')}
                            </Button>
                          ) : (
                            <Button
                              variant='outline'
                              size='sm'
                              onClick={() => handleVerify(job)}
                              disabled={job.status !== 'finished'}
                            >
                              {job.verify_status === 'idle'
                                ? t('compliance_export.verifyNow', 'Verify now')
                                : t('compliance_export.verifyAgain', 'Re-verify')}
                            </Button>
                          )}
                          <Button
                            variant='ghost'
                            size='sm'
                            onClick={() => openDetails(job)}
                          >
                            {t('compliance_export.viewDetails', 'Details')}
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                  {filtered.length === 0 && (
                    <TableRow>
                      <TableCell
                        colSpan={7}
                        className='py-8 text-center text-muted-foreground'
                      >
                        {t('compliance_export.noRecords', 'No compliance records found.')}
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>
            </div>
          )}
        </div>
      </Main>

      <Drawer
        open={selected !== null}
        onOpenChange={(open) => {
          if (!open) closeDetails()
        }}
        direction='right'
      >
        <DrawerContent side='right'>
          <DrawerTitle className='sr-only'>
            {t('compliance_export.details', 'Verification details')}
          </DrawerTitle>
          <div className='flex h-full flex-col overflow-hidden p-3'>
            <div className='flex items-center justify-between'>
              <h2 className='text-sm font-semibold'>
                {t('compliance_export.details', 'Verification details')}
              </h2>
              <Button
                variant='ghost'
                size='sm'
                onClick={closeDetails}
                title={t('compliance_export.close', 'Close')}
              >
                <X className='h-4 w-4' />
              </Button>
            </div>
            <Separator className='my-3' />
            <ScrollArea className='flex-1'>
              {selected && (
                <div className='space-y-4 pr-3'>
                  <div>
                    <div className='text-sm font-medium'>
                      {selected.saved_search_name || selected.job_id}
                    </div>
                    <div className='font-mono text-xs text-muted-foreground'>
                      {selected.job_id}
                    </div>
                  </div>
                  <div className='flex items-center gap-2 text-sm'>
                    {detailsProgress?.status === 'idle' ? (
                      <Clock className='h-4 w-4 text-muted-foreground' />
                    ) : detailsResult?.artifact_hash_match ? (
                      <ShieldCheck className='h-4 w-4 text-emerald-600' />
                    ) : detailsResult ? (
                      <XCircle className='h-4 w-4 text-destructive' />
                    ) : (
                      <Loader2 className='h-4 w-4 animate-spin' />
                    )}
                    <span className='font-medium'>
                      {t('compliance_export.artifactHash', 'Artifact SHA-256')}
                    </span>
                    <span>
                      {detailsProgress?.status === 'idle' ? (
                        t('compliance_export.statusUnverified', 'Unverified')
                      ) : detailsResult ? (
                        detailsResult.artifact_hash_match
                          ? t('compliance_export.hashMatch', 'Match')
                          : t('compliance_export.hashMismatch', 'Mismatch')
                      ) : detailsProgress?.status === 'failed' ? (
                        t('compliance_export.statusFailed', 'Failed')
                      ) : (
                        t('compliance_export.statusRunning', 'Running')
                      )}
                    </span>
                  </div>
                  {detailsError && (
                    <div className='text-sm text-destructive'>{detailsError}</div>
                  )}
                  {detailsResult ? (
                    <>
                      <div className='flex flex-wrap gap-x-6 gap-y-2 text-sm'>
                        <div>
                          <span className='text-muted-foreground'>
                            {t('compliance_export.checked', 'Checked')}:
                          </span>{' '}
                          {detailsResult.checked}
                        </div>
                        <div>
                          <span className='text-muted-foreground'>
                            {t('compliance_export.matched', 'Matched')}:
                          </span>{' '}
                          {detailsResult.matched}
                        </div>
                        <div>
                          <span className='text-muted-foreground'>
                            {t('compliance_export.mismatched', 'Mismatched')}:
                          </span>{' '}
                          {detailsResult.mismatched}
                        </div>
                      </div>
                      {detailsResult.actual_artifact_hash && (
                        <div className='space-y-1'>
                          <div className='break-all font-mono text-xs'>
                            <span className='text-muted-foreground'>
                              {t('compliance_export.expectedHash', 'Expected')}:
                            </span>{' '}
                            {detailsResult.expected_artifact_hash || '-'}
                          </div>
                          <div className='break-all font-mono text-xs'>
                            <span className='text-muted-foreground'>
                              {t('compliance_export.actualHash', 'Actual')}:
                            </span>{' '}
                            {detailsResult.actual_artifact_hash}
                          </div>
                        </div>
                      )}
                      {detailsResult.mismatches.length === 0 &&
                        detailsResult.artifact_hash_match && (
                          <div className='flex items-center gap-2 text-sm text-emerald-600'>
                            <CheckCircle2 className='h-4 w-4' />
                            {t('compliance_export.verifyPassed', 'Verification passed')}
                          </div>
                        )}
                      {detailsResult.mismatches.length > 0 && (
                        <div>
                          <div className='mb-2 text-sm font-medium'>
                            {t('compliance_export.mismatches', {
                              defaultValue: 'Mismatches ({{count}})',
                              count: detailsResult.mismatches.length,
                            })}
                          </div>
                          <div className='overflow-hidden rounded-md border'>
                            <Table>
                              <TableHeader>
                                <TableRow>
                                  <TableHead>
                                    {t('compliance_export.subject', 'Subject')}
                                  </TableHead>
                                  <TableHead>
                                    {t('compliance_export.reason', 'Reason')}
                                  </TableHead>
                                </TableRow>
                              </TableHeader>
                              <TableBody>
                                {detailsResult.mismatches.map((m) => (
                                  <TableRow key={m.envelope_id}>
                                    <TableCell>
                                      <div className='max-w-[220px] truncate text-xs font-medium'>
                                        {m.subject || '-'}
                                      </div>
                                      <div className='max-w-[220px] truncate font-mono text-xs text-muted-foreground'>
                                        {m.envelope_id}
                                      </div>
                                    </TableCell>
                                    <TableCell className='text-xs text-destructive'>
                                      {verifyReasonLabel(t, m.reason)}
                                    </TableCell>
                                  </TableRow>
                                ))}
                              </TableBody>
                            </Table>
                          </div>
                        </div>
                      )}
                    </>
                  ) : detailsProgress?.status === 'running' ? (
                    <div className='space-y-2'>
                      <Progress
                        value={
                          detailsProgress.total > 0
                            ? Math.min(
                                100,
                                Math.round(
                                  (detailsProgress.checked /
                                    detailsProgress.total) *
                                    100
                                )
                              )
                            : 0
                        }
                        className='h-2'
                      />
                      <div className='text-xs text-muted-foreground'>
                        {t('export_tasks.verifyProgress', {
                          checked: detailsProgress.checked,
                          total: detailsProgress.total,
                        })}
                      </div>
                    </div>
                  ) : detailsProgress?.status === 'idle' ? (
                    <div className='space-y-3'>
                      <div className='flex items-center gap-2 text-sm text-muted-foreground'>
                        <Clock className='h-4 w-4' />
                        {t(
                          'compliance_export.notVerified',
                          'This export has not been verified yet.'
                        )}
                      </div>
                      <Button
                        size='sm'
                        onClick={() => handleVerify(selected)}
                      >
                        <ShieldCheck className='mr-1 h-3 w-3' />
                        {t('compliance_export.verifyNow', 'Verify now')}
                      </Button>
                    </div>
                  ) : (
                    <div className='text-sm text-destructive'>
                      {detailsProgress?.error ||
                        detailsError ||
                        t('compliance_export.verifyError', 'Verification failed')}
                    </div>
                  )}
                </div>
              )}
            </ScrollArea>
          </div>
        </DrawerContent>
      </Drawer>
    </>
  )
}
