// ============================================
// memory.rs - Conversation Memory
// ============================================
// Manages conversation context and learning

use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::database::{Conversation, Database};

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
        let conversations = self.db.get_conversations(user, message_count)?;

        if conversations.is_empty() {
            return Ok(String::new());
        }

        // Build context from recent conversations
        let mut context = String::from("Recent conversation history:\n");

        for conv in conversations.iter().rev() {
            context.push_str(&format!("User: {}\n", conv.message));
            if let Some(ref response) = conv.response {
                context.push_str(&format!("Assistant: {}\n", response));
            }
            context.push('\n');
        }

        Ok(context)
    }

    /// Set a context value (persistent)
    pub async fn set(&mut self, user: &str, key: &str, value: &str) -> Result<()> {
        // Update cache
        self.context_cache
            .entry(user.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());

        // Persist to database
        self.db.set_context(user, key, value)?;

        debug!("Set context for {}: {} = {}", user, key, value);
        Ok(())
    }

    /// Get a context value
    pub async fn get(&mut self, user: &str, key: &str) -> Result<Option<String>> {
        // Check cache first
        if let Some(user_context) = self.context_cache.get(user) {
            if let Some(value) = user_context.get(key) {
                return Ok(Some(value.clone()));
            }
        }

        // Load from database
        if let Some(value) = self.db.get_context(user, key)? {
            // Update cache
            self.context_cache
                .entry(user.to_string())
                .or_insert_with(HashMap::new)
                .insert(key.to_string(), value.clone());

            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Get all context for user
    pub async fn get_all(&mut self, user: &str) -> Result<HashMap<String, String>> {
        let contexts = self.db.get_all_context(user)?;
        let map: HashMap<String, String> = contexts.into_iter().collect();

        // Update cache
        if !map.is_empty() {
            self.context_cache
                .insert(user.to_string(), map.clone());
        }

        Ok(map)
    }

    /// Learn from successful command
    pub async fn learn_pattern(&mut self, user: &str, intent: &str, command: &str) -> Result<()> {
        let key = format!("pattern_{}", intent.replace(' ', "_"));
        self.set(user, &key, command).await?;

        info!("Learned pattern for {}: {} -> {}", user, intent, command);
        Ok(())
    }

    /// Get learned pattern
    pub async fn get_pattern(&mut self, user: &str, intent: &str) -> Result<Option<String>> {
        let key = format!("pattern_{}", intent.replace(' ', "_"));
        self.get(user, &key).await
    }

    /// Get user preferences
    pub async fn get_preferences(&mut self, user: &str) -> Result<UserPreferences> {
        let mut prefs = UserPreferences::default();

        if let Some(timezone) = self.get(user, "timezone").await? {
            prefs.timezone = timezone;
        }

        if let Some(shell) = self.get(user, "preferred_shell").await? {
            prefs.preferred_shell = Some(shell);
        }

        if let Some(lang) = self.get(user, "language").await? {
            prefs.language = lang;
        }

        Ok(prefs)
    }

    /// Set user preferences
    pub async fn set_preferences(&mut self, user: &str, prefs: &UserPreferences) -> Result<()> {
        self.set(user, "timezone", &prefs.timezone).await?;

        if let Some(ref shell) = prefs.preferred_shell {
            self.set(user, "preferred_shell", shell).await?;
        }

        self.set(user, "language", &prefs.language).await?;

        Ok(())
    }

    /// Build enhanced context for AI with history + preferences
    pub async fn build_ai_context(&mut self, user: &str) -> Result<String> {
        let mut context = String::new();

        // Add user preferences
        let prefs = self.get_preferences(user).await?;
        context.push_str(&format!(
            "User preferences:\n- Timezone: {}\n- Language: {}\n",
            prefs.timezone, prefs.language
        ));

        if let Some(shell) = prefs.preferred_shell {
            context.push_str(&format!("- Preferred shell: {}\n", shell));
        }

        context.push('\n');

        // Add recent conversation history
        let history = self.get_context(user, 5).await?;
        if !history.is_empty() {
            context.push_str(&history);
        }

        // Add learned patterns
        let all_context = self.get_all(user).await?;
        let patterns: Vec<_> = all_context
            .iter()
            .filter(|(k, _)| k.starts_with("pattern_"))
            .collect();

        if !patterns.is_empty() {
            context.push_str("Learned patterns:\n");
            for (key, value) in patterns {
                let intent = key.strip_prefix("pattern_").unwrap_or(key);
                context.push_str(&format!("- {}: {}\n", intent, value));
            }
        }

        Ok(context)
    }

    /// Get statistics
    pub async fn get_stats(&self, user: Option<&str>) -> Result<MemoryStats> {
        let db_stats = self.db.get_stats(user)?;

        let cached_users = self.context_cache.len();
        let cached_items: usize = self.context_cache.values().map(|m| m.len()).sum();

        Ok(MemoryStats {
            total_conversations: db_stats.total_conversations,
            successful_commands: db_stats.successful_commands,
            cached_users,
            cached_items,
        })
    }

    /// Cleanup old data
    pub async fn cleanup(&self, days: i64) -> Result<usize> {
        self.db.cleanup_old_data(days)
    }
}

/// User preferences
#[derive(Debug, Clone)]
pub struct UserPreferences {
    pub timezone: String,
    pub preferred_shell: Option<String>,
    pub language: String,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            timezone: "UTC".to_string(),
            preferred_shell: None,
            language: "en".to_string(),
        }
    }
}

/// Memory statistics
#[derive(Debug)]
pub struct MemoryStats {
    pub total_conversations: i64,
    pub successful_commands: i64,
    pub cached_users: usize,
    pub cached_items: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_memory_save_and_retrieve() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.db")).unwrap();
        let mut memory = Memory::new(db);

        memory
            .save_conversation("+1234567890", "test", "response", None, None, None)
            .await
            .unwrap();

        let context = memory.get_context("+1234567890", 10).await.unwrap();
        assert!(context.contains("test"));
        assert!(context.contains("response"));
    }

    #[tokio::test]
    async fn test_context_cache() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.db")).unwrap();
        let mut memory = Memory::new(db);

        memory
            .set("+1234567890", "test_key", "test_value")
            .await
            .unwrap();

        let value = memory.get("+1234567890", "test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_learn_pattern() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.db")).unwrap();
        let mut memory = Memory::new(db);

        memory
            .learn_pattern("+1234567890", "check disk", "df -h")
            .await
            .unwrap();

        let pattern = memory
            .get_pattern("+1234567890", "check disk")
            .await
            .unwrap();

        assert_eq!(pattern, Some("df -h".to_string()));
    }
}