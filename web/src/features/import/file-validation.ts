//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project

// MIME types that are clearly NOT email files — reject these upfront.
const BLOCKED_MIME_PREFIXES = [
  'video/', 'audio/', 'image/', 'font/',
  'application/zip', 'application/gzip', 'application/x-tar',
  'application/x-7z', 'application/x-rar',
  'application/vnd.', 'application/pdf',
  'application/x-msdownload', 'application/x-executable',
];

// Known-good MIME types for supported extensions. Browsers/OSes report .pst
// as application/vnd.ms-outlook (e.g. via shared-mime-info) — don't let the
// generic vnd. blocklist below reject it.
const ALLOWED_MIME_TYPES = new Set([
  'application/vnd.ms-outlook',
  'application/x-pst',
  'application/mbox',
]);

export function isValidFileType(file: File, ext: string): boolean {
  // Extension is authoritative: only .eml, .mbox and .pst are accepted.
  if (ext !== 'eml' && ext !== 'mbox' && ext !== 'pst') return false;

  const mime = file.type.toLowerCase();
  if (mime && ALLOWED_MIME_TYPES.has(mime)) return true;

  // Otherwise, reject known binary types (e.g. a renamed video/audio file)
  if (mime) {
    for (const prefix of BLOCKED_MIME_PREFIXES) {
      if (mime.startsWith(prefix)) return false;
    }
  }
  return true;
}
