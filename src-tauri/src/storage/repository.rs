use super::database::Database;
use super::models::{
    CapturedContent, ContentType, ReportSection, UserFeedback, UserPreference, WeeklyReport,
};
use rusqlite::params;
use std::sync::Arc;

pub struct Repository {
    db: Arc<Database>,
}

impl Repository {
    pub fn new(db: Arc<Database>) -> Self {
        Repository { db }
    }

    // ========== Captured Content ==========

    pub fn save_content(
        &self,
        content: &CapturedContent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "INSERT INTO captured_content (id, content_type, raw_text, image_path, thumbnail_path, source_app, source_bundle_id, source_url, user_note, captured_at, content_hash, byte_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                content.id,
                content.content_type.as_str(),
                content.raw_text,
                content.image_path,
                content.thumbnail_path,
                content.source_app,
                content.source_bundle_id,
                content.source_url,
                content.user_note,
                content.captured_at,
                content.content_hash,
                content.byte_size,
            ],
        )?;
        Ok(())
    }

    pub fn get_all_content(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CapturedContent>, Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, content_type, raw_text, image_path, thumbnail_path, source_app, source_bundle_id, source_url, user_note, captured_at, content_hash, byte_size, is_deleted, created_at, updated_at
             FROM captured_content WHERE is_deleted = 0 ORDER BY captured_at DESC LIMIT ?1 OFFSET ?2"
        )?;

        let rows = stmt.query_map(params![limit, offset], |row| {
            Ok(CapturedContent {
                id: row.get(0)?,
                content_type: ContentType::from_str(&row.get::<_, String>(1)?),
                raw_text: row.get(2)?,
                image_path: row.get(3)?,
                thumbnail_path: row.get(4)?,
                source_app: row.get(5)?,
                source_bundle_id: row.get(6)?,
                source_url: row.get(7)?,
                user_note: row.get(8)?,
                captured_at: row.get(9)?,
                content_hash: row.get(10)?,
                byte_size: row.get(11)?,
                is_deleted: row.get::<_, i32>(12)? != 0,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Update the raw_text and source_url of an existing content item.
    /// Used by the URL reader to fill in fetched article content.
    pub fn update_content_for_url(
        &self,
        id: &str,
        raw_text: &str,
        source_url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE captured_content SET raw_text = ?1, source_url = ?2, byte_size = ?3, updated_at = datetime('now') WHERE id = ?4",
            params![raw_text, source_url, raw_text.len() as i64, id],
        )?;
        Ok(())
    }

    /// Update the raw_text of an existing content item (e.g. OCR result for images).
    pub fn update_raw_text(
        &self,
        id: &str,
        raw_text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE captured_content SET raw_text = ?1, byte_size = ?2, updated_at = datetime('now') WHERE id = ?3",
            params![raw_text, raw_text.len() as i64, id],
        )?;
        Ok(())
    }

    pub fn delete_content(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE captured_content SET is_deleted = 1, updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Update the user_note for an existing content item.
    pub fn update_user_note(
        &self,
        id: &str,
        note: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE captured_content SET user_note = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![note, id],
        )?;
        Ok(())
    }

    /// Find an existing content item by its content_hash (for dedup in spotlight).
    pub fn find_content_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<CapturedContent>, Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, content_type, raw_text, image_path, thumbnail_path, source_app, source_bundle_id, source_url, user_note, captured_at, content_hash, byte_size, is_deleted, created_at, updated_at
             FROM captured_content WHERE content_hash = ?1 AND is_deleted = 0 LIMIT 1"
        )?;

        let mut rows = stmt.query_map(params![hash], |row| {
            Ok(CapturedContent {
                id: row.get(0)?,
                content_type: ContentType::from_str(&row.get::<_, String>(1)?),
                raw_text: row.get(2)?,
                image_path: row.get(3)?,
                thumbnail_path: row.get(4)?,
                source_app: row.get(5)?,
                source_bundle_id: row.get(6)?,
                source_url: row.get(7)?,
                user_note: row.get(8)?,
                captured_at: row.get(9)?,
                content_hash: row.get(10)?,
                byte_size: row.get(11)?,
                is_deleted: row.get::<_, i32>(12)? != 0,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn content_exists_by_hash(&self, hash: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM captured_content WHERE content_hash = ?1 AND is_deleted = 0",
            params![hash],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get all content captured between week_start and week_end (inclusive).
    /// Dates should be in ISO 8601 / RFC 3339 format (e.g. "2025-01-06T00:00:00+00:00").
    pub fn get_content_for_week(
        &self,
        week_start: &str,
        week_end: &str,
    ) -> Result<Vec<CapturedContent>, Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, content_type, raw_text, image_path, thumbnail_path, source_app, source_bundle_id, source_url, user_note, captured_at, content_hash, byte_size, is_deleted, created_at, updated_at
             FROM captured_content
             WHERE is_deleted = 0 AND captured_at >= ?1 AND captured_at <= ?2
             ORDER BY captured_at DESC"
        )?;

        let rows = stmt.query_map(params![week_start, week_end], |row| {
            Ok(CapturedContent {
                id: row.get(0)?,
                content_type: ContentType::from_str(&row.get::<_, String>(1)?),
                raw_text: row.get(2)?,
                image_path: row.get(3)?,
                thumbnail_path: row.get(4)?,
                source_app: row.get(5)?,
                source_bundle_id: row.get(6)?,
                source_url: row.get(7)?,
                user_note: row.get(8)?,
                captured_at: row.get(9)?,
                content_hash: row.get(10)?,
                byte_size: row.get(11)?,
                is_deleted: row.get::<_, i32>(12)? != 0,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get a single content item by its ID.
    pub fn get_content_by_id(
        &self,
        id: &str,
    ) -> Result<Option<CapturedContent>, Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, content_type, raw_text, image_path, thumbnail_path, source_app, source_bundle_id, source_url, user_note, captured_at, content_hash, byte_size, is_deleted, created_at, updated_at
             FROM captured_content WHERE id = ?1 AND is_deleted = 0"
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            Ok(CapturedContent {
                id: row.get(0)?,
                content_type: ContentType::from_str(&row.get::<_, String>(1)?),
                raw_text: row.get(2)?,
                image_path: row.get(3)?,
                thumbnail_path: row.get(4)?,
                source_app: row.get(5)?,
                source_bundle_id: row.get(6)?,
                source_url: row.get(7)?,
                user_note: row.get(8)?,
                captured_at: row.get(9)?,
                content_hash: row.get(10)?,
                byte_size: row.get(11)?,
                is_deleted: row.get::<_, i32>(12)? != 0,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    // ========== Weekly Reports ==========

    /// Save a complete weekly report with its sections to the database.
    pub fn save_report(&self, report: &WeeklyReport) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        // Insert the report
        conn.execute(
            "INSERT OR REPLACE INTO weekly_reports (id, week_start, week_end, summary_text, report_json, content_count, model_used, tokens_used, generated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                report.id,
                report.week_start,
                report.week_end,
                report.summary_text,
                report.report_json.to_string(),
                report.content_count,
                report.model_used,
                report.tokens_used,
                report.generated_at,
            ],
        )?;

        // Delete old sections for this report (in case of regeneration)
        conn.execute(
            "DELETE FROM report_sections WHERE report_id = ?1",
            params![report.id],
        )?;

        // Insert sections
        for section in &report.sections {
            let content_ids_json = serde_json::to_string(&section.content_ids)
                .unwrap_or_else(|_| "[]".to_string());

            conn.execute(
                "INSERT INTO report_sections (id, report_id, section_type, title, body, relevance_score, sort_order, content_ids)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    section.id,
                    section.report_id,
                    section.section_type,
                    section.title,
                    section.body,
                    section.relevance_score,
                    section.sort_order,
                    content_ids_json,
                ],
            )?;
        }

        Ok(())
    }

    /// Get a weekly report for a specific week_start date.
    pub fn get_report_by_week(
        &self,
        week_start: &str,
    ) -> Result<Option<WeeklyReport>, Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT id, week_start, week_end, summary_text, report_json, content_count, model_used, tokens_used, generated_at
             FROM weekly_reports WHERE week_start = ?1"
        )?;

        let mut rows = stmt.query_map(params![week_start], |row| {
            let report_json_str: String = row.get(4)?;
            let report_json: serde_json::Value =
                serde_json::from_str(&report_json_str).unwrap_or(serde_json::Value::Null);

            Ok(WeeklyReport {
                id: row.get(0)?,
                week_start: row.get(1)?,
                week_end: row.get(2)?,
                summary_text: row.get(3)?,
                report_json,
                content_count: row.get(5)?,
                model_used: row.get(6)?,
                tokens_used: row.get(7)?,
                generated_at: row.get(8)?,
                sections: Vec::new(), // filled below
            })
        })?;

        let report = match rows.next() {
            Some(row) => row?,
            None => return Ok(None),
        };

        // Load sections for this report
        let sections = self.get_sections_for_report_inner(&conn, &report.id)?;

        Ok(Some(WeeklyReport {
            sections,
            ..report
        }))
    }

    /// List all weekly reports (without full sections, just metadata).
    pub fn get_all_reports(&self) -> Result<Vec<WeeklyReport>, Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT id, week_start, week_end, summary_text, report_json, content_count, model_used, tokens_used, generated_at
             FROM weekly_reports ORDER BY week_start DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            let report_json_str: String = row.get(4)?;
            let report_json: serde_json::Value =
                serde_json::from_str(&report_json_str).unwrap_or(serde_json::Value::Null);

            Ok(WeeklyReport {
                id: row.get(0)?,
                week_start: row.get(1)?,
                week_end: row.get(2)?,
                summary_text: row.get(3)?,
                report_json,
                content_count: row.get(5)?,
                model_used: row.get(6)?,
                tokens_used: row.get(7)?,
                generated_at: row.get(8)?,
                sections: Vec::new(),
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Internal helper: load sections for a report using an already-locked connection.
    fn get_sections_for_report_inner(
        &self,
        conn: &rusqlite::Connection,
        report_id: &str,
    ) -> Result<Vec<ReportSection>, Box<dyn std::error::Error>> {
        let mut stmt = conn.prepare(
            "SELECT id, report_id, section_type, title, body, relevance_score, sort_order, content_ids
             FROM report_sections WHERE report_id = ?1 ORDER BY sort_order"
        )?;

        let rows = stmt.query_map(params![report_id], |row| {
            let content_ids_str: Option<String> = row.get(7)?;
            let content_ids: Vec<String> = content_ids_str
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            Ok(ReportSection {
                id: row.get(0)?,
                report_id: row.get(1)?,
                section_type: row.get(2)?,
                title: row.get(3)?,
                body: row.get(4)?,
                relevance_score: row.get(5)?,
                sort_order: row.get(6)?,
                content_ids,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    // ========== User Feedback ==========

    /// Save user feedback (interested/dismissed/bookmarked) for a content or section.
    pub fn save_feedback(
        &self,
        feedback: &UserFeedback,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        conn.execute(
            "INSERT INTO user_feedback (id, content_id, section_id, feedback_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                feedback.id,
                feedback.content_id,
                feedback.section_id,
                feedback.feedback_type.as_str(),
                feedback.created_at,
            ],
        )?;

        Ok(())
    }

    // ========== User Preferences ==========

    /// Update or insert a topic preference. Increases weight by weight_delta
    /// and increments occurrence_count.
    pub fn update_preference(
        &self,
        topic: &str,
        weight_delta: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        // Try to update existing preference
        let rows_updated = conn.execute(
            "UPDATE user_preferences SET weight = weight + ?1, occurrence_count = occurrence_count + 1, last_updated = datetime('now')
             WHERE topic = ?2",
            params![weight_delta, topic],
        )?;

        // If no existing row, insert a new one
        if rows_updated == 0 {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO user_preferences (id, topic, weight, occurrence_count, last_updated)
                 VALUES (?1, ?2, ?3, 1, datetime('now'))",
                params![id, topic, weight_delta],
            )?;
        }

        Ok(())
    }

    /// Get all user preferences ordered by weight descending.
    pub fn get_all_preferences(
        &self,
    ) -> Result<Vec<UserPreference>, Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT id, topic, weight, occurrence_count, last_updated
             FROM user_preferences ORDER BY weight DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(UserPreference {
                id: row.get(0)?,
                topic: row.get(1)?,
                weight: row.get(2)?,
                occurrence_count: row.get(3)?,
                last_updated: row.get(4)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    // ========== Chat Messages ==========

    /// Save a chat message for a content item.
    pub fn save_chat_message(
        &self,
        content_id: &str,
        role: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO chat_messages (id, content_id, role, message) VALUES (?1, ?2, ?3, ?4)",
            params![id, content_id, role, message],
        )?;
        Ok(())
    }

    /// Get all chat messages for a content item, ordered by creation time.
    pub fn get_chat_messages(
        &self,
        content_id: &str,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT role, message FROM chat_messages WHERE content_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![content_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Delete all chat messages for a content item.
    pub fn delete_chat_messages(
        &self,
        content_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "DELETE FROM chat_messages WHERE content_id = ?1",
            params![content_id],
        )?;
        Ok(())
    }

    // ========== App Settings ==========

    /// Get a setting value by key.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get all settings as key-value pairs.
    pub fn get_all_settings(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let mut stmt = conn.prepare("SELECT key, value FROM app_settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut settings = std::collections::HashMap::new();
        for row in rows {
            let (key, value) = row?;
            settings.insert(key, value);
        }
        Ok(settings)
    }

    /// Update a setting value by key.
    pub fn update_setting(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .db
            .conn
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
            params![key, value],
        )?;
        Ok(())
    }
}
