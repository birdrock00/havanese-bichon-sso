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
import { Bookmark, Download, Loader2, Pencil, Plus, Search, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Separator } from '@/components/ui/separator'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { toast } from '@/hooks/use-toast'
import { cn } from '@/lib/utils'
import { AxiosError } from 'axios'
import { useSavedSearches } from '@/hooks/use-saved-searches'
import { SavedSearchKind, SavedSearchModel } from '@/api/saved-search/api'
import {
  createExport,
  downloadExport,
  getExportJob,
  previewExport,
  ExportJobView,
  ExportPreviewView,
} from '@/api/export/api'

interface SavedSearchesDropdownProps {
  kind: SavedSearchKind
  filter: Record<string, any>
  onApply: (filter: Record<string, any>) => void
}

const normalizeFilter = (filter: Record<string, any>): Record<string, any> => {
  const out: Record<string, any> = {}
  Object.keys(filter)
    .sort()
    .forEach((key) => {
      out[key] = filter[key]
    })
  return out
}

const filtersEqual = (a: Record<string, any>, b: Record<string, any>) =>
  JSON.stringify(normalizeFilter(a)) === JSON.stringify(normalizeFilter(b))

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

export function SavedSearchesDropdown({ kind, filter, onApply }: SavedSearchesDropdownProps) {
  const { t } = useTranslation()
  const { searches, createSearch, renameSearch, deleteSearch } = useSavedSearches(kind)

  const [open, setOpen] = React.useState(false)
  const [query, setQuery] = React.useState('')
  const [name, setName] = React.useState('')
  const [renameTarget, setRenameTarget] = React.useState<SavedSearchModel | null>(null)
  const [renameName, setRenameName] = React.useState('')
  const [deleteTarget, setDeleteTarget] = React.useState<SavedSearchModel | null>(null)
  const [exportTarget, setExportTarget] = React.useState<SavedSearchModel | null>(null)
  const [exportPreview, setExportPreview] = React.useState<ExportPreviewView | null>(null)
  const [exportJob, setExportJob] = React.useState<ExportJobView | null>(null)
  const [exportBusy, setExportBusy] = React.useState(false)
  const filtered = query.trim()
    ? searches.filter((s) => s.name.toLowerCase().includes(query.trim().toLowerCase()))
    : searches
  const pollRef = React.useRef<ReturnType<typeof setInterval> | null>(null)

  const clearExportPoll = () => {
    if (pollRef.current) {
      clearInterval(pollRef.current)
      pollRef.current = null
    }
  }

  const closeExport = () => {
    clearExportPoll()
    setExportTarget(null)
    setExportPreview(null)
    setExportJob(null)
    setExportBusy(false)
  }

  const triggerDownload = (jobId: string) => {
    downloadExport(jobId)
      .then((blob) => {
        const url = URL.createObjectURL(blob)
        const link = document.createElement('a')
        link.href = url
        link.download = `${jobId}.mbox`
        document.body.appendChild(link)
        link.click()
        document.body.removeChild(link)
        URL.revokeObjectURL(url)
        toast({ title: t('exports.downloadSaved') })
        closeExport()
      })
      .catch((error) => {
        toast({
          title: t('exports.downloadFailed'),
          description: getErrorMessage(error),
          variant: 'destructive',
        })
        closeExport()
      })
  }

  const startExport = () => {
    if (!exportTarget || !exportPreview) return
    setExportBusy(true)
    createExport(exportTarget.id)
      .then((job) => {
        setExportJob(job)
        pollRef.current = setInterval(() => {
          getExportJob(job.job_id)
            .then((updated) => {
              setExportJob(updated)
              if (updated.status === 'finished') {
                clearExportPoll()
                setExportBusy(false)
                triggerDownload(updated.job_id)
              } else if (updated.status === 'failed' || updated.status === 'cancelled') {
                clearExportPoll()
                setExportBusy(false)
                toast({
                  title:
                    updated.status === 'failed'
                      ? t('exports.failed')
                      : t('exports.cancelled'),
                  description: updated.error ?? undefined,
                  variant: 'destructive',
                })
                closeExport()
              }
            })
            .catch((error) => {
              clearExportPoll()
              setExportBusy(false)
              toast({
                title: t('exports.statusFailed'),
                description: getErrorMessage(error),
                variant: 'destructive',
              })
              closeExport()
            })
        }, 2000)
      })
      .catch((error) => {
        setExportBusy(false)
        toast({
          title: t('exports.startFailed'),
          description: getErrorMessage(error),
          variant: 'destructive',
        })
        closeExport()
      })
  }

  const handleExportClick = (item: SavedSearchModel) => {
    setExportTarget(item)
    setExportPreview(null)
    setExportJob(null)
    setExportBusy(false)
    previewExport(item.id)
      .then((preview) => setExportPreview(preview))
      .catch((error) => {
        setExportTarget(null)
        toast({
          title: t('exports.previewFailed'),
          description: getErrorMessage(error),
          variant: 'destructive',
        })
      })
  }

  React.useEffect(() => {
    return clearExportPoll
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const hasActiveFilter = Object.keys(filter).length > 0

  const handleSave = () => {
    const trimmed = name.trim()
    if (!trimmed) {
      toast({
        title: t('saved_searches.nameRequired'),
        variant: 'destructive',
      })
      return
    }
    createSearch.mutate(
      { name: trimmed, filter },
      {
        onSuccess: () => {
          setName('')
          toast({ title: t('saved_searches.savedSuccess') })
        },
        onError: (error) => {
          toast({
            title: t('saved_searches.savedError'),
            description: getErrorMessage(error),
            variant: 'destructive',
          })
        },
      }
    )
  }

  const handleRename = () => {
    if (!renameTarget) return
    const trimmed = renameName.trim()
    if (!trimmed) {
      toast({ title: t('saved_searches.nameRequired'), variant: 'destructive' })
      return
    }
    renameSearch.mutate(
      { id: renameTarget.id, name: trimmed },
      {
        onSuccess: () => {
          setRenameTarget(null)
          toast({ title: t('saved_searches.renamedSuccess') })
        },
        onError: (error) => {
          toast({
            title: t('saved_searches.renamedError'),
            description: getErrorMessage(error),
            variant: 'destructive',
          })
        },
      }
    )
  }

  const handleDelete = () => {
    if (!deleteTarget) return
    deleteSearch.mutate(deleteTarget.id, {
      onSuccess: () => {
        setDeleteTarget(null)
        toast({ title: t('saved_searches.deletedSuccess') })
      },
      onError: (error) => {
        toast({
          title: t('saved_searches.deletedError'),
          description: getErrorMessage(error),
          variant: 'destructive',
        })
      },
    })
  }

  return (
    <>
      <Popover open={open} onOpenChange={(o) => { setOpen(o); if (!o) setQuery('') }}>
        <PopoverTrigger asChild>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs gap-1.5 font-medium rounded-md text-foreground/70 hover:text-foreground hover:bg-accent transition-all duration-200"
            title={t('saved_searches.buttonTooltip')}
          >
            <Bookmark className="h-4 w-4" />
            <span>{t('saved_searches.button')}</span>
          </Button>
        </PopoverTrigger>
        <PopoverContent align="end" className="w-80 p-0">
          <div className="p-3">
            <div className="flex items-center justify-between">
              <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                {t('saved_searches.title')}
              </span>
              <span className="text-[10px] text-muted-foreground">{searches.length}</span>
            </div>
            {hasActiveFilter ? (
              <div className="mt-2">
                <p className="text-[11px] font-medium text-foreground/70">
                  {t('saved_searches.saveCurrent')}
                </p>
                <div className="mt-1 flex items-center gap-1.5">
                  <Input
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleSave()}
                    placeholder={t('saved_searches.savePlaceholder')}
                    className="h-8 text-xs"
                  />
                  <Button
                    size="sm"
                    className="h-8 shrink-0 px-2 text-xs"
                    onClick={handleSave}
                    disabled={createSearch.isPending || !name.trim()}
                  >
                    {createSearch.isPending ? (
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    ) : (
                      <Plus className="h-3.5 w-3.5" />
                    )}
                    <span className="ml-1">{t('saved_searches.save')}</span>
                  </Button>
                </div>
              </div>
            ) : (
              <p className="mt-2 text-[11px] text-muted-foreground">
                {t('saved_searches.emptyFilterHint')}
              </p>
            )}
          </div>
          <Separator />
          {searches.length > 0 && (
            <div className="border-b p-2">
              <div className="relative">
                <Search className="absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
                <Input
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder={t('saved_searches.filterPlaceholder')}
                  className="h-8 pl-7 text-xs"
                />
              </div>
            </div>
          )}
          <ScrollArea className="max-h-64">
            <div className="p-1.5">
              {searches.length === 0 ? (
                <div className="px-3 py-6 text-center text-xs text-muted-foreground">
                  {t('saved_searches.empty')}
                </div>
              ) : filtered.length === 0 ? (
                <div className="px-3 py-6 text-center text-xs text-muted-foreground">
                  {t('saved_searches.filterNoResults')}
                </div>
              ) : (
                filtered.map((item) => {
                  const isCurrent = filtersEqual(filter, item.filter)
                  return (
                    <div
                      key={item.id}
                      className="group flex items-center gap-1 rounded-md px-1.5 py-0.5 hover:bg-accent"
                    >
                      <button
                        type="button"
                        className={cn(
                          'flex flex-1 items-center gap-2 overflow-hidden rounded px-1 py-1.5 text-left text-xs',
                          isCurrent ? 'text-primary' : 'text-foreground/80 hover:text-foreground'
                        )}
                        title={t('saved_searches.apply')}
                        onClick={() => onApply(item.filter)}
                      >
                        <Search className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                        <span className="truncate font-medium">{item.name}</span>
                      </button>
                      {kind === 'Email' && (
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-7 w-7 shrink-0 text-muted-foreground hover:text-foreground"
                          title={t('exports.export')}
                          onClick={() => handleExportClick(item)}
                        >
                          <Download className="h-3.5 w-3.5" />
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 shrink-0 text-muted-foreground hover:text-foreground"
                        title={t('saved_searches.rename')}
                        onClick={() => {
                          setRenameTarget(item)
                          setRenameName(item.name)
                        }}
                      >
                        <Pencil className="h-3.5 w-3.5" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 shrink-0 text-muted-foreground hover:text-destructive"
                        title={t('saved_searches.delete')}
                        onClick={() => setDeleteTarget(item)}
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                  )
                })
              )}
            </div>
          </ScrollArea>
        </PopoverContent>
      </Popover>

      <Dialog
        open={renameTarget !== null}
        onOpenChange={(isOpen) => !isOpen && setRenameTarget(null)}
      >
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>{t('saved_searches.renameTitle')}</DialogTitle>
            <DialogDescription asChild>
              <div className="space-y-2 pt-1">
                <Label
                  htmlFor="saved-search-rename"
                  className="text-xs font-normal text-muted-foreground"
                >
                  {t('saved_searches.renameHint')}
                </Label>
                <Input
                  id="saved-search-rename"
                  value={renameName}
                  onChange={(e) => setRenameName(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && handleRename()}
                  className="text-sm"
                />
              </div>
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setRenameTarget(null)}
              disabled={renameSearch.isPending}
            >
              {t('common.cancel')}
            </Button>
            <Button
              onClick={handleRename}
              disabled={renameSearch.isPending || !renameName.trim()}
            >
              {renameSearch.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {t('saved_searches.renameSubmit')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        open={deleteTarget !== null}
        onOpenChange={(isOpen) => !isOpen && setDeleteTarget(null)}
        title={t('saved_searches.deleteTitle')}
        desc={t('saved_searches.deleteDesc', { name: deleteTarget?.name ?? '' })}
        confirmText={t('saved_searches.deleteConfirm')}
        destructive
        handleConfirm={handleDelete}
        isLoading={deleteSearch.isPending}
      />

      <Dialog
        open={exportTarget !== null}
        onOpenChange={(isOpen) => !isOpen && closeExport()}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>{t('exports.title')}</DialogTitle>
          </DialogHeader>
          {!exportPreview ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              {t('exports.exporting')}
            </div>
          ) : !exportJob ? (
            <div className="space-y-3">
              <div className="grid grid-cols-3 gap-2 text-center">
                <div className="rounded-md border p-2">
                  <div className="text-lg font-semibold">
                    {exportPreview.accounts.length}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {t('exports.previewAccounts')}
                  </div>
                </div>
                <div className="rounded-md border p-2">
                  <div className="text-lg font-semibold">
                    {exportPreview.total_emails.toLocaleString()}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {t('exports.previewEmails')}
                  </div>
                </div>
                <div className="rounded-md border p-2">
                  <div className="text-lg font-semibold">
                    {formatBytes(exportPreview.total_size)}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {t('exports.previewSize')}
                  </div>
                </div>
              </div>
              <p className="text-sm text-muted-foreground break-words">
                {t('exports.confirm', {
                  count: exportPreview.total_emails,
                  name: exportTarget?.name ?? '',
                })}
              </p>
              <DialogFooter>
                <Button
                  variant="outline"
                  onClick={closeExport}
                  disabled={exportBusy}
                >
                  {t('common.cancel')}
                </Button>
                <Button onClick={startExport} disabled={exportBusy}>
                  {exportBusy && (
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  )}
                  {t('exports.export')}
                </Button>
              </DialogFooter>
            </div>
          ) : exportJob.status === 'finished' ? (
            <div className="space-y-3">
              <div className="text-sm text-muted-foreground">
                {t('exports.complete')}
              </div>
              <DialogFooter>
                <Button onClick={() => triggerDownload(exportJob.job_id)}>
                  <Download className="mr-2 h-4 w-4" />
                  {t('exports.download')}
                </Button>
              </DialogFooter>
            </div>
          ) : exportJob.status === 'failed' ||
            exportJob.status === 'cancelled' ? (
            <div className="text-sm text-destructive">
              {exportJob.error ?? t('exports.failed')}
            </div>
          ) : (
            <div className="space-y-3">
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                {t('exports.exporting')}
              </div>
              <div className="text-xs text-muted-foreground">
                {t('exports.progress', {
                  processed: exportJob.processed,
                  total: exportJob.total_emails,
                  exported: exportJob.exported,
                  failed: exportJob.failed,
                })}
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>

    </>
  )
}