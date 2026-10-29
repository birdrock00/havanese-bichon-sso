import { describe, expect, it } from 'vitest'
import { isValidCronExpression } from '../cron'

describe('isValidCronExpression', () => {
  it('accepts the expressions the cleanup picker generates', () => {
    expect(isValidCronExpression('0 0 3 * * *')).toBe(true)
    expect(isValidCronExpression('0 0 3 * * SUN')).toBe(true)
    expect(isValidCronExpression('0 0 3 1 * *')).toBe(true)
  })

  it('accepts names, lists, ranges, steps and shorthands', () => {
    expect(isValidCronExpression('*/15 0 3 * * MON-FRI')).toBe(true)
    expect(isValidCronExpression('0 0 3 * JAN,MAR *')).toBe(true)
    expect(isValidCronExpression('0 0 3 * * * 2030')).toBe(true)
    expect(isValidCronExpression('@daily')).toBe(true)
  })

  it('rejects malformed or out-of-range expressions', () => {
    expect(isValidCronExpression('')).toBe(false)
    expect(isValidCronExpression('0 0 3 * *')).toBe(false)
    expect(isValidCronExpression('0 0 25 * * *')).toBe(false)
    expect(isValidCronExpression('0 0 3 * * FUNDAY')).toBe(false)
    expect(isValidCronExpression('0 0 3 * * */0')).toBe(false)
    expect(isValidCronExpression('not a cron')).toBe(false)
  })
})
