-- =========================
-- envelopes (main table)
-- =========================
CREATE TABLE IF NOT EXISTS envelopes (
    -- internal id (tantivy f_id)
    id UUID PRIMARY KEY,

    -- account / mailbox / uid
    account_id        UBIGINT NOT NULL,
    mailbox_id        UBIGINT NOT NULL,
    uid               UBIGINT NOT NULL,

    -- headers / content
    content_hash      VARCHAR(64) NOT NULL,
    subject           TEXT,
    body              TEXT,

    sender            TEXT,
    recipients        VARCHAR[],
    cc                VARCHAR[],
    bcc               VARCHAR[],

    -- dates
    sent_at           BIGINT,
    received_at       BIGINT,

    -- size
    size_bytes        UBIGINT,

    -- thread
    thread_id         VARCHAR NOT NULL,

    -- message-id
    message_id        TEXT,

    -- attachment summary
    attachment_count  INTEGER NOT NULL CHECK (attachment_count >= 0),
    regular_attachment_count INTEGER NOT NULL CHECK (regular_attachment_count >= 0),
    tags              VARCHAR[],
    shard_id          UBIGINT NOT NULL
);

-- =========================
-- envelope_attachments
--
-- Normalized attachment metadata for fast filtering and UI display
-- =========================
CREATE TABLE IF NOT EXISTS envelope_attachments (
    -- Reference to envelopes.id
    envelope_id      UUID NOT NULL,
    account_id       UBIGINT NOT NULL,
    mailbox_id       UBIGINT NOT NULL,
    -- Original attachment filename (for display)
    filename         TEXT,
    is_message       BOOLEAN NOT NULL DEFAULT FALSE,
    is_inline        BOOLEAN NOT NULL DEFAULT FALSE,
    cid              TEXT,
    -- Normalized file extension (lowercase, without dot)
    extension        TEXT,

    -- Extension category (document / image / archive / ...)
    ext_category     TEXT NOT NULL,
    content_type     TEXT NOT NULL,
    -- Attachment size in bytes
    -- 0 if unknown
    size_bytes       UBIGINT NOT NULL,
    content_hash         VARCHAR(64) NOT NULL,
    shard_id      UINTEGER NOT NULL
);