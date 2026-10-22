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
 * Copy text to the clipboard.
 *
 * The async Clipboard API only exists in secure contexts (HTTPS or
 * localhost). Bichon is often self-hosted over plain HTTP or reached through
 * a LAN IP, where `navigator.clipboard` is undefined and calling
 * `navigator.clipboard.writeText` throws
 * "Cannot read properties of undefined (reading 'writeText')". Fall back to
 * the legacy execCommand path in that case.
 */
export async function copyToClipboard(text: string): Promise<void> {
  const clipboard =
    typeof navigator === 'undefined' ? undefined : navigator.clipboard
  if (clipboard && typeof clipboard.writeText === 'function') {
    try {
      await clipboard.writeText(text)
      return
    } catch {
      // Permission denied or a document that is not focused: try the fallback.
    }
  }

  if (!copyWithExecCommand(text)) {
    throw new Error('Clipboard is not available')
  }
}

/** Legacy fallback for insecure contexts: select a hidden textarea and copy. */
function copyWithExecCommand(text: string): boolean {
  if (typeof document === 'undefined') return false

  const textarea = document.createElement('textarea')
  textarea.value = text
  // Keep it out of view without scrolling the page.
  textarea.style.position = 'fixed'
  textarea.style.top = '0'
  textarea.style.left = '-9999px'
  textarea.setAttribute('readonly', '')
  document.body.appendChild(textarea)

  const selection = document.getSelection()
  const previousRange =
    selection && selection.rangeCount > 0 ? selection.getRangeAt(0) : null

  try {
    textarea.select()
    textarea.setSelectionRange(0, text.length)
    return document.execCommand('copy')
  } catch {
    return false
  } finally {
    document.body.removeChild(textarea)
    if (previousRange && selection) {
      selection.removeAllRanges()
      selection.addRange(previousRange)
    }
  }
}
