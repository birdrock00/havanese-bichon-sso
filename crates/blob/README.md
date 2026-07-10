# bichon-blob

Content-addressable blob store for [Bichon](https://github.com/rustmailer/bichon) email archival.

All data is keyed by a 32-byte content hash — identical content is stored only once. The caller (Bichon) strips attachments from emails and stores the "holed" raw email + attachments into this store, tracking reference counts in its own schema.

## On-disk layout

```
<root>/
├── meta.bin              # global metadata (bincode + CRC32)
├── segments/
│   ├── 00000001.seg      # append-only segment files (≤ 256 MB each)
│   ├── 00000002.seg
│   └── ...
└── buckets/
    ├── 00.idx            # 256 bucket index files (mmap'd, binary-searchable)
    ├── 01.idx
    └── ...
```

### Segment entry format (50-byte fixed header + variable data)

```
magic(4)  crc32(4)  flags(1)  codec(1)  key(32)  raw_size(4)  data_size(4)  data(*)
```

- `magic`: `0xB3DB_0001` — entry boundary validation
- `crc32`: covers everything after this field
- `flags`: `0` = live, `1` = tombstone
- `codec`: `0` = none, `1` = Zstd, `2` = Lz4
- `key`: 32-byte content hash (BLAKE3 / SHA-256)
- `raw_size`: original uncompressed size
- `data_size`: on-disk data size (after compression)

### Bucket index record format (52 bytes)

```
key(32)  segment_id(4)  offset(8)  data_size(4)  flags(1)  _pad(3)
```

Index records are sorted by key, deduplicated (newest segment_id + offset wins), and mmap'd for zero-heap binary search. Pending writes (since the last compaction) live in a small in-memory HashMap.

## Read path

```
get(key)
  → bucket_id = (key[0..2] as u16) % 256
  → check pending HashMap (most recent wins)
  → binary search mmap'd bucket file
  → IndexRecord → (segment_id, offset, data_size)
  → pread entry from segment file
  → CRC32 verify → decompress → return value
```

## Write path

```
put(key, value, codec)
  → compress value (Zstd/Lz4 if ≥ 4 KB, else store raw)
  → append entry to active segment file
  → insert IndexRecord into bucket store (append to .idx file + HashMap)
  → update metadata (indexed_up_to_offset)
  → if segment ≥ 256 MB → seal it, create new segment
```

## Delete

Deletes are **tombstones** — an entry with `flags=1` and empty data is appended. The bucket index maps the key to this tombstone. Read returns `None`. GC later reclaims the space.

## Compaction & GC

**Bucket compaction:** when a bucket's pending HashMap grows past `compact_threshold` (default 10,000), the mmap + pending records are merged, sorted, deduplicated, and atomically rewritten. This keeps binary search fast.

**Garbage collection:** when a sealed segment's deleted-ratio exceeds `gc_deleted_ratio` (default 0.30), GC scans all segments to find the latest entry per key, then rewrites the target segment keeping only live entries. Tombstones and overwritten entries are dropped. Bucket indices are rebuilt afterward.

## Crash recovery

- Temp files from interrupted GC are cleaned up on open.
- Any segment data beyond `indexed_up_to_offset` is scanned and indexed into bucket files.
- Partial writes at the tail of a segment (detected via CRC32 mismatch near EOF) are truncated.
- Buckets are always repairable by re-scanning segments (`rebuild_from_segments`).

## Background flush

Set `Config.flush_interval_secs` to a positive value to have a background thread fsync the active segment and save metadata periodically. This bounds recovery time after a crash at the cost of a small I/O overhead.

## Config

| Field | Default | Notes |
|---|---|---|
| `compress_threshold` | 4096 | Bytes; smaller values stored uncompressed |
| `default_codec` | Zstd | Also supports Lz4 |
| `compression_level` | 0 | Zstd compression level |
| `compact_threshold` | 10000 | Pending records per bucket before auto-compact |
| `gc_deleted_ratio` | 0.30 | Trigger GC when a sealed segment exceeds this |
| `flush_interval_secs` | 0 | 0 = disabled; ≥ 5 for periodic background fsync |

## Basic usage

```rust
use bichon_blob::{Codec, Config, Engine};

let engine = Engine::open(path, Config::default())?;

// Store
let hash = blake3::hash(b"email body").into();
engine.put(hash, b"email body", Codec::Zstd)?;

// Retrieve
let value = engine.get(&hash)?; // Some(Vec<u8>) or None

// Delete (caller must track refcounts)
engine.delete(&hash)?;

// Batch operations
engine.put_batch(&[(hash1, data1, Codec::Zstd), (hash2, data2, Codec::Lz4)])?;
engine.delete_batch(&[hash1, hash2])?;

// GC
engine.gc_if_needed()?;

// Clean shutdown
engine.shutdown()?;
```
