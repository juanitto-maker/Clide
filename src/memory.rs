// ============================================
// memory.rs - Conversation Memory (CORRECTED)
// ============================================

use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::database::Database; // Removed unused Conversation import

/// Memory manager for maintaining conversation context
pub struct Memory {
    db: Database,
    context_cache: HashMap<String, HashMap<String, String>>,
}

impl Memory {
    /// Create new memory manager
    pub fn new(db: Database) -> Self {
        Self {
            db,
            context_cache: HashMap::new(),
        }
    }

    /// Save a conversation turn
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

    /// Get conversation history for context
    pub async fn get_context(&mut self, user: &str, message_count: usize) -> Result<String> {
        let history = self.db.get_recent_conversations(user, message_count)?;
        
        let mut context = String::new();
        for conv in history.into_iter().rev() {
            context.push_str(&format!("User: {}\n", conv.message));
            if let Some(resp) = conv.response {
                context.push_str(&format!("Clide: {}\n", resp));
            }
        }
        
        // Add ephemeral cache context
        if let Some(user_cache) = self.context_cache.get(user) {
            context.push_str("\nActive Variables:\n");
            for (k, v) in user_cache {
                context.push_str(&format!("{}: {}\n", k, v));
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
}
