import { describe, it, expect, vi } from 'vitest'
import type { ImportProgress } from '@/api/import/api'
import { importFiles, type ImportFilesDeps } from '../import-files'

const makeFile = (name: string, size: number) =>
  new File([new Uint8Array(size)], name)

const prog = (over: Partial<ImportProgress> = {}): ImportProgress => ({
  import_id: 'imp_1',
  status: 'Completed',
  format: 'eml',
  total: 1,
  success: 1,
  duplicates: 0,
  failed: 0,
  failed_details: [],
  ...over,
})

const makeDeps = (over: Partial<ImportFilesDeps> = {}): ImportFilesDeps => ({
  upload: vi.fn(async () => prog()),
  getProgress: vi.fn(async () => prog()),
  onUploadPct: vi.fn(),
  onProgress: vi.fn(),
  onPhase: vi.fn(),
  pollIntervalMs: 0,
  ...over,
})

describe('importFiles', () => {
  it('uploads every selected file, in order', async () => {
    const files = [
      makeFile('a.eml', 10),
      makeFile('b.eml', 10),
      makeFile('c.eml', 10),
    ]
    const upload = vi.fn(async (file: File) => prog({ import_id: file.name }))
    const deps = makeDeps({ upload })

    await importFiles(files, deps)

    expect(upload).toHaveBeenCalledTimes(3)
    expect(upload.mock.calls.map(([f]) => f.name)).toEqual([
      'a.eml',
      'b.eml',
      'c.eml',
    ])
  })

  it('reports uploading then processing for each file', async () => {
    const files = [makeFile('a.eml', 10), makeFile('b.eml', 10)]
    const deps = makeDeps({
      upload: vi.fn(async () =>
        prog({ status: 'Pending', total: 0, success: 0 })
      ),
    })

    await importFiles(files, deps)

    expect(vi.mocked(deps.onPhase).mock.calls.map(([p]) => p)).toEqual([
      'uploading',
      'processing',
      'uploading',
      'processing',
    ])
  })

  it('aggregates counts across files', async () => {
    const files = [makeFile('a.mbox', 10), makeFile('b.mbox', 10)]
    const results: Record<string, ImportProgress> = {
      'a.mbox': prog({
        import_id: 'a',
        total: 3,
        success: 2,
        failed: 1,
        duplicates: 1,
      }),
      'b.mbox': prog({ import_id: 'b', total: 2, success: 2 }),
    }
    const deps = makeDeps({
      upload: vi.fn(async (file: File) => results[file.name]),
    })

    const result = await importFiles(files, deps)

    expect(result.total).toBe(5)
    expect(result.success).toBe(4)
    expect(result.failed).toBe(1)
    expect(result.duplicates).toBe(1)
    expect(result.status).toBe('Completed')
  })

  it('polls until the import reaches a terminal status', async () => {
    const files = [makeFile('a.mbox', 10)]
    const polls = [
      prog({ status: 'Processing', total: 5, success: 2 }),
      prog({ status: 'Completed', total: 5, success: 5 }),
    ]
    const getProgress = vi.fn(async () => polls.shift()!)
    const deps = makeDeps({
      upload: vi.fn(async () =>
        prog({ status: 'Pending', total: 0, success: 0 })
      ),
      getProgress,
    })

    const result = await importFiles(files, deps)

    expect(getProgress).toHaveBeenCalledTimes(2)
    expect(result.success).toBe(5)
    const seen = vi.mocked(deps.onProgress).mock.calls.map(([p]) => p.success)
    expect(seen).toContain(2)
  })

  it('continues past a failed upload and surfaces its error', async () => {
    const files = [makeFile('bad.eml', 10), makeFile('good.eml', 10)]
    const upload = vi.fn(async (file: File) => {
      if (file.name === 'bad.eml') {
        throw { response: { data: { message: 'not a valid email file' } } }
      }
      return prog({ import_id: 'good' })
    })
    const deps = makeDeps({ upload })

    const result = await importFiles(files, deps)

    expect(upload).toHaveBeenCalledTimes(2)
    expect(result.total).toBe(2)
    expect(result.success).toBe(1)
    expect(result.failed).toBe(1)
    expect(result.failed_details).toHaveLength(1)
    expect(result.failed_details[0].error_message).toContain('bad.eml')
    expect(result.failed_details[0].error_message).toContain(
      'not a valid email file'
    )
    expect(result.status).toBe('Completed')
  })

  it('throws when every upload fails, so the caller can toast and reset', async () => {
    const files = [makeFile('a.eml', 10), makeFile('b.eml', 10)]
    const deps = makeDeps({
      upload: vi.fn(async () => {
        throw { response: { data: { message: 'server unreachable' } } }
      }),
    })

    await expect(importFiles(files, deps)).rejects.toThrow('server unreachable')
  })

  it('reports cumulative upload progress that never resets across files', async () => {
    const files = [makeFile('a.eml', 100), makeFile('b.eml', 300)]
    const deps = makeDeps({
      upload: vi.fn(async (_file: File, onPct: (pct: number) => void) => {
        onPct(50)
        onPct(100)
        return prog()
      }),
    })

    await importFiles(files, deps)

    const seen = vi.mocked(deps.onUploadPct).mock.calls.map(([pct]) => pct)
    expect(seen.length).toBeGreaterThan(0)
    for (let i = 1; i < seen.length; i++) {
      expect(seen[i]).toBeGreaterThanOrEqual(seen[i - 1])
    }
    expect(seen[seen.length - 1]).toBe(100)
  })

  it('labels failed details with the file name only for multi-file selections', async () => {
    const failing = (id: string) =>
      prog({
        import_id: id,
        total: 2,
        success: 1,
        failed: 1,
        failed_details: [{ index: 0, error_message: 'bad message' }],
      })

    const multi = await importFiles(
      [makeFile('a.mbox', 10), makeFile('b.mbox', 10)],
      makeDeps({ upload: vi.fn(async (file: File) => failing(file.name)) })
    )
    expect(multi.failed_details.map((d) => d.error_message)).toEqual([
      'a.mbox: bad message',
      'b.mbox: bad message',
    ])

    const single = await importFiles(
      [makeFile('a.mbox', 10)],
      makeDeps({ upload: vi.fn(async () => failing('a')) })
    )
    expect(single.failed_details.map((d) => d.error_message)).toEqual([
      'bad message',
    ])
  })

  it('gives up on a file after repeated poll errors and continues', async () => {
    const files = [makeFile('a.mbox', 10), makeFile('b.eml', 10)]
    const upload = vi.fn(async (file: File) =>
      file.name === 'a.mbox'
        ? prog({ import_id: 'a', status: 'Pending', total: 0, success: 0 })
        : prog({ import_id: 'b' })
    )
    const getProgress = vi.fn(async () => {
      throw new Error('network down')
    })
    const deps = makeDeps({ upload, getProgress })

    const result = await importFiles(files, deps)

    expect(upload).toHaveBeenCalledTimes(2)
    expect(result.success).toBe(1)
    expect(result.failed).toBe(1)
    expect(result.failed_details[0].error_message).toContain('a.mbox')
    expect(result.status).toBe('Completed')
  })

  it('records the file position as the index of a synthetic failure', async () => {
    const files = [
      makeFile('a.eml', 10),
      makeFile('bad.eml', 10),
      makeFile('c.eml', 10),
    ]
    const deps = makeDeps({
      upload: vi.fn(async (file: File) => {
        if (file.name === 'bad.eml') throw new Error('boom')
        return prog()
      }),
    })

    const result = await importFiles(files, deps)

    expect(result.failed_details).toHaveLength(1)
    expect(result.failed_details[0].index).toBe(1)
  })

  it('is Failed when the server reports a fatal failure with zero counts', async () => {
    const files = [makeFile('a.mbox', 10)]
    const deps = makeDeps({
      upload: vi.fn(async () =>
        prog({
          status: 'Failed',
          total: 0,
          success: 0,
          failed: 0,
          failed_details: [{ index: 0, error_message: 'mailbox not found' }],
        })
      ),
    })

    const result = await importFiles(files, deps)

    expect(result.status).toBe('Failed')
  })

  it('stops uploading and polling once aborted', async () => {
    const controller = new AbortController()
    const files = [makeFile('a.mbox', 10), makeFile('b.mbox', 10)]
    const upload = vi.fn(async () =>
      prog({ status: 'Pending', total: 0, success: 0 })
    )
    const getProgress = vi.fn(async () => {
      controller.abort()
      return prog({ status: 'Processing', total: 5, success: 1 })
    })
    const deps = makeDeps({ upload, getProgress, signal: controller.signal })

    await importFiles(files, deps)

    expect(upload).toHaveBeenCalledTimes(1)
    expect(getProgress).toHaveBeenCalledTimes(1)
  })

  it('is Failed when nothing succeeded', async () => {
    const files = [makeFile('a.mbox', 10)]
    const deps = makeDeps({
      upload: vi.fn(async () =>
        prog({
          status: 'Failed',
          total: 2,
          success: 0,
          failed: 2,
          failed_details: [
            { index: 0, error_message: 'parse error' },
            { index: 1, error_message: 'parse error' },
          ],
        })
      ),
    })

    const result = await importFiles(files, deps)

    expect(result.status).toBe('Failed')
    expect(result.failed).toBe(2)
  })
})
