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
        // Q-encoding: assemble real bytes. Literal chars become UTF-8 bytes,
        // `=HH` is one byte (so a multi-byte UTF-8 char like `=C3=A9` decodes
        // as `é`, not as two Latin-1 code points `Ã©`).
        const out: number[] = [];
        let i = 0;
        while (i < encoded.length) {
          const ch = encoded[i];
          if (ch === '_') {
            out.push(0x20);
            i += 1;
          } else if (ch === '=' && /^[0-9A-Fa-f]{2}$/.test(encoded.slice(i + 1, i + 3))) {
            out.push(parseInt(encoded.slice(i + 1, i + 3), 16));
            i += 3;
          } else {
            for (const b of new TextEncoder().encode(ch)) out.push(b);
            i += 1;
          }
        }
        bytes = new Uint8Array(out);
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
  // Unfold continuation lines: only contiguous lines starting with
  // horizontal whitespace belong to this header (RFC 5322 folding).
  let val = m[1].trim();
  const startIdx = m.index! + m[0].length;
  const rest = raw.slice(startIdx);
  for (const line of rest.split(/\r?\n/).slice(1)) {
    if (!/^[ \t]+\S/.test(line)) break;
    val += ' ' + line.trim();
  }
  return decodeRfc2047(val);
}

/**
 * Gmail system labels that describe message *state* rather than a folder.
 * They must never become the import destination. Takeout localizes these in
 * X-Gmail-Labels (e.g. French "Archivés"), so match the English canonical
 * tokens plus the known translations. Must stay in sync with
 * crates/cli/src/mbox/gmail.rs. Best-effort; only ever add *system* labels
 * here, never user-created ones (those are never localized).
 */
const STATUS_LABELS = new Set([
  // English (canonical Takeout tokens)
  'Opened', 'Unread', 'Archived',
  // French
  'Ouverts', 'Non lus', 'Archivés',
  // Spanish
  'Abiertos', 'No leídos', 'Archivados',
  // Italian
  'Aperti', 'Non letti', 'Archiviati',
  // Portuguese
  'Abertos', 'Não lidos', 'Arquivados',
  // German
  'Geöffnet', 'Ungelesen', 'Archiviert',
  // Dutch
  'Geopend', 'Ongelezen', 'Gearchiveerd',
  // Swedish
  'Öppnade', 'Olästa', 'Arkiverade',
  // Danish
  'Åbnet', 'Ulæst', 'Arkiveret',
  // Norwegian
  'Åpnet', 'Ulest', 'Arkivert',
  // Finnish
  'Avatut', 'Lukemattomat', 'Arkistoidut',
  // Polish
  'Otworzone', 'Nieprzeczytane', 'Zarchiwizowane',
  // Czech
  'Otevřené', 'Nepřečtené', 'Archivováno',
  // Hungarian
  'Megnyitott', 'Olvasatlan', 'Archivált',
  // Romanian
  'Deschise', 'Necitite', 'Arhivate',
  // Russian
  'Открытые', 'Непрочитанные', 'В архиве',
  // Ukrainian
  'Відкриті', 'Непрочитані', 'В архіві',
  // Turkish
  'Açıldı', 'Okunmamış', 'Arşivlendi',
  // Greek
  'Ανοιγμένα', 'Μη αναγνωσμένα', 'Αρχειοθέτηση',
])

/** Gmail generic location folders (real folders, never a business destination). */
const GENERIC_LOCATION_LABELS = new Set([
  // English
  'Inbox', 'Sent',
  // French
  'Boîte de réception', 'Envoyé',
  // Spanish
  'Recibidos', 'Enviados',
  // Italian
  'Posta in arrivo', 'Inviati',
  // Portuguese
  'Caixa de entrada', 'Enviados',
  // German
  'Posteingang', 'Gesendet',
  // Dutch
  'Postvak IN', 'Verzonden items',
  // Swedish
  'Inkorgen', 'Skickade',
  // Danish
  'Indbakke', 'Sendt',
  // Norwegian
  'Innboks', 'Sendt',
  // Finnish
  'Saapuneet', 'Lähetetyt',
  // Polish
  'Odebrane', 'Wysłane',
  // Czech
  'Doručená pošta', 'Odeslané',
  // Hungarian
  'Beérkezett üzenetek', 'Elküldött üzenetek',
  // Romanian
  'Căsuța de intrare', 'Trimise',
  // Russian
  'Входящие', 'Отправленные',
  // Ukrainian
  'Вхідні', 'Надіслані',
  // Turkish
  'Gelen Kutusu', 'Gönderilmiş',
  // Greek
  'Εισερχόμενα', 'Απεσταλμένα',
])

/** Determine folder from X-Gmail-Labels, mirroring the CLI's determine_folder(). */
function folderFromGmailLabels(raw: string): string | null {
  const labelsRaw = getHeader(raw, 'X-Gmail-Labels');
  if (!labelsRaw) return null;

  const allLabels = labelsRaw.split(',').map((s) => s.trim()).filter(Boolean);
  if (allLabels.length === 0) return null;

  const filtered = allLabels.filter((l) => !STATUS_LABELS.has(l));
  if (filtered.length === 0) return allLabels[0];
  if (filtered.length === 1) return filtered[0];

  // Prefer business labels over generic Inbox/Sent
  const business = filtered.find((l) => !GENERIC_LOCATION_LABELS.has(l));
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
  source: 'gmail-labels' | 'bichon-metadata' | 'filename' | 'mbox-filename' | 'pst-filename';
  /** Name of the file the hint was extracted from. */
  fileName: string;
}

/**
 * Read the first chunk of a File and return folder hints extracted from headers.
 * Returns null if no hint could be extracted.
 */
export async function extractFolderHint(file: File): Promise<FolderHint | null> {
  const ext = file.name.split('.').pop()?.toLowerCase();
  const isMbox = ext === 'mbox';
  const isPst = ext === 'pst';

  // Read first 64 KB — enough for headers of the first message
  const chunk = new Uint8Array(await file.slice(0, HEADER_READ_BYTES).arrayBuffer());
  const raw = new TextDecoder('utf-8', { fatal: false }).decode(chunk);

  // MBOX: the first line is "From ...", headers start after the first newline
  const headers = isMbox
    ? raw.replace(/^From [^\n]*\n/, '') // strip MBOX "From " separator
    : raw;

  // 1. X-Bichon-Metadata (highest priority, explicit)
  const bichonFolder = folderFromBichonMetadata(headers);
  if (bichonFolder) return { name: bichonFolder, source: 'bichon-metadata', fileName: file.name };

  // 2. X-Gmail-Labels
  const gmailFolder = folderFromGmailLabels(headers);
  if (gmailFolder) return { name: gmailFolder, source: 'gmail-labels', fileName: file.name };

  // 3. For MBOX files, use the filename
  if (isMbox) {
    const fnFolder = folderFromFileName(file.name);
    if (fnFolder) return { name: fnFolder, source: 'mbox-filename', fileName: file.name };
  }

  // 4. For EML files, try the filename
  const fnFolder = folderFromFileName(file.name);
  if (fnFolder) return { name: fnFolder, source: 'filename', fileName: file.name };

  // 5. For PST files, try the filename
  if (isPst) {
    const fnFolder = folderFromFileName(file.name);
    if (fnFolder) return { name: fnFolder, source: 'pst-filename', fileName: file.name };
  }

  return null;
}
