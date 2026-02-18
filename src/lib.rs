// ============================================
// lib.rs - Library Root
// ============================================

pub mod bot;
pub mod config;
pub mod database;
pub mod executor;
pub mod gemini;
pub mod logger;
pub mod memory;
pub mod signal;
pub mod ssh;
pub mod skills;
pub mod workflow;

pub use bot::Bot;
pub use config::Config;
pub use database::{Conversation, Database, Stats};
pub use executor::{ExecutionResult, Executor};
pub use gemini::{CommandAnalysis, GeminiClient};
pub use memory::Memory;
pub use signal::SignalClient;
pub use ssh::{SshClient, SshOutput};
pub use skills::{Skill, SkillManager, SkillResult};
pub use workflow::{Workflow, WorkflowExecutor, WorkflowResult};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

pub fn version() -> String {
    format!("{} v{}", NAME, VERSION)
}
