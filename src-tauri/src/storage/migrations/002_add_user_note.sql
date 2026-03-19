-- Migration 002: Add user_note field for Spotlight capture annotations

ALTER TABLE captured_content ADD COLUMN user_note TEXT DEFAULT NULL;
