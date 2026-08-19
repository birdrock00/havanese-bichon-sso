//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
import type { ImportProgress } from '@/api/import/api'

export interface ImportFilesDeps {
  upload: (file: File, onPct: (pct: number) => void) => Promise<ImportProgress>
  getProgress: (importId: string) => Promise<ImportProgress>
  onUploadPct: (pct: number) => void
  onProgress: (progress: ImportProgress) => void
  onPhase: (phase: 'uploading' | 'processing') => void
  signal?: AbortSignal
  pollIntervalMs?: number
}

const DEFAULT_POLL_INTERVAL_MS = 1000
const MAX_POLL_ERRORS = 5

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

const isTerminal = (status: ImportProgress['status']) =>
  status === 'Completed' || status === 'Failed'

function emptyProgress(): ImportProgress {
  return {
    import_id: '',
    status: 'Completed',
    format: '',
    total: 0,
    success: 0,
    duplicates: 0,
    failed: 0,
    failed_details: [],
  }
}

export function errorMessage(err: unknown): string {
  if (err && typeof err === 'object') {
    const maybe = err as {
      response?: { data?: { message?: unknown } }
      message?: unknown
    }
    const serverMessage = maybe.response?.data?.message
    if (typeof serverMessage === 'string' && serverMessage) return serverMessage
    if (typeof maybe.message === 'string' && maybe.message) return maybe.message
  }
  return String(err)
}

function mergeProgress(
  aggregate: ImportProgress,
  fileProgress: ImportProgress,
  label: string | null
): ImportProgress {
  return {
    ...aggregate,
    import_id: fileProgress.import_id || aggregate.import_id,
    format: fileProgress.format || aggregate.format,
    total: aggregate.total + fileProgress.total,
    success: aggregate.success + fileProgress.success,
    duplicates: aggregate.duplicates + fileProgress.duplicates,
    failed: aggregate.failed + fileProgress.failed,
    failed_details: [
      ...aggregate.failed_details,
      ...fileProgress.failed_details.map((d) =>
        label ? { ...d, error_message: `${label}: ${d.error_message}` } : d
      ),
    ],
  }
}

function fileFailure(fileIndex: number, message: string): ImportProgress {
  return {
    ...emptyProgress(),
    status: 'Failed',
    total: 1,
    failed: 1,
    failed_details: [{ index: fileIndex, error_message: message }],
  }
}

async function waitForTerminal(
  initial: ImportProgress,
  getProgress: (importId: string) => Promise<ImportProgress>,
  pollIntervalMs: number,
  signal: AbortSignal | undefined,
  onTick: (progress: ImportProgress) => void
): Promise<ImportProgress> {
  let current = initial
  let consecutiveErrors = 0
  while (!isTerminal(current.status)) {
    await sleep(pollIntervalMs)
    if (signal?.aborted) return current
    try {
      current = await getProgress(initial.import_id)
      consecutiveErrors = 0
      onTick(current)
    } catch (err) {
      consecutiveErrors++
      if (consecutiveErrors > MAX_POLL_ERRORS) {
        throw new Error(
          `lost track of import progress (the import may still be running, check import history): ${errorMessage(err)}`
        )
      }
    }
  }
  return current
}

export async function importFiles(
  files: File[],
  deps: ImportFilesDeps
): Promise<ImportProgress> {
  const { upload, getProgress, onUploadPct, onProgress, onPhase, signal } = deps
  const pollIntervalMs = deps.pollIntervalMs ?? DEFAULT_POLL_INTERVAL_MS
  const totalBytes = files.reduce((sum, f) => sum + f.size, 0)
  const label = files.length > 1 ? (file: File) => file.name : () => null

  let aggregate = emptyProgress()
  let uploadedBytes = 0
  let transportFailures = 0
  let sawFailure = false

  for (const [index, file] of files.entries()) {
    if (signal?.aborted) break

    const startBytes = uploadedBytes
    const reportPct = (filePct: number) => {
      if (totalBytes === 0) return
      const bytes = startBytes + (filePct / 100) * file.size
      onUploadPct(Math.min(100, Math.round((bytes / totalBytes) * 100)))
    }

    onPhase('uploading')
    let initial: ImportProgress
    try {
      initial = await upload(file, reportPct)
    } catch (err) {
      transportFailures++
      sawFailure = true
      aggregate = mergeProgress(
        aggregate,
        fileFailure(index, `${file.name}: ${errorMessage(err)}`),
        null
      )
      uploadedBytes = startBytes + file.size
      reportPct(100)
      onProgress(aggregate)
      continue
    }
    uploadedBytes = startBytes + file.size
    reportPct(100)

    onPhase('processing')
    let final: ImportProgress
    try {
      final = await waitForTerminal(
        initial,
        getProgress,
        pollIntervalMs,
        signal,
        (current) => onProgress(mergeProgress(aggregate, current, label(file)))
      )
    } catch (err) {
      sawFailure = true
      aggregate = mergeProgress(
        aggregate,
        fileFailure(index, `${file.name}: ${errorMessage(err)}`),
        null
      )
      onProgress(aggregate)
      continue
    }
    if (final.status === 'Failed' || final.failed > 0) sawFailure = true
    aggregate = mergeProgress(aggregate, final, label(file))
    onProgress(aggregate)
  }

  if (files.length > 0 && transportFailures === files.length) {
    throw new Error(
      aggregate.failed_details[0]?.error_message ?? 'Upload failed'
    )
  }

  const result: ImportProgress = {
    ...aggregate,
    status: aggregate.success === 0 && sawFailure ? 'Failed' : 'Completed',
  }
  onProgress(result)
  return result
}
