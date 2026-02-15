// ============================================
// logger.rs - Logging Configuration
// ============================================
// Sets up structured logging with tracing
// Supports console and file output, JSON format

use anyhow::Result;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use std::path::PathBuf;

/// Logger configuration
pub struct LoggerConfig {
    pub level: String,
    pub file_path: Option<PathBuf>,
    pub json_format: bool,
    pub with_timestamps: bool,
    pub with_caller: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_path: None,
            json_format: false,
            with_timestamps: true,
            with_caller: false,
        }
    }
}

/// Initialize logging system
/// Returns WorkerGuard that must be kept alive for file logging
pub fn init(config: LoggerConfig) -> Result<Option<WorkerGuard>> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    let mut guard = None;

    // If file logging is enabled
    if let Some(file_path) = config.file_path {
        // Create parent directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file_appender = tracing_appender::rolling::daily(
            file_path.parent().unwrap_or_else(|| std::path::Path::new(".")),
            file_path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("clide.log")),
        );

        let (non_blocking, worker_guard) = tracing_appender::non_blocking(file_appender);
        guard = Some(worker_guard);

        // File layer (JSON format)
        let file_layer = if config.json_format {
            fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_timer(fmt::time::ChronoLocal::rfc3339())
                .boxed()
        } else {
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_timer(fmt::time::ChronoLocal::rfc3339())
                .boxed()
        };

        // Console layer (human-readable)
        let console_layer = fmt::layer()
            .pretty()
            .with_timer(fmt::time::ChronoLocal::rfc3339())
            .with_line_number(config.with_caller)
            .with_file(config.with_caller);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .with(console_layer)
            .init();
    } else {
        // Console only
        let console_layer = fmt::layer()
            .pretty()
            .with_timer(fmt::time::ChronoLocal::rfc3339())
            .with_line_number(config.with_caller)
            .with_file(config.with_caller);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .init();
    }

    Ok(guard)
}

/// Initialize with defaults (console only, info level)
pub fn init_default() -> Result<()> {
    init(LoggerConfig::default())?;
    Ok(())
}

/// Log levels helper
pub fn parse_level(level: &str) -> Level {
    match level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_level() {
        assert_eq!(parse_level("debug"), Level::DEBUG);
        assert_eq!(parse_level("INFO"), Level::INFO);
        assert_eq!(parse_level("error"), Level::ERROR);
        assert_eq!(parse_level("unknown"), Level::INFO);
    }

    #[test]
    fn test_default_config() {
        let config = LoggerConfig::default();
        assert_eq!(config.level, "info");
        assert!(!config.json_format);
        assert!(config.with_timestamps);
    }
}
