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

/**
 * Structural validation for the cron expressions the backend accepts.
 *
 * The server parses schedules with the Rust `cron` crate (0.17), whose
 * grammar is: six fields `sec min hour day-of-month month day-of-week`, an
 * optional seventh `year`, lists/ranges/steps, month and weekday names, `?`
 * for day-of-month/day-of-week, and the `@yearly/@monthly/@weekly/@daily/
 * @hourly` shorthands. This is the same subset, kept in sync deliberately:
 * the server remains authoritative, this only catches mistakes before a save
 * round-trip.
 */

const MONTHS: Record<string, number> = {
  jan: 1,
  january: 1,
  feb: 2,
  february: 2,
  mar: 3,
  march: 3,
  apr: 4,
  april: 4,
  may: 5,
  jun: 6,
  june: 6,
  jul: 7,
  july: 7,
  aug: 8,
  august: 8,
  sep: 9,
  sept: 9,
  september: 9,
  oct: 10,
  october: 10,
  nov: 11,
  november: 11,
  dec: 12,
  december: 12,
}

// The Rust cron crate numbers days as 1 = Sunday … 7 = Saturday.
const WEEKDAYS: Record<string, number> = {
  sun: 1,
  sunday: 1,
  mon: 2,
  monday: 2,
  tue: 3,
  tues: 3,
  tuesday: 3,
  wed: 4,
  wednesday: 4,
  thu: 5,
  thurs: 5,
  thursday: 5,
  fri: 6,
  friday: 6,
  sat: 7,
  saturday: 7,
}

interface FieldSpec {
  min: number
  max: number
  names?: Record<string, number>
  allowAny?: boolean
}

const FIELDS: FieldSpec[] = [
  { min: 0, max: 59 }, // seconds
  { min: 0, max: 59 }, // minutes
  { min: 0, max: 23 }, // hours
  { min: 1, max: 31, allowAny: true }, // day of month
  { min: 1, max: 12, names: MONTHS }, // month
  { min: 1, max: 7, names: WEEKDAYS, allowAny: true }, // day of week
  { min: 1970, max: 2100 }, // optional year
]

const SHORTHANDS = new Set([
  '@yearly',
  '@annually',
  '@monthly',
  '@weekly',
  '@daily',
  '@hourly',
])

function resolveToken(token: string, field: FieldSpec): number | null {
  const normalized = token.trim().toLowerCase()
  if (/^\d+$/.test(normalized)) {
    const value = Number(normalized)
    return value >= field.min && value <= field.max ? value : null
  }
  if (field.names) {
    const named = field.names[normalized]
    return named ?? null
  }
  return null
}

function isValidItem(item: string, field: FieldSpec): boolean {
  if (!item) return false
  if (item === '*') return true
  if (item === '?' && field.allowAny) return true

  const [base, step, ...rest] = item.split('/')
  if (rest.length > 0) return false
  if (step !== undefined) {
    if (!/^\d+$/.test(step) || Number(step) === 0) return false
  }

  if (base === '*' || base === '?') {
    return base === '*' || (field.allowAny === true && base === '?')
  }

  if (base.includes('-')) {
    const parts = base.split('-')
    if (parts.length !== 2) return false
    const start = resolveToken(parts[0], field)
    const end = resolveToken(parts[1], field)
    // A reversed range is harmless to the parser (it simply yields an empty
    // set on some versions); format-wise it is still a range.
    return start !== null && end !== null
  }

  return resolveToken(base, field) !== null
}

/** True when `expression` has a structure the backend's cron parser accepts. */
export function isValidCronExpression(expression: string): boolean {
  const trimmed = expression.trim()
  if (!trimmed) return false
  if (SHORTHANDS.has(trimmed.toLowerCase())) return true

  const fields = trimmed.split(/\s+/)
  if (fields.length < 6 || fields.length > 7) return false

  for (let index = 0; index < fields.length; index += 1) {
    const spec = FIELDS[index]
    const items = fields[index].split(',')
    if (items.some((item) => !isValidItem(item, spec))) return false
  }
  return true
}
