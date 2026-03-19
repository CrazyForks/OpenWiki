use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let db_path = Self::get_db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let db = Database {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;

        Ok(db)
    }

    fn get_db_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let data_dir = dirs::data_dir()
            .ok_or("Could not find data directory")?
            .join("com.xiaoyun.app");
        Ok(data_dir.join("xiaoyun.db"))
    }

    fn run_migrations(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        let migration_sql = include_str!("migrations/001_initial.sql");
        conn.execute_batch(migration_sql)?;

        // Migration 002: Add user_note column (idempotent check)
        let has_user_note: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('captured_content') WHERE name = 'user_note'")?
            .query_row([], |row| row.get::<_, i32>(0))
            .map(|count| count > 0)
            .unwrap_or(false);

        if !has_user_note {
            let migration_002 = include_str!("migrations/002_add_user_note.sql");
            conn.execute_batch(migration_002)?;
            log::info!("Migration 002 applied: added user_note column");
        }

        // Migration 003: Add chat_messages table
        let has_chat_messages: bool = conn
            .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='chat_messages'")?
            .query_row([], |row| row.get::<_, i32>(0))
            .map(|count| count > 0)
            .unwrap_or(false);

        if !has_chat_messages {
            let migration_003 = include_str!("migrations/003_add_chat_messages.sql");
            conn.execute_batch(migration_003)?;
            log::info!("Migration 003 applied: added chat_messages table");
        }

        log::info!("Database migrations completed successfully");
        Ok(())
    }
}
