import { describe, it, expect } from 'vitest'
import { extractFolderHint } from '../folder-hint'

const emlWithLabels = [
  'From: a@example.com',
  'To: b@example.com',
  'Subject: hello',
  'X-Gmail-Labels: TestBatch',
  '',
  'body',
].join('\n')

const emlPlain = ['From: a@example.com', 'Subject: hello', '', 'body'].join(
  '\n'
)

describe('extractFolderHint', () => {
  it('extracts the folder from X-Gmail-Labels and records the source file', async () => {
    const hint = await extractFolderHint(
      new File([emlWithLabels], 'sample-01.eml')
    )

    expect(hint).toEqual({
      name: 'TestBatch',
      source: 'gmail-labels',
      fileName: 'sample-01.eml',
    })
  })

  it('unfolds folded header lines without swallowing the body', async () => {
    const folded = [
      'From: a@example.com',
      'X-Gmail-Labels: Inbox,',
      ' Receipts',
      'Subject: hello',
      '',
      'body line',
    ].join('\n')

    const hint = await extractFolderHint(new File([folded], 'x.eml'))

    expect(hint?.name).toBe('Receipts')
  })

  it('falls back to the file name and records it as the source file', async () => {
    const hint = await extractFolderHint(new File([emlPlain], 'Receipts.eml'))

    expect(hint).toEqual({
      name: 'Receipts',
      source: 'filename',
      fileName: 'Receipts.eml',
    })
  })
})
