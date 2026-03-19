CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    content_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
    message TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (content_id) REFERENCES captured_content(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_chat_messages_content_id ON chat_messages(content_id);
