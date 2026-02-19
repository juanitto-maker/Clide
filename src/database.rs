// ============================================
// database.rs - SQLite Memory DB (CORRECTED)
// ============================================

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: i64,
    pub user: String,
    pub message: String,
    pub response: Option<String>,
    pub command: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub timestamp: i64,
}

/// SQLite database wrapped in a Mutex so it is Send + Sync and can live
/// inside tokio::spawn futures.
pub struct Database {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub total_messages: usize,
    pub total_commands: usize,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user TEXT NOT NULL,
                message TEXT NOT NULL,
                response TEXT,
                command TEXT,
                exit_code INTEGER,
                duration_ms INTEGER,
                timestamp INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn save_conversation(
        &self,
        user: &str,
        message: &str,
        response: Option<&str>,
        command: Option<&str>,
        exit_code: Option<i32>,
        duration_ms: Option<u64>,
    ) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp();
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(
            r#"
            INSERT INTO conversations
                (user, message, response, command, exit_code, duration_ms, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                user,
                message,
                response,
                command,
                exit_code,
                duration_ms.map(|v| v as i64),
                timestamp
            ],
        )?;
        Ok(())
    }

    pub fn get_recent_conversations(&self, user: &str, count: usize) -> Result<Vec<Conversation>> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user, message, response, command, exit_code, duration_ms, timestamp
            FROM conversations
            WHERE user = ?1
            ORDER BY timestamp DESC
            LIMIT ?2
            "#,
        )?;
        let rows = stmt.query_map(params![user, count as i64], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                user: row.get(1)?,
                message: row.get(2)?,
                response: row.get(3)?,
                command: row.get(4)?,
                exit_code: row.get(5)?,
                duration_ms: row.get(6)?,
                timestamp: row.get(7)?,
            })
        })?;

        let mut conversations = Vec::new();
        for conv in rows {
            conversations.push(conv?);
        }
        Ok(conversations)
    }
}
