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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeFact {
    pub id: i64,
    pub user: String,
    pub fact_type: String,
    pub key: String,
    pub value: String,
    pub confidence: f64,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: i64,
    pub user: String,
    pub summary: String,
    pub message_count: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub total_messages: usize,
    pub total_commands: usize,
    pub total_users: usize,
    pub total_facts: usize,
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

            CREATE TABLE IF NOT EXISTS knowledge (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user TEXT NOT NULL,
                fact_type TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.9,
                last_seen INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_knowledge_user ON knowledge(user);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_knowledge_unique ON knowledge(user, fact_type, key);

            CREATE TABLE IF NOT EXISTS summaries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user TEXT NOT NULL,
                summary TEXT NOT NULL,
                message_count INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_summaries_user ON summaries(user);

            CREATE TABLE IF NOT EXISTS stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                user TEXT,
                detail TEXT,
                timestamp INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_stats_type ON stats(event_type);
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

    // ── Knowledge facts ───────────────────────────────────────────────────

    /// Upsert a knowledge fact. If the (user, fact_type, key) already exists,
    /// update the value, confidence, and last_seen.
    pub fn upsert_fact(
        &self,
        user: &str,
        fact_type: &str,
        key: &str,
        value: &str,
        confidence: f64,
    ) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp();
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(
            r#"
            INSERT INTO knowledge (user, fact_type, key, value, confidence, last_seen)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(user, fact_type, key)
            DO UPDATE SET value = excluded.value,
                          confidence = excluded.confidence,
                          last_seen = excluded.last_seen
            "#,
            params![user, fact_type, key, value, confidence, timestamp],
        )?;
        Ok(())
    }

    /// Get all knowledge facts for a user.
    pub fn get_facts(&self, user: &str) -> Result<Vec<KnowledgeFact>> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, user, fact_type, key, value, confidence, last_seen \
             FROM knowledge WHERE user = ?1 ORDER BY last_seen DESC",
        )?;
        let rows = stmt.query_map(params![user], |row| {
            Ok(KnowledgeFact {
                id: row.get(0)?,
                user: row.get(1)?,
                fact_type: row.get(2)?,
                key: row.get(3)?,
                value: row.get(4)?,
                confidence: row.get(5)?,
                last_seen: row.get(6)?,
            })
        })?;
        let mut facts = Vec::new();
        for f in rows {
            facts.push(f?);
        }
        Ok(facts)
    }

    // ── Conversation summaries ────────────────────────────────────────────

    /// Save a rolling conversation summary.
    pub fn save_summary(&self, user: &str, summary: &str, message_count: i64) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp();
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(
            "INSERT INTO summaries (user, summary, message_count, created_at) \
             VALUES (?1, ?2, ?3, ?4)",
            params![user, summary, message_count, timestamp],
        )?;
        Ok(())
    }

    /// Get the most recent summary for a user.
    pub fn get_latest_summary(&self, user: &str) -> Result<Option<ConversationSummary>> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, user, summary, message_count, created_at \
             FROM summaries WHERE user = ?1 ORDER BY created_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![user], |row| {
            Ok(ConversationSummary {
                id: row.get(0)?,
                user: row.get(1)?,
                summary: row.get(2)?,
                message_count: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        match rows.next() {
            Some(Ok(s)) => Ok(Some(s)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    // ── Stats ─────────────────────────────────────────────────────────────

    /// Record a stats event (e.g., "message", "command", "error").
    pub fn record_stat(&self, event_type: &str, user: Option<&str>, detail: Option<&str>) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp();
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(
            "INSERT INTO stats (event_type, user, detail, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![event_type, user, detail, timestamp],
        )?;
        Ok(())
    }

    /// Get aggregate stats.
    pub fn get_stats(&self) -> Result<Stats> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let total_messages: usize = conn
            .query_row("SELECT COUNT(*) FROM conversations", [], |r| r.get(0))
            .unwrap_or(0);
        let total_commands: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM conversations WHERE command IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total_users: usize = conn
            .query_row(
                "SELECT COUNT(DISTINCT user) FROM conversations",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total_facts: usize = conn
            .query_row("SELECT COUNT(*) FROM knowledge", [], |r| r.get(0))
            .unwrap_or(0);
        Ok(Stats {
            total_messages,
            total_commands,
            total_users,
            total_facts,
        })
    }

    /// Count total conversations for a user (used to decide when to summarize).
    pub fn count_conversations(&self, user: &str) -> Result<i64> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM conversations WHERE user = ?1",
            params![user],
            |r| r.get(0),
        )?;
        Ok(count)
    }
}
