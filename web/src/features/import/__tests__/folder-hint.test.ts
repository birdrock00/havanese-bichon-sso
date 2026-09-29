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

  it('files French Archived,Sent under Envoyé (non-English system labels)', async () => {
    // Mirrors the CLI regression test: French "Archivés,Envoyé" must file
    // under "Envoyé", like English "Archived,Sent" files under "Sent".
    const fr = [
      'From: a@example.com',
      'To: b@example.com',
      'Subject: hello',
      'X-Gmail-Labels: Archivés,Envoyé',
      '',
      'body',
    ].join('\n')

    const hint = await extractFolderHint(new File([fr], 'french.eml'))
    expect(hint?.name).toBe('Envoyé')
  })

  it('decodes RFC 2047 labels straight from the issue payload', async () => {
    // Exactly the header format the reporter's real French Takeout export had.
    const fr = [
      'From: a@example.com',
      'Subject: hello',
      'X-Gmail-Labels: =?UTF-8?Q?Archiv=C3=A9s,Envoy=C3=A9?=',
      '',
      'body',
    ].join('\n')

    const hint = await extractFolderHint(new File([fr], 'french.eml'))
    expect(hint?.name).toBe('Envoyé')
  })

  it('files French Inbox + custom label under the custom label', async () => {
    // Mirrors the CLI regression test: prefer the business label over the
    // localized generic "Boîte de réception".
    const fr = [
      'From: a@example.com',
      'Subject: hello',
      'X-Gmail-Labels: Boîte de réception,Ma boîte',
      '',
      'body',
    ].join('\n')

    const hint = await extractFolderHint(new File([fr], 'french.eml'))
    expect(hint?.name).toBe('Ma boîte')
  })

  it('handles other European system labels (Spanish and German)', async () => {
    const es = [
      'From: a@example.com',
      'Subject: hello',
      'X-Gmail-Labels: Archivados,Enviados',
      '',
      'body',
    ].join('\n')
    const de = [
      'From: a@example.com',
      'Subject: hello',
      'X-Gmail-Labels: Archiviert,Gesendet',
      '',
      'body',
    ].join('\n')

    expect((await extractFolderHint(new File([es], 'spanish.eml')))?.name).toBe(
      'Enviados'
    )
    expect((await extractFolderHint(new File([de], 'german.eml')))?.name).toBe(
      'Gesendet'
    )
  })

  it('keeps an only-status-label message as-is (no business folder)', async () => {
    // Mirrors the CLI fallback: a message whose only label is the archive
    // status cannot be filed anywhere meaningful.
    const fr = [
      'From: a@example.com',
      'Subject: hello',
      'X-Gmail-Labels: Archivés',
      '',
      'body',
    ].join('\n')

    const hint = await extractFolderHint(new File([fr], 'archived.eml'))
    expect(hint?.name).toBe('Archivés')
  })
})
