// ============================================
// lib.rs - Library Root
// ============================================
// Module declarations and public API exports

pub mod bot;
pub mod config;
pub mod database;
pub mod executor;
pub mod gemini;
pub mod logger;
pub mod memory;
pub mod ssh;
pub mod skills;
pub mod workflow;

// Re-export commonly used types
pub use bot::Bot;
pub use config::Config;
pub use database::{Conversation, Database, Stats};
pub use executor::{ExecutionResult, Executor};
pub use gemini::{CommandAnalysis, GeminiClient};
pub use memory::{Memory, MemoryStats, UserPreferences};
pub use skills::{Skill, SkillManager, SkillResult};
pub use ssh::{SshClient, SshOutput};
pub use workflow::{Workflow, WorkflowExecutor, WorkflowResult};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Get version string
pub fn version() -> String {
    format!("{} v{}", NAME, VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let ver = version();
        assert!(ver.contains("clide"));
    }
}