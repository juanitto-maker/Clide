// ============================================
// database.rs - Database Abstraction
// ============================================
// SQLite-based persistent storage

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OpenFlags};
use std::path::Path;
use tracing::{debug, info};

/// Database manager
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create database
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        info!("Opening database: {:?}", path);

        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .context("Failed to open database")?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        let db = Self { conn };
        db.init_schema()?;

        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        debug!("Initializing database schema");

        // Conversations table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user TEXT NOT NULL,
                message TEXT NOT NULL,
                response TEXT,
                timestamp INTEGER NOT NULL,
                command TEXT,
                exit_code INTEGER,
                duration_ms INTEGER
            )",
            [],
        )?;

        // Context/Memory table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS context (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                UNIQUE(user, key)
            )",
            [],
        )?;

        // Skills execution history
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS skill_executions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                skill_name TEXT NOT NULL,
                user TEXT NOT NULL,
                input TEXT,
                output TEXT,
                success INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        // Workflows
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS workflows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                steps TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Scheduled tasks
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS scheduled_tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                command TEXT NOT NULL,
                schedule TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                last_run INTEGER,
                next_run INTEGER NOT NULL
            )",
            [],
        )?;

        // User preferences
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS user_preferences (
                user TEXT PRIMARY KEY,
                preferences TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indices
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_conversations_user 
             ON conversations(user, timestamp DESC)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_context_user 
             ON context(user)",
            [],
        )?;

        info!("Database schema initialized");
        Ok(())
    }

    /// Save conversation
    pub fn save_conversation(
        &self,
        user: &str,
        message: &str,
        response: Option<&str>,
        command: Option<&str>,
        exit_code: Option<i32>,
        duration_ms: Option<u64>,
    ) -> Result<i64> {
        let timestamp = chrono::Utc::now().timestamp();

        let id = self.conn.execute(
            "INSERT INTO conversations 
             (user, message, response, timestamp, command, exit_code, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                user,
                message,
                response,
                timestamp,
                command,
                exit_code,
                duration_ms.map(|d| d as i64)
            ],
        )?;

        Ok(id as i64)
    }

    /// Get recent conversations for user
    pub fn get_conversations(&self, user: &str, limit: usize) -> Result<Vec<Conversation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user, message, response, timestamp, command, exit_code, duration_ms
             FROM conversations
             WHERE user = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let conversations = stmt
            .query_map(params![user, limit], |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    user: row.get(1)?,
                    message: row.get(2)?,
                    response: row.get(3)?,
                    timestamp: row.get(4)?,
                    command: row.get(5)?,
                    exit_code: row.get(6)?,
                    duration_ms: row.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(conversations)
    }

    /// Save context key-value
    pub fn set_context(&self, user: &str, key: &str, value: &str) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp();

        self.conn.execute(
            "INSERT OR REPLACE INTO context (user, key, value, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![user, key, value, timestamp],
        )?;

        Ok(())
    }

    /// Get context value
    pub fn get_context(&self, user: &str, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT value FROM context WHERE user = ?1 AND key = ?2",
        )?;

        let result = stmt
            .query_row(params![user, key], |row| row.get(0))
            .optional()?;

        Ok(result)
    }

    /// Get all context for user
    pub fn get_all_context(&self, user: &str) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT key, value FROM context WHERE user = ?1",
        )?;

        let contexts = stmt
            .query_map(params![user], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(contexts)
    }

    /// Save skill execution
    pub fn save_skill_execution(
        &self,
        skill_name: &str,
        user: &str,
        input: Option<&str>,
        output: Option<&str>,
        success: bool,
    ) -> Result<i64> {
        let timestamp = chrono::Utc::now().timestamp();

        let id = self.conn.execute(
            "INSERT INTO skill_executions 
             (skill_name, user, input, output, success, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![skill_name, user, input, output, success as i32, timestamp],
        )?;

        Ok(id as i64)
    }

    /// Get conversation statistics
    pub fn get_stats(&self, user: Option<&str>) -> Result<Stats> {
        let (total_conversations, successful_commands) = if let Some(user) = user {
            let total: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM conversations WHERE user = ?1",
                params![user],
                |row| row.get(0),
            )?;

            let successful: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM conversations WHERE user = ?1 AND exit_code = 0",
                params![user],
                |row| row.get(0),
            )?;

            (total, successful)
        } else {
            let total: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM conversations",
                [],
                |row| row.get(0),
            )?;

            let successful: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM conversations WHERE exit_code = 0",
                [],
                |row| row.get(0),
            )?;

            (total, successful)
        };

        Ok(Stats {
            total_conversations,
            successful_commands,
        })
    }

    /// Clean old data
    pub fn cleanup_old_data(&self, days: i64) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - (days * 24 * 60 * 60);

        let deleted = self.conn.execute(
            "DELETE FROM conversations WHERE timestamp < ?1",
            params![cutoff],
        )?;

        info!("Cleaned up {} old conversations", deleted);
        Ok(deleted)
    }
}

/// Conversation record
#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: i64,
    pub user: String,
    pub message: String,
    pub response: Option<String>,
    pub timestamp: i64,
    pub command: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
}

/// Statistics
#[derive(Debug)]
pub struct Stats {
    pub total_conversations: i64,
    pub successful_commands: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let db = Database::new(&db_path).unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn test_save_conversation() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.db")).unwrap();

        let id = db
            .save_conversation("+1234567890", "test message", Some("response"), None, None, None)
            .unwrap();

        assert!(id > 0);
    }

    #[test]
    fn test_context() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.db")).unwrap();

        db.set_context("+1234567890", "test_key", "test_value")
            .unwrap();

        let value = db.get_context("+1234567890", "test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));
    }
}