-- Migration 004: Add digest fields for the "消化" feature
ALTER TABLE captured_content ADD COLUMN digested_at TEXT;
ALTER TABLE captured_content ADD COLUMN digest_action TEXT;

-- Index for efficient undigested content queries
CREATE INDEX IF NOT EXISTS idx_content_undigested ON captured_content(digested_at, captured_at);
