-- Migration 010: Multi-turn chat sessions for wiki Q&A

CREATE TABLE IF NOT EXISTS wiki_chat_sessions (
    id              TEXT PRIMARY KEY,
    title           TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS wiki_chat_messages (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL REFERENCES wiki_chat_sessions(id) ON DELETE CASCADE,
    role            TEXT NOT NULL,
    content         TEXT NOT NULL,
    pages_used      TEXT,
    source_mode     TEXT,
    turn_index      INTEGER NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON wiki_chat_messages(session_id);
CREATE INDEX IF NOT EXISTS idx_chat_messages_order ON wiki_chat_messages(session_id, turn_index);
