// ============================================
// database.rs - Database Abstraction (CORRECTED)
// ============================================

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension}; // Added OptionalExtension here
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

        conn.execute("PRAGMA foreign_keys = ON", [])?;

        let db = Self { conn };
        db.init_schema()?;

        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        debug!("Initializing database schema");

        // Conversations table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY,
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

        // Learning table (for patterns)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS learning (
                id INTEGER PRIMARY KEY,
                user TEXT NOT NULL,
                intent TEXT NOT NULL,
                command TEXT NOT NULL,
                success_count INTEGER DEFAULT 0,
                last_used INTEGER
            )",
            [],
        )?;

        Ok(())
    }

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
        
        self.conn.execute(
            "INSERT INTO conversations (user, message, response, timestamp, command, exit_code, duration_ms)
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

        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_recent_conversations(&self, user: &str, limit: usize) -> Result<Vec<Conversation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user, message, response, timestamp, command, exit_code, duration_ms 
             FROM conversations WHERE user = ?1 ORDER BY timestamp DESC LIMIT ?2"
        )?;

        let rows = stmt.query_map(params![user, limit], |row| {
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
        })?;

        let mut conversations = Vec::new();
        for row in rows {
            conversations.push(row?);
        }
        Ok(conversations)
    }

    pub fn save_pattern(&self, user: &str, intent: &str, command: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO learning (user, intent, command, last_used) 
             VALUES (?1, ?2, ?3, ?4)",
            params![user, intent, command, chrono::Utc::now().timestamp()],
        )?;
        Ok(())
    }

    pub fn get_pattern(&self, user: &str, intent: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT command FROM learning WHERE user = ?1 AND intent = ?2"
        )?;
        
        // This is where OptionalExtension is used
        let command: Option<String> = stmt.query_row(params![user, intent], |row| row.get(0)).optional()?;
        Ok(command)
    }
}

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

pub struct Stats {
    pub total_conversations: i64,
    pub successful_commands: i64,
}
