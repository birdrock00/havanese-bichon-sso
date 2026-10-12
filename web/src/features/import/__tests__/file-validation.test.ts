import { describe, it, expect } from 'vitest'
import { isValidFileType } from '../file-validation'

const makeFile = (name: string, type = '') =>
  new File([new Uint8Array(8)], name, { type })

describe('isValidFileType', () => {
  it.each(['eml', 'mbox', 'pst'])('accepts .%s with no MIME type', (ext) => {
    expect(isValidFileType(makeFile(`archive.${ext}`), ext)).toBe(true)
  })

  it('accepts .pst reported as application/vnd.ms-outlook', () => {
    const file = makeFile('backup.pst', 'application/vnd.ms-outlook')
    expect(isValidFileType(file, 'pst')).toBe(true)
  })

  it('accepts .pst reported as application/x-pst', () => {
    const file = makeFile('backup.pst', 'application/x-pst')
    expect(isValidFileType(file, 'pst')).toBe(true)
  })

  it('accepts .mbox reported as application/mbox', () => {
    const file = makeFile('inbox.mbox', 'application/mbox')
    expect(isValidFileType(file, 'mbox')).toBe(true)
  })

  it.each([
    ['video/mp4'],
    ['application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'],
    ['application/zip'],
    ['application/pdf'],
  ])('rejects supported extensions with blocked MIME %s', (mime) => {
    expect(isValidFileType(makeFile('archive.pst', mime), 'pst')).toBe(false)
  })

  it('rejects unsupported extensions regardless of MIME type', () => {
    expect(isValidFileType(makeFile('movie.avi', 'video/x-msvideo'), 'avi')).toBe(false)
    expect(isValidFileType(makeFile('noext', ''), '')).toBe(false)
  })
})
