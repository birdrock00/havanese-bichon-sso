import { afterEach, describe, expect, it, vi } from 'vitest'
import { copyToClipboard } from '../clipboard'

const setClipboard = (value: unknown) => {
  Object.defineProperty(navigator, 'clipboard', {
    value,
    configurable: true,
  })
}

const setExecCommand = (value: unknown) => {
  Object.defineProperty(document, 'execCommand', {
    value,
    configurable: true,
  })
}

describe('copyToClipboard', () => {
  afterEach(() => {
    setClipboard(undefined)
    setExecCommand(undefined)
  })

  it('uses the async Clipboard API when it is available', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    setClipboard({ writeText })

    await copyToClipboard('hello')

    expect(writeText).toHaveBeenCalledWith('hello')
  })

  it('falls back to execCommand when navigator.clipboard is undefined', async () => {
    // The Clipboard API is missing on insecure origins (plain HTTP / LAN IP),
    // which is exactly what issue #384 reported.
    setClipboard(undefined)
    const execCommand = vi.fn().mockReturnValue(true)
    setExecCommand(execCommand)

    await copyToClipboard('hello')

    expect(execCommand).toHaveBeenCalledWith('copy')
    expect(document.querySelector('textarea')).toBeNull()
  })

  it('throws when neither clipboard path is available', async () => {
    setClipboard(undefined)
    setExecCommand(undefined)

    await expect(copyToClipboard('hello')).rejects.toThrow(
      'Clipboard is not available'
    )
  })
})
