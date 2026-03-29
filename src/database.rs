// ============================================
// database.rs - SQLite Memory DB (Enhanced)
// ============================================
// Provides conversation storage, structured knowledge base,
// conversation summaries, and usage statistics.

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

/// A structured fact extracted from conversations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub id: i64,
    pub user: String,
    pub fact_type: String,
    pub key: String,
    pub value: String,
    pub confidence: f64,
    pub last_seen: i64,
}

/// A rolling summary of recent conversations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub id: i64,
    pub user: String,
    pub summary: String,
    pub message_count: i64,
    pub from_timestamp: i64,
    pub to_timestamp: i64,
    pub created_at: i64,
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
    pub total_facts: usize,
    pub total_summaries: usize,
    pub unique_users: usize,
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
            CREATE INDEX IF NOT EXISTS idx_knowledge_user_key ON knowledge(user, key);

            CREATE TABLE IF NOT EXISTS summaries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user TEXT NOT NULL,
                summary TEXT NOT NULL,
                message_count INTEGER NOT NULL DEFAULT 0,
                from_timestamp INTEGER NOT NULL,
                to_timestamp INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_summaries_user ON summaries(user);

            CREATE TABLE IF NOT EXISTS usage_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user TEXT NOT NULL,
                event_type TEXT NOT NULL,
                model TEXT,
                tokens_in INTEGER DEFAULT 0,
                tokens_out INTEGER DEFAULT 0,
                duration_ms INTEGER DEFAULT 0,
                timestamp INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_usage_user ON usage_stats(user);
            "#,
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    // ── Conversations ─────────────────────────────────────────────────────

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

    /// Get total conversation count for a user (used to trigger summarization).
    pub fn get_conversation_count(&self, user: &str) -> Result<usize> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM conversations WHERE user = ?1",
            params![user],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get conversations in a timestamp range (for summarization).
    pub fn get_conversations_in_range(
        &self,
        user: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> Result<Vec<Conversation>> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user, message, response, command, exit_code, duration_ms, timestamp
            FROM conversations
            WHERE user = ?1 AND timestamp >= ?2 AND timestamp <= ?3
            ORDER BY timestamp ASC
            "#,
        )?;
        let rows = stmt.query_map(params![user, from_ts, to_ts], |row| {
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
        let mut convs = Vec::new();
        for c in rows {
            convs.push(c?);
        }
        Ok(convs)
    }

    /// Get the count of unsummarized conversations for a user
    /// (conversations newer than the latest summary).
    pub fn get_unsummarized_count(&self, user: &str) -> Result<usize> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let latest_summary_ts: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(to_timestamp), 0) FROM summaries WHERE user = ?1",
                params![user],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM conversations WHERE user = ?1 AND timestamp > ?2",
            params![user, latest_summary_ts],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get conversations that haven't been summarized yet.
    pub fn get_unsummarized_conversations(
        &self,
        user: &str,
        limit: usize,
    ) -> Result<Vec<Conversation>> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let latest_summary_ts: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(to_timestamp), 0) FROM summaries WHERE user = ?1",
                params![user],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user, message, response, command, exit_code, duration_ms, timestamp
            FROM conversations
            WHERE user = ?1 AND timestamp > ?2
            ORDER BY timestamp ASC
            LIMIT ?3
            "#,
        )?;
        let rows = stmt.query_map(params![user, latest_summary_ts, limit as i64], |row| {
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
        let mut convs = Vec::new();
        for c in rows {
            convs.push(c?);
        }
        Ok(convs)
    }

    // ── Knowledge Base ────────────────────────────────────────────────────

    /// Save or update a structured fact. If a fact with the same (user, key)
    /// exists, update its value and last_seen timestamp.
    pub fn save_fact(
        &self,
        user: &str,
        fact_type: &str,
        key: &str,
        value: &str,
        confidence: f64,
    ) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp();
        let conn = self.conn.lock().expect("db mutex poisoned");
        // Upsert: update if same user+key exists, insert otherwise
        let updated = conn.execute(
            r#"
            UPDATE knowledge SET value = ?1, fact_type = ?2, confidence = ?3, last_seen = ?4
            WHERE user = ?5 AND key = ?6
            "#,
            params![value, fact_type, confidence, timestamp, user, key],
        )?;
        if updated == 0 {
            conn.execute(
                r#"
                INSERT INTO knowledge (user, fact_type, key, value, confidence, last_seen)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![user, fact_type, key, value, confidence, timestamp],
            )?;
        }
        Ok(())
    }

    /// Retrieve all facts for a user, ordered by last_seen (most recent first).
    pub fn get_facts(&self, user: &str, limit: usize) -> Result<Vec<Fact>> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user, fact_type, key, value, confidence, last_seen
            FROM knowledge
            WHERE user = ?1
            ORDER BY last_seen DESC
            LIMIT ?2
            "#,
        )?;
        let rows = stmt.query_map(params![user, limit as i64], |row| {
            Ok(Fact {
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

    /// Search facts by keyword (matches key or value).
    pub fn search_facts(&self, user: &str, query: &str, limit: usize) -> Result<Vec<Fact>> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user, fact_type, key, value, confidence, last_seen
            FROM knowledge
            WHERE user = ?1 AND (key LIKE ?2 OR value LIKE ?2)
            ORDER BY confidence DESC, last_seen DESC
            LIMIT ?3
            "#,
        )?;
        let rows = stmt.query_map(params![user, pattern, limit as i64], |row| {
            Ok(Fact {
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

    // ── Summaries ─────────────────────────────────────────────────────────

    /// Save a conversation summary.
    pub fn save_summary(
        &self,
        user: &str,
        summary: &str,
        message_count: usize,
        from_timestamp: i64,
        to_timestamp: i64,
    ) -> Result<()> {
        let created_at = chrono::Utc::now().timestamp();
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(
            r#"
            INSERT INTO summaries (user, summary, message_count, from_timestamp, to_timestamp, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![user, summary, message_count as i64, from_timestamp, to_timestamp, created_at],
        )?;
        Ok(())
    }

    /// Get the most recent summaries for a user.
    pub fn get_recent_summaries(&self, user: &str, limit: usize) -> Result<Vec<Summary>> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user, summary, message_count, from_timestamp, to_timestamp, created_at
            FROM summaries
            WHERE user = ?1
            ORDER BY created_at DESC
            LIMIT ?2
            "#,
        )?;
        let rows = stmt.query_map(params![user, limit as i64], |row| {
            Ok(Summary {
                id: row.get(0)?,
                user: row.get(1)?,
                summary: row.get(2)?,
                message_count: row.get(3)?,
                from_timestamp: row.get(4)?,
                to_timestamp: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        let mut summaries = Vec::new();
        for s in rows {
            summaries.push(s?);
        }
        Ok(summaries)
    }

    // ── Usage Stats ───────────────────────────────────────────────────────

    /// Record a usage event (task completion, model call, etc.).
    pub fn record_usage(
        &self,
        user: &str,
        event_type: &str,
        model: Option<&str>,
        tokens_in: i64,
        tokens_out: i64,
        duration_ms: i64,
    ) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp();
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(
            r#"
            INSERT INTO usage_stats (user, event_type, model, tokens_in, tokens_out, duration_ms, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![user, event_type, model, tokens_in, tokens_out, duration_ms, timestamp],
        )?;
        Ok(())
    }

    /// Get aggregate statistics.
    pub fn get_stats(&self) -> Result<Stats> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let total_messages: usize = conn
            .query_row("SELECT COUNT(*) FROM conversations", [], |row| row.get(0))
            .unwrap_or(0);
        let total_commands: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM conversations WHERE command IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let total_facts: usize = conn
            .query_row("SELECT COUNT(*) FROM knowledge", [], |row| row.get(0))
            .unwrap_or(0);
        let total_summaries: usize = conn
            .query_row("SELECT COUNT(*) FROM summaries", [], |row| row.get(0))
            .unwrap_or(0);
        let unique_users: usize = conn
            .query_row(
                "SELECT COUNT(DISTINCT user) FROM conversations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(Stats {
            total_messages,
            total_commands,
            total_facts,
            total_summaries,
            unique_users,
        })
    }

    // ── Maintenance ───────────────────────────────────────────────────────

    /// Archive (delete) conversations older than `days` days.
    /// Returns the number of rows deleted.
    pub fn archive_old_conversations(&self, days: i64) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - (days * 86400);
        let conn = self.conn.lock().expect("db mutex poisoned");
        let deleted = conn.execute(
            "DELETE FROM conversations WHERE timestamp < ?1",
            params![cutoff],
        )?;
        Ok(deleted)
    }

    /// Run VACUUM to reclaim disk space.
    pub fn vacuum(&self) -> Result<()> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute_batch("VACUUM")?;
        Ok(())
    }
}
