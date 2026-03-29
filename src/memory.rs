// ============================================
// memory.rs - Enhanced Conversation Memory
// ============================================
// Implements tiered memory retrieval:
//   Hot:  Current conversation (full messages, in-memory)
//   Warm: Rolling summary of recent conversations (from DB)
//   Cold: Structured knowledge base (facts extracted from all history)

use anyhow::Result;
use std::collections::HashMap;
use tracing::debug;

use crate::database::{Conversation, Database, Fact, Summary};

/// Maximum number of facts to inject into context.
const MAX_FACTS: usize = 30;
/// Maximum number of summaries to inject.
const MAX_SUMMARIES: usize = 3;

pub struct Memory {
    db: Database,
    context_cache: HashMap<String, HashMap<String, String>>,
}

impl Memory {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            context_cache: HashMap::new(),
        }
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
        debug!("Saved conversation for user: {}", user);
        Ok(())
    }

    /// Build a tiered context string for the given user.
    ///
    /// Layers:
    /// 1. **Cold** — Structured facts from the knowledge base (persistent memory)
    /// 2. **Warm** — Rolling summaries of older conversations
    /// 3. **Hot**  — Last N raw conversation messages
    /// 4. Active in-memory variables
    pub async fn get_context(&mut self, user: &str, message_count: usize) -> Result<String> {
        let mut context = String::with_capacity(4096);

        // ── Cold layer: Knowledge base facts ──────────────────────────────
        let facts = self.db.get_facts(user, MAX_FACTS)?;
        if !facts.is_empty() {
            context.push_str("Known facts about this user:\n");
            for fact in &facts {
                context.push_str(&format!(
                    "  [{}] {} = {}\n",
                    fact.fact_type, fact.key, fact.value
                ));
            }
            context.push('\n');
        }

        // ── Warm layer: Conversation summaries ────────────────────────────
        let summaries = self.db.get_recent_summaries(user, MAX_SUMMARIES)?;
        if !summaries.is_empty() {
            context.push_str("Summary of earlier conversations:\n");
            for summary in summaries.iter().rev() {
                context.push_str(&format!("{}\n", summary.summary));
            }
            context.push('\n');
        }

        // ── Hot layer: Recent raw messages ────────────────────────────────
        let history = self.db.get_recent_conversations(user, message_count)?;
        if !history.is_empty() {
            context.push_str("Recent conversation:\n");
            for conv in history.into_iter().rev() {
                context.push_str(&format!("User: {}\n", conv.message));
                if let Some(resp) = conv.response {
                    context.push_str(&format!("Clide: {}\n", resp));
                }
            }
        }

        // ── Active in-memory variables ────────────────────────────────────
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

    pub async fn set(&mut self, user: &str, key: &str, value: &str) -> Result<()> {
        self.context_cache
            .entry(user.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub async fn get(&self, user: &str, key: &str) -> Result<Option<String>> {
        Ok(self.context_cache
            .get(user)
            .and_then(|m| m.get(key).cloned()))
    }

    // ── Knowledge Base Operations ─────────────────────────────────────────

    /// Store a structured fact extracted from conversation.
    pub async fn save_fact(
        &self,
        user: &str,
        fact_type: &str,
        key: &str,
        value: &str,
        confidence: f64,
    ) -> Result<()> {
        self.db.save_fact(user, fact_type, key, value, confidence)?;
        debug!("Saved fact for {}: [{}] {} = {}", user, fact_type, key, value);
        Ok(())
    }

    /// Retrieve facts for a user.
    pub async fn get_facts(&self, user: &str, limit: usize) -> Result<Vec<Fact>> {
        self.db.get_facts(user, limit)
    }

    /// Search facts by keyword relevance.
    pub async fn search_facts(&self, user: &str, query: &str) -> Result<Vec<Fact>> {
        self.db.search_facts(user, query, MAX_FACTS)
    }

    // ── Summary Operations ────────────────────────────────────────────────

    /// Save a conversation summary.
    pub async fn save_summary(
        &self,
        user: &str,
        summary: &str,
        message_count: usize,
        from_timestamp: i64,
        to_timestamp: i64,
    ) -> Result<()> {
        self.db
            .save_summary(user, summary, message_count, from_timestamp, to_timestamp)?;
        debug!(
            "Saved summary for {} ({} messages, ts {}-{})",
            user, message_count, from_timestamp, to_timestamp
        );
        Ok(())
    }

    /// Get recent summaries for a user.
    pub async fn get_summaries(&self, user: &str, limit: usize) -> Result<Vec<Summary>> {
        self.db.get_recent_summaries(user, limit)
    }

    /// Check how many conversations haven't been summarized yet.
    pub async fn unsummarized_count(&self, user: &str) -> Result<usize> {
        self.db.get_unsummarized_count(user)
    }

    /// Get unsummarized conversations for summary generation.
    pub async fn get_unsummarized_conversations(
        &self,
        user: &str,
        limit: usize,
    ) -> Result<Vec<Conversation>> {
        self.db.get_unsummarized_conversations(user, limit)
    }

    // ── Usage Stats ───────────────────────────────────────────────────────

    /// Record a usage event.
    pub async fn record_usage(
        &self,
        user: &str,
        event_type: &str,
        model: Option<&str>,
        tokens_in: i64,
        tokens_out: i64,
        duration_ms: i64,
    ) -> Result<()> {
        self.db
            .record_usage(user, event_type, model, tokens_in, tokens_out, duration_ms)
    }

    // ── Maintenance ───────────────────────────────────────────────────────

    /// Archive old conversations (older than N days).
    pub async fn archive_old(&self, days: i64) -> Result<usize> {
        self.db.archive_old_conversations(days)
    }

    /// Reclaim disk space.
    pub async fn vacuum(&self) -> Result<()> {
        self.db.vacuum()
    }

    /// Get aggregate stats.
    pub async fn get_stats(&self) -> Result<crate::database::Stats> {
        self.db.get_stats()
    }
}
