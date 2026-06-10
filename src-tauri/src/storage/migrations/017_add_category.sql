-- Migration 017: Add category column to captured_content.
-- Category is the AI-assigned notebook this capture belongs to.
-- NULL = captured before categories existed (shown as "未分类").
ALTER TABLE captured_content ADD COLUMN category TEXT;
CREATE INDEX IF NOT EXISTS idx_content_category ON captured_content(category);
