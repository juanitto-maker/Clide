// ============================================
// memory.rs - Tiered Conversation Memory
// ============================================
// Implements a three-tier memory system:
//   Hot:  Current conversation (full messages, in-memory)
//   Warm: Rolling summary of recent conversations (compressed, from DB)
//   Cold: Persistent knowledge facts (structured, from DB)

use anyhow::Result;
use std::collections::HashMap;
use tracing::debug;

use crate::database::Database;

/// How many conversations between automatic summarization passes.
const SUMMARIZE_EVERY: i64 = 5;

pub struct Memory {
    db: Database,
    /// In-memory key-value store per user (ephemeral session variables).
    context_cache: HashMap<String, HashMap<String, String>>,
    /// Tracks how many messages since last summarization per user.
    messages_since_summary: HashMap<String, i64>,
}

impl Memory {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            context_cache: HashMap::new(),
            messages_since_summary: HashMap::new(),
        }
    }

    /// Access the underlying database (for fact extraction, stats, etc.).
    pub fn db(&self) -> &Database {
        &self.db
    }

    pub async fn save_conversation(
        &self,
        user: &str,
        message: &str,
        response: &str,
        command: Option<&str>,
        exit_code: Option<i32>,
        duration_ms: Option<u64>,
    ) -> Result<()> {
        self.db.save_conversation(
            user,
            message,
            Some(response),
            command,
            exit_code,
            duration_ms,
        )?;
        // Also record a stat event
        let _ = self.db.record_stat("message", Some(user), None);
        debug!("Saved conversation for user: {}", user);
        Ok(())
    }

    /// Build a tiered context string for the agent's system prompt.
    ///
    /// Layers (injected in this order):
    /// 1. **Cold** — Persistent knowledge facts (always relevant)
    /// 2. **Warm** — Rolling conversation summary (compressed history)
    /// 3. **Hot**  — Recent full messages (immediate context)
    /// 4. **Session variables** — In-memory key-value pairs
    pub async fn get_context(&mut self, user: &str, message_count: usize) -> Result<String> {
        let mut context = String::with_capacity(4096);

        // ── Cold tier: knowledge facts ────────────────────────────────────
        let facts = self.db.get_facts(user).unwrap_or_default();
        if !facts.is_empty() {
            context.push_str("Known facts about this user:\n");
            for f in &facts {
                context.push_str(&format!(
                    "  [{}] {} = {} (confidence: {:.0}%)\n",
                    f.fact_type,
                    f.key,
                    f.value,
                    f.confidence * 100.0
                ));
            }
            context.push('\n');
        }

        // ── Warm tier: rolling summary ────────────────────────────────────
        if let Ok(Some(summary)) = self.db.get_latest_summary(user) {
            context.push_str("Conversation summary (older history):\n");
            context.push_str(&summary.summary);
            context.push_str("\n\n");
        }

        // ── Hot tier: recent messages ─────────────────────────────────────
        let history = self.db.get_recent_conversations(user, message_count)?;
        if !history.is_empty() {
            context.push_str("Recent messages:\n");
            for conv in history.into_iter().rev() {
                context.push_str(&format!("User: {}\n", conv.message));
                if let Some(resp) = conv.response {
                    // Truncate long responses in context to save tokens
                    let trimmed = if resp.len() > 500 {
                        format!("{}…", &resp[..500])
                    } else {
                        resp
                    };
                    context.push_str(&format!("Clide: {}\n", trimmed));
                }
            }
        }

        // ── Session variables ─────────────────────────────────────────────
        if let Some(user_cache) = self.context_cache.get(user) {
            if !user_cache.is_empty() {
                context.push_str("\nActive Variables:\n");
                for (k, v) in user_cache {
                    context.push_str(&format!("{}: {}\n", k, v));
                }
            }
        }

        Ok(context)
    }

    /// Store a knowledge fact persistently in the database.
    pub async fn store_fact(
        &self,
        user: &str,
        fact_type: &str,
        key: &str,
        value: &str,
        confidence: f64,
    ) -> Result<()> {
        self.db.upsert_fact(user, fact_type, key, value, confidence)?;
        debug!("Stored fact for {}: [{}] {} = {}", user, fact_type, key, value);
        Ok(())
    }

    /// Check if summarization is needed and return true if so.
    /// Call this after saving a conversation; the caller (agent) should then
    /// generate a summary via the LLM and call `save_summary`.
    pub fn needs_summarization(&mut self, user: &str) -> bool {
        let count = self.messages_since_summary
            .entry(user.to_string())
            .or_insert(0);
        *count += 1;
        if *count >= SUMMARIZE_EVERY {
            *count = 0;
            true
        } else {
            false
        }
    }

    /// Save a conversation summary to the warm tier.
    pub async fn save_summary(&self, user: &str, summary: &str, message_count: i64) -> Result<()> {
        self.db.save_summary(user, summary, message_count)?;
        debug!("Saved conversation summary for {} ({} messages)", user, message_count);
        Ok(())
    }

    /// Get conversation count for a user (used for summarization decisions).
    pub fn conversation_count(&self, user: &str) -> i64 {
        self.db.count_conversations(user).unwrap_or(0)
    }

    pub async fn set(&mut self, user: &str, key: &str, value: &str) -> Result<()> {
        self.context_cache
            .entry(user.to_string())
            .or_default()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub async fn get(&self, user: &str, key: &str) -> Result<Option<String>> {
        Ok(self.context_cache
            .get(user)
            .and_then(|m| m.get(key).cloned()))
    }
}
