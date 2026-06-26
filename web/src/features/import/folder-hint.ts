//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project

/**
 * Parse raw EML/MBOX headers from the first few KB of a file and return a
 * suggested folder name, or null if nothing useful was found.
 *
 * Mirrors the CLI logic in crates/cli/src/mbox/gmail.rs (determine_folder).
 */

const HEADER_READ_BYTES = 64 * 1024; // read first 64 KB to get headers

/** RFC 2047 encoded-word prefix. We do a best-effort decode. */
function decodeRfc2047(raw: string): string {
  return raw.replace(/=\?[^?]+\?[BbQq]\?[^?]*\?=/gi, (match) => {
    try {
      const parts = match.split('?');
      const charset = parts[1];
      const encoding = parts[2].toUpperCase();
      const encoded = parts[3];
      let bytes: Uint8Array;
      if (encoding === 'B') {
        const bin = atob(encoded);
        bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      } else {
        // Q-encoding
        const hex = encoded.replace(/_/g, ' ').replace(/=([0-9A-Fa-f]{2})/g, (_, h) =>
          String.fromCharCode(parseInt(h, 16)),
        );
        bytes = new TextEncoder().encode(hex);
      }
      return new TextDecoder(charset).decode(bytes);
    } catch {
      return match;
    }
  });
}

/** Extract a single header value from raw email text. Case-insensitive. */
function getHeader(raw: string, name: string): string | null {
  const re = new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*:\\s*(.+)$`, 'im');
  const m = raw.match(re);
  if (!m) return null;
  // Unfold continuation lines (leading whitespace)
  let val = m[1].trim();
  const startIdx = m.index! + m[0].length;
  const rest = raw.slice(startIdx);
  const contRe = /^\s+(.+)$/gm;
  let cm: RegExpExecArray | null;
  while ((cm = contRe.exec(rest)) !== null) {
    val += ' ' + cm[1].trim();
  }
  return decodeRfc2047(val);
}

/** Determine folder from X-Gmail-Labels, mirroring the CLI's determine_folder(). */
function folderFromGmailLabels(raw: string): string | null {
  const labelsRaw = getHeader(raw, 'X-Gmail-Labels');
  if (!labelsRaw) return null;

  const statusBlacklist = new Set(['Opened', 'Unread', 'Archived']);
  const allLabels = labelsRaw.split(',').map((s) => s.trim()).filter(Boolean);
  if (allLabels.length === 0) return null;

  const filtered = allLabels.filter((l) => !statusBlacklist.has(l));
  if (filtered.length === 0) return allLabels[0];
  if (filtered.length === 1) return filtered[0];

  // Prefer business labels over generic Inbox/Sent
  const business = filtered.find((l) => l !== 'Inbox' && l !== 'Sent');
  return business ?? filtered[0];
}

/** Try to read mailbox_name from X-Bichon-Metadata JSON header. */
function folderFromBichonMetadata(raw: string): string | null {
  const metaRaw = getHeader(raw, 'X-Bichon-Metadata');
  if (!metaRaw) return null;
  try {
    const meta = JSON.parse(metaRaw);
    if (meta?.mailbox_name && typeof meta.mailbox_name === 'string') {
      return meta.mailbox_name;
    }
  } catch {
    // ignore parse errors
  }
  return null;
}

/** Derive a folder from the file name (e.g. "Inbox.mbox" → "Inbox"). */
function folderFromFileName(fileName: string): string | null {
  const base = fileName.replace(/\.[^.]+$/, ''); // strip extension
  if (!base || base === fileName) return null;
  // Common patterns
  if (/^[a-zA-Z0-9_/\-.\s]+$/.test(base) && base.length > 0 && base.length < 128) {
    return base;
  }
  return null;
}

export interface FolderHint {
  /** The suggested folder name. */
  name: string;
  /** Where the hint came from. */
  source: 'gmail-labels' | 'bichon-metadata' | 'filename' | 'mbox-filename';
}

/**
 * Read the first chunk of a File and return folder hints extracted from headers.
 * Returns null if no hint could be extracted.
 */
export async function extractFolderHint(file: File): Promise<FolderHint | null> {
  const ext = file.name.split('.').pop()?.toLowerCase();
  const isMbox = ext === 'mbox';

  // Read first 64 KB — enough for headers of the first message
  const chunk = new Uint8Array(await file.slice(0, HEADER_READ_BYTES).arrayBuffer());
  const raw = new TextDecoder('utf-8', { fatal: false }).decode(chunk);

  // MBOX: the first line is "From ...", headers start after the first newline
  const headers = isMbox
    ? raw.replace(/^From [^\n]*\n/, '') // strip MBOX "From " separator
    : raw;

  // 1. X-Bichon-Metadata (highest priority, explicit)
  const bichonFolder = folderFromBichonMetadata(headers);
  if (bichonFolder) return { name: bichonFolder, source: 'bichon-metadata' };

  // 2. X-Gmail-Labels
  const gmailFolder = folderFromGmailLabels(headers);
  if (gmailFolder) return { name: gmailFolder, source: 'gmail-labels' };

  // 3. For MBOX files, use the filename
  if (isMbox) {
    const fnFolder = folderFromFileName(file.name);
    if (fnFolder) return { name: fnFolder, source: 'mbox-filename' };
  }

  // 4. For EML files, try the filename
  const fnFolder = folderFromFileName(file.name);
  if (fnFolder) return { name: fnFolder, source: 'filename' };

  return null;
}
