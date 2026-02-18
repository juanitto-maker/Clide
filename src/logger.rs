// ============================================
// logger.rs - Logging Configuration (CORRECTED)
// ============================================

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
pub fn init(config: LoggerConfig) -> Result<Option<WorkerGuard>> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    let mut guard = None;

    // SystemTime requires no extra feature flags (UtcTime requires "time" feature)
    let timer = fmt::time::SystemTime;

    if let Some(file_path) = config.file_path {
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file_appender = tracing_appender::rolling::daily(
            file_path.parent().unwrap(),
            file_path.file_name().unwrap().to_str().unwrap(),
        );
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        guard = Some(_guard);

        let file_layer = fmt::layer()
            .with_ansi(false)
            .with_timer(timer.clone())
            .with_writer(non_blocking)
            .with_target(config.with_caller);

        let console_layer = fmt::layer()
            .with_timer(timer)
            .with_target(config.with_caller);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .with(console_layer)
            .init();
    } else {
        let console_layer = fmt::layer()
            .with_timer(timer)
            .with_target(config.with_caller);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .init();
    }

    Ok(guard)
}

pub fn init_default() -> Result<()> {
    init(LoggerConfig::default())?;
    Ok(())
}

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
