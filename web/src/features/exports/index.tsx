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

import React from 'react'
import { useTranslation } from 'react-i18next'
import { Download, Loader2, RefreshCw, Trash2, XCircle } from 'lucide-react'
import { AxiosError } from 'axios'
import { Main } from '@/components/layout/main'
import { FixedHeader } from '@/components/layout/fixed-header'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Progress } from '@/components/ui/progress'
import { Separator } from '@/components/ui/separator'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { toast } from '@/hooks/use-toast'
import {
  cancelExport,
  deleteExport,
  downloadExport,
  listExports,
  type ExportJobView,
} from '@/api/export/api'

const POLL_INTERVAL_MS = 3000

const ACTIVE_STATUSES = ['pending', 'running']

const getErrorMessage = (error: unknown) => {
  if (error instanceof AxiosError) {
    return (
      (error.response?.data as { message?: string } | undefined)?.message ||
      error.message
    )
  }
  return error instanceof Error ? error.message : String(error)
}

const formatBytes = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`
}

const formatTime = (ts: number) => {
  const d = new Date(ts)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}
export default function ExportTasksPage() {
  const { t } = useTranslation()
  const [jobs, setJobs] = React.useState<ExportJobView[]>([])
  const [loading, setLoading] = React.useState(true)
  const [refreshing, setRefreshing] = React.useState(false)
  const [cancelTarget, setCancelTarget] = React.useState<ExportJobView | null>(
    null
  )
  const [deleteTarget, setDeleteTarget] = React.useState<ExportJobView | null>(
    null
  )
  const pollRef = React.useRef<ReturnType<typeof setInterval> | null>(null)

  const stopPolling = () => {
    if (pollRef.current) {
      clearInterval(pollRef.current)
      pollRef.current = null
    }
  }

  const refresh = React.useCallback(
    async (silent = false) => {
      if (!silent) {
        setLoading(true)
      } else {
        setRefreshing(true)
      }
      try {
        const data = await listExports()
        setJobs(data)
        const hasActive = data.some((job) =>
          ACTIVE_STATUSES.includes(job.status)
        )
        if (hasActive && !pollRef.current) {
          pollRef.current = setInterval(
            () => refresh(true),
            POLL_INTERVAL_MS
          )
        } else if (!hasActive) {
          stopPolling()
        }
      } catch (error) {
        toast({
          title: t('export_tasks.loadFailed'),
          description: getErrorMessage(error),
          variant: 'destructive',
        })
      } finally {
        setLoading(false)
        setRefreshing(false)
      }
    },
    [t]
  )

  React.useEffect(() => {
    refresh()
    return stopPolling
  }, [refresh])
  const handleDownload = (job: ExportJobView) => {
    downloadExport(job.job_id)
      .then((blob) => {
        const url = URL.createObjectURL(blob)
        const link = document.createElement('a')
        link.href = url
        link.download = job.artifact_name ?? `${job.job_id}.mbox`
        document.body.appendChild(link)
        link.click()
        document.body.removeChild(link)
        URL.revokeObjectURL(url)
        toast({ title: t('export_tasks.downloadSaved') })
      })
      .catch((error) => {
        toast({
          title: t('export_tasks.downloadFailed'),
          description: getErrorMessage(error),
          variant: 'destructive',
        })
      })
  }

  const handleCancel = () => {
    if (!cancelTarget) return
    cancelExport(cancelTarget.job_id)
      .then(() => {
        setCancelTarget(null)
        toast({ title: t('export_tasks.cancelStarted') })
        refresh(true)
      })
      .catch((error) => {
        toast({
          title: t('export_tasks.cancelFailed'),
          description: getErrorMessage(error),
          variant: 'destructive',
        })
      })
  }

  const handleDelete = () => {
    if (!deleteTarget) return
    deleteExport(deleteTarget.job_id)
      .then(() => {
        setDeleteTarget(null)
        toast({ title: t('export_tasks.deleted') })
        refresh(true)
      })
      .catch((error) => {
        toast({
          title: t('export_tasks.deleteFailed'),
          description: getErrorMessage(error),
          variant: 'destructive',
        })
      })
  }

  const statusInfo = (
    status: string
  ): {
    label: string
    variant: 'default' | 'secondary' | 'destructive' | 'outline'
  } => {
    switch (status) {
      case 'pending':
        return { label: t('export_tasks.statusPending'), variant: 'outline' }
      case 'running':
        return { label: t('export_tasks.statusRunning'), variant: 'default' }
      case 'finished':
        return { label: t('export_tasks.statusFinished'), variant: 'secondary' }
      case 'failed':
        return { label: t('export_tasks.statusFailed'), variant: 'destructive' }
      case 'cancelled':
        return { label: t('export_tasks.statusCancelled'), variant: 'outline' }
      default:
        return { label: status, variant: 'outline' }
    }
  }

  const renderProgress = (job: ExportJobView) => {
    if (job.status === 'finished') {
      return (
        <span className="text-xs text-muted-foreground">
          {job.exported}/{job.total_emails}
        </span>
      )
    }
    if (job.status === 'failed' || job.status === 'cancelled') {
      return job.error ? (
        <span
          className="block max-w-[220px] truncate text-xs text-destructive"
          title={job.error}
        >
          {job.error}
        </span>
      ) : (
        <span className="text-xs text-muted-foreground">-</span>
      )
    }
    const pct =
      job.total_emails > 0
        ? Math.min(100, Math.round((job.processed / job.total_emails) * 100))
        : 0
    return (
      <div className="flex w-full max-w-[180px] flex-col gap-1">
        <Progress value={pct} className="h-1.5" />
        <span className="text-xs text-muted-foreground">
          {t('export_tasks.progress', {
            processed: job.processed,
            total: job.total_emails,
            exported: job.exported,
            failed: job.failed,
          })}
        </span>
      </div>
    )
  }
  return (
    <>
      <FixedHeader />
      <Main>
        <div className="mx-auto w-full max-w-7xl px-4">
          <div className="mb-4 flex items-center justify-between">
            <h1 className="text-lg font-semibold">{t('export_tasks.title')}</h1>
            <Button
              variant="outline"
              size="sm"
              onClick={() => refresh(true)}
              disabled={refreshing}
            >
              {refreshing ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <RefreshCw className="mr-2 h-4 w-4" />
              )}
              {t('export_tasks.refresh')}
            </Button>
          </div>
          <Separator className="mt-2 mb-4 lg:mt-3 lg:mb-6" />

          <div className="overflow-x-auto rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="text-xs">
                    {t('export_tasks.name')}
                  </TableHead>
                  <TableHead className="text-xs">
                    {t('export_tasks.status')}
                  </TableHead>
                  <TableHead className="text-xs">
                    {t('export_tasks.progressLabel')}
                  </TableHead>
                  <TableHead className="text-xs">
                    {t('export_tasks.emails')}
                  </TableHead>
                  <TableHead className="text-xs">
                    {t('export_tasks.size')}
                  </TableHead>
                  <TableHead className="text-xs">
                    {t('export_tasks.created')}
                  </TableHead>
                  <TableHead className="text-xs">
                    {t('export_tasks.actions')}
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {loading ? (
                  <TableRow>
                    <TableCell
                      colSpan={7}
                      className="py-8 text-center text-muted-foreground"
                    >
                      <Loader2 className="mx-auto h-5 w-5 animate-spin" />
                    </TableCell>
                  </TableRow>
                ) : jobs.length === 0 ? (
                  <TableRow>
                    <TableCell
                      colSpan={7}
                      className="py-8 text-center text-muted-foreground"
                    >
                      {t('export_tasks.empty')}
                    </TableCell>
                  </TableRow>
                ) : (
                  jobs.map((job) => {
                    const info = statusInfo(job.status)
                    return (
                      <TableRow key={job.job_id}>
                        <TableCell className="text-xs font-medium">
                          <div
                            className="max-w-[200px] truncate"
                            title={job.saved_search_name}
                          >
                            {job.saved_search_name}
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge variant={info.variant}>{info.label}</Badge>
                        </TableCell>
                        <TableCell>{renderProgress(job)}</TableCell>
                        <TableCell className="text-xs">
                          {job.total_emails.toLocaleString()}
                        </TableCell>
                        <TableCell className="text-xs">
                          {formatBytes(job.total_size)}
                        </TableCell>
                        <TableCell className="whitespace-nowrap text-xs">
                          {formatTime(job.created_at)}
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center gap-1">
                            {job.status === 'finished' && (
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-7 w-7"
                                title={t('export_tasks.download')}
                                onClick={() => handleDownload(job)}
                              >
                                <Download className="h-3.5 w-3.5" />
                              </Button>
                            )}
                            {(job.status === 'pending' ||
                              job.status === 'running') && (
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-7 w-7 text-muted-foreground hover:text-destructive"
                                title={t('export_tasks.cancel')}
                                onClick={() => setCancelTarget(job)}
                              >
                                <XCircle className="h-3.5 w-3.5" />
                              </Button>
                            )}
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-7 w-7 text-muted-foreground hover:text-destructive"
                              title={t('export_tasks.delete')}
                              onClick={() => setDeleteTarget(job)}
                            >
                              <Trash2 className="h-3.5 w-3.5" />
                            </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    )
                  })
                )}
              </TableBody>
            </Table>
          </div>
        </div>
      </Main>

      <ConfirmDialog
        open={cancelTarget !== null}
        onOpenChange={(isOpen) => !isOpen && setCancelTarget(null)}
        title={t('export_tasks.cancelTitle')}
        desc={t('export_tasks.cancelDesc', {
          name: cancelTarget?.saved_search_name ?? '',
        })}
        confirmText={t('export_tasks.cancelConfirm')}
        handleConfirm={handleCancel}
      />
      <ConfirmDialog
        open={deleteTarget !== null}
        onOpenChange={(isOpen) => !isOpen && setDeleteTarget(null)}
        title={t('export_tasks.deleteTitle')}
        desc={t('export_tasks.deleteDesc', {
          name: deleteTarget?.saved_search_name ?? '',
        })}
        confirmText={t('export_tasks.deleteConfirm')}
        destructive
        handleConfirm={handleDelete}
      />
    </>
  )
}
