-- Migration 011: Add source_message_id to wiki_pages for Q&A dedup
ALTER TABLE wiki_pages ADD COLUMN source_message_id TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_wiki_pages_source_msg ON wiki_pages(source_message_id) WHERE source_message_id IS NOT NULL;
