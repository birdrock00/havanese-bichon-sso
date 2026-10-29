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
import { useCallback, useEffect, useRef, useState } from 'react'
import type { CleanupJob } from '@/api/account/api'
import {
  CleanupJobStatus,
  cancel_remote_cleanup_job,
  remote_cleanup_job,
} from '@/api/account/api'

const POLL_INTERVAL_MS = 2500
/** Polling cadence while the dialog that started the job is closed. */
const IDLE_POLL_INTERVAL_MS = 15_000

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

/**
 * Drives one background cleanup job: start it, poll it, optionally cancel it.
 *
 * Previews and moves run server-side for as long as strict verification takes,
 * so the HTTP call only starts the work and returns a job id immediately.
 * Polling turns that job into dialog state.
 *
 * Polling pauses while `active` is false (the dialog is closed) instead of
 * discarding the job: closing and reopening the dialog picks the same run back
 * up, and the run keeps going on the server either way.
 *
 * A job that disappears (expired, server restarted) ends the poll rather than
 * spinning; the cleanup reports dialog keeps the durable outcome.
 */
export function useCleanupJob(accountId: number, active: boolean) {
  const [job, setJob] = useState<CleanupJob | null>(null)
  const [starting, setStarting] = useState(false)
  const [pollError, setPollError] = useState<string | null>(null)
  const mounted = useRef(true)
  const activeRef = useRef(active)
  activeRef.current = active
  // The job the current poll loop follows, and a generation counter so an
  // older loop can never overwrite state after a newer job took over.
  const pollingJobId = useRef<string | null>(null)
  const pollGeneration = useRef(0)

  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
    }
  }, [])

  // A different account must never inherit the previous account's job.
  useEffect(() => {
    setJob(null)
    setStarting(false)
    setPollError(null)
  }, [accountId])

  const poll = useCallback(
    async (initial: CleanupJob) => {
      const generation = ++pollGeneration.current
      pollingJobId.current = initial.job_id
      let current = initial
      while (current.status === CleanupJobStatus.Running && mounted.current) {
        // Slow down while the dialog is closed, but keep polling: the job
        // still ends on the server, and a fast loop would never notice.
        await sleep(activeRef.current ? POLL_INTERVAL_MS : IDLE_POLL_INTERVAL_MS)
        if (!mounted.current || generation !== pollGeneration.current) break
        try {
          current = await remote_cleanup_job(accountId, current.job_id)
        } catch (error) {
          if (
            mounted.current &&
            activeRef.current &&
            generation === pollGeneration.current
          ) {
            setPollError(errorMessage(error))
          }
          break
        }
        if (generation === pollGeneration.current) setJob(current)
      }
      if (generation === pollGeneration.current) pollingJobId.current = null
      return current
    },
    [accountId]
  )

  /**
   * Runs `begin` (the POST that starts the job), then polls it to completion.
   * Returns the final snapshot; a job that could not be started rethrows so the
   * caller can show the server's message.
   */
  const start = useCallback(
    async (begin: () => Promise<CleanupJob>): Promise<CleanupJob> => {
      setStarting(true)
      setJob(null)
      setPollError(null)
      let created: CleanupJob
      try {
        created = await begin()
      } finally {
        if (mounted.current) setStarting(false)
      }
      if (!mounted.current) return created
      setJob(created)
      return poll(created)
    },
    [poll]
  )

  /**
   * Take over a job the server started (often by an earlier visit of this
   * dialog, or another tab). A running job resumes polling; a finished one is
   * adopted so its report is visible again.
   *
   * Without this, the client's only reference to a job would be the id it got
   * when starting it: a reload loses it, and asking to start a second job is
   * refused while the first still runs.
   */
  const adopt = useCallback(
    (incoming: CleanupJob) => {
      const alreadyPolling =
        incoming.status === CleanupJobStatus.Running &&
        pollingJobId.current === incoming.job_id
      // Always refresh the snapshot (adoption can win back state that a stale
      // listing briefly replaced), but never start a second poll loop for the
      // same running job.
      setJob(incoming)
      setPollError(null)
      if (incoming.status === CleanupJobStatus.Running && !alreadyPolling) {
        void poll(incoming)
      }
    },
    [poll]
  )

  const cancel = useCallback(async () => {
    const running = job
    if (!running || running.status !== CleanupJobStatus.Running) return
    try {
      const next = await cancel_remote_cleanup_job(accountId, running.job_id)
      if (mounted.current) setJob(next)
    } catch {
      // The poll stays the source of truth; nothing useful to recover here.
    }
  }, [accountId, job])

  const reset = useCallback(() => {
    setJob(null)
    setPollError(null)
  }, [])

  return {
    job,
    /** A job is being started or is still running. */
    busy: starting || job?.status === CleanupJobStatus.Running,
    pollError,
    start,
    adopt,
    cancel,
    reset,
  }
}

/**
 * Whole seconds since `since` (defaults to now); resets when `active` turns
 * false. Passing the job's `created_at` keeps the counter truthful after the
 * dialog reattaches to a job that started earlier.
 */
export function useElapsedSeconds(active: boolean, since?: number | null): number {
  const [seconds, setSeconds] = useState(0)
  useEffect(() => {
    if (!active) {
      setSeconds(0)
      return
    }
    const tick = () =>
      setSeconds(Math.max(0, Math.floor((Date.now() - (since ?? Date.now())) / 1000)))
    tick()
    const id = window.setInterval(tick, 1000)
    return () => window.clearInterval(id)
  }, [active, since])
  return seconds
}

/** The server's message when it sent one, otherwise the client error text. */
export function errorMessage(error: unknown): string {
  const data = (error as { response?: { data?: { message?: string } } })
    ?.response?.data?.message
  if (typeof data === 'string' && data) return data
  if (error instanceof Error) return error.message
  return String(error)
}
